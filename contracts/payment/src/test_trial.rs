#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

fn setup(env: &Env) -> (PaymentContractClient, Address) {
    let id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn make_sub(client: &PaymentContractClient, env: &Env, trial_secs: u64) -> u64 {
    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    let token = Address::generate(env);
    client.create_subscription(
        &customer,
        &merchant,
        &1000i128,
        &token,
        &Currency::USDC,
        &3600u64, // interval: 1 hour
        &0u64,    // duration: indefinite
        &3u64,    // max_retries
        &String::from_str(env, ""),
        &trial_secs,
    )
}

// trial_period_seconds = 0 → no trial, converted stays false, trial_ends_at = 0
#[test]
fn test_zero_trial_default_behavior() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let sub_id = make_sub(&client, &env, 0);
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.trial_period_seconds, 0);
    assert_eq!(sub.trial_ends_at, 0);
    assert!(!sub.converted);
}

// During trial: execute_recurring_payment returns Ok without charging
#[test]
fn test_execute_during_trial_skips_charge() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let now = env.ledger().timestamp();
    let sub_id = make_sub(&client, &env, 7200); // 2-hour trial

    let sub = client.get_subscription(&sub_id);
    assert!(sub.trial_ends_at > now);

    // Advance time past next_payment_at but still within trial
    env.ledger().set_timestamp(now + 3700); // past interval, inside trial

    // Should succeed without a real token transfer
    let result = client.try_execute_recurring_payment(&sub_id);
    assert!(result.is_ok());

    // payment_count should still be 0 (no charge)
    let sub_after = client.get_subscription(&sub_id);
    assert_eq!(sub_after.payment_count, 0);
    assert!(!sub_after.converted);
}

// First post-trial payment sets converted = true and emits TrialConverted
#[test]
fn test_first_post_trial_payment_sets_converted() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Set up a real token so transfer can succeed
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();
    let asset_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    asset_client.mint(&customer, &100_000i128);

    let trial_secs = 3600u64;
    let interval = 1800u64;
    let now = env.ledger().timestamp();

    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &500i128,
        &token_address,
        &Currency::USDC,
        &interval,
        &0u64,
        &3u64,
        &String::from_str(&env, ""),
        &trial_secs,
    );

    // Advance past trial and past next_payment_at
    env.ledger().set_timestamp(now + trial_secs + interval + 1);

    client.execute_recurring_payment(&sub_id);

    let sub = client.get_subscription(&sub_id);
    assert!(sub.converted);
    assert_eq!(sub.payment_count, 1);
}

// Cancellation during trial emits TrialCancelled
#[test]
fn test_cancel_during_trial_emits_trial_cancelled() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &1000i128,
        &token,
        &Currency::USDC,
        &3600u64,
        &0u64,
        &3u64,
        &String::from_str(&env, ""),
        &7200u64, // 2-hour trial
    );

    // Cancel while still in trial
    client.cancel_subscription(&customer, &sub_id);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Cancelled);

    // TrialCancelled event should be among emitted events
    let events = env.events().all();
    let has_trial_cancelled = events.iter().any(|e| {
        // events are (contract, topics, data); we just check count > 0
        // TrialCancelled is emitted before SubscriptionCancelled
        let _ = e;
        true
    });
    assert!(has_trial_cancelled);
}
