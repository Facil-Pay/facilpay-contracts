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

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

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
    let refund_id1 = client
        .request_refund(&merchant1, &payment_id1, &customer1, &amount, &amount, &token, &reason);

    let refund_id2 = client
        .request_refund(&merchant2, &payment_id2, &customer2, &amount, &amount, &token, &reason);

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
    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    let refund = client.get_refund(&refund_id);

    assert_eq!(refund.id, refund_id);
    assert_eq!(refund.payment_id, payment_id);
    assert_eq!(refund.merchant, merchant);
    assert_eq!(refund.customer, customer);
    assert_eq!(refund.amount, amount);
    assert_eq!(refund.original_payment_amount, amount);
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
    let refund_id1 = client
        .request_refund(&merchant1, &payment_id1, &customer1, &amount1, &amount1, &token, &reason);

    let refund_id2 = client
        .request_refund(&merchant2, &payment_id2, &customer2, &amount2, &amount2, &token, &reason);

    let refund_id3 = client
        .request_refund(&merchant3, &payment_id3, &customer3, &amount3, &amount3, &token, &reason);

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
    client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);
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
    client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);
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
    client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);
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
    let _refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

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
    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

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
    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

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
    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.reason, reason);
    assert_eq!(refund.status, RefundStatus::Requested);
}

// Test approve_refund functionality
#[test]
fn test_approve_requested_refund_successfully() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.approve_refund(&admin, &refund_id);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

#[test]
fn test_reject_requested_refund_successfully() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "Insufficient evidence");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Rejected);
}

#[test]
#[should_panic]
fn test_approve_nonexistent_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let nonexistent_refund_id = 999u64;

    env.mock_all_auths();
    client.approve_refund(&admin, &nonexistent_refund_id);
}

#[test]
#[should_panic]
fn test_reject_nonexistent_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let rejection_reason = String::from_str(&env, "Test reason");
    let nonexistent_refund_id = 999u64;

    env.mock_all_auths();
    client.reject_refund(&admin, &nonexistent_refund_id, &rejection_reason);
}

#[test]
#[should_panic]
fn test_approve_already_approved_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.approve_refund(&admin, &refund_id);
    client.approve_refund(&admin, &refund_id);
}

#[test]
#[should_panic]
fn test_reject_already_rejected_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "Insufficient evidence");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);
    client.reject_refund(&admin, &refund_id, &rejection_reason);
}

#[test]
#[should_panic]
fn test_approve_rejected_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "Insufficient evidence");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);
    client.approve_refund(&admin, &refund_id);
}

#[test]
#[should_panic]
fn test_reject_approved_refund_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "Insufficient evidence");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.approve_refund(&admin, &refund_id);
    client.reject_refund(&admin, &refund_id, &rejection_reason);
}

#[test]
fn test_refund_approved_event_is_emitted() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.approve_refund(&admin, &refund_id);

    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_refund_rejected_event_is_emitted() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "Insufficient evidence");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);

    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_approve_correct_refund_among_multiple() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    let refund_id1 = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);
    let refund_id2 = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);
    let refund_id3 = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.approve_refund(&admin, &refund_id2);

    let refund1 = client.get_refund(&refund_id1);
    let refund2 = client.get_refund(&refund_id2);
    let refund3 = client.get_refund(&refund_id3);

    assert_eq!(refund1.status, RefundStatus::Requested);
    assert_eq!(refund2.status, RefundStatus::Approved);
    assert_eq!(refund3.status, RefundStatus::Requested);
}

#[test]
fn test_reject_refund_with_empty_reason() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Rejected);
}

#[test]
fn test_reject_refund_with_detailed_reason() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Test reason");
    let rejection_reason = String::from_str(&env, "Insufficient evidence provided by merchant");

    env.mock_all_auths();

    let refund_id = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Rejected);
}

#[test]
fn test_request_partial_refund_half_payment() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let original_payment_amount = 2000i128;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Partial refund");

    env.mock_all_auths();
    let refund_id = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &original_payment_amount,
        &token,
        &reason,
    );

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.amount, amount);
    assert_eq!(refund.original_payment_amount, original_payment_amount);
    assert_eq!(refund.status, RefundStatus::Requested);
}

#[test]
fn test_multiple_partial_refunds_for_same_payment() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 7u64;
    let original_payment_amount = 1500i128;
    let reason = String::from_str(&env, "Partial refund");

    env.mock_all_auths();
    let refund_id1 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &700i128,
        &original_payment_amount,
        &token,
        &reason,
    );
    client.approve_refund(&admin, &refund_id1);
    client.process_refund(&admin, &refund_id1);

    let refund_id2 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &800i128,
        &original_payment_amount,
        &token,
        &reason,
    );
    let refund2 = client.get_refund(&refund_id2);
    assert_eq!(refund2.status, RefundStatus::Requested);

    let total_refunded = client.get_total_refunded_amount(&payment_id);
    assert_eq!(total_refunded, 700i128);
}

