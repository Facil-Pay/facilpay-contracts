#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String, Vec};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn make_policy(env: &Env, merchant: &Address, client: &RefundContractClient) {
    let tiers = Vec::from_array(
        env,
        &[RefundTier {
            days_from_purchase: 30,
            max_refund_bps: 10000,
        }],
    );
    client.set_refund_policy(merchant, &tiers);
}

/// When a customer has a tier assigned but no corresponding tier policy entry
/// exists for the merchant, the refund request should fall back to the default
/// merchant policy cap and succeed for an amount within that cap.
#[test]
fn missing_tier_policy_falls_back_to_default_cap() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    make_policy(&env, &merchant, &client);

    // Assign customer to tier 1 but do NOT set a tier policy for tier 1
    client.set_customer_tier(&admin, &customer, &1u32);

    // Merchant has no tier policy for tier 1 → should fall back to 100% default
    // Payment created at timestamp 0, current time is 1000 (< 30 days) → within window
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &800i128,
        &1000i128,
        &token,
        &String::from_str(&env, "fallback test"),
        &RefundReasonCode::CustomerRequest,
        &0u64,
    );
    assert_eq!(refund_id, 1u64, "refund must succeed using default cap fallback");
}

/// When strict tier policy mode is enabled for a merchant and a customer has a
/// tier ID with no matching tier policy entry, request_refund must return
/// TierPolicyNotFound instead of falling back to the default cap.
#[test]
fn missing_tier_policy_returns_error_when_strict_mode_enabled() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    make_policy(&env, &merchant, &client);

    // Assign customer to tier 2 with no tier policy entry
    client.set_customer_tier(&admin, &customer, &2u32);
    // Enable strict mode for this merchant
    client.set_strict_tier_policy(&merchant, &true);

    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &500i128,
        &1000i128,
        &token,
        &String::from_str(&env, "strict mode test"),
        &RefundReasonCode::CustomerRequest,
        &0u64,
    );
    assert_eq!(
        result,
        Err(Ok(Error::TierPolicyNotFound)),
        "strict mode must reject refunds when tier policy is missing"
    );
}

/// When a customer has a tier assigned and the merchant has a matching tier
/// policy, the refund must be validated against that tier-specific cap rather
/// than the default merchant policy.
#[test]
fn existing_tier_policy_applied_correctly() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    // Default policy allows 100% refund within 30 days
    make_policy(&env, &merchant, &client);

    // Tier 3 (bronze) capped at 50%
    const TIER_ID: u32 = 3;
    client.set_customer_tier(&admin, &customer, &TIER_ID);
    client.set_customer_tier_policy(&merchant, &TIER_ID, &5000u32);

    // Requesting 60% of the original payment → must be rejected (exceeds 50% cap)
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &600i128,
        &1000i128,
        &token,
        &String::from_str(&env, "over tier cap"),
        &RefundReasonCode::CustomerRequest,
        &0u64,
    );
    assert_eq!(
        result,
        Err(Ok(Error::RefundExceedsPolicy)),
        "refund exceeding tier cap must be rejected"
    );

    // Requesting 40% → must succeed (within 50% cap)
    let refund_id = client.request_refund(
        &merchant,
        &2u64,
        &customer,
        &400i128,
        &1000i128,
        &token,
        &String::from_str(&env, "within tier cap"),
        &RefundReasonCode::CustomerRequest,
        &0u64,
    );
    assert_eq!(refund_id, 1u64, "refund within tier cap must succeed");
}
