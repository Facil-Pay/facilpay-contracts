#![cfg(test)]

//! Tests for #803: customer trust score and merchant auto-approve boost.

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

/// Stand-in for the payment contract's `get_payment_count_by_customer`, used
/// only to drive `check_fraud_signals` (which otherwise always returns `None`
/// with no payment contract configured).
#[contract]
pub struct MockPaymentCounts;

#[contractimpl]
impl MockPaymentCounts {
    pub fn check_payment_customer(_env: Env, _payment_id: u64, _customer: Address) -> bool {
        true
    }

    pub fn get_payment_count_by_customer(env: Env, _customer: Address) -> u64 {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "count"))
            .unwrap_or(0)
    }

    pub fn set_count(env: Env, count: u64) {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "count"), &count);
    }
}

struct Setup<'a> {
    env: Env,
    client: RefundContractClient<'a>,
    admin: Address,
    merchant: Address,
    token: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let token = crate::test_funded_token(&env, &contract_id);

    Setup {
        env,
        client,
        admin,
        merchant,
        token,
    }
}

/// `get_effective_refund_policy` falls back to the global *default* policy
/// (whose `merchant` field is a placeholder admin address, set during
/// `initialize`) when a merchant has no explicit policy of their own. The
/// auto-approve path in `create_refund` resolves `effective_merchant` from
/// that policy's `merchant` field, so any test exercising auto-approval
/// behavior must first give the merchant their own explicit policy —
/// otherwise `effective_merchant` silently becomes the admin placeholder
/// instead of the merchant, and merchant-level config (auto-approve-below,
/// requires-admin-approval, trust boost) is read from the wrong address.
fn give_merchant_own_policy(s: &Setup) {
    let mut tiers = Vec::new(&s.env);
    tiers.push_back(RefundTier {
        days_from_purchase: 365,
        max_refund_bps: 10000,
    });
    s.client.set_refund_policy(&s.merchant, &tiers);
}

fn request_refund(
    s: &Setup,
    customer: &Address,
    payment_id: u64,
    amount: i128,
    original_payment_amount: i128,
) -> u64 {
    s.client.request_refund(
        &s.merchant,
        &payment_id,
        customer,
        &amount,
        &original_payment_amount,
        &s.token,
        &String::from_str(&s.env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    )
}

#[test]
fn score_is_zero_with_no_history() {
    let s = setup();
    let customer = Address::generate(&s.env);
    assert_eq!(s.client.get_customer_trust_score(&customer), 0);
}

#[test]
fn score_increases_with_approved_history() {
    let s = setup();
    let customer = Address::generate(&s.env);

    let r1 = request_refund(&s, &customer, 1, 100, 10_000);
    s.client.approve_refund(&s.admin, &r1);
    let score_after_one = s.client.get_customer_trust_score(&customer);
    assert!(score_after_one > 0);

    // Advance time so tenure contributes too.
    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + 30 * 86400);

    let r2 = request_refund(&s, &customer, 2, 100, 10_000);
    s.client.approve_refund(&s.admin, &r2);
    let score_after_two = s.client.get_customer_trust_score(&customer);
    assert!(score_after_two > score_after_one);
    assert!(score_after_two <= 10000);
}

#[test]
fn flagged_customer_scores_zero_despite_good_history() {
    let s = setup();
    let mock_id = s.env.register(MockPaymentCounts, ());
    s.client.set_payment_contract_address(&s.admin, &mock_id);

    let customer = Address::generate(&s.env);

    // Build up enough refund history to exceed the default 20% fraud
    // threshold once payments are reported as low relative to refunds.
    for i in 0..6u64 {
        let r = request_refund(&s, &customer, i + 1, 10, 10_000);
        s.client.approve_refund(&s.admin, &r);
    }

    let mock_client = MockPaymentCountsClient::new(&s.env, &mock_id);
    // 6 refunds / 10 payments = 60% > default 20% fraud threshold, and
    // total_payments (10) >= default min_transactions_for_check (5).
    mock_client.set_count(&10u64);

    // Trigger fraud detection (check_fraud_signals is itself stateful).
    assert!(s.client.check_fraud_signals(&customer).is_some());

    assert_eq!(s.client.get_customer_trust_score(&customer), 0);
}

