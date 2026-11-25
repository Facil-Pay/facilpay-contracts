#![cfg(test)]

//! Issue #901: rolling reserve held back from new merchants' payouts.

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, String, Symbol, TryFromVal,
};

use crate::{
    BasicError, Currency, Error, FeatureError, FinalityConfig, PaymentContract,
    PaymentContractClient, RollingReserveConfig,
};

const HOLD: u64 = 1_000;

struct Setup<'a> {
    env: Env,
    client: PaymentContractClient<'a>,
    admin: Address,
    customer: Address,
    merchant: Address,
    token: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(10_000);
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let customer = Address::generate(&env);
    fund_customer(&env, &contract_id, &token, &customer);

    Setup {
        merchant: Address::generate(&env),
        env,
        client,
        admin,
        customer,
        token,
    }
}

fn fund_customer(env: &Env, contract_id: &Address, token: &Address, customer: &Address) {
    token::StellarAssetClient::new(env, token).mint(customer, &1_000_000);
    token::Client::new(env, token).approve(customer, contract_id, &1_000_000, &100_000);
}

fn pay(s: &Setup, token: &Address, amount: i128) -> u64 {
    let payment_id = s.client.create_payment(
        &s.customer,
        &s.merchant,
        &amount,
        token,
        &Currency::USDC,
        &0,
        &String::from_str(&s.env, ""),
    );
    s.client.complete_payment(&s.admin, &payment_id);
    payment_id
}

fn balance(s: &Setup, token: &Address, who: &Address) -> i128 {
    token::Client::new(&s.env, token).balance(who)
}

fn has_event(env: &Env, name: &str) -> bool {
    let expected = Symbol::new(env, name);
    env.events().all().iter().any(|(_, topics, _)| {
        topics
            .get(0)
            .and_then(|t| Symbol::try_from_val(env, &t).ok())
            .is_some_and(|sym| sym == expected)
    })
}

fn advance(s: &Setup, seconds: u64) {
    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + seconds);
}

fn release_err(s: &Setup) -> Option<Error> {
    s.client
        .try_release_rolling_reserve(&s.merchant)
        .err()
        .and_then(|e| e.ok())
}

#[test]
fn reserve_is_off_by_default() {
    let s = setup();
    assert_eq!(s.client.get_rolling_reserve(&s.merchant), None);

    pay(&s, &s.token, 1_000);
    assert_eq!(balance(&s, &s.token, &s.merchant), 1_000);
    assert_eq!(s.client.get_rolling_reserve_held(&s.merchant, &s.token), 0);
}

#[test]
fn set_rolling_reserve_stores_config_and_emits_event() {
    let s = setup();
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);
    assert!(has_event(&s.env, "rolling_reserve_set"));
    assert_eq!(
        s.client.get_rolling_reserve(&s.merchant),
        Some(RollingReserveConfig {
            bps: 1_000,
            hold_seconds: HOLD,
        })
    );
}

#[test]
fn set_rolling_reserve_validates_input_and_caller() {
    let s = setup();
    let invalid = Err(Ok(Error::Feature(FeatureError::InvalidRollingReserve)));
    assert_eq!(
        s.client
            .try_set_rolling_reserve(&s.admin, &s.merchant, &10_001, &HOLD),
        invalid
    );
    assert_eq!(
        s.client
            .try_set_rolling_reserve(&s.admin, &s.merchant, &500, &0),
        invalid
    );
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_set_rolling_reserve(&stranger, &s.merchant, &500, &HOLD),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
}

#[test]
fn settlement_withholds_bps_into_a_slice() {
    let s = setup();
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);

    pay(&s, &s.token, 1_000);

    assert_eq!(balance(&s, &s.token, &s.merchant), 900);
    assert_eq!(
        s.client.get_rolling_reserve_held(&s.merchant, &s.token),
        100
    );
    let slices = s.client.get_rolling_reserve_slices(&s.merchant);
    assert_eq!(slices.len(), 1);
    let slice = slices.get(0).unwrap();
    assert_eq!(slice.amount, 100);
    assert_eq!(slice.token, s.token);
    assert_eq!(slice.held_at, 10_000);
    assert_eq!(slice.release_at, 10_000 + HOLD);
}

#[test]
fn slice_is_released_only_after_hold() {
    let s = setup();
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);
    pay(&s, &s.token, 1_000);

    assert_eq!(
        release_err(&s),
        Some(Error::Feature(FeatureError::NoReleasableReserve))
    );
    advance(&s, HOLD - 1);
    assert_eq!(
        release_err(&s),
        Some(Error::Feature(FeatureError::NoReleasableReserve))
    );

    advance(&s, 1);
    assert_eq!(s.client.release_rolling_reserve(&s.merchant), 1);
    assert!(has_event(&s.env, "rolling_reserve_released"));
    assert_eq!(balance(&s, &s.token, &s.merchant), 1_000);
    assert_eq!(s.client.get_rolling_reserve_held(&s.merchant, &s.token), 0);
    assert_eq!(s.client.get_rolling_reserve_slices(&s.merchant).len(), 0);

    // Nothing left to release.
    assert_eq!(
        release_err(&s),
        Some(Error::Feature(FeatureError::NoReleasableReserve))
    );
}

