#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{Address, Env, String, Symbol, TryFromVal};

struct Setup<'a> {
    env: Env,
    client: RefundContractClient<'a>,
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
    let token = crate::test_funded_token(&env, &contract_id);

    Setup {
        env,
        client,
        admin,
        merchant,
        token,
    }
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

fn count_events(env: &Env, name: &str) -> usize {
    let expected = Symbol::new(env, name);
    env.events()
        .all()
        .iter()
        .filter(|(_, topics, _)| {
            topics
                .get(0)
                .and_then(|t| Symbol::try_from_val(env, &t).ok())
                .is_some_and(|s| s == expected)
        })
        .count()
}

fn request_and_approve(
    s: &Setup,
    payment_id: u64,
    refund_amount: i128,
    original_payment_amount: i128,
) -> u64 {
    let customer = Address::generate(&s.env);
    let refund_id = s.client.request_refund(
        &s.merchant,
        &payment_id,
        &customer,
        &refund_amount,
        &original_payment_amount,
        &s.token,
        &String::from_str(&s.env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    );
    s.client.approve_refund(&s.admin, &refund_id);
    refund_id
}

#[test]
fn no_config_never_emits() {
    let s = setup();
    request_and_approve(&s, 1, 900, 1000);
    assert!(!has_event(&s.env, "merchant_refund_ratio_exceeded"));
    assert!(s
        .client
        .get_merchant_refund_ratio_window(&s.merchant)
        .is_none());
}

#[test]
fn disabled_config_never_emits() {
    let s = setup();
    s.client.set_refund_ratio_alert_config(
        &s.admin,
        &RefundRatioAlertConfig {
            threshold_bps: 1000,
            window_seconds: 3600,
            enabled: false,
        },
    );
    request_and_approve(&s, 1, 900, 1000);
    assert!(!has_event(&s.env, "merchant_refund_ratio_exceeded"));
}

#[test]
fn ratio_under_threshold_never_emits() {
    let s = setup();
    s.client.set_refund_ratio_alert_config(
        &s.admin,
        &RefundRatioAlertConfig {
            threshold_bps: 5000,
            window_seconds: 3600,
            enabled: true,
        },
    );
    // 100/1000 = 10% ratio, well under 50% threshold.
    request_and_approve(&s, 1, 100, 1000);
    assert!(!has_event(&s.env, "merchant_refund_ratio_exceeded"));
    let window = s
        .client
        .get_merchant_refund_ratio_window(&s.merchant)
        .unwrap();
    assert_eq!(window.refund_volume, 100);
    assert_eq!(window.payment_volume, 1000);
    assert!(!window.alert_fired);
}

#[test]
fn ratio_over_threshold_fires_once_per_window() {
    let s = setup();
    s.client.set_refund_ratio_alert_config(
        &s.admin,
        &RefundRatioAlertConfig {
            threshold_bps: 2000, // 20%
            window_seconds: 3600,
            enabled: true,
        },
    );

    // 300/1000 = 30% > 20% threshold -> fires.
    request_and_approve(&s, 1, 300, 1000);
    assert_eq!(count_events(&s.env, "merchant_refund_ratio_exceeded"), 1);

    let window = s
        .client
        .get_merchant_refund_ratio_window(&s.merchant)
        .unwrap();
    assert!(window.alert_fired);

    // Still over threshold in the same window: must not fire again. (Each
    // call's event log only reflects that call, so the absence of the event
    // here proves no re-emission, regardless of call #1's event.)
    request_and_approve(&s, 2, 50, 100);
    assert!(!has_event(&s.env, "merchant_refund_ratio_exceeded"));

    let window = s
        .client
        .get_merchant_refund_ratio_window(&s.merchant)
        .unwrap();
    assert_eq!(window.refund_volume, 350);
    assert_eq!(window.payment_volume, 1100);
}

#[test]
fn alert_fires_again_after_window_rolls_over() {
    let s = setup();
    s.client.set_refund_ratio_alert_config(
        &s.admin,
        &RefundRatioAlertConfig {
            threshold_bps: 2000,
            window_seconds: 1000,
            enabled: true,
        },
    );

    request_and_approve(&s, 1, 300, 1000);
    assert_eq!(count_events(&s.env, "merchant_refund_ratio_exceeded"), 1);

    // Roll the window over. Note: advancing the ledger timestamp clears the
    // recorded event log (ledger-close semantics in the test harness), so we
    // check for the second alert's presence after this call rather than a
    // cumulative count across the time jump.
    let now = s.env.ledger().timestamp();
    s.env.ledger().set_timestamp(now + 1001);

    request_and_approve(&s, 2, 300, 1000);
    assert!(has_event(&s.env, "merchant_refund_ratio_exceeded"));

    let window = s
        .client
        .get_merchant_refund_ratio_window(&s.merchant)
        .unwrap();
    assert_eq!(window.refund_volume, 300);
    assert_eq!(window.payment_volume, 1000);
    assert!(window.alert_fired);
}

#[test]
fn approval_always_succeeds_regardless_of_alert() {
    let s = setup();
    s.client.set_refund_ratio_alert_config(
        &s.admin,
        &RefundRatioAlertConfig {
            threshold_bps: 1,
            window_seconds: 3600,
            enabled: true,
        },
    );
    let refund_id = request_and_approve(&s, 1, 999, 1000);
    assert_eq!(
        s.client.get_refund(&refund_id).status,
        RefundStatus::Approved
    );
}

#[test]
fn only_admin_can_set_config() {
    let s = setup();
    let not_admin = Address::generate(&s.env);
    let result = s.client.try_set_refund_ratio_alert_config(
        &not_admin,
        &RefundRatioAlertConfig {
            threshold_bps: 1000,
            window_seconds: 3600,
            enabled: true,
        },
    );
    assert_eq!(result, Err(Ok(Error::Core(CoreError::Unauthorized))));
}