#[test]
fn boost_applies_only_when_score_meets_minimum() {
    let s = setup();
    give_merchant_own_policy(&s);
    let base_threshold = 100i128;
    let boosted_threshold = 5_000i128;

    s.client
        .set_auto_approve_below_ceiling(&s.admin, &boosted_threshold);
    s.client.set_requires_admin_approval(&s.merchant, &false);
    s.client
        .set_auto_approve_below(&s.merchant, &base_threshold);
    s.client.set_trust_boost_config(
        &s.merchant,
        &5000u32, // require a mid-range score
        &boosted_threshold,
    );

    // A brand-new customer has score 0: boost does not apply, and an amount
    // above the base threshold (but below the boosted one) is NOT auto-approved.
    let new_customer = Address::generate(&s.env);
    let r1 = request_refund(&s, &new_customer, 1, 1_000, 10_000);
    assert_eq!(s.client.get_refund(&r1).status, RefundStatus::Requested);

    // Build up a 100% approval history for a second customer: approval_bps
    // (50% weight) alone reaches 10000 * 0.5 = 5000bps, clearing the bar
    // without needing to also simulate tenure. These small amounts
    // auto-approve immediately against the merchant's own base threshold
    // (100), which still counts toward `total_approved`.
    let trusted_customer = Address::generate(&s.env);
    for i in 0..3u64 {
        let r = request_refund(&s, &trusted_customer, 100 + i, 10, 10_000);
        assert_eq!(s.client.get_refund(&r).status, RefundStatus::Approved);
    }

    let score = s.client.get_customer_trust_score(&trusted_customer);
    assert!(score >= 5000, "expected score >= 5000, got {score}");

    // Now an amount above the base threshold but within the boosted one
    // auto-approves for the trusted customer.
    let r2 = request_refund(&s, &trusted_customer, 200, 1_000, 10_000);
    assert_eq!(s.client.get_refund(&r2).status, RefundStatus::Approved);
}

#[test]
fn boost_is_still_capped_by_platform_ceiling() {
    let s = setup();
    give_merchant_own_policy(&s);
    s.client.set_auto_approve_below_ceiling(&s.admin, &500i128);
    s.client.set_requires_admin_approval(&s.merchant, &false);
    s.client.set_auto_approve_below(&s.merchant, &100i128);
    s.client
        .set_trust_boost_config(&s.merchant, &0u32, &5_000i128);

    // min_trust_score_bps = 0, so even a zero-history customer qualifies for
    // the boost, but the platform ceiling (500) must still cap it below the
    // configured boosted_threshold (5000).
    let customer = Address::generate(&s.env);
    let r1 = request_refund(&s, &customer, 1, 500, 10_000);
    assert_eq!(s.client.get_refund(&r1).status, RefundStatus::Approved);

    let r2 = request_refund(&s, &customer, 2, 501, 10_000);
    assert_eq!(s.client.get_refund(&r2).status, RefundStatus::Requested);
}

#[test]
fn remove_trust_boost_config_turns_it_off() {
    let s = setup();
    give_merchant_own_policy(&s);
    s.client
        .set_auto_approve_below_ceiling(&s.admin, &5_000i128);
    s.client.set_requires_admin_approval(&s.merchant, &false);
    s.client.set_auto_approve_below(&s.merchant, &100i128);
    s.client
        .set_trust_boost_config(&s.merchant, &0u32, &5_000i128);
    assert!(s.client.get_trust_boost_config(&s.merchant).is_some());

    // While the boost is active, an amount between the base and boosted
    // thresholds auto-approves (min_trust_score_bps = 0 qualifies anyone).
    let customer_a = Address::generate(&s.env);
    let r_before = request_refund(&s, &customer_a, 1, 1_000, 10_000);
    assert_eq!(
        s.client.get_refund(&r_before).status,
        RefundStatus::Approved
    );

    s.client.remove_trust_boost_config(&s.merchant);
    assert!(s.client.get_trust_boost_config(&s.merchant).is_none());

    // After removal, the same amount for a different customer falls back to
    // the unboosted base threshold and no longer auto-approves.
    let customer_b = Address::generate(&s.env);
    let r_after = request_refund(&s, &customer_b, 2, 1_000, 10_000);
    assert_eq!(
        s.client.get_refund(&r_after).status,
        RefundStatus::Requested
    );
}

#[test]
fn only_merchant_can_set_or_remove_their_boost() {
    let s = setup();
    let not_merchant = Address::generate(&s.env);

    // require_auth is satisfied by mock_all_auths, but the stored config is
    // always keyed by the `merchant` parameter itself (self-service), so
    // this documents that a caller can only configure their own boost by
    // construction, not a dedicated auth-mismatch error. Confirm the config
    // lands under the address actually passed in.
    s.client
        .set_trust_boost_config(&not_merchant, &1000u32, &2_000i128);
    assert!(s.client.get_trust_boost_config(&s.merchant).is_none());
    assert!(s.client.get_trust_boost_config(&not_merchant).is_some());
}

#[test]
fn invalid_boost_config_is_rejected() {
    let s = setup();
    let result = s
        .client
        .try_set_trust_boost_config(&s.merchant, &10001u32, &100i128);
    assert_eq!(result, Err(Ok(Error::Core(CoreError::InvalidAmount))));

    let result2 = s
        .client
        .try_set_trust_boost_config(&s.merchant, &100u32, &-1i128);
    assert_eq!(result2, Err(Ok(Error::Core(CoreError::InvalidAmount))));
}
