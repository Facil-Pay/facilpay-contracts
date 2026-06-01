#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String,
};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = contract.address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

/// Returns (env, client, contract_id, admin, merchant, customer, arb1, arb2, arb3, token_client)
fn setup() -> (
    Env,
    RefundContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let arb1 = Address::generate(&env);
    let arb2 = Address::generate(&env);
    let arb3 = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&merchant, &100_000);
    token_admin.mint(&customer, &100_000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arb1);
    client.register_arbitrator(&admin, &arb2);
    client.register_arbitrator(&admin, &arb3);

    (env, client, contract_id, admin, merchant, customer, arb1, arb2, arb3, token_client)
}

fn create_open_case(
    env: &Env,
    client: &RefundContractClient,
    contract_id: &Address,
    admin: &Address,
    merchant: &Address,
    customer: &Address,
    token_client: &token::Client,
) -> (u64, u64) {
    let refund_id = client.request_refund(
        merchant,
        &1u64,
        customer,
        &1000i128,
        &10000i128,
        &token_client.address,
        &String::from_str(env, "reason"),
        &RefundReasonCode::Other,
        &1000u64,
    );
    client.reject_refund(admin, &refund_id, &String::from_str(env, "rejected"));
    let fee = 300i128;
    token_client.transfer(merchant, contract_id, &fee);
    let case_id = client.escalate_to_arbitration(merchant, &refund_id, &token_client.address, &fee);
    (refund_id, case_id)
}

// ── Test 1: trigger fires after timeout_at ────────────────────────────────────

#[test]
fn test_trigger_arbitration_timeout_success() {
    let (env, client, contract_id, admin, merchant, customer, _arb1, _arb2, _arb3, token_client) =
        setup();

    // Set a short timeout of 100 seconds
    client.set_arbitration_timeout(&admin, &100u64);

    let (refund_id, case_id) =
        create_open_case(&env, &client, &contract_id, &admin, &merchant, &customer, &token_client);

    // Advance time past timeout
    env.ledger().with_mut(|l| l.timestamp += 200);

    client.trigger_arbitration_timeout(&case_id);

    let case = client.get_arbitration_case(&case_id);
    assert_eq!(case.status, ArbitrationStatus::Decided);

    // default_favor_customer=true → refund should be Approved
    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

// ── Test 2: trigger rejected before timeout_at ────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #19)")]
fn test_trigger_arbitration_timeout_too_early() {
    let (env, client, contract_id, admin, merchant, customer, _arb1, _arb2, _arb3, token_client) =
        setup();

    // Set a long timeout of 1000 seconds
    client.set_arbitration_timeout(&admin, &1000u64);

    let (_refund_id, case_id) =
        create_open_case(&env, &client, &contract_id, &admin, &merchant, &customer, &token_client);

    // Do NOT advance time — still before timeout_at
    let _ = env.ledger().timestamp(); // ensure time hasn't advanced
    client.trigger_arbitration_timeout(&case_id);
}

// ── Test 3: trigger blocked when quorum already reached ───────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_trigger_arbitration_timeout_blocked_by_quorum() {
    let (env, client, contract_id, admin, merchant, customer, arb1, arb2, arb3, token_client) =
        setup();

    client.set_arbitration_timeout(&admin, &100u64);

    let (_refund_id, case_id) =
        create_open_case(&env, &client, &contract_id, &admin, &merchant, &customer, &token_client);

    // Cast 3 votes to reach quorum
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arb1, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arb2, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arb3, &case_id, &true, &hash);

    // Advance time past timeout
    env.ledger().with_mut(|l| l.timestamp += 200);

    // Should be blocked because quorum (3 votes) is already reached
    client.trigger_arbitration_timeout(&case_id);
}
