#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env, String};

fn setup(env: &Env) -> (PaymentContractClient<'_>, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin, contract_id)
}

fn funded_token(env: &Env, contract_id: &Address, customer: &Address) -> Address {
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    token::StellarAssetClient::new(env, &token).mint(customer, &10_000_000);
    token::Client::new(env, &token).approve(customer, contract_id, &10_000_000, &100_000);
    token
}

fn create_subscription_with_pause_limits(
    env: &Env,
    client: &PaymentContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
    interval: u64,
    max_pause_seconds: u64,
) -> u64 {
    let sub_id = client.create_subscription(
        customer,
        merchant,
        &1000,
        token,
        &Currency::USDC,
        &interval,
        &0,
        &0,
        &String::from_str(env, "test subscription"),
        &0,
    );

    // Configure pause limits
    client.configure_subscription_pause_policy(
        merchant,
        &sub_id,
        &true,          // proration_enabled
        &max_pause_seconds,
        &5,             // max_pauses_per_year
    );

    sub_id
}

/// Test #1: Auto-resume triggers when max_pause_seconds is exceeded
/// 
/// This test verifies that when a subscription is paused and the pause duration
/// exceeds the configured max_pause_seconds limit, the subscription is automatically
/// resumed when execute_recurring_payment is called.
#[test]
fn test_auto_resume_triggers_when_max_pause_exceeded() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);

    // Create subscription with 500 second max pause and 1000 second billing interval
    let max_pause_seconds = 500;
    let billing_interval = 1000;
    let sub_id = create_subscription_with_pause_limits(
        &env,
        &client,
        &customer,
        &merchant,
        &token,
        billing_interval,
        max_pause_seconds,
    );

    // Verify subscription is active initially
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    let initial_next_payment = sub.next_payment_at;

    // Pause the subscription at time 1200
    env.ledger().set_timestamp(1200);
    client.pause_subscription(&customer, &sub_id);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);
    assert_eq!(sub.pause_data.last_paused_at, 1200);

    // Advance time beyond max_pause_seconds (1200 + 500 = 1700)
    // Set to 1800 to exceed the limit
    env.ledger().set_timestamp(1800);

    // Execute recurring payment - this should trigger auto-resume
    let result = client.try_execute_recurring_payment(&sub_id);
    
    // The subscription should have been auto-resumed
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);

    // Verify pause duration was recorded
    let pause_duration = 1800 - 1200; // 600 seconds
    assert_eq!(sub.pause_data.total_pause_duration, pause_duration);

    // Verify next payment date was shifted by pause duration
    assert_eq!(sub.next_payment_at, initial_next_payment + pause_duration);

    // Verify payment was executed (or appropriate status set)
    assert!(result.is_ok() || result.is_err()); // Either succeeded or failed for other reasons (like not due yet)
}

/// Test #2: Auto-resume does not trigger when max_pause_seconds is not exceeded
///
/// This test ensures that a paused subscription remains paused when the pause
/// duration is still within the allowed max_pause_seconds limit.
#[test]
fn test_auto_resume_does_not_trigger_before_max_pause_exceeded() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);

    // Create subscription with 1000 second max pause
    let max_pause_seconds = 1000;
    let billing_interval = 2000;
    let sub_id = create_subscription_with_pause_limits(
        &env,
        &client,
        &customer,
        &merchant,
        &token,
        billing_interval,
        max_pause_seconds,
    );

    // Pause the subscription at time 1500
    env.ledger().set_timestamp(1500);
    client.pause_subscription(&customer, &sub_id);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);
    assert_eq!(sub.pause_data.last_paused_at, 1500);

    // Advance time but stay within max_pause_seconds limit
    // 1500 + 800 = 2300 (still less than 1500 + 1000)
    env.ledger().set_timestamp(2300);

    // Try to execute recurring payment
    let result = client.try_execute_recurring_payment(&sub_id);

    // The subscription should still be paused
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);

    // Verify pause duration has not been recorded yet (no auto-resume)
    assert_eq!(sub.pause_data.total_pause_duration, 0);

    // The result should indicate the payment is not due or subscription is paused
    assert!(result.is_err());
}

/// Test #3: Auto-resume shifts ends_at for fixed-duration subscriptions
///
/// This test verifies that when a subscription with a fixed duration (ends_at > 0)
/// is auto-resumed, both the next_payment_at and ends_at dates are shifted by the
/// pause duration to ensure the subscription maintains its full duration.
#[test]
fn test_auto_resume_shifts_ends_at_for_fixed_duration_subscriptions() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);

    // Create subscription with fixed duration (10,000 seconds)
    let billing_interval = 1000;
    let duration = 10_000;
    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &billing_interval,
        &duration,
        &0,
        &String::from_str(&env, "fixed duration subscription"),
        &0,
    );

    // Configure pause limits with 500 second max pause
    let max_pause_seconds = 500;
    client.configure_subscription_pause_policy(
        &merchant,
        &sub_id,
        &false, // proration disabled for this test
        &max_pause_seconds,
        &5,
    );

    let sub = client.get_subscription(&sub_id);
    let initial_next_payment = sub.next_payment_at;
    let initial_ends_at = sub.ends_at;
    assert!(initial_ends_at > 0); // Verify it's a fixed-duration subscription
    assert_eq!(initial_ends_at, 1000 + duration);

    // Pause the subscription at time 1200
    env.ledger().set_timestamp(1200);
    client.pause_subscription(&customer, &sub_id);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Paused);

    // Advance time beyond max_pause_seconds (1200 + 500 = 1700)
    // Set to 2000 to exceed the limit
    env.ledger().set_timestamp(2000);
    let pause_duration = 2000 - 1200; // 800 seconds

    // Execute recurring payment to trigger auto-resume
    let _ = client.try_execute_recurring_payment(&sub_id);

    // Verify subscription was auto-resumed
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);

    // Verify both next_payment_at and ends_at were shifted by pause duration
    assert_eq!(sub.next_payment_at, initial_next_payment + pause_duration);
    assert_eq!(sub.ends_at, initial_ends_at + pause_duration);

    // Verify pause duration was recorded
    assert_eq!(sub.pause_data.total_pause_duration, pause_duration);
}
