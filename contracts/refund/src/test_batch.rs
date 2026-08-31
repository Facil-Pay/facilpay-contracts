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

fn make_refund(
    client: &RefundContractClient,
    env: &Env,
    merchant: &Address,
    payment_id: u64,
) -> u64 {
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

fn refund_status(client: &RefundContractClient, refund_id: u64) -> RefundStatus {
    client.get_refund(&refund_id).status
}

// ── Required batch_refund() semantics tests ─────────────────────────────────
// approve_refund_batch / process_refund_batch implement partial-success mode:
// per-item validation failures are skipped while valid items continue.
// Batch-level validation failures (e.g. oversized batch) abort the entire call.

#[test]
fn batch_with_all_valid_items_completes_successfully() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);
    let r3 = make_refund(&client, &env, &merchant, 3);

    let mut approve_ids = Vec::new(&env);
    approve_ids.push_back(r1);
    approve_ids.push_back(r2);
    approve_ids.push_back(r3);

    let approve_results = client.approve_refund_batch(&admin, &approve_ids);
    assert_eq!(approve_results.len(), 3);
    for i in 0..3 {
        let result = approve_results.get(i).unwrap();
        assert!(result.success);
        assert_eq!(result.error_code, 0);
        assert_eq!(
            refund_status(&client, result.refund_id),
            RefundStatus::Approved
        );
    }

    let mut process_ids = Vec::new(&env);
    process_ids.push_back(r1);
    process_ids.push_back(r2);
    process_ids.push_back(r3);

    let process_results = client.process_refund_batch(&admin, &process_ids);
    assert_eq!(process_results.len(), 3);
    for i in 0..3 {
        let result = process_results.get(i).unwrap();
        assert!(result.success);
        assert_eq!(result.error_code, 0);
        assert_eq!(result.amount_refunded, 500i128);
        assert_eq!(
            refund_status(&client, result.refund_id),
            RefundStatus::Processed
        );
    }
}

#[test]
fn batch_processes_valid_items_and_skips_invalid() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);
    let r3 = make_refund(&client, &env, &merchant, 3);
    let missing_id = 9999u64;

    // Pre-approve r2 so a later approve attempt fails with InvalidStatus.
    client.approve_refund(&admin, &r2);

    let mut approve_ids = Vec::new(&env);
    approve_ids.push_back(r1);
    approve_ids.push_back(missing_id);
    approve_ids.push_back(r2);
    approve_ids.push_back(r3);

    let approve_results = client.approve_refund_batch(&admin, &approve_ids);
    assert_eq!(approve_results.len(), 4);
    assert!(approve_results.get(0).unwrap().success);
    assert!(!approve_results.get(1).unwrap().success);
    assert_eq!(
        approve_results.get(1).unwrap().error_code,
        Error::Core(CoreError::RefundNotFound).to_u32()
    );
    assert!(!approve_results.get(2).unwrap().success);
    assert_eq!(
        approve_results.get(2).unwrap().error_code,
        Error::Core(CoreError::InvalidStatus).to_u32()
    );
    assert!(approve_results.get(3).unwrap().success);

    assert_eq!(refund_status(&client, r1), RefundStatus::Approved);
    assert_eq!(refund_status(&client, r2), RefundStatus::Approved);
    assert_eq!(refund_status(&client, r3), RefundStatus::Approved);

    // Pre-process r1 so batch processing hits an already-processed item.
    client.process_refund(&admin, &r1);

    let mut process_ids = Vec::new(&env);
    process_ids.push_back(r1);
    process_ids.push_back(missing_id);
    process_ids.push_back(r2);
    process_ids.push_back(r3);

    let process_results = client.process_refund_batch(&admin, &process_ids);
    assert_eq!(process_results.len(), 4);
    assert!(!process_results.get(0).unwrap().success);
    assert_eq!(
        process_results.get(0).unwrap().error_code,
        Error::Core(CoreError::InvalidStatus).to_u32()
    );
    assert!(!process_results.get(1).unwrap().success);
    assert_eq!(
        process_results.get(1).unwrap().error_code,
        Error::Core(CoreError::RefundNotFound).to_u32()
    );
    assert!(process_results.get(2).unwrap().success);
    assert!(process_results.get(3).unwrap().success);

    assert_eq!(refund_status(&client, r1), RefundStatus::Processed);
    assert_eq!(refund_status(&client, r2), RefundStatus::Processed);
    assert_eq!(refund_status(&client, r3), RefundStatus::Processed);
}

