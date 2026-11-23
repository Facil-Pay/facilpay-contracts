#![cfg(test)]
//! Tests for #920: third-party deposits into a merchant's refund reserve.

use super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env, Symbol, TryFromVal};

struct Setup<'a> {
    env: Env,
    client: RefundContractClient<'a>,
    contract_id: Address,
    admin: Address,
    merchant: Address,
    funder: Address,
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
    let funder = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let sac = token::StellarAssetClient::new(&env, &token);
    sac.mint(&merchant, &10_000);
    sac.mint(&funder, &10_000);
    Setup {
        env,
        client,
        contract_id,
        admin,
        merchant,
        funder,
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

#[test]
fn deposit_for_transfers_from_funder_and_emits_event_with_funder() {
    let s = setup();
    let new_balance = s
        .client
        .deposit_refund_reserve_for(&s.funder, &s.merchant, &s.token, &3_000);
    assert_eq!(new_balance, 3_000);

    let event = RefundReserveDepositedFor {
        funder: s.funder.clone(),
        merchant: s.merchant.clone(),
        token: s.token.clone(),
        amount: 3_000,
        new_balance: 3_000,
    };
    let all = s.env.events().all();
    assert_eq!(
        all.slice(all.len() - 1..),
        soroban_sdk::vec![
            &s.env,
            (
                s.contract_id.clone(),
                soroban_sdk::Event::topics(&event, &s.env),
                soroban_sdk::Event::data(&event, &s.env)
            )
        ]
    );
    assert!(has_event(&s.env, "refund_reserve_deposited_for"));

    assert_eq!(balance(&s, &s.funder), 7_000);
    assert_eq!(balance(&s, &s.merchant), 10_000);
    assert_eq!(balance(&s, &s.contract_id), 3_000);
    assert!(s.client.has_refund_reserve(&s.merchant, &s.token));
    assert_eq!(
        s.client
            .get_reserve_contribution(&s.merchant, &s.token, &s.funder),
        3_000
    );
    assert_eq!(
        s.client.get_locked_refund_reserve(&s.merchant, &s.token),
        3_000
    );
}

#[test]
fn merchant_cannot_withdraw_platform_funds_until_admin_releases_them() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &1_000);
    s.client
        .deposit_refund_reserve_for(&s.funder, &s.merchant, &s.token, &2_000);

    // The merchant's own 1_000 is withdrawable; the funder's 2_000 is not.
    assert_eq!(
        s.client
            .try_withdraw_refund_reserve(&s.merchant, &s.token, &1_001),
        Err(Ok(Error::Core(CoreError::ThirdPartyReserveLocked)))
    );
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &1_000);
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 2_000);

    // Only the admin can release, and not more than was contributed.
    assert_eq!(
        s.client.try_release_reserve_contribution(
            &s.merchant,
            &s.merchant,
            &s.token,
            &s.funder,
            &500
        ),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
    assert_eq!(
        s.client.try_release_reserve_contribution(
            &s.admin,
            &s.merchant,
            &s.token,
            &s.funder,
            &2_001
        ),
        Err(Ok(Error::Core(CoreError::ContributionExceeded)))
    );
    assert_eq!(
        s.client
            .release_reserve_contribution(&s.admin, &s.merchant, &s.token, &s.funder, &500),
        1_500
    );
    assert!(has_event(&s.env, "reserve_contribution_released"));
    assert_eq!(
        s.client.get_locked_refund_reserve(&s.merchant, &s.token),
        1_500
    );
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &500);
    assert_eq!(
        s.client
            .try_withdraw_refund_reserve(&s.merchant, &s.token, &1),
        Err(Ok(Error::Core(CoreError::ThirdPartyReserveLocked)))
    );
}

#[test]
fn low_reserve_alert_rearms_on_third_party_top_up() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &3_000);
    s.client
        .set_refund_reserve_warning_level(&s.admin, &s.merchant, &s.token, &2_000);

    // Drop below: fires.
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &1_500);
    assert!(has_event(&s.env, "refund_reserve_low"));

    // Third-party top-up brings it back above the level.
    s.client
        .deposit_refund_reserve_for(&s.funder, &s.merchant, &s.token, &1_000);
    assert!(!has_event(&s.env, "refund_reserve_low"));
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 2_500);

    // Releasing the funder's lock lets the merchant drop below again: re-fires.
    s.client
        .release_reserve_contribution(&s.admin, &s.merchant, &s.token, &s.funder, &1_000);
    s.client
        .withdraw_refund_reserve(&s.merchant, &s.token, &1_000);
    assert!(has_event(&s.env, "refund_reserve_low"));
}

#[test]
fn deposit_for_rejects_non_positive_amounts() {
    let s = setup();
    assert_eq!(
        s.client
            .try_deposit_refund_reserve_for(&s.funder, &s.merchant, &s.token, &0),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
}
