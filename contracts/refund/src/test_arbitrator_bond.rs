#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String,
};

struct Setup {
    env: Env,
    client: RefundContractClient<'static>,
    contract_id: Address,
    admin: Address,
    merchant: Address,
    customer: Address,
    arbs: [Address; 3],
    token: token::Client<'static>,
}

fn setup() -> Setup {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let arbs = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token = token::Client::new(&env, &sac.address());
    let token_admin = token::StellarAssetClient::new(&env, &sac.address());
    token_admin.mint(&merchant, &100_000);
    for a in arbs.iter() {
        token_admin.mint(a, &10_000);
    }
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);
    for a in arbs.iter() {
        client.register_arbitrator(&admin, a);
    }
    Setup {
        env,
        client,
        contract_id,
        admin,
        merchant,
        customer,
        arbs,
        token,
    }
}

fn open_case(s: &Setup) -> u64 {
    let refund_id = s.client.request_refund(
        &s.merchant,
        &1u64,
        &s.customer,
        &1000i128,
        &10000i128,
        &s.token.address,
        &String::from_str(&s.env, "reason"),
        &RefundReasonCode::Other,
        &1000u64,
    );
    s.client
        .reject_refund(&s.admin, &refund_id, &String::from_str(&s.env, "no"));
    s.token.transfer(&s.merchant, &s.contract_id, &300);
    s.client
        .escalate_to_arbitration(&s.merchant, &refund_id, &s.token.address, &300)
}

fn bond_all(s: &Setup) {
    s.client
        .set_arbitrator_bond(&s.admin, &s.token.address, &1000, &2000);
    for a in s.arbs.iter() {
        s.client.post_arbitrator_bond(a);
    }
}

#[test]
fn test_post_bond_transfers_and_records() {
    let s = setup();
    s.client
        .set_arbitrator_bond(&s.admin, &s.token.address, &1000, &2000);
    assert_eq!(s.client.post_arbitrator_bond(&s.arbs[0]), 1000);
    assert_eq!(s.token.balance(&s.arbs[0]), 9_000);
    let bond = s.client.get_arbitrator_bond(&s.arbs[0]).unwrap();
    assert_eq!(bond.amount, 1000);
    assert_eq!(
        s.client.try_post_arbitrator_bond(&s.arbs[0]),
        Err(Ok(Error::Core(CoreError::ArbitratorBondAlreadyPosted)))
    );
}

#[test]
fn test_post_bond_requires_config() {
    let s = setup();
    assert_eq!(
        s.client.try_post_arbitrator_bond(&s.arbs[0]),
        Err(Ok(Error::Core(CoreError::ArbitratorBondNotConfigured)))
    );
}

#[test]
fn test_unbonded_arbitrators_not_assignable() {
    let s = setup();
    s.client
        .set_arbitrator_bond(&s.admin, &s.token.address, &1000, &2000);
    s.client.post_arbitrator_bond(&s.arbs[0]);
    let available = s.client.get_available_arbitrators();
    assert_eq!(available.len(), 1);
    assert_eq!(available.get(0).unwrap(), s.arbs[0]);
}

#[test]
fn test_timeout_slashes_non_voters_only() {
    let s = setup();
    bond_all(&s);
    s.client.set_arbitration_timeout(&s.admin, &100u64);
    let case_id = open_case(&s);
    let hash = BytesN::from_array(&s.env, &[1u8; 32]);
    s.client
        .cast_arbitration_vote(&s.arbs[0], &case_id, &true, &hash);
    s.env.ledger().with_mut(|l| l.timestamp += 200);
    s.client.trigger_arbitration_timeout(&case_id);

    assert_eq!(
        s.client.get_arbitrator_bond(&s.arbs[0]).unwrap().amount,
        1000
    );
    assert_eq!(
        s.client.get_arbitrator_bond(&s.arbs[1]).unwrap().amount,
        800
    );
    assert_eq!(
        s.client.get_arbitrator_bond(&s.arbs[2]).unwrap().amount,
        800
    );
    assert_eq!(s.client.get_slashed_bond_balance(&s.token.address), 400);

    let treasury = Address::generate(&s.env);
    assert_eq!(
        s.client
            .withdraw_slashed_bonds(&s.admin, &s.token.address, &treasury),
        400
    );
    assert_eq!(s.token.balance(&treasury), 400);
    // Slashed arbitrators are below the requirement until they top up.
    assert_eq!(s.client.get_available_arbitrators().len(), 1);
    assert_eq!(s.client.post_arbitrator_bond(&s.arbs[1]), 1000);
    assert_eq!(s.token.balance(&s.arbs[1]), 8_800);
}

#[test]
fn test_withdraw_blocked_with_open_case() {
    let s = setup();
    bond_all(&s);
    open_case(&s);
    assert_eq!(
        s.client.try_withdraw_arbitrator_bond(&s.arbs[0]),
        Err(Ok(Error::Core(CoreError::ArbitratorHasOpenCases)))
    );
}

#[test]
fn test_withdraw_after_case_closed() {
    let s = setup();
    bond_all(&s);
    s.client.set_arbitration_timeout(&s.admin, &100u64);
    let case_id = open_case(&s);
    s.env.ledger().with_mut(|l| l.timestamp += 200);
    s.client.trigger_arbitration_timeout(&case_id);
    assert_eq!(s.client.withdraw_arbitrator_bond(&s.arbs[0]), 800);
    assert_eq!(s.token.balance(&s.arbs[0]), 9_800);
    assert!(s.client.get_arbitrator_bond(&s.arbs[0]).is_none());
    assert_eq!(
        s.client.try_withdraw_arbitrator_bond(&s.arbs[0]),
        Err(Ok(Error::Core(CoreError::ArbitratorNotBonded)))
    );
}

#[test]
fn test_set_bond_rejects_invalid_slash_bps() {
    let s = setup();
    assert_eq!(
        s.client
            .try_set_arbitrator_bond(&s.admin, &s.token.address, &1000, &10_001),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
}
