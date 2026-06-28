#![cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::{Currency, Error, PaymentContract, PaymentContractClient, SubscriptionError};

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