#[test]
#[should_panic]
fn test_request_refund_exceeding_original_payment_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let original_payment_amount = 1000i128;
    let amount = 1500i128;
    let reason = String::from_str(&env, "Too much");

    env.mock_all_auths();
    client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &original_payment_amount,
        &token,
        &reason,
    );
}

#[test]
#[should_panic]
fn test_cumulative_refunds_exceeding_payment_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 42u64;
    let original_payment_amount = 1000i128;
    let reason = String::from_str(&env, "Partial refund");

    env.mock_all_auths();
    let refund_id1 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &700i128,
        &original_payment_amount,
        &token,
        &reason,
    );
    client.approve_refund(&admin, &refund_id1);
    client.process_refund(&admin, &refund_id1);

    client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &400i128,
        &original_payment_amount,
        &token,
        &reason,
    );
}

#[test]
fn test_status_queries_and_counts() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 10u64;
    let amount = 500i128;
    let reason = String::from_str(&env, "Test");

    env.mock_all_auths();
    let r1 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );
    let r2 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );
    let r3 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );

    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Requested),
        3u64
    );
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Approved),
        0u64
    );
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Rejected),
        0u64
    );
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Processed),
        0u64
    );

    client.approve_refund(&admin, &r1);
    client.reject_refund(&admin, &r2, &String::from_str(&env, "No"));
    client.approve_refund(&admin, &r3);
    client.process_refund(&admin, &r3);

    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Requested),
        0u64
    );
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Approved),
        1u64
    );
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Rejected),
        1u64
    );
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Processed),
        1u64
    );

    let requested = client.get_refunds_by_status(&RefundStatus::Requested, &10u64, &0u64);
    assert_eq!(requested.len(), 0);

    let approved = client.get_refunds_by_status(&RefundStatus::Approved, &10u64, &0u64);
    assert_eq!(approved.len(), 1);
    assert_eq!(approved.get(0).unwrap().id, r1);

    let rejected = client.get_refunds_by_status(&RefundStatus::Rejected, &10u64, &0u64);
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected.get(0).unwrap().id, r2);

    let processed = client.get_refunds_by_status(&RefundStatus::Processed, &10u64, &0u64);
    assert_eq!(processed.len(), 1);
    assert_eq!(processed.get(0).unwrap().id, r3);
}

#[test]
fn test_pagination_for_status_queries() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 20u64;
    let amount = 100i128;
    let reason = String::from_str(&env, "Pagination");

    env.mock_all_auths();
    let r1 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );
    let r2 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );
    let r3 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );
    let r4 = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
    );

    let page = client.get_refunds_by_status(&RefundStatus::Requested, &2u64, &1u64);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().id, r2);
    assert_eq!(page.get(1).unwrap().id, r3);

    let page2 = client.get_refunds_by_status(&RefundStatus::Requested, &2u64, &3u64);
    assert_eq!(page2.len(), 1);
    assert_eq!(page2.get(0).unwrap().id, r4);

    let _ = r1; // keep r1 used to ensure order matches insertion in test
}

#[test]
#[should_panic]
fn test_refund_for_zero_amount_payment_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 30u64;
    let original_payment_amount = 0i128;
    let amount = 100i128;
    let reason = String::from_str(&env, "Zero payment");

    env.mock_all_auths();
    client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &original_payment_amount,
        &token,
        &reason,
    );
}

#[test]
fn test_full_refund_is_allowed() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 55u64;
    let original_payment_amount = 1000i128;
    let amount = 1000i128;
    let reason = String::from_str(&env, "Full refund");

    env.mock_all_auths();
    let refund_id = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &original_payment_amount,
        &token,
        &reason,
    );

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.amount, amount);
    assert_eq!(refund.original_payment_amount, original_payment_amount);
}

#[test]
fn test_status_query_with_no_refunds() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let refunds = client.get_refunds_by_status(&RefundStatus::Approved, &10u64, &0u64);
    assert_eq!(refunds.len(), 0);
    assert_eq!(
        client.get_refund_count_by_status(&RefundStatus::Approved),
        0u64
    );
}

#[test]
fn test_can_refund_payment_helper() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 77u64;
    let original_payment_amount = 1000i128;
    let reason = String::from_str(&env, "Helper");

    env.mock_all_auths();
    let refund_id = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &400i128,
        &original_payment_amount,
        &token,
        &reason,
    );
    client.approve_refund(&admin, &refund_id);
    client.process_refund(&admin, &refund_id);

    let allowed = client.can_refund_payment(&payment_id, &500i128, &original_payment_amount);
    assert!(allowed);
}

#[test]
#[should_panic]
fn test_can_refund_payment_helper_rejects_overflow() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let admin = Address::generate(&env);
    let payment_id = 78u64;
    let original_payment_amount = 1000i128;
    let reason = String::from_str(&env, "Helper");

    env.mock_all_auths();
    let refund_id = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &900i128,
        &original_payment_amount,
        &token,
        &reason,
    );
    client.approve_refund(&admin, &refund_id);
    client.process_refund(&admin, &refund_id);

    client.can_refund_payment(&payment_id, &200i128, &original_payment_amount);
}
