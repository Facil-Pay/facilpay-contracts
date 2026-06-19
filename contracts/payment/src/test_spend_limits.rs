#![cfg(test)]
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

use crate::{Currency, Error, PaymentContract, PaymentContractClient};

fn setup() -> (Env, PaymentContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PaymentContract);
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn test_set_and_get_spend_limit() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    client.set_customer_spend_limit(&admin, &customer, &1000, &3600);
    let limit = client.get_spend_limit(&customer).unwrap();
    assert_eq!(limit.limit_amount, 1000);
    assert_eq!(limit.period_seconds, 3600);
    assert_eq!(limit.used, 0);
}

#[test]
fn test_check_spend_allowance_no_limit() {
    let (env, client, _) = setup();
    let customer = Address::generate(&env);
    // No limit configured — always allowed
    assert!(client.check_spend_allowance(&customer, &999_999));
}

#[test]
fn test_check_spend_allowance_within_limit() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    client.set_customer_spend_limit(&admin, &customer, &1000, &3600);
    assert!(client.check_spend_allowance(&customer, &500));
}

#[test]
fn test_check_spend_allowance_exceeds_limit() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    client.set_customer_spend_limit(&admin, &customer, &1000, &3600);
    assert!(!client.check_spend_allowance(&customer, &1001));
}

#[test]
fn test_remove_spend_limit() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    client.set_customer_spend_limit(&admin, &customer, &1000, &3600);
    client.remove_customer_spend_limit(&admin, &customer);
    assert!(client.get_spend_limit(&customer).is_none());
    // After removal, any amount is allowed
    assert!(client.check_spend_allowance(&customer, &999_999));
}

#[test]
fn test_spend_limit_enforced_on_create_payment() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Register a token
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&customer, &10_000);

    // Set a tight spend limit
    client.set_customer_spend_limit(&admin, &customer, &100, &3600);

    // Payment of 200 should fail
    let result = client.try_create_payment(
        &customer,
        &merchant,
        &200,
        &token_addr,
        &Currency::USDC,
        &0,
        &soroban_sdk::String::from_str(&env, ""),
    );
    assert_eq!(result, Err(Ok(Error::SpendLimitExceeded)));
}

#[test]
fn test_period_auto_reset() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    client.set_customer_spend_limit(&admin, &customer, &1000, &3600);

    // Advance time past the period
    env.ledger().with_mut(|l| l.timestamp = 7200);

    // After period reset, full limit is available again
    assert!(client.check_spend_allowance(&customer, &1000));
}
