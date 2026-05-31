#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String,
};

fn setup_env() -> (Env, Address, Address, Address, Address, token::Client<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let customer = Address::generate(&env);
    let arbitrator1 = Address::generate(&env);
    let arbitrator2 = Address::generate(&env);
    let arbitrator3 = Address::generate(&env);

    // token contract
    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    let token_client = token::Client::new(&env, &contract_address);
    let token_admin = token::StellarAssetClient::new(&env, &contract_address);
    token_admin.mint(&merchant, &100000);
    token_admin.mint(&merchant2, &100000);
    token_admin.mint(&customer, &100000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    (
        env,
        admin,
        merchant,
        merchant2,
        customer,
        token_client.into_static(),
    )
}

#[test]
fn test_bucket_accuracy_and_range_aggregation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let customer = Address::generate(&env);

    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    let token_client = token::Client::new(&env, &contract_address);
    let token_admin = token::StellarAssetClient::new(&env, &contract_address);
    token_admin.mint(&merchant, &1000000);
    token_admin.mint(&merchant2, &1000000);
    token_admin.mint(&customer, &1000000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Disable fraud checks to avoid needing a payment contract.
    client.set_fraud_config(
        &admin,
        &FraudConfig {
            max_refund_rate_bps: 10000,
            min_transactions_for_check: 1000,
            enabled: false,
        },
    );

    let day = 86400u64;
    let day_start1 = 1 * day;
    let day_start2 = 2 * day;

    // Day 1 timestamps
    env.ledger().with_mut(|li| li.timestamp = day_start1 + 10);
    let r1 = client.request_refund(
        &merchant,
        &1,
        &customer,
        &100,
        &100,
        &token_client.address,
        &String::from_str(&env, "r1"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp() - 5),
    );

    client.approve_refund(&admin, &r1);
    env.ledger().with_mut(|li| li.timestamp = day_start1 + 20);
    client.process_refund(&admin, &r1);

    // Day 1 rejected
    env.ledger().with_mut(|li| li.timestamp = day_start1 + 200);
    let r2 = client.request_refund(
        &merchant,
        &2,
        &customer,
        &200,
        &200,
        &token_client.address,
        &String::from_str(&env, "r2"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp() - 5),
    );
    client.reject_refund(&admin, &r2, &String::from_str(&env, "no"));

    // Day 2 approved+processed for merchant2
    env.ledger().with_mut(|li| li.timestamp = day_start2 + 30);
    let r3 = client.request_refund(
        &merchant2,
        &3,
        &customer,
        &300,
        &300,
        &token_client.address,
        &String::from_str(&env, "r3"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp() - 5),
    );
    client.approve_refund(&admin, &r3);
    env.ledger().with_mut(|li| li.timestamp = day_start2 + 50);
    client.process_refund(&admin, &r3);

    // Range query day1..day2
    let range = client.get_refund_analytics_range(&(day_start1), &(day_start2 + day - 1));
    assert!(range.len() >= 2);

    let b1 = range.iter().find(|b| b.bucket_start == day_start1).unwrap();
    assert_eq!(b1.total_requests, 2);
    assert_eq!(b1.approved_count, 1);
    assert_eq!(b1.rejected_count, 1);
    assert_eq!(b1.total_amount, 100 + 200);

    let b2 = range.iter().find(|b| b.bucket_start == day_start2).unwrap();
    assert_eq!(b2.total_requests, 1);
    assert_eq!(b2.approved_count, 1);
    assert_eq!(b2.rejected_count, 0);
    assert_eq!(b2.total_amount, 300);

    // Merchant isolation
    let m1 = client.get_merchant_refund_analytics(&merchant);
    assert_eq!(m1.total_requested, 2);
    assert_eq!(m1.total_approved, 1);
    assert_eq!(m1.total_rejected, 1);
    assert_eq!(m1.total_amount_refunded, 100);

    let m2 = client.get_merchant_refund_analytics(&merchant2);
    assert_eq!(m2.total_requested, 1);
    assert_eq!(m2.total_approved, 1);
    assert_eq!(m2.total_rejected, 0);
    assert_eq!(m2.total_amount_refunded, 300);
}

#[test]
fn test_bucket_boundaries() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    let token_client = token::Client::new(&env, &contract_address);
    let token_admin = token::StellarAssetClient::new(&env, &contract_address);
    token_admin.mint(&merchant, &1000000);
    token_admin.mint(&customer, &1000000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.set_fraud_config(
        &admin,
        &FraudConfig {
            max_refund_rate_bps: 10000,
            min_transactions_for_check: 1000,
            enabled: false,
        },
    );

    let day = 86400u64;
    let ds = 5 * day;

    // start-of-day
    env.ledger().with_mut(|li| li.timestamp = ds);
    let r_start = client.request_refund(
        &merchant,
        &1,
        &customer,
        &100,
        &100,
        &token_client.address,
        &String::from_str(&env, "start"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp()),
    );
    client.approve_refund(&admin, &r_start);

    env.ledger().with_mut(|li| li.timestamp = ds + 1);
    client.process_refund(&admin, &r_start);

    // last-second-of-day
    env.ledger().with_mut(|li| li.timestamp = ds + day - 1);
    let r_end = client.request_refund(
        &merchant,
        &2,
        &customer,
        &200,
        &200,
        &token_client.address,
        &String::from_str(&env, "end"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp()),
    );
    client.approve_refund(&admin, &r_end);

    env.ledger().with_mut(|li| li.timestamp = ds + day - 1);
    client.process_refund(&admin, &r_end);

    let range = client.get_refund_analytics_range(&ds, &(ds + day - 1));
    let b = range.iter().find(|b| b.bucket_start == ds).unwrap();
    assert_eq!(b.total_requests, 2);
    assert_eq!(b.approved_count, 2);
    assert_eq!(b.rejected_count, 0);
}

#[test]
fn test_avg_processing_time_seconds_updates_on_process_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    let token_client = token::Client::new(&env, &contract_address);
    let token_admin = token::StellarAssetClient::new(&env, &contract_address);
    token_admin.mint(&merchant, &1000000);
    token_admin.mint(&customer, &1000000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.set_fraud_config(
        &admin,
        &FraudConfig {
            max_refund_rate_bps: 10000,
            min_transactions_for_check: 1000,
            enabled: false,
        },
    );

    let day = 86400u64;
    let ds = 10 * day;

    // Refund 1: processing_time = 50
    env.ledger().with_mut(|li| li.timestamp = ds + 100);
    let r1 = client.request_refund(
        &merchant,
        &1,
        &customer,
        &100,
        &100,
        &token_client.address,
        &String::from_str(&env, "r1"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp() - 1),
    );
    client.approve_refund(&admin, &r1);
    env.ledger().with_mut(|li| li.timestamp = ds + 150);
    client.process_refund(&admin, &r1);

    // Refund 2: processing_time = 100
    env.ledger().with_mut(|li| li.timestamp = ds + 200);
    let r2 = client.request_refund(
        &merchant,
        &2,
        &customer,
