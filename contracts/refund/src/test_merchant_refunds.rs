#![cfg(test)]

use super::*;
use soroban_sdk::{ testutils::Address as _, Address, Env, String };

fn request_refund_for_merchant(
    client: &RefundContractClient<'_>,
    env: &Env,
    merchant: &Address,
    customer: &Address,
    token: &Address,
    payment_id: u64,
    amount: i128,
) -> u64 {
    client.request_refund(
        merchant,
        &payment_id,
        customer,
        &amount,
        &amount,
        token,
        &String::from_str(env, "merchant dashboard"),
        &RefundReasonCode::Other,
        &0_u64,
    )
}

fn disable_fraud_checks(client: &RefundContractClient<'_>, admin: &Address) {
    client.set_fraud_config(
        admin,
        &FraudConfig {
            max_refund_rate_bps: 10000,
            min_transactions_for_check: 1000,
            enabled: false,
        },
    );
}

#[test]
fn test_get_merchant_refunds_paginates_in_merchant_order() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant_a = Address::generate(&env);
    let merchant_b = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    disable_fraud_checks(&client, &admin);

    let a1 = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 1, 100);
    let _b1 = request_refund_for_merchant(&client, &env, &merchant_b, &customer, &token, 2, 200);
    let a2 = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 3, 300);
    let a3 = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 4, 400);
    let a4 = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 5, 500);

    let page = client.get_merchant_refunds(&merchant_a, &2u64, &1u64);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().id, a2);
    assert_eq!(page.get(1).unwrap().id, a3);

    let tail = client.get_merchant_refunds(&merchant_a, &3u64, &3u64);
    assert_eq!(tail.len(), 1);
    assert_eq!(tail.get(0).unwrap().id, a4);

    let empty_limit = client.get_merchant_refunds(&merchant_a, &0u64, &0u64);
    assert_eq!(empty_limit.len(), 0);

    let empty_offset = client.get_merchant_refunds(&merchant_a, &2u64, &10u64);
    assert_eq!(empty_offset.len(), 0);

    let all_a = client.get_merchant_refunds(&merchant_a, &10u64, &0u64);
    assert_eq!(all_a.len(), 4);
    assert_eq!(all_a.get(0).unwrap().id, a1);
    assert_eq!(all_a.get(1).unwrap().id, a2);
    assert_eq!(all_a.get(2).unwrap().id, a3);
    assert_eq!(all_a.get(3).unwrap().id, a4);
}

#[test]
fn test_get_merchant_refunds_by_status_filters_matches_and_offset() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant_a = Address::generate(&env);
    let merchant_b = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    disable_fraud_checks(&client, &admin);

    let approved_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 11, 110);
    let approved_two = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 12, 120);
    let rejected_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 13, 130);
    let processed_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 14, 140);
    let _pending_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 15, 150);
    let other_approved = request_refund_for_merchant(&client, &env, &merchant_b, &customer, &token, 16, 160);
    let other_rejected = request_refund_for_merchant(&client, &env, &merchant_b, &customer, &token, 17, 170);

    client.approve_refund(&admin, &approved_one);
    client.approve_refund(&admin, &approved_two);
    client.reject_refund(&admin, &rejected_one, &String::from_str(&env, "No"));
    client.approve_refund(&admin, &processed_one);
    client.process_refund(&admin, &processed_one);
    client.approve_refund(&admin, &other_approved);
    client.reject_refund(&admin, &other_rejected, &String::from_str(&env, "No"));

    let approved = client.get_merchant_refunds_by_status(
        &merchant_a,
        &RefundStatus::Approved,
        &10u64,
        &0u64,
    );
    assert_eq!(approved.len(), 2);
    assert_eq!(approved.get(0).unwrap().id, approved_one);
    assert_eq!(approved.get(1).unwrap().id, approved_two);

    let approved_offset = client.get_merchant_refunds_by_status(
        &merchant_a,
        &RefundStatus::Approved,
        &1u64,
        &1u64,
    );
    assert_eq!(approved_offset.len(), 1);
    assert_eq!(approved_offset.get(0).unwrap().id, approved_two);

    let rejected = client.get_merchant_refunds_by_status(
        &merchant_a,
        &RefundStatus::Rejected,
        &10u64,
        &0u64,
    );
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected.get(0).unwrap().id, rejected_one);

    let processed = client.get_merchant_refunds_by_status(
        &merchant_a,
        &RefundStatus::Processed,
        &10u64,
        &0u64,
    );
    assert_eq!(processed.len(), 1);
    assert_eq!(processed.get(0).unwrap().id, processed_one);

    let merchant_b_approved = client.get_merchant_refunds_by_status(
        &merchant_b,
        &RefundStatus::Approved,
        &10u64,
        &0u64,
    );
    assert_eq!(merchant_b_approved.len(), 1);
    assert_eq!(merchant_b_approved.get(0).unwrap().id, other_approved);
}

