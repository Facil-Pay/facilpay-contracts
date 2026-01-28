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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant1, &payment_id1, &customer1, &amount, &token, &reason);

    let refund_id2 = client
        .request_refund(&merchant2, &payment_id2, &customer2, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
    let refund_id1 = client
        .request_refund(&merchant1, &payment_id1, &customer1, &amount1, &token, &reason);

    let refund_id2 = client
        .request_refund(&merchant2, &payment_id2, &customer2, &amount2, &token, &reason);

    let refund_id3 = client
        .request_refund(&merchant3, &payment_id3, &customer3, &amount3, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
    let refund_id2 = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);
    let refund_id3 = client
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

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
        .request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    client.reject_refund(&admin, &refund_id, &rejection_reason);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Rejected);
}

// Tests for query functions
#[test]
fn test_get_refunds_by_merchant() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create refunds for merchant1
    let refund_id1 = client.request_refund(&merchant1, &1u64, &customer, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant1, &2u64, &customer, &2000i128, &token, &reason);
    let refund_id3 = client.request_refund(&merchant1, &3u64, &customer, &3000i128, &token, &reason);

    // Create refund for merchant2
    let _refund_id4 = client.request_refund(&merchant2, &4u64, &customer, &4000i128, &token, &reason);

    // Query refunds for merchant1 with limit 2, offset 0
    let refunds = client.get_refunds_by_merchant(&merchant1, &2u64, &0u64);
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);

    // Query refunds for merchant1 with limit 2, offset 2
    let refunds = client.get_refunds_by_merchant(&merchant1, &2u64, &2u64);
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds.get(0).unwrap().id, refund_id3);

    // Query refunds for merchant2
    let refunds = client.get_refunds_by_merchant(&merchant2, &10u64, &0u64);
    assert_eq!(refunds.len(), 1);
}

#[test]
fn test_get_refund_count_by_merchant() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    // Check initial count is 0
    assert_eq!(client.get_refund_count_by_merchant(&merchant1), 0u64);

    env.mock_all_auths();

    // Create refunds for merchant1
    client.request_refund(&merchant1, &1u64, &customer, &1000i128, &token, &reason);
    client.request_refund(&merchant1, &2u64, &customer, &2000i128, &token, &reason);

    // Create refund for merchant2
    client.request_refund(&merchant2, &3u64, &customer, &3000i128, &token, &reason);

    // Check counts
    assert_eq!(client.get_refund_count_by_merchant(&merchant1), 2u64);
    assert_eq!(client.get_refund_count_by_merchant(&merchant2), 1u64);
}

#[test]
fn test_get_refunds_by_customer() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create refunds for customer1
    let refund_id1 = client.request_refund(&merchant, &1u64, &customer1, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant, &2u64, &customer1, &2000i128, &token, &reason);

    // Create refund for customer2
    let _refund_id3 = client.request_refund(&merchant, &3u64, &customer2, &3000i128, &token, &reason);

    // Query refunds for customer1
    let refunds = client.get_refunds_by_customer(&customer1, &10u64, &0u64);
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);

    // Query refunds for customer2
    let refunds = client.get_refunds_by_customer(&customer2, &10u64, &0u64);
    assert_eq!(refunds.len(), 1);
}

#[test]
fn test_get_refund_count_by_customer() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    // Check initial count is 0
    assert_eq!(client.get_refund_count_by_customer(&customer1), 0u64);

    env.mock_all_auths();

    // Create refunds for customer1
    client.request_refund(&merchant, &1u64, &customer1, &1000i128, &token, &reason);
    client.request_refund(&merchant, &2u64, &customer1, &2000i128, &token, &reason);
    client.request_refund(&merchant, &3u64, &customer1, &3000i128, &token, &reason);

    // Create refund for customer2
    client.request_refund(&merchant, &4u64, &customer2, &4000i128, &token, &reason);

    // Check counts
    assert_eq!(client.get_refund_count_by_customer(&customer1), 3u64);
    assert_eq!(client.get_refund_count_by_customer(&customer2), 1u64);
}

#[test]
fn test_get_refunds_by_payment() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create multiple refunds for the same payment
    let refund_id1 = client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant, &1u64, &customer, &2000i128, &token, &reason);

    // Create refund for different payment
    let _refund_id3 = client.request_refund(&merchant, &2u64, &customer, &3000i128, &token, &reason);

    // Query refunds for payment 1
    let refunds = client.get_refunds_by_payment(&1u64, &10u64, &0u64);
    assert_eq!(refunds.len(), 2);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);

    // Query refunds for payment 2
    let refunds = client.get_refunds_by_payment(&2u64, &10u64, &0u64);
    assert_eq!(refunds.len(), 1);
}

