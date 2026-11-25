#![cfg(test)]

use super::*;
use escrow::EscrowContract;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env, String};

fn setup(env: &Env) -> (PaymentContractClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    client.initialize(&Address::generate(env));
    (client, contract_id)
}

fn funded_token(env: &Env, spender: &Address, holders: &[&Address]) -> Address {
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();
    for h in holders {
        token::StellarAssetClient::new(env, &token).mint(h, &1_000_000);
        token::Client::new(env, &token).approve(h, spender, &1_000_000, &10_000);
    }
    token
}

// ── #904: payment link revenue stats ─────────────────────────────────────────

#[test]
fn test_payment_link_stats_track_total_and_unique_payers() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let merchant = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&alice, &bob]);

    let link_id = client.create_payment_link(
        &merchant,
        &250,
        &token,
        &Currency::USDC,
        &0,
        &0,
        &String::from_str(&env, ""),
        &String::from_str(&env, "link"),
    );
    let empty = client.get_payment_link_stats(&link_id);
    assert_eq!(empty.total_collected, 0);
    assert_eq!(empty.unique_payers, 0);
    assert_eq!(empty.usage_count, 0);

    env.ledger().set_timestamp(500);
    client.use_payment_link(&alice, &link_id);
    client.use_payment_link(&alice, &link_id);
    client.use_payment_link(&bob, &link_id);

    let stats = client.get_payment_link_stats(&link_id);
    assert_eq!(stats.usage_count, 3);
    assert_eq!(stats.total_collected, 750);
    assert_eq!(stats.unique_payers, 2);
    assert_eq!(stats.last_used_at, Some(500));
    assert_eq!(client.get_payment_link(&link_id).usage_count, 3);
}

#[test]
fn test_payment_link_stats_unchanged_on_failed_use() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let merchant = Address::generate(&env);
    let alice = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&alice]);
    let link_id = client.create_payment_link(
        &merchant,
        &100,
        &token,
        &Currency::USDC,
        &1,
        &0,
        &String::from_str(&env, ""),
        &String::from_str(&env, ""),
    );
    client.use_payment_link(&alice, &link_id);
    assert!(client.try_use_payment_link(&alice, &link_id).is_err());
    let stats = client.get_payment_link_stats(&link_id);
    assert_eq!(stats.usage_count, 1);
    assert_eq!(stats.total_collected, 100);
}

#[test]
fn test_payment_link_stats_not_found() {
    let env = Env::default();
    let (client, _) = setup(&env);
    assert_eq!(
        client.try_get_payment_link_stats(&99),
        Err(Ok(Error::Payment(PaymentError::PaymentLinkNotFound)))
    );
}

// ── #905: create_escrowed_payment_v2 ─────────────────────────────────────────

#[test]
fn test_create_escrowed_payment_v2_sets_timeout_atomically() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let escrow_id = env.register(EscrowContract, ());
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&customer]);

    let (payment_id, _) = client.create_escrowed_payment_v2(
        &customer,
        &merchant,
        &1_000,
        &token,
        &Currency::USDC,
        &escrow_id,
        &EscrowedPaymentOptions {
            release_timestamp: 1000,
            min_hold_period: 0,
            metadata: String::from_str(&env, "v2"),
            auto_release_on_complete: true,
            confirmation_timeout_seconds: Some(3600),
        },
    );
    let bridge = client.get_escrowed_payment(&payment_id);
    assert_eq!(bridge.confirmation_timeout_seconds, Some(3600));
    assert!(bridge.auto_release_on_complete);
}

#[test]
fn test_create_escrowed_payment_wrapper_still_works() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let escrow_id = env.register(EscrowContract, ());
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&customer]);

    let (payment_id, _) = client.create_escrowed_payment(
        &customer,
        &merchant,
        &1_000,
        &token,
        &Currency::USDC,
        &escrow_id,
        &1000,
        &0,
        &String::from_str(&env, "v1"),
        &false,
    );
    let bridge = client.get_escrowed_payment(&payment_id);
    assert_eq!(bridge.confirmation_timeout_seconds, None);
    assert!(!bridge.auto_release_on_complete);
}

// ── #906: price lock for existing subscribers ────────────────────────────────

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
        &1_000,
        &0,
        &0,
        &String::from_str(env, ""),
        &0,
    )
}

#[test]
fn test_price_lock_blocks_proposals_for_existing_subscribers_only() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&customer]);

    env.ledger().set_timestamp(100);
    let old_sub = create_sub(&env, &client, &customer, &merchant, &token);
    env.ledger().set_timestamp(200);
    client.set_subscription_price_lock(&merchant, &200, &5_000);
    let new_sub = create_sub(&env, &client, &customer, &merchant, &token);

    let lock = client.get_subscription_price_lock(&merchant).unwrap();
    assert_eq!(lock.created_before, 200);
    assert_eq!(lock.lock_until, 5_000);
    assert_eq!(client.get_sub_price_locked_until(&old_sub), Some(5_000));
    assert_eq!(client.get_sub_price_locked_until(&new_sub), None);

    assert_eq!(
        client.try_propose_subscription_price(&merchant, &old_sub, &150, &0),
        Err(Ok(Error::Subscription(SubscriptionError::PriceLocked)))
    );
    // New subscriptions are unaffected.
    client.propose_subscription_price(&merchant, &new_sub, &150, &0);

    // After the lock ends, proposals for old subscriptions are accepted.
    env.ledger().set_timestamp(5_000);
    assert_eq!(client.get_sub_price_locked_until(&old_sub), None);
    client.propose_subscription_price(&merchant, &old_sub, &150, &0);
}

#[test]
fn test_accepted_price_applies_on_next_charge() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&customer]);

    env.ledger().set_timestamp(100);
    let sub_id = create_sub(&env, &client, &customer, &merchant, &token);
    client.propose_subscription_price(&merchant, &sub_id, &150, &0);
    client.accept_subscription_price(&customer, &sub_id);

    env.ledger().set_timestamp(1_100);
    client.execute_recurring_payment(&sub_id);
    assert_eq!(client.get_subscription(&sub_id).amount, 150);
}

#[test]
fn test_unaccepted_price_is_not_applied() {
    let env = Env::default();
    let (client, contract_id) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = funded_token(&env, &contract_id, &[&customer]);

    env.ledger().set_timestamp(100);
    let sub_id = create_sub(&env, &client, &customer, &merchant, &token);
    client.propose_subscription_price(&merchant, &sub_id, &150, &0);

    env.ledger().set_timestamp(1_100);
    client.execute_recurring_payment(&sub_id);
    assert_eq!(client.get_subscription(&sub_id).amount, 100);
}

#[test]
fn test_price_lock_validation_and_clear() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);
    env.ledger().set_timestamp(1_000);
    assert_eq!(
        client.try_set_subscription_price_lock(&merchant, &2_000, &5_000),
        Err(Ok(Error::Subscription(SubscriptionError::InvalidPriceLock)))
    );
    assert_eq!(
        client.try_set_subscription_price_lock(&merchant, &500, &1_000),
        Err(Ok(Error::Subscription(SubscriptionError::InvalidPriceLock)))
    );
    client.set_subscription_price_lock(&merchant, &500, &5_000);
    client.set_subscription_price_lock(&merchant, &0, &0);
    assert!(client.get_subscription_price_lock(&merchant).is_none());
}