#[test]
fn batch_aborts_all_on_single_validation_failure() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    client.set_batch_refund_limit(&admin, &2u32);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);
    let r3 = make_refund(&client, &env, &merchant, 3);

    let mut approve_ids = Vec::new(&env);
    approve_ids.push_back(r1);
    approve_ids.push_back(r2);
    approve_ids.push_back(r3);

    let approve_results = client.approve_refund_batch(&admin, &approve_ids);
    assert_eq!(approve_results.len(), 1);
    assert!(!approve_results.get(0).unwrap().success);
    assert_eq!(
        approve_results.get(0).unwrap().error_code,
        Error::Core(CoreError::BatchRefundTooLarge).to_u32()
    );

    // Batch-level rejection must not mutate any refund in the batch.
    assert_eq!(refund_status(&client, r1), RefundStatus::Requested);
    assert_eq!(refund_status(&client, r2), RefundStatus::Requested);
    assert_eq!(refund_status(&client, r3), RefundStatus::Requested);

    // Approve two items individually, then attempt an oversized process batch.
    client.approve_refund(&admin, &r1);
    client.approve_refund(&admin, &r2);

    let mut process_ids = Vec::new(&env);
    process_ids.push_back(r1);
    process_ids.push_back(r2);
    process_ids.push_back(r3);

    let process_results = client.process_refund_batch(&admin, &process_ids);
    assert_eq!(process_results.len(), 1);
    assert!(!process_results.get(0).unwrap().success);
    assert_eq!(
        process_results.get(0).unwrap().error_code,
        Error::Core(CoreError::BatchRefundTooLarge).to_u32()
    );

    assert_eq!(refund_status(&client, r1), RefundStatus::Approved);
    assert_eq!(refund_status(&client, r2), RefundStatus::Approved);
    assert_eq!(refund_status(&client, r3), RefundStatus::Requested);
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
    assert_eq!(
        results.get(0).unwrap().error_code,
        Error::Core(CoreError::BatchRefundTooLarge).to_u32()
    );
}

// ── 5 additional batch tests ─────────────────────────────────────────────────

/// Empty batch returns an empty results vector without panicking.
#[test]
fn batch_with_empty_ids_returns_empty_results() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let approve_results = client.approve_refund_batch(&admin, &Vec::new(&env));
    assert_eq!(approve_results.len(), 0);

    let process_results = client.process_refund_batch(&admin, &Vec::new(&env));
    assert_eq!(process_results.len(), 0);
}

/// Duplicate IDs in the same batch: the first occurrence succeeds, the
/// second fails with InvalidStatus because the refund is already Approved.
#[test]
fn batch_approve_duplicate_id_second_occurrence_fails() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);

    let mut ids = Vec::new(&env);
    ids.push_back(r1);
    ids.push_back(r1); // duplicate

    let results = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results.len(), 2);

    // First attempt succeeds.
    assert!(results.get(0).unwrap().success);
    assert_eq!(results.get(0).unwrap().refund_id, r1);

    // Second attempt on the same ID fails because it's already Approved.
    assert!(!results.get(1).unwrap().success);
    assert_eq!(
        results.get(1).unwrap().error_code,
        Error::Core(CoreError::InvalidStatus).to_u32()
    );

    assert_eq!(refund_status(&client, r1), RefundStatus::Approved);
}

