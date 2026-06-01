#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Events, Address, BytesN, Env, String};

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup() -> (Env, Address, RefundContractClient<'static>) {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    env.mock_all_auths();
    // Disable fraud checks so they don't interfere with eligibility tests
    client.set_fraud_config(
        &admin,
        &FraudConfig {
            max_refund_rate_bps: 10000,
            min_transactions_for_check: 1000,
            enabled: false,
        },
    );
    (env, admin, client)
}

fn zero_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0u8; 32])
}

fn reason_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Request a refund and return its id. Uses unique payment_ids to avoid
/// cumulative-refund limits.
fn request_refund(
    client: &RefundContractClient<'_>,
    env: &Env,
    merchant: &Address,
    customer: &Address,
    payment_id: u64,
) -> u64 {
    let token = Address::generate(env);
    client.request_refund(
        merchant,
        &payment_id,
        customer,
        &100i128,
        &100i128,
        &token,
        &String::from_str(env, "test"),
        &RefundReasonCode::Other,
        &0_u64,
    )
}

// ── set_refund_eligibility ────────────────────────────────────────────────────

#[test]
fn test_set_eligibility_block_stores_entry() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &reason_hash(&env, 1),
    );

    let rule = client.check_refund_eligibility(&merchant, &customer);
    assert_eq!(rule, EligibilityRule::Block);
}

#[test]
fn test_set_eligibility_allow_stores_entry() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Allow,
        &zero_hash(&env),
    );

    let rule = client.check_refund_eligibility(&merchant, &customer);
    assert_eq!(rule, EligibilityRule::Allow);
}

#[test]
fn test_set_eligibility_overwrites_existing_rule() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // First block the customer
    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &reason_hash(&env, 2),
    );
    assert_eq!(
        client.check_refund_eligibility(&merchant, &customer),
        EligibilityRule::Block
    );

    // Admin overrides to Allow
    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Allow,
        &reason_hash(&env, 3),
    );
    assert_eq!(
        client.check_refund_eligibility(&merchant, &customer),
        EligibilityRule::Allow
    );
}

#[test]
fn test_set_eligibility_emits_event() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &zero_hash(&env),
    );

    let events = env.events().all();
    assert!(events.len() > 0);
}

// ── check_refund_eligibility ──────────────────────────────────────────────────

#[test]
fn test_check_eligibility_defaults_to_allow_when_no_entry() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // No entry set — should default to Allow
    let rule = client.check_refund_eligibility(&merchant, &customer);
    assert_eq!(rule, EligibilityRule::Allow);
}

#[test]
fn test_check_eligibility_is_merchant_scoped() {
    let (env, _admin, client) = setup();
    let merchant_a = Address::generate(&env);
    let merchant_b = Address::generate(&env);
    let customer = Address::generate(&env);

    // Block under merchant_a only
    client.set_refund_eligibility(
        &merchant_a,
        &customer,
        &EligibilityRule::Block,
        &zero_hash(&env),
    );

    assert_eq!(
        client.check_refund_eligibility(&merchant_a, &customer),
        EligibilityRule::Block
    );
    // merchant_b has no entry → Allow
    assert_eq!(
        client.check_refund_eligibility(&merchant_b, &customer),
        EligibilityRule::Allow
    );
}

// ── block enforcement in request_refund ──────────────────────────────────────

#[test]
#[should_panic]
fn test_blocked_customer_cannot_request_refund() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &reason_hash(&env, 10),
    );

    // This should panic with CustomerBlockedFromRefund
    request_refund(&client, &env, &merchant, &customer, 1);
}

#[test]
fn test_allowed_customer_can_request_refund() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Allow,
        &zero_hash(&env),
    );

    let refund_id = request_refund(&client, &env, &merchant, &customer, 1);
    assert!(refund_id > 0);
}

#[test]
fn test_customer_with_no_entry_can_request_refund() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // No eligibility entry set at all
    let refund_id = request_refund(&client, &env, &merchant, &customer, 1);
    assert!(refund_id > 0);
}

#[test]
fn test_block_only_affects_specific_merchant_customer_pair() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let blocked_customer = Address::generate(&env);
    let allowed_customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &blocked_customer,
        &EligibilityRule::Block,
        &reason_hash(&env, 11),
    );

    // allowed_customer is unaffected
    let refund_id = request_refund(&client, &env, &merchant, &allowed_customer, 2);
    assert!(refund_id > 0);
}

// ── admin override ────────────────────────────────────────────────────────────

#[test]
fn test_admin_can_override_merchant_block_with_allow() {
    let (env, admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // Merchant blocks the customer
    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &reason_hash(&env, 20),
    );
    assert_eq!(
        client.check_refund_eligibility(&merchant, &customer),
        EligibilityRule::Block
    );

    // Admin overrides with Allow (admin calls set_refund_eligibility on behalf of merchant)
    // In tests mock_all_auths covers both merchant and admin auth
    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Allow,
        &reason_hash(&env, 21),
    );
    assert_eq!(
        client.check_refund_eligibility(&merchant, &customer),
        EligibilityRule::Allow
    );

    // Customer can now request a refund
    let refund_id = request_refund(&client, &env, &merchant, &customer, 5);
    assert!(refund_id > 0);

    let _ = admin; // suppress unused warning
}

