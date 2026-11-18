#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

const ESCROW_AMOUNT: i128 = 1_000;
const COLLATERAL: i128 = 100;

struct Setup<'a> {
    env: Env,
    client: EscrowContractClient<'a>,
    contract_id: Address,
    admin: Address,
    customer: Address,
    merchant: Address,
    token: Address,
    token_client: token::Client<'a>,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_client = token::Client::new(&env, &token);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    token_admin.mint(&customer, &(ESCROW_AMOUNT + COLLATERAL));
    token_admin.mint(&merchant, &COLLATERAL);

    Setup {
        env,
        client,
        contract_id,
        admin,
        customer,
        merchant,
        token,
        token_client,
    }
}

fn enable_collateral(s: &Setup) {
    s.client.set_dispute_config(
        &s.admin,
        &DisputeConfig {
            collateral_token: s.token.clone(),
            collateral_amount: COLLATERAL,
            collateral_enabled: true,
            min_collateral_ratio_bps: 1_000, // 10% of ESCROW_AMOUNT
            dispute_fee_bps: 0,
            dispute_cooldown_seconds: 0,
        },
    );
}

fn create_escrow(s: &Setup) -> u64 {
    s.client.create_escrow(
        &s.customer,
        &s.merchant,
        &ESCROW_AMOUNT,
        &s.token,
        &0_u64,
        &0_u64,
        &0_u64,
        &false,
    )
}

#[test]
fn test_disputing_party_can_withdraw_and_escrow_returns_to_locked() {
    let s = setup();
    let escrow_id = create_escrow(&s);

    s.client.dispute_escrow(&s.customer, &escrow_id);
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Disputed
    );

    s.client.withdraw_dispute(&s.customer, &escrow_id);

    let escrow = s.client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.evidence_deadline, None);
    assert_eq!(escrow.escalated_at, None);

    // The escrow is back on the normal path and can be released.
    s.client.release_escrow(&s.admin, &escrow_id, &false);
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Released
    );
    assert_eq!(
        s.token_client.balance(&s.merchant),
        ESCROW_AMOUNT + COLLATERAL
    );
}

#[test]
fn test_withdraw_returns_collateral_to_disputing_party() {
    let s = setup();
    enable_collateral(&s);
    let escrow_id = create_escrow(&s);

    s.client.dispute_escrow(&s.customer, &escrow_id);
    assert_eq!(s.token_client.balance(&s.customer), 0);
    assert_eq!(
        s.token_client.balance(&s.contract_id),
        ESCROW_AMOUNT + COLLATERAL
    );

    s.client.withdraw_dispute(&s.customer, &escrow_id);

    assert_eq!(s.token_client.balance(&s.customer), COLLATERAL);
    assert_eq!(s.token_client.balance(&s.contract_id), ESCROW_AMOUNT);
    assert_eq!(
        s.client.try_get_dispute_collateral(&escrow_id).err(),
        Some(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
}

#[test]
fn test_merchant_opened_dispute_returns_collateral_to_merchant() {
    let s = setup();
    enable_collateral(&s);
    let escrow_id = create_escrow(&s);

    s.client.dispute_escrow(&s.merchant, &escrow_id);
    assert_eq!(s.token_client.balance(&s.merchant), 0);

    s.client.withdraw_dispute(&s.merchant, &escrow_id);

    assert_eq!(s.token_client.balance(&s.merchant), COLLATERAL);
    assert_eq!(s.token_client.balance(&s.contract_id), ESCROW_AMOUNT);
    assert_eq!(s.client.get_escrow(&escrow_id).status, EscrowStatus::Locked);
}

#[test]
fn test_counterparty_cannot_withdraw_dispute() {
    let s = setup();
    enable_collateral(&s);
    let escrow_id = create_escrow(&s);
    s.client.dispute_escrow(&s.customer, &escrow_id);

    assert_eq!(
        s.client.try_withdraw_dispute(&s.merchant, &escrow_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );

    let escrow = s.client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
    assert_eq!(
        s.token_client.balance(&s.contract_id),
        ESCROW_AMOUNT + COLLATERAL
    );
}

#[test]
fn test_outsider_and_admin_cannot_withdraw_dispute() {
    let s = setup();
    let escrow_id = create_escrow(&s);
    s.client.dispute_escrow(&s.customer, &escrow_id);

    let outsider = Address::generate(&s.env);
    assert_eq!(
        s.client.try_withdraw_dispute(&outsider, &escrow_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        s.client.try_withdraw_dispute(&s.admin, &escrow_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Disputed
    );
}

#[test]
fn test_resolved_dispute_cannot_be_withdrawn() {
    let s = setup();
    enable_collateral(&s);
    let escrow_id = create_escrow(&s);
    s.client.dispute_escrow(&s.customer, &escrow_id);

    s.client.resolve_dispute(&s.admin, &escrow_id, &true);
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Released
    );

    assert_eq!(
        s.client.try_withdraw_dispute(&s.customer, &escrow_id),
        Err(Ok(Error::Action(ActionError::NotDisputed)))
    );
}

#[test]
fn test_undisputed_or_missing_escrow_cannot_be_withdrawn() {
    let s = setup();
    let escrow_id = create_escrow(&s);

    assert_eq!(
        s.client.try_withdraw_dispute(&s.customer, &escrow_id),
        Err(Ok(Error::Action(ActionError::NotDisputed)))
    );
    assert_eq!(
        s.client.try_withdraw_dispute(&s.customer, &999_u64),
        Err(Ok(Error::Escrow(EscrowError::NotFound)))
    );
}

#[test]
fn test_withdraw_twice_fails() {
    let s = setup();
    let escrow_id = create_escrow(&s);
    s.client.dispute_escrow(&s.customer, &escrow_id);
    s.client.withdraw_dispute(&s.customer, &escrow_id);

    assert_eq!(
        s.client.try_withdraw_dispute(&s.customer, &escrow_id),
        Err(Ok(Error::Action(ActionError::NotDisputed)))
    );
}

#[test]
fn test_new_dispute_after_withdrawal_belongs_to_new_opener() {
    let s = setup();
    let escrow_id = create_escrow(&s);

    s.client.dispute_escrow(&s.customer, &escrow_id);
    s.client.withdraw_dispute(&s.customer, &escrow_id);

    // The merchant re-opens; the customer no longer owns the dispute.
    s.client.dispute_escrow(&s.merchant, &escrow_id);
    assert_eq!(
        s.client.try_withdraw_dispute(&s.customer, &escrow_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    s.client.withdraw_dispute(&s.merchant, &escrow_id);
    assert_eq!(s.client.get_escrow(&escrow_id).status, EscrowStatus::Locked);
}

#[test]
fn test_withdraw_dequeues_pending_escalation() {
    let s = setup();
    let escrow_id = create_escrow(&s);
    s.client.dispute_escrow(&s.customer, &escrow_id);
    s.client.escalate_dispute(&s.customer, &escrow_id);

    s.client.withdraw_dispute(&s.customer, &escrow_id);

    let timeout = s.client.get_escrow(&escrow_id).escalation_timeout;
    s.env
        .ledger()
        .set_timestamp(1_000 + timeout.saturating_add(1));

    assert!(!s.client.check_escalation_timeout(&escrow_id));
    assert_eq!(s.client.process_escalation_timeouts(&10_u32), 0);
    assert_eq!(s.client.get_escrow(&escrow_id).status, EscrowStatus::Locked);
    assert_eq!(s.token_client.balance(&s.contract_id), ESCROW_AMOUNT);
}
