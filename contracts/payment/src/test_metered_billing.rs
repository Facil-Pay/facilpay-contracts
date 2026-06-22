#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, Ledger as _}, token, Address, Env, String};

fn setup(env: &Env) -> (PaymentContractClient, Address, Address, Address, Address) {
    let id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);

    let token_admin_addr = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr.clone());
    let token_address = token_contract.address();
    let asset_client = token::StellarAssetClient::new(env, &token_address);

    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    asset_client.mint(&customer, &1_000_000i128);
    token::Client::new(env, &token_address).approve(&customer, &id, &1_000_000i128, &10_000);

    (client, admin, merchant, customer, token_address)
}

#[test]
fn test_report_usage_accumulates() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &10i128,
        &String::from_str(&env, "api_call"),
        &token,
        &None,
    );

    client.report_usage(&merchant, &sub_id, &5u64);
    client.report_usage(&merchant, &sub_id, &3u64);

    let usage = client.get_current_usage(&sub_id);
    assert_eq!(usage.accumulated_units, 8);
}

#[test]
fn test_report_usage_unauthorized() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);
    let stranger = Address::generate(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &10i128,
        &String::from_str(&env, "api_call"),
        &token,
        &None,
    );

    let result = client.try_report_usage(&stranger, &sub_id, &5u64);
    assert_eq!(result, Err(Ok(Error::Basic(BasicError::Unauthorized))));
}

#[test]
fn test_execute_metered_billing_charges_and_resets() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &100i128,
        &String::from_str(&env, "gb"),
        &token,
        &None,
    );

    client.report_usage(&merchant, &sub_id, &10u64);

    let token_client = token::Client::new(&env, &token);
    let merchant_balance_before = token_client.balance(&merchant);

    let amount = client.execute_metered_billing(&sub_id);
    assert_eq!(amount, 1000i128); // 10 units * 100 per unit

    let usage_after = client.get_current_usage(&sub_id);
    assert_eq!(usage_after.accumulated_units, 0);

    let merchant_balance_after = token_client.balance(&merchant);
    assert_eq!(merchant_balance_after - merchant_balance_before, 1000i128);
}

#[test]
fn test_billing_cap_limits_charge() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &100i128,
        &String::from_str(&env, "gb"),
        &token,
        &Some(500i128),
    );

    // 10 units * 100 = 1000, but cap is 500
    client.report_usage(&merchant, &sub_id, &10u64);

    let amount = client.execute_metered_billing(&sub_id);
    assert_eq!(amount, 500i128);

    let usage_after = client.get_current_usage(&sub_id);
    assert_eq!(usage_after.accumulated_units, 0);
}

#[test]
fn test_set_billing_cap() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &10i128,
        &String::from_str(&env, "req"),
        &token,
        &None,
    );

    client.set_billing_cap(&merchant, &sub_id, &250i128);

    let usage = client.get_current_usage(&sub_id);
    assert_eq!(usage.billing_cap, Some(250i128));
}

/// accumulated_units * price_per_unit must not silently overflow or saturate.
/// When the product would exceed i128::MAX the function must return
/// Error::BillingOverflow and leave the subscription state unchanged.
#[test]
fn test_billing_overflow_returns_error() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    // price_per_unit set to i128::MAX so that even a single reported unit
    // causes the multiplication to overflow.
    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &i128::MAX,
        &String::from_str(&env, "op"),
        &token,
        &None,
    );

    // Two units * i128::MAX overflows i128.
    client.report_usage(&merchant, &sub_id, &2u64);

    let result = client.try_execute_metered_billing(&sub_id);
    assert_eq!(result, Err(Ok(Error::BillingOverflow)));

    // Accumulated units must still be intact — the state was not mutated.
    let usage = client.get_current_usage(&sub_id);
    assert_eq!(usage.accumulated_units, 2, "state must not change on overflow");
}

/// Billing on a subscription with zero accumulated units returns 0 and does
/// not transfer anything — no tokens should move.
#[test]
fn test_execute_metered_billing_zero_units_no_transfer() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &100i128,
        &String::from_str(&env, "req"),
        &token,
        &None,
    );

    let token_client = token::Client::new(&env, &token);
    let merchant_before = token_client.balance(&merchant);
    let customer_before = token_client.balance(&customer);

    let amount = client.execute_metered_billing(&sub_id);
    assert_eq!(amount, 0i128);

    assert_eq!(token_client.balance(&merchant), merchant_before, "merchant balance must not change");
    assert_eq!(token_client.balance(&customer), customer_before, "customer balance must not change");
}

/// Multiple sequential billing cycles each reset the counter independently.
/// Cycle 2 must only charge for units reported after cycle 1's reset.
#[test]
fn test_sequential_billing_cycles_charge_independently() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &10i128,
        &String::from_str(&env, "call"),
        &token,
        &None,
    );

    // Cycle 1: 5 units → charge 50
    client.report_usage(&merchant, &sub_id, &5u64);
    let charge1 = client.execute_metered_billing(&sub_id);
    assert_eq!(charge1, 50i128);

    // Cycle 2: 3 units reported after reset → charge 30, not 80
    client.report_usage(&merchant, &sub_id, &3u64);
    let charge2 = client.execute_metered_billing(&sub_id);
    assert_eq!(charge2, 30i128);

    // After cycle 2 the counter is reset to 0 again
    assert_eq!(client.get_current_usage(&sub_id).accumulated_units, 0);
}

