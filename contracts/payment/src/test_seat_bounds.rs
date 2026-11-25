#![cfg(test)]
//! Tests for #902: min/max seats on seat-based subscriptions.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env, String};

struct Fixture<'a> {
    client: PaymentContractClient<'a>,
    customer: Address,
    merchant: Address,
    sub_id: u64,
    env: Env,
}

fn setup<'a>() -> Fixture<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env));
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&customer, &1_000_000);
    token::Client::new(&env, &token).approve(&customer, &contract_id, &1_000_000, &10_000);
    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &100,
        &token,
        &Currency::USDC,
        &1_000,
        &0,
        &0,
        &String::from_str(&env, ""),
        &0,
    );
    Fixture {
        client,
        customer,
        merchant,
        sub_id,
        env,
    }
}

#[test]
fn quantity_updates_outside_bounds_are_rejected() {
    let f = setup();
    assert_eq!(f.client.get_subscription_seat_bounds(&f.sub_id), None);
    f.client
        .set_subscription_seat_bounds(&f.merchant, &f.sub_id, &Some(2), &Some(5));
    assert_eq!(
        f.client.get_subscription_seat_bounds(&f.sub_id),
        Some(SeatBounds {
            min_seats: Some(2),
            max_seats: Some(5),
        })
    );

    f.client
        .update_subscription_quantity(&f.customer, &f.sub_id, &5);
    assert_eq!(f.client.get_subscription_seats(&f.sub_id).quantity, 5);
    assert_eq!(
        f.client
            .try_update_subscription_quantity(&f.customer, &f.sub_id, &6),
        Err(Ok(Error::Subscription(SubscriptionError::SeatsOutOfBounds)))
    );
    assert_eq!(
        f.client
            .try_update_subscription_quantity(&f.customer, &f.sub_id, &1),
        Err(Ok(Error::Subscription(SubscriptionError::SeatsOutOfBounds)))
    );
    f.client
        .update_subscription_quantity(&f.customer, &f.sub_id, &2);
    assert_eq!(f.client.get_subscription_seats(&f.sub_id).quantity, 2);
}

#[test]
fn merchant_can_update_and_clear_bounds() {
    let f = setup();
    f.client
        .set_subscription_seat_bounds(&f.merchant, &f.sub_id, &None, &Some(3));
    assert_eq!(
        f.client
            .try_update_subscription_quantity(&f.customer, &f.sub_id, &4),
        Err(Ok(Error::Subscription(SubscriptionError::SeatsOutOfBounds)))
    );
    f.client
        .set_subscription_seat_bounds(&f.merchant, &f.sub_id, &None, &Some(10));
    f.client
        .update_subscription_quantity(&f.customer, &f.sub_id, &4);
    f.client
        .set_subscription_seat_bounds(&f.merchant, &f.sub_id, &None, &None);
    assert_eq!(f.client.get_subscription_seat_bounds(&f.sub_id), None);
    f.client
        .update_subscription_quantity(&f.customer, &f.sub_id, &20);
}

#[test]
fn invalid_bounds_and_non_merchant_callers_are_rejected() {
    let f = setup();
    assert_eq!(
        f.client
            .try_set_subscription_seat_bounds(&f.merchant, &f.sub_id, &Some(5), &Some(2)),
        Err(Ok(Error::Subscription(
            SubscriptionError::InvalidSeatBounds
        )))
    );
    assert_eq!(
        f.client
            .try_set_subscription_seat_bounds(&f.merchant, &f.sub_id, &Some(0), &None),
        Err(Ok(Error::Subscription(
            SubscriptionError::InvalidSeatBounds
        )))
    );
    let stranger = Address::generate(&f.env);
    assert_eq!(
        f.client
            .try_set_subscription_seat_bounds(&stranger, &f.sub_id, &Some(1), &None),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        f.client
            .try_set_subscription_seat_bounds(&f.merchant, &999, &Some(1), &None),
        Err(Ok(Error::Subscription(SubscriptionError::NotFound)))
    );
}