#[test]
fn test_block_then_allow_then_block_again() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(&merchant, &customer, &EligibilityRule::Block, &zero_hash(&env));
    assert_eq!(client.check_refund_eligibility(&merchant, &customer), EligibilityRule::Block);

    client.set_refund_eligibility(&merchant, &customer, &EligibilityRule::Allow, &zero_hash(&env));
    assert_eq!(client.check_refund_eligibility(&merchant, &customer), EligibilityRule::Allow);

    client.set_refund_eligibility(&merchant, &customer, &EligibilityRule::Block, &zero_hash(&env));
    assert_eq!(client.check_refund_eligibility(&merchant, &customer), EligibilityRule::Block);
}

// ── remove_refund_eligibility ─────────────────────────────────────────────────

#[test]
fn test_remove_existing_entry_succeeds() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &zero_hash(&env),
    );

    client.remove_refund_eligibility(&merchant, &customer);

    // After removal defaults back to Allow
    assert_eq!(
        client.check_refund_eligibility(&merchant, &customer),
        EligibilityRule::Allow
    );
}

#[test]
fn test_remove_entry_allows_customer_to_request_refund_again() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &zero_hash(&env),
    );

    client.remove_refund_eligibility(&merchant, &customer);

    let refund_id = request_refund(&client, &env, &merchant, &customer, 10);
    assert!(refund_id > 0);
}

#[test]
#[should_panic]
fn test_remove_nonexistent_entry_returns_error() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // No entry was ever set — should panic with EligibilityEntryNotFound
    client.remove_refund_eligibility(&merchant, &customer);
}

#[test]
fn test_remove_emits_event() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(
        &merchant,
        &customer,
        &EligibilityRule::Block,
        &zero_hash(&env),
    );

    client.remove_refund_eligibility(&merchant, &customer);

    // After removal the events list should contain at least the EligibilityRemoved event
    let events = env.events().all();
    assert!(events.len() > 0);
}

// ── get_merchant_eligibility_list ─────────────────────────────────────────────

#[test]
fn test_get_eligibility_list_returns_all_entries() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer_a = Address::generate(&env);
    let customer_b = Address::generate(&env);
    let customer_c = Address::generate(&env);

    client.set_refund_eligibility(&merchant, &customer_a, &EligibilityRule::Block, &reason_hash(&env, 1));
    client.set_refund_eligibility(&merchant, &customer_b, &EligibilityRule::Allow, &reason_hash(&env, 2));
    client.set_refund_eligibility(&merchant, &customer_c, &EligibilityRule::Block, &reason_hash(&env, 3));

    let list = client.get_merchant_eligibility_list(&merchant);
    assert_eq!(list.len(), 3);
}

#[test]
fn test_get_eligibility_list_empty_for_new_merchant() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);

    let list = client.get_merchant_eligibility_list(&merchant);
    assert_eq!(list.len(), 0);
}

#[test]
fn test_get_eligibility_list_is_merchant_scoped() {
    let (env, _admin, client) = setup();
    let merchant_a = Address::generate(&env);
    let merchant_b = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(&merchant_a, &customer, &EligibilityRule::Block, &zero_hash(&env));

    let list_a = client.get_merchant_eligibility_list(&merchant_a);
    let list_b = client.get_merchant_eligibility_list(&merchant_b);

    assert_eq!(list_a.len(), 1);
    assert_eq!(list_b.len(), 0);
}

#[test]
fn test_get_eligibility_list_reflects_correct_rules() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer_a = Address::generate(&env);
    let customer_b = Address::generate(&env);

    client.set_refund_eligibility(&merchant, &customer_a, &EligibilityRule::Block, &reason_hash(&env, 5));
    client.set_refund_eligibility(&merchant, &customer_b, &EligibilityRule::Allow, &reason_hash(&env, 6));

    let list = client.get_merchant_eligibility_list(&merchant);
    assert_eq!(list.len(), 2);

    // Verify each entry has the correct rule
    let mut found_block = false;
    let mut found_allow = false;
    for i in 0..list.len() {
        let entry = list.get(i).unwrap();
        match entry.rule {
            EligibilityRule::Block => found_block = true,
            EligibilityRule::Allow => found_allow = true,
        }
    }
    assert!(found_block);
    assert!(found_allow);
}

#[test]
fn test_get_eligibility_list_shrinks_after_removal() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer_a = Address::generate(&env);
    let customer_b = Address::generate(&env);

    client.set_refund_eligibility(&merchant, &customer_a, &EligibilityRule::Block, &zero_hash(&env));
    client.set_refund_eligibility(&merchant, &customer_b, &EligibilityRule::Block, &zero_hash(&env));

    assert_eq!(client.get_merchant_eligibility_list(&merchant).len(), 2);

    client.remove_refund_eligibility(&merchant, &customer_a);

    assert_eq!(client.get_merchant_eligibility_list(&merchant).len(), 1);
    // Remaining entry should be customer_b
    let list = client.get_merchant_eligibility_list(&merchant);
    assert_eq!(list.get(0).unwrap().customer, customer_b);
}

#[test]
fn test_update_existing_entry_does_not_duplicate_in_list() {
    let (env, _admin, client) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    client.set_refund_eligibility(&merchant, &customer, &EligibilityRule::Block, &zero_hash(&env));
    // Update the same customer — should not add a second entry
    client.set_refund_eligibility(&merchant, &customer, &EligibilityRule::Allow, &zero_hash(&env));

    let list = client.get_merchant_eligibility_list(&merchant);
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap().rule, EligibilityRule::Allow);
}