#[test]
fn test_get_refund_count_by_payment() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    // Check initial count is 0
    assert_eq!(client.get_refund_count_by_payment(&1u64), 0u64);

    env.mock_all_auths();

    // Create multiple refunds for payment 1
    client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    client.request_refund(&merchant, &1u64, &customer, &2000i128, &token, &reason);

    // Create refund for payment 2
    client.request_refund(&merchant, &2u64, &customer, &3000i128, &token, &reason);

    // Check counts
    assert_eq!(client.get_refund_count_by_payment(&1u64), 2u64);
    assert_eq!(client.get_refund_count_by_payment(&2u64), 1u64);
}

#[test]
fn test_query_functions_with_empty_results() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // Query with no refunds should return empty results
    let refunds = client.get_refunds_by_merchant(&merchant, &10u64, &0u64);
    assert_eq!(refunds.len(), 0);

    let refunds = client.get_refunds_by_customer(&customer, &10u64, &0u64);
    assert_eq!(refunds.len(), 0);

    let refunds = client.get_refunds_by_payment(&999u64, &10u64, &0u64);
    assert_eq!(refunds.len(), 0);

    // Counts should be 0
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 0u64);
    assert_eq!(client.get_refund_count_by_customer(&customer), 0u64);
    assert_eq!(client.get_refund_count_by_payment(&999u64), 0u64);
}

#[test]
fn test_query_pagination_with_offset_beyond_total() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create 2 refunds
    client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    client.request_refund(&merchant, &2u64, &customer, &2000i128, &token, &reason);

    // Query with offset beyond total count
    let refunds = client.get_refunds_by_merchant(&merchant, &10u64, &5u64);
    assert_eq!(refunds.len(), 0);
}

#[test]
fn test_multiple_refunds_for_same_merchant() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create 5 refunds for the same merchant
    let refund_id1 = client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant, &2u64, &customer, &2000i128, &token, &reason);
    let refund_id3 = client.request_refund(&merchant, &3u64, &customer, &3000i128, &token, &reason);
    let refund_id4 = client.request_refund(&merchant, &4u64, &customer, &4000i128, &token, &reason);
    let refund_id5 = client.request_refund(&merchant, &5u64, &customer, &5000i128, &token, &reason);

    // Query all refunds for merchant
    let refunds = client.get_refunds_by_merchant(&merchant, &10u64, &0u64);
    assert_eq!(refunds.len(), 5);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);
    assert_eq!(refunds.get(2).unwrap().id, refund_id3);
    assert_eq!(refunds.get(3).unwrap().id, refund_id4);
    assert_eq!(refunds.get(4).unwrap().id, refund_id5);

    // Verify count
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 5u64);
}

#[test]
fn test_multiple_refunds_for_same_customer() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create 4 refunds for the same customer
    let refund_id1 = client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant, &2u64, &customer, &2000i128, &token, &reason);
    let refund_id3 = client.request_refund(&merchant, &3u64, &customer, &3000i128, &token, &reason);
    let refund_id4 = client.request_refund(&merchant, &4u64, &customer, &4000i128, &token, &reason);

    // Query all refunds for customer
    let refunds = client.get_refunds_by_customer(&customer, &10u64, &0u64);
    assert_eq!(refunds.len(), 4);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);
    assert_eq!(refunds.get(2).unwrap().id, refund_id3);
    assert_eq!(refunds.get(3).unwrap().id, refund_id4);

    // Verify count
    assert_eq!(client.get_refund_count_by_customer(&customer), 4u64);
}

#[test]
fn test_multiple_refunds_for_same_payment() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");
    let payment_id = 1u64;

    env.mock_all_auths();

    // Create 3 refunds for the same payment
    let refund_id1 = client.request_refund(&merchant, &payment_id, &customer, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant, &payment_id, &customer, &2000i128, &token, &reason);
    let refund_id3 = client.request_refund(&merchant, &payment_id, &customer, &3000i128, &token, &reason);

    // Query all refunds for payment
    let refunds = client.get_refunds_by_payment(&payment_id, &10u64, &0u64);
    assert_eq!(refunds.len(), 3);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);
    assert_eq!(refunds.get(2).unwrap().id, refund_id3);

    // Verify count
    assert_eq!(client.get_refund_count_by_payment(&payment_id), 3u64);
}

#[test]
fn test_pagination_first_5_refunds() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create 7 refunds
    let mut refund_ids: Vec<u64> = Vec::new(&env);
    for i in 0..7 {
        let id = client.request_refund(&merchant, &(i as u64 + 1), &customer, &((i as i128 + 1) * 1000), &token, &reason);
        refund_ids.push_back(id);
    }

    // Get first 5 refunds (limit=5, offset=0)
    let refunds = client.get_refunds_by_merchant(&merchant, &5u64, &0u64);
    assert_eq!(refunds.len(), 5);
    for i in 0..5 {
        assert_eq!(refunds.get(i as u32).unwrap().id, refund_ids.get(i as u32).unwrap());
    }
}

