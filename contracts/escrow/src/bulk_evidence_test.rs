#![cfg(test)]

use crate::*;
use soroban_sdk::{testutils::{Address as _, Ledger as _}, token, Address, Bytes, Env, Vec};

fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address, Address) {
    env.mock_all_auths();
    let id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let customer = Address::generate(env);
    token_admin.mint(&customer, &10_000i128);
    token_admin.mint(&id, &10_000i128);

    (client, admin, customer, Address::generate(env), token_addr)
}

fn make_disputed_escrow(
    env: &Env,
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> u64 {
    let escrow_id = client.create_escrow(
        customer, merchant, &500i128, token,
        &9999u64, &0u64, &0u64, &false,
    );
    client.dispute_escrow(customer, &escrow_id);
    escrow_id
}

#[test]
fn test_batch_submission_stores_all_items() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items: Vec<Bytes> = Vec::new(&env);
    items.push_back(Bytes::from_array(&env, &[1u8; 32]));
    items.push_back(Bytes::from_array(&env, &[2u8; 32]));
    items.push_back(Bytes::from_array(&env, &[3u8; 32]));

    let page_count = client.submit_evidence_batch(&customer, &escrow_id, &items);
    assert_eq!(page_count, 1u32);

    let page = client.get_evidence_page(&escrow_id, &0u32);
    assert_eq!(page.len(), 3u32);
}

#[test]
fn test_oversized_batch_rejected() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items: Vec<Bytes> = Vec::new(&env);
    for _ in 0..11u32 {
        items.push_back(Bytes::from_array(&env, &[0u8; 32]));
    }

    let result = client.try_submit_evidence_batch(&customer, &escrow_id, &items);
    assert_eq!(result, Err(Ok(Error::BatchTooLarge)));
}

#[test]
fn test_pagination_two_pages() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut batch1: Vec<Bytes> = Vec::new(&env);
    for _ in 0..10u32 {
        batch1.push_back(Bytes::from_array(&env, &[1u8; 32]));
    }
    client.submit_evidence_batch(&customer, &escrow_id, &batch1);

    let mut batch2: Vec<Bytes> = Vec::new(&env);
    batch2.push_back(Bytes::from_array(&env, &[2u8; 32]));
    let page_count = client.submit_evidence_batch(&merchant, &escrow_id, &batch2);
    assert_eq!(page_count, 2u32);

    assert_eq!(client.get_evidence_page(&escrow_id, &0u32).len(), 10u32);
    assert_eq!(client.get_evidence_page(&escrow_id, &1u32).len(), 1u32);
}

/// Exactly 10 items sits on the boundary — must be accepted, not rejected.
#[test]
fn test_exact_max_batch_accepted() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items: Vec<Bytes> = Vec::new(&env);
    for i in 0..10u32 {
        items.push_back(Bytes::from_array(&env, &[i as u8; 32]));
    }

    let result = client.try_submit_evidence_batch(&customer, &escrow_id, &items);
    assert!(result.is_ok(), "10-item batch should be accepted");

    let page = client.get_evidence_page(&escrow_id, &0u32);
    assert_eq!(page.len(), 10u32);
}

/// An empty batch (0 items) should succeed and create an empty page rather than
/// panic or error out.
#[test]
fn test_empty_batch_creates_empty_page() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let items: Vec<Bytes> = Vec::new(&env);
    let page_count = client.submit_evidence_batch(&customer, &escrow_id, &items);

    assert_eq!(page_count, 1u32);
    assert_eq!(client.get_evidence_page(&escrow_id, &0u32).len(), 0u32);
}

/// Calling submit_evidence_batch on a non-disputed escrow must return
/// `Error::NotDisputed`, regardless of batch size.
#[test]
fn test_batch_on_non_disputed_escrow_rejected() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);

    // Create escrow but do NOT dispute it — status stays Locked.
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500i128, &token,
        &9999u64, &0u64, &0u64, &false,
    );

    let mut items: Vec<Bytes> = Vec::new(&env);
    items.push_back(Bytes::from_array(&env, &[1u8; 32]));

    let result = client.try_submit_evidence_batch(&customer, &escrow_id, &items);
    assert_eq!(result, Err(Ok(Error::NotDisputed)));
}

/// A third party that is neither the customer nor the merchant must be rejected
/// with `Error::Unauthorized`.
#[test]
fn test_unauthorized_caller_rejected() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let stranger = Address::generate(&env);

    let mut items: Vec<Bytes> = Vec::new(&env);
    items.push_back(Bytes::from_array(&env, &[9u8; 32]));

    let result = client.try_submit_evidence_batch(&stranger, &escrow_id, &items);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// A rejected oversized batch must leave no partial state: the page count for
/// the escrow must remain 0 after the failed call.
#[test]
fn test_oversized_batch_leaves_no_partial_state() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items: Vec<Bytes> = Vec::new(&env);
    for _ in 0..15u32 {
        items.push_back(Bytes::from_array(&env, &[0u8; 32]));
    }

    // Must fail.
    let result = client.try_submit_evidence_batch(&customer, &escrow_id, &items);
    assert_eq!(result, Err(Ok(Error::BatchTooLarge)));

    // No page should have been written — the page is empty.
    let page = client.get_evidence_page(&escrow_id, &0u32);
    assert_eq!(page.len(), 0u32, "no partial state should be stored on failure");
}