#[test]
fn each_slice_is_released_on_its_own_schedule() {
    let s = setup();
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);
    pay(&s, &s.token, 1_000); // releases at 11_000
    advance(&s, 500);
    pay(&s, &s.token, 2_000); // releases at 11_500

    advance(&s, 600); // t = 11_100
    assert_eq!(s.client.release_rolling_reserve(&s.merchant), 1);
    assert_eq!(
        s.client.get_rolling_reserve_held(&s.merchant, &s.token),
        200
    );
    assert_eq!(balance(&s, &s.token, &s.merchant), 900 + 1_800 + 100);

    advance(&s, 400); // t = 11_500
    assert_eq!(s.client.release_rolling_reserve(&s.merchant), 1);
    assert_eq!(s.client.get_rolling_reserve_held(&s.merchant, &s.token), 0);
    assert_eq!(balance(&s, &s.token, &s.merchant), 3_000);
}

#[test]
fn disabling_reserve_does_not_release_held_funds_early() {
    let s = setup();
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);
    pay(&s, &s.token, 1_000);

    s.client.set_rolling_reserve(&s.admin, &s.merchant, &0, &0);
    assert_eq!(s.client.get_rolling_reserve(&s.merchant), None);

    // Still held, and still not releasable before its hold elapses.
    assert_eq!(
        s.client.get_rolling_reserve_held(&s.merchant, &s.token),
        100
    );
    assert_eq!(
        release_err(&s),
        Some(Error::Feature(FeatureError::NoReleasableReserve))
    );

    // New settlements are no longer withheld.
    pay(&s, &s.token, 1_000);
    assert_eq!(balance(&s, &s.token, &s.merchant), 900 + 1_000);
    assert_eq!(
        s.client.get_rolling_reserve_held(&s.merchant, &s.token),
        100
    );

    advance(&s, HOLD);
    assert_eq!(s.client.release_rolling_reserve(&s.merchant), 1);
    assert_eq!(balance(&s, &s.token, &s.merchant), 2_000);
}

#[test]
fn shortening_hold_does_not_release_older_slices_early() {
    let s = setup();
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);
    pay(&s, &s.token, 1_000); // releases at 11_000

    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &10);
    pay(&s, &s.token, 1_000); // releases at 10_010, queued behind the first

    advance(&s, 10);
    assert_eq!(
        release_err(&s),
        Some(Error::Feature(FeatureError::NoReleasableReserve))
    );

    advance(&s, HOLD);
    assert_eq!(s.client.release_rolling_reserve(&s.merchant), 2);
    assert_eq!(balance(&s, &s.token, &s.merchant), 2_000);
}

#[test]
fn held_funds_are_tracked_per_token() {
    let s = setup();
    let other = s
        .env
        .register_stellar_asset_contract_v2(s.admin.clone())
        .address();
    fund_customer(&s.env, &s.client.address, &other, &s.customer);
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &2_500, &HOLD);

    pay(&s, &s.token, 1_000);
    pay(&s, &other, 400);
    assert_eq!(
        s.client.get_rolling_reserve_held(&s.merchant, &s.token),
        250
    );
    assert_eq!(s.client.get_rolling_reserve_held(&s.merchant, &other), 100);

    advance(&s, HOLD);
    assert_eq!(s.client.release_rolling_reserve(&s.merchant), 2);
    assert_eq!(balance(&s, &s.token, &s.merchant), 1_000);
    assert_eq!(balance(&s, &other, &s.merchant), 400);
}

#[test]
fn finality_delayed_settlement_is_withheld_on_finalize() {
    let s = setup();
    // Finality-delayed settlements are paid from the contract's own balance.
    token::StellarAssetClient::new(&s.env, &s.token).mint(&s.client.address, &1_000);
    s.client.configure_finality_delay(
        &s.admin,
        &FinalityConfig {
            delay_seconds: 100,
            min_amount_threshold: 1,
            active: true,
        },
    );
    s.client
        .set_rolling_reserve(&s.admin, &s.merchant, &1_000, &HOLD);

    let payment_id = pay(&s, &s.token, 1_000);
    assert_eq!(balance(&s, &s.token, &s.merchant), 0);

    advance(&s, 100);
    s.client.finalize_pending_settlement(&payment_id);
    assert_eq!(balance(&s, &s.token, &s.merchant), 900);
    assert_eq!(
        s.client.get_rolling_reserve_held(&s.merchant, &s.token),
        100
    );
    let slice = s
        .client
        .get_rolling_reserve_slices(&s.merchant)
        .get(0)
        .unwrap();
    assert_eq!(slice.release_at, 10_100 + HOLD);
}
