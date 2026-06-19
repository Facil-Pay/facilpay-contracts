#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

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
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
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
