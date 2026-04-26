#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

// If payment contract address is not set, verification is skipped (backward-compatible)
#[test]
fn test_request_refund_without_payment_contract_set() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    // No payment contract set — should succeed normally
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &500i128,
        &1000i128,
        &token,
        &String::from_str(&env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    );
    assert_eq!(refund_id, 1u64);
}

// set/get payment contract address
#[test]
fn test_set_get_payment_contract_address() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let payment_contract = Address::generate(&env);

    assert!(client.get_payment_contract_address().is_none());
    client.set_payment_contract_address(&admin, &payment_contract);
    assert_eq!(client.get_payment_contract_address().unwrap(), payment_contract);
}

// verify_payment_ownership returns false when no contract set
#[test]
fn test_verify_ownership_no_contract_returns_false() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let customer = Address::generate(&env);
    let result = client.verify_payment_ownership(&1u64, &customer);
    assert!(!result);
}

// Ownership mismatch: wrong customer returns PaymentOwnershipMismatch
// (tested via mock: set a fake payment contract address and expect the
//  cross-contract call to return false → refund request rejected)
#[test]
fn test_ownership_mismatch_rejects_refund() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    // Point to a random address as "payment contract" — calls will fail → false
    let fake_payment_contract = Address::generate(&env);
    client.set_payment_contract_address(&admin, &fake_payment_contract);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    // Should fail with PaymentOwnershipMismatch because cross-contract call returns false
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &500i128,
        &1000i128,
        &token,
        &String::from_str(&env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    );
    assert!(result.is_err());
}
