#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Events, Address, Env, String};

#[test]
fn test_request_refund_with_valid_data() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Customer requested refund");

    env.mock_all_auths();

    let refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    assert_eq!(refund_id, 1u64);
}

#[test]
fn test_refund_id_increments_correctly() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id1 = 1u64;
    let payment_id2 = 2u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    let refund_id1 = client.request_refund(
        &merchant1,
        &payment_id1,
        &customer1,
        &amount,
        &token,
        &reason,
    );

    let refund_id2 = client.request_refund(
        &merchant2,
        &payment_id2,
        &customer2,
        &amount,
        &token,
        &reason,
    );

    assert_eq!(refund_id1, 1u64);
    assert_eq!(refund_id2, 2u64);
}

#[test]
fn test_get_refund_by_id() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    let refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    let refund = client.get_refund(&refund_id);

    assert_eq!(refund.id, refund_id);
    assert_eq!(refund.payment_id, payment_id);
    assert_eq!(refund.merchant, merchant);
    assert_eq!(refund.customer, customer);
    assert_eq!(refund.amount, amount);
    assert_eq!(refund.token, token);
    assert_eq!(refund.status, RefundStatus::Requested);
    assert_eq!(refund.reason, reason);
}

#[test]
fn test_request_multiple_refunds_and_retrieve_each() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let merchant3 = Address::generate(&env);
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let customer3 = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id1 = 1u64;
    let payment_id2 = 2u64;
    let payment_id3 = 3u64;
    let amount1 = 1000i128;
    let amount2 = 2000i128;
    let amount3 = 3000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    let refund_id1 = client.request_refund(
        &merchant1,
        &payment_id1,
        &customer1,
        &amount1,
        &token,
        &reason,
    );

    let refund_id2 = client.request_refund(
        &merchant2,
        &payment_id2,
        &customer2,
        &amount2,
        &token,
        &reason,
    );

    let refund_id3 = client.request_refund(
        &merchant3,
        &payment_id3,
        &customer3,
        &amount3,
        &token,
        &reason,
    );

    let refund1 = client.get_refund(&refund_id1);
    let refund2 = client.get_refund(&refund_id2);
    let refund3 = client.get_refund(&refund_id3);

    assert_eq!(refund1.id, refund_id1);
    assert_eq!(refund1.amount, amount1);
    assert_eq!(refund1.payment_id, payment_id1);

    assert_eq!(refund2.id, refund_id2);
    assert_eq!(refund2.amount, amount2);
    assert_eq!(refund2.payment_id, payment_id2);

    assert_eq!(refund3.id, refund_id3);
    assert_eq!(refund3.amount, amount3);
    assert_eq!(refund3.payment_id, payment_id3);
}

#[test]
#[should_panic]
fn test_request_refund_with_zero_amount_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 0i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
}

#[test]
#[should_panic]
fn test_request_refund_with_negative_amount_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = -100i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
}

#[test]
#[should_panic]
fn test_request_refund_with_invalid_payment_id_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 0u64; // Invalid payment_id
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
}

#[test]
#[should_panic]
fn test_get_nonexistent_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let nonexistent_refund_id = 999u64;
    client.get_refund(&nonexistent_refund_id);
}

#[test]
fn test_refund_requested_event_is_emitted() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    let _refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    // Check that the event was emitted
    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let _event_data = events.get(0).unwrap();
    // Verify event structure (the actual event data structure may vary)
    // The event should contain the RefundRequested data
}

#[test]
fn test_refund_stored_with_requested_status() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();
    let refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Requested);
}

#[test]
fn test_request_refund_without_reason() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, ""); // Empty reason

    env.mock_all_auths();
    let refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.reason, String::from_str(&env, ""));
    assert_eq!(refund.status, RefundStatus::Requested);
}

#[test]
fn test_request_refund_with_reason() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Customer not satisfied with product quality");

    env.mock_all_auths();
    let refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.reason, reason);
    assert_eq!(refund.status, RefundStatus::Requested);
}
