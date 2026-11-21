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
    token::StellarAssetClient::new(env, &token).mint(customer, &1_000_000);
    token::Client::new(env, &token).approve(customer, contract_id, &1_000_000, &10_000);
    token
}

fn create_sub(
    env: &Env,
    client: &PaymentContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
    interval: u64,
) -> u64 {
    client.create_subscription(
        customer,
        merchant,
        &100,
        token,
        &Currency::USDC,
        &interval,
        &0,
        &0,
        &String::from_str(env, ""),
        &0,
    )
}

#[test]
fn due_subscription_is_returned_and_future_one_is_not() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);
    // Due at 1000 + 100 = 1100
    let due_soon = create_sub(&env, &client, &customer, &merchant, &token, 100);
    // Due at 1000 + 10_000 = 11000
    let due_later = create_sub(&env, &client, &customer, &merchant, &token, 10_000);

    let due = client.get_subscriptions_due_before(&1100, &0, &10);
    assert!(due.iter().any(|s| s.id == due_soon));
    assert!(due.iter().all(|s| s.id != due_later));

    let none_yet = client.get_subscriptions_due_before(&1099, &0, &10);
    assert!(none_yet.iter().all(|s| s.id != due_soon));
}

#[test]
fn paused_subscription_is_excluded() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);
    let sub_id = create_sub(&env, &client, &customer, &merchant, &token, 100);

    client.pause_subscription(&customer, &sub_id);
    assert_eq!(
        client.get_subscription(&sub_id).status,
        SubscriptionStatus::Paused
    );

    let due = client.get_subscriptions_due_before(&1100, &0, &10);
    assert!(due.iter().all(|s| s.id != sub_id));
}

#[test]
fn cancelled_subscription_is_excluded() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);
    let sub_id = create_sub(&env, &client, &customer, &merchant, &token, 100);

    client.cancel_subscription(&customer, &sub_id);
    assert_eq!(
        client.get_subscription(&sub_id).status,
        SubscriptionStatus::Cancelled
    );

    let due = client.get_subscriptions_due_before(&1100, &0, &10);
    assert!(due.iter().all(|s| s.id != sub_id));
}

#[test]
fn pagination_offset_and_limit_walk_through_due_subscriptions() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &customer);

    env.ledger().set_timestamp(1000);
    // Three subscriptions, each due one second apart, all before the query timestamp.
    let id_a = create_sub(&env, &client, &customer, &merchant, &token, 10); // due 1010
    let id_b = create_sub(&env, &client, &customer, &merchant, &token, 20); // due 1020
    let id_c = create_sub(&env, &client, &customer, &merchant, &token, 30); // due 1030

    let page1 = client.get_subscriptions_due_before(&2000, &0, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().id, id_a);
    assert_eq!(page1.get(1).unwrap().id, id_b);

    let page2 = client.get_subscriptions_due_before(&2000, &2, &2);
    assert_eq!(page2.len(), 1);
    assert_eq!(page2.get(0).unwrap().id, id_c);

    let page3 = client.get_subscriptions_due_before(&2000, &3, &2);
    assert_eq!(page3.len(), 0);
}