/// A subscription starts with no cap. set_billing_cap adds one; a subsequent
/// billing call respects it. Verifies cap is applied retroactively to pending units.
#[test]
fn test_cap_set_after_usage_is_respected() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &100i128,
        &String::from_str(&env, "gb"),
        &token,
        &None,
    );

    // Report 20 units (would be 2000 uncapped)
    client.report_usage(&merchant, &sub_id, &20u64);

    // Set cap to 800 after usage is already accumulated
    client.set_billing_cap(&merchant, &sub_id, &800i128);

    let amount = client.execute_metered_billing(&sub_id);
    assert_eq!(amount, 800i128, "cap set after usage accumulation must be honoured");
}

/// set_billing_cap must be rejected for a caller that is not the subscription's
/// merchant, even if they are a valid contract participant.
#[test]
fn test_set_billing_cap_unauthorized() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &10i128,
        &String::from_str(&env, "req"),
        &token,
        &None,
    );

    // Customer tries to set the cap — should be rejected
    let result = client.try_set_billing_cap(&customer, &sub_id, &500i128);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));

    // Cap must still be None
    assert_eq!(client.get_current_usage(&sub_id).billing_cap, None);
}

/// execute_metered_billing on an unknown subscription_id must return
/// MeteredSubscriptionNotFound, not panic.
#[test]
fn test_billing_on_nonexistent_subscription() {
    let env = Env::default();
    let (client, _admin, _merchant, _customer, _token) = setup(&env);

    let result = client.try_execute_metered_billing(&9999u64);
    assert_eq!(result, Err(Ok(Error::MeteredSubscriptionNotFound)));
}

/// report_usage on an unknown subscription_id must return
/// MeteredSubscriptionNotFound.
#[test]
fn test_report_usage_on_nonexistent_subscription() {
    let env = Env::default();
    let (client, _admin, merchant, _customer, _token) = setup(&env);

    let result = client.try_report_usage(&merchant, &9999u64, &5u64);
    assert_eq!(result, Err(Ok(Error::MeteredSubscriptionNotFound)));
}

/// Accumulated units saturate at u64::MAX instead of wrapping. This ensures
/// that extremely high usage counters never silently loop back to zero.
#[test]
fn test_report_usage_saturates_at_u64_max() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &1i128,
        &String::from_str(&env, "op"),
        &token,
        &None,
    );

    // Push counter to u64::MAX
    client.report_usage(&merchant, &sub_id, &u64::MAX);

    // Adding more must not wrap around
    client.report_usage(&merchant, &sub_id, &1u64);

    let usage = client.get_current_usage(&sub_id);
    assert_eq!(usage.accumulated_units, u64::MAX, "counter must saturate, not wrap");
}

/// price_per_unit of 1 with a single reported unit produces exactly 1 token
/// transferred. Validates the minimum non-trivial billing amount.
#[test]
fn test_single_unit_at_price_one_transfers_exactly_one_token() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &1i128,
        &String::from_str(&env, "event"),
        &token,
        &None,
    );

    client.report_usage(&merchant, &sub_id, &1u64);

    let token_client = token::Client::new(&env, &token);
    let before = token_client.balance(&merchant);

    let amount = client.execute_metered_billing(&sub_id);
    assert_eq!(amount, 1i128);
    assert_eq!(token_client.balance(&merchant) - before, 1i128);
}

/// Two independent subscriptions for the same merchant/customer pair must
/// track their own usage independently — billing one must not affect the other.
#[test]
fn test_two_subscriptions_track_usage_independently() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_a = client.create_metered_subscription(
        &merchant, &customer, &10i128,
        &String::from_str(&env, "calls"), &token, &None,
    );
    let sub_b = client.create_metered_subscription(
        &merchant, &customer, &50i128,
        &String::from_str(&env, "gb"), &token, &None,
    );

    client.report_usage(&merchant, &sub_a, &4u64);
    client.report_usage(&merchant, &sub_b, &2u64);

    // Bill sub_a only — sub_b must stay untouched
    let charge_a = client.execute_metered_billing(&sub_a);
    assert_eq!(charge_a, 40i128);

    assert_eq!(client.get_current_usage(&sub_a).accumulated_units, 0);
    assert_eq!(client.get_current_usage(&sub_b).accumulated_units, 2,
        "sub_b must not be reset when sub_a is billed");
}

/// When the billing cap exactly equals the computed charge, the full amount is
/// transferred and the cap-hit event path is NOT taken (amount == cap is not
/// exceeding the cap).
#[test]
fn test_billing_cap_equal_to_charge_no_cap_hit() {
    let env = Env::default();
    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &100i128,
        &String::from_str(&env, "unit"),
        &token,
        &Some(500i128), // cap == 5 units * 100
    );

    client.report_usage(&merchant, &sub_id, &5u64);

    let amount = client.execute_metered_billing(&sub_id);
    // Charge (500) == cap (500) → full amount transferred, cap not "hit"
    assert_eq!(amount, 500i128);

    // Units must still be reset after a clean (non-capped) billing
    assert_eq!(client.get_current_usage(&sub_id).accumulated_units, 0);
}

/// last_reset_at is updated to the current ledger timestamp after a successful
/// billing cycle, giving callers an accurate record of when the window reset.
#[test]
fn test_last_reset_at_updated_after_billing() {
    let env = Env::default();
    // Set a known base timestamp
    env.ledger().set_timestamp(1_000);

    let (client, _admin, merchant, customer, token) = setup(&env);

    let sub_id = client.create_metered_subscription(
        &merchant,
        &customer,
        &10i128,
        &String::from_str(&env, "call"),
        &token,
        &None,
    );

    client.report_usage(&merchant, &sub_id, &3u64);

    // Advance ledger time before billing
    env.ledger().set_timestamp(5_000);
    client.execute_metered_billing(&sub_id);

    let usage = client.get_current_usage(&sub_id);
    assert_eq!(usage.last_reset_at, 5_000, "last_reset_at must reflect the billing timestamp");
}
