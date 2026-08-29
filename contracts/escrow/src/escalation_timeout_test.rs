#![cfg(test)]

use crate::*;
use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env};
use crate::*;

fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address, Address) {
    env.mock_all_auths();
    let id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let customer = Address::generate(env);
    token_admin.mint(&customer, &10_000i128);
    token_admin.mint(&id, &10_000i128);

    (client, admin, customer, Address::generate(env), token_addr)
}

fn make_disputed_escrow(
    env: &Env,
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> u64 {
    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        customer, merchant, &1000i128, token,
        &9000u64, &0u64, &0u64, &false,
    );
    client.dispute_escrow(customer, &escrow_id);
    escrow_id
}

#[test]
fn test_trigger_timeout_fails_if_not_escalated() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(100_000);
    // Never called escalate_dispute, so escalated_at is None → InvalidStatus
    let result = client.try_trigger_timeout_resolution(&escrow_id);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::InvalidStatus))));
}

#[test]
fn test_trigger_timeout_fails_before_deadline() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_escalation_config(&admin, &300u64, &AutoResolveFavor::Customer);

    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(1500);
    client.escalate_dispute(&customer, &escrow_id);

    // Only 100s elapsed, timeout=300
    env.ledger().set_timestamp(1600);
    let result = client.try_trigger_timeout_resolution(&escrow_id);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::TimeoutNotReached))));
}

#[test]
fn test_trigger_timeout_resolves_in_favor_of_customer() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_escalation_config(&admin, &300u64, &AutoResolveFavor::Customer);

    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(1500);
    client.escalate_dispute(&customer, &escrow_id);

    env.ledger().set_timestamp(1500 + 301);
    client.trigger_timeout_resolution(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_trigger_timeout_split_equal() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_escalation_config(&admin, &300u64, &AutoResolveFavor::SplitEqual);

    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let token_client = token::Client::new(&env, &token);
    let customer_before = token_client.balance(&customer);
    let merchant_before = token_client.balance(&merchant);

    env.ledger().set_timestamp(1500);
    client.escalate_dispute(&customer, &escrow_id);
    env.ledger().set_timestamp(1500 + 301);
    client.trigger_timeout_resolution(&escrow_id);

    let customer_after = token_client.balance(&customer);
    let merchant_after = token_client.balance(&merchant);

    // escrow amount=1000 split evenly
    assert_eq!(customer_after - customer_before, 500i128);
    assert_eq!(merchant_after - merchant_before, 500i128);
}

#[test]
fn test_check_escalation_timeout_false_before_escalation() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(999_999);
    assert!(!client.check_escalation_timeout(&escrow_id));
}

#[test]
fn test_process_escalation_timeouts_uses_queue() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_escalation_config(&admin, &300u64, &AutoResolveFavor::Customer);

    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(1500);
    client.escalate_dispute(&customer, &escrow_id);
    env.ledger().set_timestamp(1500 + 301);

    let processed = client.process_escalation_timeouts(&10_u32);
    assert_eq!(processed, 1);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_process_escalation_timeouts_skips_future_deadlines() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_escalation_config(&admin, &300u64, &AutoResolveFavor::Customer);

    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(2000);
    client.escalate_dispute(&customer, &escrow_id);
    env.ledger().set_timestamp(2200);

    let processed = client.process_escalation_timeouts(&10_u32);
    assert_eq!(processed, 0);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}
