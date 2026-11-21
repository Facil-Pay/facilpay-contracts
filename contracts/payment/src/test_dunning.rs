#![cfg(test)]
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String,
};

use crate::{
    BasicError, Currency, DunningConfig, Error, PaymentContract, PaymentContractClient,
    SubscriptionError, SubscriptionStatus,
};

fn setup() -> (Env, PaymentContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

/// Creates a subscription with no token allowance/funding for the customer,
/// so the first `execute_recurring_payment` after it's due fails and the
/// subscription enters dunning.
fn setup_failing_subscription(
    env: &Env,
    client: &PaymentContractClient<'_>,
    customer: &Address,
    merchant: &Address,
) -> (u64, Address) {
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    env.ledger().set_timestamp(1000);

    let sub_id = client.create_subscription(
        customer,
        merchant,
        &500i128,
        &token_address,
        &Currency::USDC,
        &86400u64, // 1-day interval
        &0u64,
        &3u64,
        &String::from_str(env, ""),
        &0u64,
    );
    (sub_id, token_address)
}

/// Mints and approves enough of `token` for `customer` to cover `amount`,
/// so a subsequent charge attempt succeeds.
fn fund_customer(env: &Env, token: &Address, customer: &Address, contract: &Address, amount: i128) {
    let token_admin_client = token::StellarAssetClient::new(env, token);
    token_admin_client.mint(customer, &amount);
    let token_client = token::Client::new(env, token);
    token_client.approve(customer, contract, &amount, &200_000u32);
}

#[test]
fn test_retry_failed_payment_too_early_returns_error() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.set_dunning_config(
        &admin,
        &DunningConfig {
            initial_backoff_seconds: 3600,
            max_retries: 4,
        },
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Advance to payment due time and trigger the first failure → enters dunning.
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);

    let dunning = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(dunning.retry_count, 0);

    // retry_failed_payment before next_retry_at must fail with RetryTooEarly.
    let result = client.try_retry_failed_payment(&sub_id);
    assert_eq!(
        result,
        Err(Ok(Error::Subscription(SubscriptionError::RetryTooEarly)))
    );
}

#[test]
fn test_retry_failed_payment_recovers_after_funding() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.set_dunning_config(
        &admin,
        &DunningConfig {
            initial_backoff_seconds: 3600,
            max_retries: 4,
        },
    );

    let (sub_id, token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Initial charge fails (customer has no funds/allowance) → enters dunning.
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);
    let dunning = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(dunning.retry_count, 0);

    // Fund the customer, then retry via `retry_failed_payment` once due.
    fund_customer(&env, &token, &customer, &client.address, 500);
    env.ledger().set_timestamp(dunning.next_retry_at);
    client.retry_failed_payment(&sub_id);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.payment_count, 1);
    assert_eq!(sub.retry_count, 0);
    assert!(client.get_dunning_state(&sub_id).is_none());
}

#[test]
fn test_retry_failed_payment_past_max_retries_suspends_subscription() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.set_dunning_config(
        &admin,
        &DunningConfig {
            initial_backoff_seconds: 60,
            max_retries: 2,
        },
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Initial failure → enters dunning (retry_count = 0), still unfunded.
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);

    // Exhaust retries via `retry_failed_payment` without ever funding the
    // customer: retry_count goes 0 -> 1 -> 2, suspension fires at 2 >= max_retries(2).
    for _ in 0..2 {
        let d = client.get_dunning_state(&sub_id).unwrap();
        env.ledger().set_timestamp(d.next_retry_at);
        let _ = client.try_retry_failed_payment(&sub_id);
    }

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Suspended);

    // Dunning state is kept (not removed) with the final retry_count.
    let dunning = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(dunning.retry_count, 2);

    // A further retry attempt now fails since the subscription is no longer InDunning.
    let result = client.try_retry_failed_payment(&sub_id);
    assert_eq!(
        result,
        Err(Ok(Error::Subscription(SubscriptionError::NotInDunning)))
    );
}

#[test]
fn test_resolve_dunning_requires_admin_and_resets_to_active() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let not_admin = Address::generate(&env);

    client.set_dunning_config(
        &admin,
        &DunningConfig {
            initial_backoff_seconds: 3600,
            max_retries: 4,
        },
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);
    assert_eq!(
        client.get_subscription(&sub_id).status,
        SubscriptionStatus::InDunning
    );

    // Non-admin cannot resolve dunning.
    let result = client.try_resolve_dunning(&not_admin, &sub_id);
    assert_eq!(result, Err(Ok(Error::Basic(BasicError::Unauthorized))));

    // Admin can resolve it, resetting the subscription to Active and clearing dunning.
    client.resolve_dunning(&admin, &sub_id);
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.retry_count, 0);
    assert!(client.get_dunning_state(&sub_id).is_none());
}

#[test]
fn test_manually_resolve_dunning_alias_resets_suspended_subscription() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let not_admin = Address::generate(&env);

    client.set_dunning_config(
        &admin,
        &DunningConfig {
            initial_backoff_seconds: 60,
            max_retries: 1,
        },
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Initial failure enters dunning; one more failed retry suspends it
    // (max_retries == 1).
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);
    let d = client.get_dunning_state(&sub_id).unwrap();
    env.ledger().set_timestamp(d.next_retry_at);
    let _ = client.try_retry_failed_payment(&sub_id);

    assert_eq!(
        client.get_subscription(&sub_id).status,
        SubscriptionStatus::Suspended
    );

    // Non-admin cannot use the alias either.
    let result = client.try_manually_resolve_dunning(&not_admin, &sub_id);
    assert_eq!(result, Err(Ok(Error::Basic(BasicError::Unauthorized))));

    // Admin resolves the suspended subscription via the alias.
    client.manually_resolve_dunning(&admin, &sub_id);
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert!(client.get_dunning_state(&sub_id).is_none());
}

#[test]
fn test_update_dunning_config_alias_requires_admin_and_applies_to_new_entries() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let not_admin = Address::generate(&env);

    // Non-admin cannot update the global dunning config via the alias.
    let result = client.try_update_dunning_config(
        &not_admin,
        &DunningConfig {
            initial_backoff_seconds: 42,
            max_retries: 7,
        },
    );
    assert_eq!(result, Err(Ok(Error::Basic(BasicError::Unauthorized))));

    // Admin updates it via the alias.
    client.update_dunning_config(
        &admin,
        &DunningConfig {
            initial_backoff_seconds: 42,
            max_retries: 7,
        },
    );
    let stored = client.get_dunning_config();
    assert_eq!(stored.initial_backoff_seconds, 42);
    assert_eq!(stored.max_retries, 7);

    // A fresh dunning entry picks up the updated config.
    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);

    let dunning = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(dunning.backoff_seconds, 42);
    assert_eq!(dunning.max_retries, 7);
    assert_eq!(dunning.next_retry_at, 1000 + 86400 + 1 + 42);
}