/// Batch results preserve the exact input order, including the positions
/// of failures, so callers can correlate results by index.
#[test]
fn batch_results_preserve_input_order() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);
    let r3 = make_refund(&client, &env, &merchant, 3);
    let bad = 8888u64;

    // Insert bad ID in the middle.
    let mut ids = Vec::new(&env);
    ids.push_back(r3); // index 0
    ids.push_back(bad); // index 1  (not found)
    ids.push_back(r1); // index 2
    ids.push_back(r2); // index 3

    let results = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results.len(), 4);

    assert_eq!(results.get(0).unwrap().refund_id, r3);
    assert!(results.get(0).unwrap().success);

    assert_eq!(results.get(1).unwrap().refund_id, bad);
    assert!(!results.get(1).unwrap().success);
    assert_eq!(
        results.get(1).unwrap().error_code,
        Error::Core(CoreError::RefundNotFound).to_u32()
    );

    assert_eq!(results.get(2).unwrap().refund_id, r1);
    assert!(results.get(2).unwrap().success);

    assert_eq!(results.get(3).unwrap().refund_id, r2);
    assert!(results.get(3).unwrap().success);
}

/// Batch process returns the correct amount_refunded for each item and
/// the amount is zero for failed items.
#[test]
fn batch_process_amount_refunded_correctness() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    let r1 = make_refund(&client, &env, &merchant, 1); // amount = 500
    let r2 = make_refund(&client, &env, &merchant, 2); // amount = 500
    let bad = 7777u64;

    // Approve both valid refunds first.
    let mut approve_ids = Vec::new(&env);
    approve_ids.push_back(r1);
    approve_ids.push_back(r2);
    client.approve_refund_batch(&admin, &approve_ids);

    let mut process_ids = Vec::new(&env);
    process_ids.push_back(r1);
    process_ids.push_back(bad); // fails
    process_ids.push_back(r2);

    let results = client.process_refund_batch(&admin, &process_ids);
    assert_eq!(results.len(), 3);

    // Successful items carry the actual refund amount.
    assert!(results.get(0).unwrap().success);
    assert_eq!(results.get(0).unwrap().amount_refunded, 500i128);

    // Failed item carries a zero amount.
    assert!(!results.get(1).unwrap().success);
    assert_eq!(results.get(1).unwrap().amount_refunded, 0i128);

    assert!(results.get(2).unwrap().success);
    assert_eq!(results.get(2).unwrap().amount_refunded, 500i128);
}

/// Raising the batch limit after it was lowered allows a previously
/// rejected oversized batch to succeed.
#[test]
fn batch_limit_increase_allows_previously_oversized_batch() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);

    // Start with a tight limit.
    client.set_batch_refund_limit(&admin, &2u32);

    let r1 = make_refund(&client, &env, &merchant, 1);
    let r2 = make_refund(&client, &env, &merchant, 2);
    let r3 = make_refund(&client, &env, &merchant, 3);

    let mut ids = Vec::new(&env);
    ids.push_back(r1);
    ids.push_back(r2);
    ids.push_back(r3);

    // Three items against a limit of 2 — must be rejected.
    let results = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results.len(), 1);
    assert!(!results.get(0).unwrap().success);
    assert_eq!(
        results.get(0).unwrap().error_code,
        Error::Core(CoreError::BatchRefundTooLarge).to_u32()
    );
    assert_eq!(client.get_batch_refund_limit(), 2u32);

    // All refunds remain untouched.
    assert_eq!(refund_status(&client, r1), RefundStatus::Requested);
    assert_eq!(refund_status(&client, r2), RefundStatus::Requested);
    assert_eq!(refund_status(&client, r3), RefundStatus::Requested);

    // Raise the limit to accommodate the batch.
    client.set_batch_refund_limit(&admin, &5u32);
    assert_eq!(client.get_batch_refund_limit(), 5u32);

    // Same batch now succeeds.
    let results2 = client.approve_refund_batch(&admin, &ids);
    assert_eq!(results2.len(), 3);
    for i in 0..3 {
        assert!(results2.get(i).unwrap().success);
    }
    assert_eq!(refund_status(&client, r1), RefundStatus::Approved);
    assert_eq!(refund_status(&client, r2), RefundStatus::Approved);
    assert_eq!(refund_status(&client, r3), RefundStatus::Approved);
}
