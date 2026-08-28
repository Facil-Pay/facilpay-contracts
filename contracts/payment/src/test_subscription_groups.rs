#![cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::{Currency, Error, PaymentContract, PaymentContractClient, SubscriptionError, SubscriptionStatus};

fn setup() -> (Env, PaymentContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PaymentContract);
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn create_sub(
    env: &Env,
    client: &PaymentContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> u64 {
    client.create_subscription(
        customer,
        merchant,
        &100,
        token,
        &Currency::USDC,
        &2592000,
        &0,
        &3,
        &String::from_str(env, ""),
        &0,
    )
}

#[test]
fn test_create_group() {
    let (env, client, _) = setup();
    let owner = Address::generate(&env);
    let group_id = client.create_subscription_group(&owner, &500);
    assert_eq!(group_id, 1);
    let group = client.get_subscription_group(&group_id).unwrap();
    assert_eq!(group.owner, owner);
    assert_eq!(group.discount_bps, 500);
    assert!(group.active);
    assert_eq!(group.subscription_ids.len(), 0);
}

#[test]
fn test_add_and_remove_from_group() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&owner, &100_000);

    let group_id = client.create_subscription_group(&owner, &200);
    let sub_id = create_sub(&env, &client, &owner, &merchant, &token_addr);

    client.add_to_group(&owner, &group_id, &sub_id);
    let group = client.get_subscription_group(&group_id).unwrap();
    assert_eq!(group.subscription_ids.len(), 1);

    client.remove_from_group(&owner, &group_id, &sub_id);
    let group = client.get_subscription_group(&group_id).unwrap();
    assert_eq!(group.subscription_ids.len(), 0);
}

#[test]
fn test_subscription_already_in_group() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&owner, &100_000);

    let group_id = client.create_subscription_group(&owner, &200);
    let sub_id = create_sub(&env, &client, &owner, &merchant, &token_addr);

    client.add_to_group(&owner, &group_id, &sub_id);
    let result = client.try_add_to_group(&owner, &group_id, &sub_id);
    assert_eq!(
        result,
        Err(Ok(Error::Subscription(SubscriptionError::AlreadyInGroup)))
    );
}

#[test]
fn test_group_size_limit() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&owner, &10_000_000);

    let group_id = client.create_subscription_group(&owner, &100);

    for _ in 0..20 {
        let sub_id = create_sub(&env, &client, &owner, &merchant, &token_addr);
        client.add_to_group(&owner, &group_id, &sub_id);
    }

    // 21st should fail
    let sub_id = create_sub(&env, &client, &owner, &merchant, &token_addr);
    let result = client.try_add_to_group(&owner, &group_id, &sub_id);
    assert_eq!(
        result,
        Err(Ok(Error::Subscription(
            SubscriptionError::GroupSizeLimitExceeded
        )))
    );
}

#[test]
fn test_group_not_found() {
    let (env, client, _) = setup();
    let owner = Address::generate(&env);
    let result = client.try_add_to_group(&owner, &999, &1);
    assert_eq!(
        result,
        Err(Ok(Error::Subscription(SubscriptionError::GroupNotFound)))
    );
}

#[test]
fn test_get_group_next_billing() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&owner, &100_000);

    let group_id = client.create_subscription_group(&owner, &0);
    let sub_id = create_sub(&env, &client, &owner, &merchant, &token_addr);
    client.add_to_group(&owner, &group_id, &sub_id);

    let next = client.get_group_next_billing(&group_id);
    assert!(next > 0);
}

#[test]
fn test_group_discount_applied_on_recurring_payment() {
    let (env, client, admin) = setup();
    let owner = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&owner, &100_000);
    soroban_sdk::token::Client::new(&env, &token_addr).approve(
        &owner,
        &client.address,
        &100_000,
        &10_000,
    );

    env.ledger().set_timestamp(1000);
    let sub_amount = 1000_i128;
    let sub_id = client.create_subscription(
        &owner,
        &merchant,
        &sub_amount,
        &token_addr,
        &Currency::USDC,
        &2592000,
        &0,
        &0,
        &String::from_str(&env, ""),
        &0,
    );

    // 10% group discount
    let group_id = client.create_subscription_group(&owner, &1000);
    client.add_to_group(&owner, &group_id, &sub_id);

    let merchant_before = soroban_sdk::token::Client::new(&env, &token_addr).balance(&merchant);
    env.ledger().set_timestamp(1000 + 2592000);
    client.execute_recurring_payment(&sub_id);
    let merchant_after = soroban_sdk::token::Client::new(&env, &token_addr).balance(&merchant);

    let expected_charge = sub_amount * 9000 / 10000;
    assert_eq!(merchant_after - merchant_before, expected_charge);
}

#[test]
fn test_resume_subscription_charges_prorated_amount() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&customer, &100_000);
    soroban_sdk::token::Client::new(&env, &token_addr).approve(
        &customer,
        &client.address,
        &100_000,
        &10_000,
    );

    let interval = 2_592_000_u64;
    let sub_amount = 1000_i128;
    env.ledger().set_timestamp(1000);
    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &sub_amount,
        &token_addr,
        &Currency::USDC,
        &interval,
        &0,
        &0,
        &String::from_str(&env, ""),
        &0,
    );
    client.set_subscription_proration(&customer, &sub_id, &true);

    let pause_at = 1000 + interval / 2;
    env.ledger().set_timestamp(pause_at);
    client.pause_subscription(&customer, &sub_id);

    let resume_at = pause_at + interval / 2;
    env.ledger().set_timestamp(resume_at);

    let merchant_before = soroban_sdk::token::Client::new(&env, &token_addr).balance(&merchant);
    client.resume_subscription(&customer, &sub_id);
    let merchant_after = soroban_sdk::token::Client::new(&env, &token_addr).balance(&merchant);

    let expected_proration = sub_amount / 2;
    assert_eq!(merchant_after - merchant_before, expected_proration);

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.payment_count, 1);
}