#[test]
fn test_get_merchant_pending_refunds_returns_only_requested() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant_a = Address::generate(&env);
    let merchant_b = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    disable_fraud_checks(&client, &admin);

    let pending_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 21, 210);
    let approved_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 22, 220);
    let rejected_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 23, 230);
    let processed_one = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 24, 240);
    let pending_two = request_refund_for_merchant(&client, &env, &merchant_a, &customer, &token, 25, 250);
    let _other_pending = request_refund_for_merchant(&client, &env, &merchant_b, &customer, &token, 26, 260);

    client.approve_refund(&admin, &approved_one);
    client.reject_refund(&admin, &rejected_one, &String::from_str(&env, "No"));
    client.approve_refund(&admin, &processed_one);
    client.process_refund(&admin, &processed_one);

    let pending = client.get_merchant_pending_refunds(&merchant_a);
    assert_eq!(pending.len(), 2);
    assert_eq!(pending.get(0).unwrap().id, pending_one);
    assert_eq!(pending.get(0).unwrap().status, RefundStatus::Requested);
    assert_eq!(pending.get(1).unwrap().id, pending_two);
    assert_eq!(pending.get(1).unwrap().status, RefundStatus::Requested);
}

#[test]
fn test_get_merchant_refund_summary_counts_current_statuses_and_amounts() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let empty_merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    disable_fraud_checks(&client, &admin);

    let pending_one = request_refund_for_merchant(&client, &env, &merchant, &customer, &token, 31, 50);
    let approved_one = request_refund_for_merchant(&client, &env, &merchant, &customer, &token, 32, 60);
    let rejected_one = request_refund_for_merchant(&client, &env, &merchant, &customer, &token, 33, 70);
    let processed_one = request_refund_for_merchant(&client, &env, &merchant, &customer, &token, 34, 80);
    let pending_two = request_refund_for_merchant(&client, &env, &merchant, &customer, &token, 35, 90);

    let _ = pending_one;
    let _ = pending_two;

    client.approve_refund(&admin, &approved_one);
    client.reject_refund(&admin, &rejected_one, &String::from_str(&env, "No"));
    client.approve_refund(&admin, &processed_one);
    client.process_refund(&admin, &processed_one);

    let summary = client.get_merchant_refund_summary(&merchant);
    assert_eq!(summary.total_requests, 5);
    assert_eq!(summary.total_approved, 1);
    assert_eq!(summary.total_rejected, 1);
    assert_eq!(summary.total_amount_refunded, 80);
    assert_eq!(summary.pending_count, 2);
    assert_eq!(summary.pending_amount, 140);

    let empty_summary = client.get_merchant_refund_summary(&empty_merchant);
    assert_eq!(empty_summary.total_requests, 0);
    assert_eq!(empty_summary.total_approved, 0);
    assert_eq!(empty_summary.total_rejected, 0);
    assert_eq!(empty_summary.total_amount_refunded, 0);
    assert_eq!(empty_summary.pending_count, 0);
    assert_eq!(empty_summary.pending_amount, 0);
}
