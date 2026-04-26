#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn setup() -> (Env, EscrowContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    (env, client)
}

// ── is_escrow_released ────────────────────────────────────────────────────────

#[test]
fn test_is_escrow_released_false_on_locked_escrow() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    assert!(!client.is_escrow_released(&escrow_id));
}

#[test]
fn test_is_escrow_released_true_after_release() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);
    client.release_escrow(&admin, &escrow_id, &true);

    assert!(client.is_escrow_released(&escrow_id));
}

#[test]
fn test_is_escrow_released_false_for_nonexistent_id() {
    let (_env, client) = setup();
    assert!(!client.is_escrow_released(&9999_u64));
}

// ── is_escrow_disputed ────────────────────────────────────────────────────────

#[test]
fn test_is_escrow_disputed_false_on_locked_escrow() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    assert!(!client.is_escrow_disputed(&escrow_id));
}

#[test]
fn test_is_escrow_disputed_true_after_dispute() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    assert!(client.is_escrow_disputed(&escrow_id));
}

#[test]
fn test_is_escrow_disputed_false_for_nonexistent_id() {
    let (_env, client) = setup();
    assert!(!client.is_escrow_disputed(&9999_u64));
}

// ── get_escrow_status ─────────────────────────────────────────────────────────

#[test]
fn test_get_escrow_status_locked() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    assert_eq!(client.get_escrow_status(&escrow_id), EscrowStatus::Locked);
}

#[test]
fn test_get_escrow_status_released() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);
    client.release_escrow(&admin, &escrow_id, &true);

    assert_eq!(client.get_escrow_status(&escrow_id), EscrowStatus::Released);
}

#[test]
fn test_get_escrow_status_disputed() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    assert_eq!(client.get_escrow_status(&escrow_id), EscrowStatus::Disputed);
}

#[test]
fn test_get_escrow_status_resolved() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);
    // release_to_merchant=false → funds go to customer → status becomes Resolved
    client.resolve_dispute(&admin, &escrow_id, &false);

    assert_eq!(client.get_escrow_status(&escrow_id), EscrowStatus::Resolved);
}

#[test]
fn test_get_escrow_status_nonexistent_returns_error() {
    let (_env, client) = setup();
    let result = client.try_get_escrow_status(&9999_u64);
    assert_eq!(result, Err(Ok(Error::EscrowNotFound)));
}

// ── get_escrow_parties ────────────────────────────────────────────────────────

#[test]
fn test_get_escrow_parties_returns_correct_addresses() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    let (returned_customer, returned_merchant) = client.get_escrow_parties(&escrow_id);
    assert_eq!(returned_customer, customer);
    assert_eq!(returned_merchant, merchant);
}

#[test]
fn test_get_escrow_parties_nonexistent_returns_error() {
    let (_env, client) = setup();
    let result = client.try_get_escrow_parties(&9999_u64);
    assert_eq!(result, Err(Ok(Error::EscrowNotFound)));
}

// ── get_escrow_amount ─────────────────────────────────────────────────────────

#[test]
fn test_get_escrow_amount_returns_correct_value() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &2500_i128, &token, &5000_u64, &0_u64);

    assert_eq!(client.get_escrow_amount(&escrow_id), 2500_i128);
}

#[test]
fn test_get_escrow_amount_nonexistent_returns_error() {
    let (_env, client) = setup();
    let result = client.try_get_escrow_amount(&9999_u64);
    assert_eq!(result, Err(Ok(Error::EscrowNotFound)));
}

// ── verify_escrow_participant ─────────────────────────────────────────────────

#[test]
fn test_verify_escrow_participant_customer_returns_true() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    assert!(client.verify_escrow_participant(&escrow_id, &customer));
}

#[test]
fn test_verify_escrow_participant_merchant_returns_true() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    assert!(client.verify_escrow_participant(&escrow_id, &merchant));
}

#[test]
fn test_verify_escrow_participant_unrelated_address_returns_false() {
    let (env, client) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let unrelated = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

    assert!(!client.verify_escrow_participant(&escrow_id, &unrelated));
}

#[test]
fn test_verify_escrow_participant_nonexistent_id_returns_false() {
    let (env, client) = setup();
    let address = Address::generate(&env);
    assert!(!client.verify_escrow_participant(&9999_u64, &address));
}
