#![cfg(test)]

use crate::{
    CustomerRefundSummary, RefundContract, RefundContractClient, RefundReasonCode, RefundStatus,
};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String};

fn setup_test_env<'a>() -> (Env, RefundContractClient<'a>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, RefundContract);
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.initialize(&admin);

    (env, client, admin, merchant, customer)
}

#[test]
fn test_lifecycle_timestamps_on_request() {
    let (env, client, _admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    env.ledger().set_timestamp(50); // Set a non-zero timestamp

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    let refund = client.get_refund(&refund_id);
    
    // Verify requested_at is set
    assert_eq!(refund.requested_at, 50);
    
    // Verify other timestamps are None
    assert_eq!(refund.approved_at, None);
    assert_eq!(refund.rejected_at, None);
    assert_eq!(refund.processed_at, None);
}

#[test]
fn test_lifecycle_timestamps_on_approve() {
    let (env, client, admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Advance ledger time
    env.ledger().set_timestamp(100);

    client.approve_refund(&admin, &refund_id);

    let refund = client.get_refund(&refund_id);
    
    // Verify approved_at is set
    assert!(refund.approved_at.is_some());
    assert_eq!(refund.approved_at.unwrap(), 100);
    
    // Verify other timestamps
    assert!(refund.requested_at < refund.approved_at.unwrap());
    assert_eq!(refund.rejected_at, None);
    assert_eq!(refund.processed_at, None);
}

#[test]
fn test_lifecycle_timestamps_on_reject() {
    let (env, client, admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Advance ledger time
    env.ledger().set_timestamp(100);

    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Invalid"));

    let refund = client.get_refund(&refund_id);
    
    // Verify rejected_at is set
    assert!(refund.rejected_at.is_some());
    assert_eq!(refund.rejected_at.unwrap(), 100);
    
    // Verify other timestamps
    assert!(refund.requested_at < refund.rejected_at.unwrap());
    assert_eq!(refund.approved_at, None);
    assert_eq!(refund.processed_at, None);
}

#[test]
fn test_lifecycle_timestamps_on_process() {
    let (env, client, admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Advance ledger time for approval
    env.ledger().set_timestamp(100);
    client.approve_refund(&admin, &refund_id);

    // Advance ledger time for processing
    env.ledger().set_timestamp(200);
    client.process_refund(&admin, &refund_id);

    let refund = client.get_refund(&refund_id);
    
    // Verify all timestamps are set correctly
    assert!(refund.requested_at == 0);
    assert_eq!(refund.approved_at, Some(100));
    assert_eq!(refund.processed_at, Some(200));
    assert_eq!(refund.rejected_at, None);
    
    // Verify chronological order
    assert!(refund.requested_at < refund.approved_at.unwrap());
    assert!(refund.approved_at.unwrap() < refund.processed_at.unwrap());
}

#[test]
fn test_get_customer_refund_history_empty() {
    let (_env, client, _admin, _merchant, customer) = setup_test_env();

    let history = client.get_customer_refund_history(&customer, &10, &0);
    assert_eq!(history.len(), 0);
}

#[test]
fn test_get_customer_refund_history_single() {
    let (env, client, _admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    let history = client.get_customer_refund_history(&customer, &10, &0);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().amount, 1000);
}

#[test]
fn test_get_customer_refund_history_newest_first() {
    let (env, client, _admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    // Create 3 refunds at different times
    env.ledger().set_timestamp(100);
    let refund_id1 = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "First refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    env.ledger().set_timestamp(200);
    let refund_id2 = client.request_refund(
        &merchant,
        &2,
        &customer,
        &2000,
        &5000,
        &token,
        &String::from_str(&env, "Second refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    env.ledger().set_timestamp(300);
    let refund_id3 = client.request_refund(
        &merchant,
        &3,
        &customer,
        &3000,
        &5000,
        &token,
        &String::from_str(&env, "Third refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    let history = client.get_customer_refund_history(&customer, &10, &0);
    assert_eq!(history.len(), 3);
    
    // Verify newest first ordering
    assert_eq!(history.get(0).unwrap().id, refund_id3);
    assert_eq!(history.get(1).unwrap().id, refund_id2);
    assert_eq!(history.get(2).unwrap().id, refund_id1);
    
    // Verify timestamps are in descending order
    assert!(history.get(0).unwrap().requested_at > history.get(1).unwrap().requested_at);
    assert!(history.get(1).unwrap().requested_at > history.get(2).unwrap().requested_at);
}

#[test]
fn test_get_customer_refund_history_pagination() {
    let (env, client, admin, merchant, _customer) = setup_test_env();
    let token = Address::generate(&env);

    // Disable fraud detection for this test
    let fraud_config = crate::FraudConfig {
        enabled: false,
        max_refund_rate_bps: 10000,
        min_transactions_for_check: 100,
    };
    client.set_fraud_config(&admin, &fraud_config);

    // Use a fresh customer
    let customer = Address::generate(&env);

    // Create 5 refunds
    for i in 1..=5 {
        env.ledger().set_timestamp(i * 100);
        client.request_refund(
            &merchant,
            &i,
            &customer,
            &(i as i128 * 1000),
            &10000, // Higher original amount to avoid exceeding
            &token,
            &String::from_str(&env, "Test refund"),
            &RefundReasonCode::CustomerRequest,
            &env.ledger().timestamp(),
        );
    }

    // Get first page (2 items)
    let page1 = client.get_customer_refund_history(&customer, &2, &0);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().amount, 5000); // Newest
    assert_eq!(page1.get(1).unwrap().amount, 4000);

    // Get second page (2 items)
    let page2 = client.get_customer_refund_history(&customer, &2, &2);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().amount, 3000);
    assert_eq!(page2.get(1).unwrap().amount, 2000);

    // Get third page (1 item)
    let page3 = client.get_customer_refund_history(&customer, &2, &4);
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().amount, 1000); // Oldest
}

#[test]
fn test_get_customer_refund_history_offset_beyond_total() {
    let (env, client, _admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    let history = client.get_customer_refund_history(&customer, &10, &100);
    assert_eq!(history.len(), 0);
}

#[test]
fn test_get_customer_refund_count() {
    let (env, client, _admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    assert_eq!(client.get_customer_refund_count_public(&customer), 0);

    client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund 1"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    assert_eq!(client.get_customer_refund_count_public(&customer), 1);

    client.request_refund(
        &merchant,
        &2,
        &customer,
        &2000,
        &5000,
        &token,
        &String::from_str(&env, "Test refund 2"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    assert_eq!(client.get_customer_refund_count_public(&customer), 2);
}

#[test]
fn test_get_customer_refund_summary_empty() {
    let (_env, client, _admin, _merchant, customer) = setup_test_env();

    let summary = client.get_customer_refund_summary(&customer);
    assert_eq!(summary.total_requested, 0);
    assert_eq!(summary.total_approved, 0);
    assert_eq!(summary.total_amount_refunded, 0);
    assert_eq!(summary.avg_processing_time, 0);
}

#[test]
fn test_get_customer_refund_summary_with_data() {
    let (env, client, admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    // Create refund 1 - will be processed
    env.ledger().set_timestamp(100);
    let refund_id1 = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Refund 1"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    env.ledger().set_timestamp(150);
    client.approve_refund(&admin, &refund_id1);

    env.ledger().set_timestamp(200);
    client.process_refund(&admin, &refund_id1);

    // Create refund 2 - will be processed
    env.ledger().set_timestamp(300);
    let refund_id2 = client.request_refund(
        &merchant,
        &2,
        &customer,
        &2000,
        &5000,
        &token,
        &String::from_str(&env, "Refund 2"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    env.ledger().set_timestamp(350);
    client.approve_refund(&admin, &refund_id2);

    env.ledger().set_timestamp(500);
    client.process_refund(&admin, &refund_id2);

    // Create refund 3 - will be rejected
    env.ledger().set_timestamp(600);
    let refund_id3 = client.request_refund(
        &merchant,
        &3,
        &customer,
        &3000,
        &5000,
        &token,
        &String::from_str(&env, "Refund 3"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    env.ledger().set_timestamp(650);
    client.reject_refund(&admin, &refund_id3, &String::from_str(&env, "Invalid"));

    let summary = client.get_customer_refund_summary(&customer);
    
    assert_eq!(summary.total_requested, 3);
    assert_eq!(summary.total_approved, 2); // Only processed ones count as approved
    assert_eq!(summary.total_amount_refunded, 3000); // 1000 + 2000
    
    // Average processing time: ((200-100) + (500-300)) / 2 = (100 + 200) / 2 = 150
    assert_eq!(summary.avg_processing_time, 150);
}

#[test]
fn test_get_customer_refund_summary_only_requested() {
    let (env, client, _admin, merchant, customer) = setup_test_env();
    let token = Address::generate(&env);

    // Create refunds that are only requested (not approved/processed)
    client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Refund 1"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.request_refund(
        &merchant,
        &2,
        &customer,
        &2000,
        &5000,
        &token,
        &String::from_str(&env, "Refund 2"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    let summary = client.get_customer_refund_summary(&customer);
    
    assert_eq!(summary.total_requested, 2);
    assert_eq!(summary.total_approved, 0);
    assert_eq!(summary.total_amount_refunded, 0);
    assert_eq!(summary.avg_processing_time, 0);
}

#[test]
fn test_customer_refund_history_multiple_customers() {
    let (env, client, _admin, merchant, _customer) = setup_test_env();
    let token = Address::generate(&env);
    
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);

    // Create refunds for customer1
    client.request_refund(
        &merchant,
        &1,
        &customer1,
        &1000,
        &5000,
        &token,
        &String::from_str(&env, "Customer 1 refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Create refunds for customer2
    client.request_refund(
        &merchant,
        &2,
        &customer2,
        &2000,
        &5000,
        &token,
        &String::from_str(&env, "Customer 2 refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Verify each customer only sees their own refunds
    let history1 = client.get_customer_refund_history(&customer1, &10, &0);
    assert_eq!(history1.len(), 1);
    assert_eq!(history1.get(0).unwrap().amount, 1000);

    let history2 = client.get_customer_refund_history(&customer2, &10, &0);
    assert_eq!(history2.len(), 1);
    assert_eq!(history2.get(0).unwrap().amount, 2000);
}