#[test]
fn test_pagination_next_5_refunds() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create 12 refunds
    let mut refund_ids: Vec<u64> = Vec::new(&env);
    for i in 0..12 {
        let id = client.request_refund(&merchant, &(i as u64 + 1), &customer, &((i as i128 + 1) * 1000), &token, &reason);
        refund_ids.push_back(id);
    }

    // Get next 5 refunds (limit=5, offset=5)
    let refunds = client.get_refunds_by_merchant(&merchant, &5u64, &5u64);
    assert_eq!(refunds.len(), 5);
    for i in 0..5 {
        assert_eq!(refunds.get(i as u32).unwrap().id, refund_ids.get((i + 5) as u32).unwrap());
    }
}

#[test]
fn test_pagination_limit_larger_than_total() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create 3 refunds
    let refund_id1 = client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    let refund_id2 = client.request_refund(&merchant, &2u64, &customer, &2000i128, &token, &reason);
    let refund_id3 = client.request_refund(&merchant, &3u64, &customer, &3000i128, &token, &reason);

    // Query with limit larger than total (limit=10, total=3)
    let refunds = client.get_refunds_by_merchant(&merchant, &10u64, &0u64);
    assert_eq!(refunds.len(), 3);
    assert_eq!(refunds.get(0).unwrap().id, refund_id1);
    assert_eq!(refunds.get(1).unwrap().id, refund_id2);
    assert_eq!(refunds.get(2).unwrap().id, refund_id3);
}

#[test]
fn test_refunds_not_mixed_between_merchants() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create refunds for merchant1
    let m1_refund1 = client.request_refund(&merchant1, &1u64, &customer, &1000i128, &token, &reason);
    let m1_refund2 = client.request_refund(&merchant1, &2u64, &customer, &2000i128, &token, &reason);

    // Create refunds for merchant2
    let m2_refund1 = client.request_refund(&merchant2, &3u64, &customer, &3000i128, &token, &reason);
    let m2_refund2 = client.request_refund(&merchant2, &4u64, &customer, &4000i128, &token, &reason);

    // Query merchant1 refunds - should only get merchant1's refunds
    let m1_refunds = client.get_refunds_by_merchant(&merchant1, &10u64, &0u64);
    assert_eq!(m1_refunds.len(), 2);
    assert_eq!(m1_refunds.get(0).unwrap().id, m1_refund1);
    assert_eq!(m1_refunds.get(1).unwrap().id, m1_refund2);
    assert_eq!(m1_refunds.get(0).unwrap().merchant, merchant1);
    assert_eq!(m1_refunds.get(1).unwrap().merchant, merchant1);

    // Query merchant2 refunds - should only get merchant2's refunds
    let m2_refunds = client.get_refunds_by_merchant(&merchant2, &10u64, &0u64);
    assert_eq!(m2_refunds.len(), 2);
    assert_eq!(m2_refunds.get(0).unwrap().id, m2_refund1);
    assert_eq!(m2_refunds.get(1).unwrap().id, m2_refund2);
    assert_eq!(m2_refunds.get(0).unwrap().merchant, merchant2);
    assert_eq!(m2_refunds.get(1).unwrap().merchant, merchant2);
}

#[test]
fn test_refunds_not_mixed_between_customers() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create refunds for customer1
    let c1_refund1 = client.request_refund(&merchant, &1u64, &customer1, &1000i128, &token, &reason);
    let c1_refund2 = client.request_refund(&merchant, &2u64, &customer1, &2000i128, &token, &reason);

    // Create refunds for customer2
    let c2_refund1 = client.request_refund(&merchant, &3u64, &customer2, &3000i128, &token, &reason);

    // Query customer1 refunds - should only get customer1's refunds
    let c1_refunds = client.get_refunds_by_customer(&customer1, &10u64, &0u64);
    assert_eq!(c1_refunds.len(), 2);
    assert_eq!(c1_refunds.get(0).unwrap().id, c1_refund1);
    assert_eq!(c1_refunds.get(1).unwrap().id, c1_refund2);
    assert_eq!(c1_refunds.get(0).unwrap().customer, customer1);
    assert_eq!(c1_refunds.get(1).unwrap().customer, customer1);

    // Query customer2 refunds - should only get customer2's refunds
    let c2_refunds = client.get_refunds_by_customer(&customer2, &10u64, &0u64);
    assert_eq!(c2_refunds.len(), 1);
    assert_eq!(c2_refunds.get(0).unwrap().id, c2_refund1);
    assert_eq!(c2_refunds.get(0).unwrap().customer, customer2);
}