#[test]
fn test_backward_compat_get_evidence_still_works() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    client.submit_evidence(
        &customer,
        &escrow_id,
        &soroban_sdk::String::from_str(&env, "QmOldHash"),
    );

    let items = client.get_evidence(&escrow_id, &10u64, &0u64);
    assert_eq!(items.len(), 1u32);
    assert_eq!(
        items.get(0).unwrap().ipfs_hash,
        soroban_sdk::String::from_str(&env, "QmOldHash")
    );
}

/// The merchant (not just the customer) must be allowed to submit a batch.
#[test]
fn test_merchant_can_submit_batch() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items: Vec<Bytes> = Vec::new(&env);
    items.push_back(Bytes::from_array(&env, &[0xaau8; 32]));
    items.push_back(Bytes::from_array(&env, &[0xbbu8; 32]));

    let result = client.try_submit_evidence_batch(&merchant, &escrow_id, &items);
    assert!(result.is_ok(), "merchant should be allowed to submit evidence");

    let page = client.get_evidence_page(&escrow_id, &0u32);
    assert_eq!(page.len(), 2u32);
}

/// Each successful batch call increments the page counter by exactly 1.
/// After three calls the returned value must be 3.
#[test]
fn test_page_count_increments_with_each_batch() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut batch: Vec<Bytes> = Vec::new(&env);
    batch.push_back(Bytes::from_array(&env, &[1u8; 32]));

    let c1 = client.submit_evidence_batch(&customer, &escrow_id, &batch);
    let c2 = client.submit_evidence_batch(&merchant, &escrow_id, &batch);
    let c3 = client.submit_evidence_batch(&customer, &escrow_id, &batch);

    assert_eq!(c1, 1u32);
    assert_eq!(c2, 2u32);
    assert_eq!(c3, 3u32);
}

/// Evidence pages are scoped per escrow. A batch submitted to escrow A must not
/// appear in escrow B's pages and vice-versa.
#[test]
fn test_evidence_pages_are_isolated_per_escrow() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);

    let escrow_a = make_disputed_escrow(&env, &client, &customer, &merchant, &token);
    let escrow_b = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items_a: Vec<Bytes> = Vec::new(&env);
    items_a.push_back(Bytes::from_array(&env, &[0xaau8; 32]));
    items_a.push_back(Bytes::from_array(&env, &[0xbbu8; 32]));

    let mut items_b: Vec<Bytes> = Vec::new(&env);
    items_b.push_back(Bytes::from_array(&env, &[0xccu8; 32]));

    client.submit_evidence_batch(&customer, &escrow_a, &items_a);
    client.submit_evidence_batch(&customer, &escrow_b, &items_b);

    // Escrow A has 2 items, escrow B has 1 — no bleed-over.
    assert_eq!(client.get_evidence_page(&escrow_a, &0u32).len(), 2u32);
    assert_eq!(client.get_evidence_page(&escrow_b, &0u32).len(), 1u32);
}

/// Every Evidence entry stored in a batch must record the caller as submitter.
#[test]
fn test_submitter_address_recorded_on_each_evidence_entry() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    let mut items: Vec<Bytes> = Vec::new(&env);
    items.push_back(Bytes::from_array(&env, &[1u8; 32]));
    items.push_back(Bytes::from_array(&env, &[2u8; 32]));
    items.push_back(Bytes::from_array(&env, &[3u8; 32]));

    client.submit_evidence_batch(&merchant, &escrow_id, &items);

    let page = client.get_evidence_page(&escrow_id, &0u32);
    for i in 0..page.len() {
        assert_eq!(
            page.get(i).unwrap().submitter,
            merchant,
            "entry {} should record merchant as submitter",
            i
        );
    }
}

/// After the 7-day evidence deadline has passed, submit_evidence_batch must
/// refuse with EvidenceDeadlinePassed (same guard as the single-item path).
#[test]
fn test_batch_rejected_after_evidence_deadline() {
    let env = Env::default();
    // Set a known base timestamp so we can calculate the deadline precisely.
    env.ledger().set_timestamp(1_000);

    let (client, _admin, customer, merchant, token) = setup(&env);

    // create_escrow and dispute_escrow are called at timestamp 1_000.
    // dispute_escrow sets evidence_deadline = dispute_started_at + 7 days.
    let escrow_id = make_disputed_escrow(&env, &client, &customer, &merchant, &token);

    const SEVEN_DAYS_SECS: u64 = 7 * 24 * 60 * 60;
    // Advance past the deadline.
    env.ledger().set_timestamp(1_000 + SEVEN_DAYS_SECS + 1);

    let mut items: Vec<Bytes> = Vec::new(&env);
    items.push_back(Bytes::from_array(&env, &[0xffu8; 32]));

    let result = client.try_submit_evidence_batch(&customer, &escrow_id, &items);
    assert_eq!(result, Err(Ok(Error::EvidenceDeadlinePassed)));
}
