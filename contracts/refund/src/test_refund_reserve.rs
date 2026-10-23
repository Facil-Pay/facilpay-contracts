#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events};
use soroban_sdk::{token, Address, Env, Symbol, TryFromVal};

const MERCHANT_FUNDS: i128 = 10_000;

struct Setup<'a> {
    env: Env,
    client: RefundContractClient<'a>,
    contract_id: Address,
    admin: Address,
    merchant: Address,
    token: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&merchant, &MERCHANT_FUNDS);

    Setup {
        env,
        client,
        contract_id,
        admin,
        merchant,
        token,
    }
}

fn balance(s: &Setup, who: &Address) -> i128 {
    token::Client::new(&s.env, &s.token).balance(who)
}

fn has_event(env: &Env, name: &str) -> bool {
    let expected = Symbol::new(env, name);
    env.events().all().iter().any(|(_, topics, _)| {
        topics
            .get(0)
            .and_then(|t| Symbol::try_from_val(env, &t).ok())
            .is_some_and(|s| s == expected)
    })
}

/// True if `who` authorized a top-level call to `fn_name` in the last invocation.
fn authorized(env: &Env, who: &Address, fn_name: &str) -> bool {
    let expected = Symbol::new(env, fn_name);
    env.auths().iter().any(|(addr, invocation)| {
        addr == who
            && matches!(
                &invocation.function,
                AuthorizedFunction::Contract((_, f, _)) if *f == expected
            )
    })
}

#[test]
fn deposit_and_withdraw_move_tokens_and_track_storage() {
    let s = setup();
    assert!(!s.client.has_refund_reserve(&s.merchant, &s.token));
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 0);

    assert_eq!(
        s.client
            .deposit_refund_reserve(&s.merchant, &s.token, &4_000),
        4_000
    );
    assert!(has_event(&s.env, "refund_reserve_deposited"));
    assert!(s.client.has_refund_reserve(&s.merchant, &s.token));
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 4_000);
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS - 4_000);
    assert_eq!(balance(&s, &s.contract_id), 4_000);

    // A second deposit accumulates.
    assert_eq!(
        s.client
            .deposit_refund_reserve(&s.merchant, &s.token, &1_000),
        5_000
    );
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 5_000);
    assert_eq!(balance(&s, &s.contract_id), 5_000);

    assert_eq!(
        s.client
            .withdraw_refund_reserve(&s.merchant, &s.token, &1_500),
        3_500
    );
    assert!(has_event(&s.env, "refund_reserve_withdrawn"));
    assert!(!has_event(&s.env, "refund_reserve_low"));
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 3_500);
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS - 3_500);
    assert_eq!(balance(&s, &s.contract_id), 3_500);

    // Draining the reserve keeps the merchant opted in, at a zero balance.
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &3_500);
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 0);
    assert!(!has_event(&s.env, "refund_reserve_low"));
    assert!(s.client.has_refund_reserve(&s.merchant, &s.token));
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS);
    assert_eq!(balance(&s, &s.contract_id), 0);
}

#[test]
fn refund_reserve_low_fires_only_when_crossing_below_warning_level() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &5_000);
    s.client
        .set_refund_reserve_warning_level(&s.admin, &s.merchant, &s.token, &2_000);
    assert!(has_event(&s.env, "refund_reserve_warning_level_set"));
    assert_eq!(
        s.client.get_refund_reserve_warning_level(&s.merchant, &s.token),
        2_000
    );

    // Still above the warning level: no alert.
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &1_000);
    assert!(!has_event(&s.env, "refund_reserve_low"));

    // Crossing below the warning level fires the alert once.
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &2_500);
    assert!(has_event(&s.env, "refund_reserve_low"));

    // Staying below does not re-fire on subsequent withdrawals.
    s.env.events().all();
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &500);
    assert!(!has_event(&s.env, "refund_reserve_low"));

    // Topping back up above the level re-arms the alert.
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &2_000);
    assert!(!has_event(&s.env, "refund_reserve_low"));
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &2_500);
    assert!(has_event(&s.env, "refund_reserve_low"));
}

#[test]
fn refund_reserve_low_fires_on_refund() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &3_000);
    s.client
        .set_refund_reserve_warning_level(&s.admin, &s.merchant, &s.token, &1_000);

    s.client
        .refund(&s.merchant, &s.token, &s.merchant, &1_500);
    assert!(!has_event(&s.env, "refund_reserve_low"));

    s.client
        .refund(&s.merchant, &s.token, &s.merchant, &1_000);
    assert!(has_event(&s.env, "refund_reserve_low"));
}

#[test]
fn set_refund_reserve_warning_level_is_admin_only() {
    let s = setup();
    let stranger = Address::generate(&s.env);

    s.client
        .set_refund_reserve_warning_level(&s.admin, &s.merchant, &s.token, &1_000);
    assert!(authorized(&s.env, &s.admin, "set_refund_reserve_warning_level"));

    assert_eq!(
        s.client
            .try_set_refund_reserve_warning_level(&s.merchant, &s.merchant, &s.token, &0),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
    assert_eq!(
        s.client
            .try_set_refund_reserve_warning_level(&stranger, &s.merchant, &s.token, &0),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
    assert_eq!(
        s.client
            .try_set_refund_reserve_warning_level(&s.admin, &s.merchant, &s.token, &-1),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
    assert_eq!(
        s.client.get_refund_reserve_warning_level(&s.merchant, &s.token),
        1_000
    );
}

#[test]
fn reserves_are_isolated_per_merchant_and_token() {
    let s = setup();
    let other_merchant = Address::generate(&s.env);
    let other_token = s
        .env
        .register_stellar_asset_contract_v2(s.admin.clone())
        .address();

    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &2_000);
    s.client
        .set_min_refund_reserve(&s.admin, &s.merchant, &s.token, &500);

    assert!(!s.client.has_refund_reserve(&other_merchant, &s.token));
    assert!(!s.client.has_refund_reserve(&s.merchant, &other_token));
    assert_eq!(s.client.get_refund_reserve(&other_merchant, &s.token), 0);
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &other_token), 0);
    assert_eq!(
        s.client.get_min_refund_reserve(&other_merchant, &s.token),
        0
    );
    assert_eq!(
        s.client.get_min_refund_reserve(&s.merchant, &other_token),
        0
    );

    // Another merchant cannot draw on this merchant's reserve.
    assert_eq!(
        s.client
            .try_withdraw_refund_reserve(&other_merchant, &s.token, &1),
        Err(Ok(Error::Ext(ExtError::InsufficientRefundReserve)))
    );
}