#[test]
fn test_refunds_not_mixed_between_payments() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Create refunds for payment 1
    let p1_refund1 = client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    let p1_refund2 = client.request_refund(&merchant, &1u64, &customer, &2000i128, &token, &reason);

    // Create refunds for payment 2
    let p2_refund1 = client.request_refund(&merchant, &2u64, &customer, &3000i128, &token, &reason);
    let p2_refund2 = client.request_refund(&merchant, &2u64, &customer, &4000i128, &token, &reason);
    let p2_refund3 = client.request_refund(&merchant, &2u64, &customer, &5000i128, &token, &reason);

    // Query payment 1 refunds - should only get payment 1's refunds
    let p1_refunds = client.get_refunds_by_payment(&1u64, &10u64, &0u64);
    assert_eq!(p1_refunds.len(), 2);
    assert_eq!(p1_refunds.get(0).unwrap().id, p1_refund1);
    assert_eq!(p1_refunds.get(1).unwrap().id, p1_refund2);
    assert_eq!(p1_refunds.get(0).unwrap().payment_id, 1u64);
    assert_eq!(p1_refunds.get(1).unwrap().payment_id, 1u64);

    // Query payment 2 refunds - should only get payment 2's refunds
    let p2_refunds = client.get_refunds_by_payment(&2u64, &10u64, &0u64);
    assert_eq!(p2_refunds.len(), 3);
    assert_eq!(p2_refunds.get(0).unwrap().id, p2_refund1);
    assert_eq!(p2_refunds.get(1).unwrap().id, p2_refund2);
    assert_eq!(p2_refunds.get(2).unwrap().id, p2_refund3);
    assert_eq!(p2_refunds.get(0).unwrap().payment_id, 2u64);
    assert_eq!(p2_refunds.get(1).unwrap().payment_id, 2u64);
    assert_eq!(p2_refunds.get(2).unwrap().payment_id, 2u64);
}

#[test]
fn test_merchant_count_accurate_after_multiple_requests() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Count should be 0 initially
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 0u64);

    // Create refunds one by one and verify count
    client.request_refund(&merchant, &1u64, &customer, &1000i128, &token, &reason);
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 1u64);

    client.request_refund(&merchant, &2u64, &customer, &2000i128, &token, &reason);
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 2u64);

    client.request_refund(&merchant, &3u64, &customer, &3000i128, &token, &reason);
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 3u64);

    client.request_refund(&merchant, &4u64, &customer, &4000i128, &token, &reason);
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 4u64);

    client.request_refund(&merchant, &5u64, &customer, &5000i128, &token, &reason);
    assert_eq!(client.get_refund_count_by_merchant(&merchant), 5u64);
}

#[test]
fn test_customer_count_accurate_after_multiple_requests() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");

    env.mock_all_auths();

    // Count should be 0 initially
    assert_eq!(client.get_refund_count_by_customer(&customer), 0u64);

    // Create refunds one by one and verify count
    for i in 1..=6 {
        client.request_refund(&merchant, &(i as u64), &customer, &((i as i128) * 1000), &token, &reason);
        assert_eq!(client.get_refund_count_by_customer(&customer), i as u64);
    }

    assert_eq!(client.get_refund_count_by_customer(&customer), 6u64);
}

#[test]
fn test_payment_count_accurate_after_multiple_requests() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Test reason");
    let payment_id = 42u64;

    env.mock_all_auths();

    // Count should be 0 initially
    assert_eq!(client.get_refund_count_by_payment(&payment_id), 0u64);

    // Create refunds one by one and verify count
    for i in 1..=4 {
        client.request_refund(&merchant, &payment_id, &customer, &((i as i128) * 1000), &token, &reason);
        assert_eq!(client.get_refund_count_by_payment(&payment_id), i as u64);
    }

    assert_eq!(client.get_refund_count_by_payment(&payment_id), 4u64);
}

#[test]
fn test_all_query_functions_return_empty_for_nonexistent_entities() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let nonexistent_merchant = Address::generate(&env);
    let nonexistent_customer = Address::generate(&env);
    let nonexistent_payment_id = 99999u64;

    // All query functions should return empty/zero for nonexistent entities
    let merchant_refunds = client.get_refunds_by_merchant(&nonexistent_merchant, &10u64, &0u64);
    assert_eq!(merchant_refunds.len(), 0);
    assert_eq!(client.get_refund_count_by_merchant(&nonexistent_merchant), 0u64);

    let customer_refunds = client.get_refunds_by_customer(&nonexistent_customer, &10u64, &0u64);
    assert_eq!(customer_refunds.len(), 0);
    assert_eq!(client.get_refund_count_by_customer(&nonexistent_customer), 0u64);

    let payment_refunds = client.get_refunds_by_payment(&nonexistent_payment_id, &10u64, &0u64);
    assert_eq!(payment_refunds.len(), 0);
    assert_eq!(client.get_refund_count_by_payment(&nonexistent_payment_id), 0u64);
}
