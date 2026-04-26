#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn make_refund(client: &RefundContractClient, env: &Env, merchant: &Address, payment_id: u64) -> u64 {
    let customer = Address::generate(env);
    let token = Address::generate(env);
    client.request_refund(
        merchant,
        &payment_id,
        &customer,
        &500i128,
        &1000i128,
        &token,
        &String::from_str(env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    )
}

// Default batch limit is 20
#[test]
fn test_default_batch_limit() {
    let env = Env::default();
    let (client, _) = setup(&env);
    assert_eq!(client.get_batch_refund_limit(), 20u32);
}

// Admin can change batch limit
#[test]
fn test_set_batch_limit() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    client.set_batch_refund_limit(&admin, &10u32);
    assert_eq!(client.get_batch_refund_limit(), 10u32);
}

// Full-success batch approve
#[test]
fn test_approve_refund_batch_all_success() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);
    let r3 = make_refund(&client, &env, &merchant, 3);

    let mut ids = Vec::new(&env);
    ids.push_back(r1);
    ids.push_back(r2);
    ids.push_back(r3);

    let results = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results.len(), 3);
    for i in 0..3 {
        assert!(results.get(i).unwrap().success);
        assert_eq!(results.get(i).unwrap().error_code, 0);
    }
}

// Full-success batch process
#[test]
fn test_process_refund_batch_all_success() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);

    // Approve first
    let mut approve_ids = Vec::new(&env);
    approve_ids.push_back(r1);
    approve_ids.push_back(r2);
    client.approve_refund_batch(&admin, &approve_ids);

    let mut process_ids = Vec::new(&env);
    process_ids.push_back(r1);
    process_ids.push_back(r2);

    let results = client.process_refund_batch(&admin, &process_ids);
    assert_eq!(results.len(), 2);
    for i in 0..2 {
        assert!(results.get(i).unwrap().success);
        assert_eq!(results.get(i).unwrap().amount_refunded, 500i128);
    }
}

// Partial failure: one bad id doesn't block others
#[test]
fn test_batch_partial_failure_isolation() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let bad_id = 9999u64; // does not exist
    let r3 = make_refund(&client, &env, &merchant, 3);

    let mut ids = Vec::new(&env);
    ids.push_back(r1);
    ids.push_back(bad_id);
    ids.push_back(r3);

    let results = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results.len(), 3);
    assert!(results.get(0).unwrap().success);
    assert!(!results.get(1).unwrap().success); // bad_id fails
    assert!(results.get(2).unwrap().success);
}

// Oversized batch is rejected
#[test]
fn test_oversized_batch_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    // Set limit to 2
    client.set_batch_refund_limit(&admin, &2u32);

    let mut ids = Vec::new(&env);
    ids.push_back(1u64);
    ids.push_back(2u64);
    ids.push_back(3u64); // exceeds limit of 2

    let results = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results.len(), 1);
    assert!(!results.get(0).unwrap().success);
    assert_eq!(results.get(0).unwrap().error_code, Error::BatchRefundTooLarge as u32);
}