#[test]
fn withdrawal_below_minimum_is_rejected() {
    let s = setup();
    assert_eq!(s.client.get_min_refund_reserve(&s.merchant, &s.token), 0);

    s.client
        .set_min_refund_reserve(&s.admin, &s.merchant, &s.token, &2_000);
    assert!(has_event(&s.env, "min_refund_reserve_set"));
    assert_eq!(
        s.client.get_min_refund_reserve(&s.merchant, &s.token),
        2_000
    );
    // Setting a minimum does not by itself opt the merchant into a reserve.
    assert!(!s.client.has_refund_reserve(&s.merchant, &s.token));

    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &5_000);

    assert_eq!(
        s.client
            .try_withdraw_refund_reserve(&s.merchant, &s.token, &3_001),
        Err(Ok(Error::Ext(ExtError::ReserveBelowMinimum)))
    );
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 5_000);
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS - 5_000);
    assert_eq!(balance(&s, &s.contract_id), 5_000);

    // Withdrawing down to exactly the minimum is allowed.
    assert_eq!(
        s.client
            .withdraw_refund_reserve(&s.merchant, &s.token, &3_000),
        2_000
    );
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS - 2_000);
    assert_eq!(balance(&s, &s.contract_id), 2_000);

    assert_eq!(
        s.client
            .try_withdraw_refund_reserve(&s.merchant, &s.token, &1),
        Err(Ok(Error::Ext(ExtError::ReserveBelowMinimum)))
    );

    // Lowering the minimum frees the rest.
    s.client
        .set_min_refund_reserve(&s.admin, &s.merchant, &s.token, &0);
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &2_000);
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 0);
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS);
}

#[test]
fn invalid_amounts_are_rejected() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &1_000);

    for amount in [0, -1] {
        assert_eq!(
            s.client
                .try_deposit_refund_reserve(&s.merchant, &s.token, &amount),
            Err(Ok(Error::Core(CoreError::InvalidAmount)))
        );
        assert_eq!(
            s.client
                .try_withdraw_refund_reserve(&s.merchant, &s.token, &amount),
            Err(Ok(Error::Core(CoreError::InvalidAmount)))
        );
    }
    assert_eq!(
        s.client
            .try_set_min_refund_reserve(&s.admin, &s.merchant, &s.token, &-1),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
    assert_eq!(
        s.client
            .try_withdraw_refund_reserve(&s.merchant, &s.token, &1_001),
        Err(Ok(Error::Ext(ExtError::InsufficientRefundReserve)))
    );

    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 1_000);
    assert_eq!(s.client.get_min_refund_reserve(&s.merchant, &s.token), 0);
    assert_eq!(balance(&s, &s.contract_id), 1_000);
}

#[test]
fn deposit_and_withdraw_require_merchant_auth() {
    let s = setup();

    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &1_000);
    assert!(authorized(&s.env, &s.merchant, "deposit_refund_reserve"));

    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &400);
    assert!(authorized(&s.env, &s.merchant, "withdraw_refund_reserve"));

    // With no authorizations available, neither call goes through.
    s.env.set_auths(&[]);
    assert!(s
        .client
        .try_deposit_refund_reserve(&s.merchant, &s.token, &100)
        .is_err());
    assert!(s
        .client
        .try_withdraw_refund_reserve(&s.merchant, &s.token, &100)
        .is_err());
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 600);
    assert_eq!(balance(&s, &s.merchant), MERCHANT_FUNDS - 600);
    assert_eq!(balance(&s, &s.contract_id), 600);
}

#[test]
fn set_min_refund_reserve_is_admin_only() {
    let s = setup();
    let stranger = Address::generate(&s.env);

    s.client
        .set_min_refund_reserve(&s.admin, &s.merchant, &s.token, &1_000);
    assert!(authorized(&s.env, &s.admin, "set_min_refund_reserve"));

    // Neither the merchant itself nor an arbitrary address can change it.
    assert_eq!(
        s.client
            .try_set_min_refund_reserve(&s.merchant, &s.merchant, &s.token, &0),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
    assert_eq!(
        s.client
            .try_set_min_refund_reserve(&stranger, &s.merchant, &s.token, &0),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
    assert_eq!(
        s.client.get_min_refund_reserve(&s.merchant, &s.token),
        1_000
    );

    // The real admin still needs to sign.
    s.env.set_auths(&[]);
    assert!(s
        .client
        .try_set_min_refund_reserve(&s.admin, &s.merchant, &s.token, &0)
        .is_err());
    assert_eq!(
        s.client.get_min_refund_reserve(&s.merchant, &s.token),
        1_000
    );
}
