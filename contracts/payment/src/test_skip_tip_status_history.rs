#![cfg(test)]
//! Tests for #666 (skip a subscription billing cycle), #675 (fee-exempt tips)
//! and #682 (payment status history).

use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String, Vec,
};

use crate::{
    BasicError, Currency, Error, FeeConfig, PaymentContract, PaymentContractClient, PaymentError,
    PaymentStatus, SplitRecipient, SubscriptionError, SubscriptionStatus, SKIP_WINDOW_SECONDS,
};

const INTERVAL: u64 = 2_592_000; // 30 days

struct Fixture {
    env: Env,
    client: PaymentContractClient<'static>,
    contract_id: Address,
    admin: Address,
    customer: Address,
    merchant: Address,
    token: Address,
    token_client: token::Client<'static>,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let token_client = token::Client::new(&env, &token);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&customer, &100_000);
    token_client.approve(&customer, &contract_id, &100_000, &100_000);

    Fixture {
        env,
        client,
        contract_id,
        admin,
        customer,
        merchant,
        token,
        token_client,
    }
}

fn enable_fees(f: &Fixture) {
    f.client.set_fee_config(
        &f.admin,
        &FeeConfig {
            fee_bps: 100, // 1%
            min_fee: 0,
            max_fee: 0,
            treasury: f.admin.clone(),
            fee_token: f.token.clone(),
            active: true,
        },
    );
}

fn create_plain(f: &Fixture, amount: i128) -> u64 {
    f.client.create_payment(
        &f.customer,
        &f.merchant,
        &amount,
        &f.token,
        &Currency::USDC,
        &0u64,
        &String::from_str(&f.env, ""),
    )
}

fn create_tipped(f: &Fixture, amount: i128, tip: i128) -> u64 {
    let payment_id = create_plain(f, amount);
    if tip > 0 {
        f.client.add_tip(&f.customer, &payment_id, &tip);
    }
    payment_id
}

fn create_sub(f: &Fixture) -> u64 {
    f.client.create_subscription(
        &f.customer,
        &f.merchant,
        &100,
        &f.token,
        &Currency::USDC,
        &INTERVAL,
        &0,
        &3,
        &String::from_str(&f.env, ""),
        &0,
    )
}

// ── #666 skip_next_cycle ──────────────────────────────────────────────────────

#[test]
fn skip_advances_next_billing_by_exactly_one_interval() {
    let f = setup();
    let sub_id = create_sub(&f);
    let before = f.client.get_subscription(&sub_id);
    let customer_balance = f.token_client.balance(&f.customer);

    let next = f.client.skip_next_cycle(&f.customer, &sub_id);

    let after = f.client.get_subscription(&sub_id);
    assert_eq!(next, before.next_payment_at + INTERVAL);
    assert_eq!(after.next_payment_at, before.next_payment_at + INTERVAL);
    assert_eq!(after.status, SubscriptionStatus::Active);
    assert_eq!(after.payment_count, before.payment_count);
    assert_eq!(f.token_client.balance(&f.customer), customer_balance);
    assert_eq!(f.client.get_skip_usage(&sub_id).count, 1);
}

#[test]
fn skipped_cycle_is_not_due_until_the_following_interval() {
    let f = setup();
    let sub_id = create_sub(&f);
    let original_due = f.client.get_subscription(&sub_id).next_payment_at;
    f.client.skip_next_cycle(&f.customer, &sub_id);

    f.env.ledger().set_timestamp(original_due);
    assert_eq!(
        f.client.try_execute_recurring_payment(&sub_id),
        Err(Ok(Error::Subscription(SubscriptionError::PaymentNotDue)))
    );
}

#[test]
fn skips_beyond_merchant_cap_are_rejected() {
    let f = setup();
    f.client.set_skip_cap(&f.merchant, &2);
    assert_eq!(f.client.get_skip_cap(&f.merchant), Some(2));
    let sub_id = create_sub(&f);

    f.client.skip_next_cycle(&f.customer, &sub_id);
    let next = f.client.skip_next_cycle(&f.customer, &sub_id);
    assert_eq!(
        f.client.try_skip_next_cycle(&f.customer, &sub_id),
        Err(Ok(Error::Subscription(SubscriptionError::SkipCapExceeded)))
    );
    // The rejected skip did not move the billing date.
    assert_eq!(f.client.get_subscription(&sub_id).next_payment_at, next);
}

#[test]
fn zero_cap_disables_skipping() {
    let f = setup();
    f.client.set_skip_cap(&f.merchant, &0);
    let sub_id = create_sub(&f);
    assert_eq!(
        f.client.try_skip_next_cycle(&f.customer, &sub_id),
        Err(Ok(Error::Subscription(SubscriptionError::SkipCapExceeded)))
    );
}

#[test]
fn skip_cap_resets_after_a_rolling_year() {
    let f = setup();
    f.client.set_skip_cap(&f.merchant, &1);
    let sub_id = create_sub(&f);

    f.client.skip_next_cycle(&f.customer, &sub_id);
    assert!(f.client.try_skip_next_cycle(&f.customer, &sub_id).is_err());

    f.env.ledger().set_timestamp(1_000 + SKIP_WINDOW_SECONDS);
    f.client.skip_next_cycle(&f.customer, &sub_id);
    let usage = f.client.get_skip_usage(&sub_id);
    assert_eq!(usage.count, 1);
    assert_eq!(usage.window_start, 1_000 + SKIP_WINDOW_SECONDS);
}

#[test]
fn skip_rejects_non_customer_and_inactive_subscription() {
    let f = setup();
    let sub_id = create_sub(&f);
    assert_eq!(
        f.client.try_skip_next_cycle(&f.merchant, &sub_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );

    f.client.pause_subscription(&f.customer, &sub_id);
    assert_eq!(
        f.client.try_skip_next_cycle(&f.customer, &sub_id),
        Err(Ok(Error::Subscription(SubscriptionError::NotActive)))
    );
    assert_eq!(
        f.client.try_skip_next_cycle(&f.customer, &999),
        Err(Ok(Error::Subscription(SubscriptionError::NotFound)))
    );
}

// ── #675 tips ─────────────────────────────────────────────────────────────────

#[test]
fn tip_is_escrowed_at_creation() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 50);
    assert_eq!(f.client.get_payment_tip(&payment_id), 50);
    assert_eq!(f.token_client.balance(&f.contract_id), 50);
    assert_eq!(f.token_client.balance(&f.customer), 100_000 - 50);
}

#[test]
fn fees_are_computed_on_amount_only_and_tip_goes_to_merchant_in_full() {
    let f = setup();
    enable_fees(&f);
    let payment_id = create_tipped(&f, 1_000, 50);

    f.client.complete_payment(&f.admin, &payment_id);

    // 1% of 1_000 = 10 fee; the 50 tip is untouched by fees.
    assert_eq!(f.client.get_accumulated_fees(), 10);
    assert_eq!(f.token_client.balance(&f.merchant), 990 + 50);
    assert_eq!(f.token_client.balance(&f.customer), 100_000 - 1_000 - 50);
    // Only the collected fee remains in the contract.
    assert_eq!(f.token_client.balance(&f.contract_id), 10);
}

#[test]
fn refund_returns_tip_to_customer() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 50);
    f.client.refund_payment(&f.admin, &payment_id);
    assert_eq!(f.token_client.balance(&f.customer), 100_000);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);
}

#[test]
fn cancel_returns_tip_to_customer() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 50);
    f.client.cancel_payment(&f.customer, &payment_id);
    assert_eq!(f.token_client.balance(&f.customer), 100_000);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);
}

#[test]
fn full_partial_refund_returns_tip_to_customer() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 50);
    f.client.partial_refund(&f.admin, &payment_id, &400);
    assert_eq!(f.token_client.balance(&f.contract_id), 50);
    f.client.partial_refund(&f.admin, &payment_id, &600);
    assert_eq!(f.token_client.balance(&f.contract_id), 0);
    assert_eq!(f.token_client.balance(&f.customer), 100_000);
}

#[test]
fn refund_of_completed_payment_returns_amount_and_tip() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 50);
    f.client.complete_payment(&f.admin, &payment_id);
    assert_eq!(f.token_client.balance(&f.merchant), 1_050);

    f.client.refund_completed_payment(&f.merchant, &payment_id);

    assert_eq!(f.token_client.balance(&f.merchant), 0);
    assert_eq!(f.token_client.balance(&f.customer), 100_000);
    let payment = f.client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Refunded);
    assert_eq!(payment.refunded_amount, 1_000);
}

#[test]
fn repeated_tips_accumulate() {
    let f = setup();
    let payment_id = create_plain(&f, 1_000);
    assert_eq!(f.client.add_tip(&f.customer, &payment_id, &20), 20);
    assert_eq!(f.client.add_tip(&f.customer, &payment_id, &30), 50);
    assert_eq!(f.client.get_payment_tip(&payment_id), 50);
    assert_eq!(f.token_client.balance(&f.contract_id), 50);
}

#[test]
fn non_positive_tip_is_rejected() {
    let f = setup();
    let payment_id = create_plain(&f, 1_000);
    for tip in [0i128, -1] {
        assert_eq!(
            f.client.try_add_tip(&f.customer, &payment_id, &tip),
            Err(Ok(Error::Basic(BasicError::InvalidAmount)))
        );
    }
}

#[test]
fn tip_rejected_for_wrong_customer_or_non_pending_payment() {
    let f = setup();
    let payment_id = create_plain(&f, 1_000);
    assert_eq!(
        f.client.try_add_tip(&f.merchant, &payment_id, &10),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        f.client.try_add_tip(&f.customer, &999, &10),
        Err(Ok(Error::Payment(PaymentError::NotFound)))
    );
    f.client.complete_payment(&f.admin, &payment_id);
    assert_eq!(
        f.client.try_add_tip(&f.customer, &payment_id, &10),
        Err(Ok(Error::Payment(PaymentError::InvalidStatus)))
    );
}

#[test]
fn tip_rejected_on_split_payment() {
    let f = setup();
    let mut recipients = Vec::new(&f.env);
    recipients.push_back(SplitRecipient {
        address: f.merchant.clone(),
        share_bps: 10_000,
    });
    let payment_id =
        f.client
            .create_split_payment(&f.customer, &f.merchant, &1_000, &f.token, &recipients);
    assert_eq!(
        f.client.try_add_tip(&f.customer, &payment_id, &10),
        Err(Ok(Error::Payment(PaymentError::InvalidStatus)))
    );
}

#[test]
fn refund_completed_payment_rejects_wrong_merchant_and_status() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 0);
    assert_eq!(
        f.client
            .try_refund_completed_payment(&f.merchant, &payment_id),
        Err(Ok(Error::Payment(PaymentError::InvalidStatus)))
    );
    f.client.complete_payment(&f.admin, &payment_id);
    assert_eq!(
        f.client
            .try_refund_completed_payment(&f.customer, &payment_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
}

// ── #682 status history ───────────────────────────────────────────────────────

#[test]
fn completed_then_refunded_payment_has_three_ordered_entries() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 0);

    f.env.ledger().set_timestamp(2_000);
    f.client.complete_payment(&f.admin, &payment_id);
    f.env.ledger().set_timestamp(3_000);
    f.client.refund_completed_payment(&f.merchant, &payment_id);

    let history = f.client.get_payment_status_history(&payment_id);
    assert_eq!(history.len(), 3);

    let created = history.get(0).unwrap();
    assert_eq!(created.status, PaymentStatus::Pending);
    assert_eq!(created.timestamp, 1_000);
    assert_eq!(created.actor, f.customer);

    let completed = history.get(1).unwrap();
    assert_eq!(completed.status, PaymentStatus::Completed);
    assert_eq!(completed.timestamp, 2_000);
    assert_eq!(completed.actor, f.admin);

    let refunded = history.get(2).unwrap();
    assert_eq!(refunded.status, PaymentStatus::Refunded);
    assert_eq!(refunded.timestamp, 3_000);
    assert_eq!(refunded.actor, f.merchant);
}

#[test]
fn cancel_is_recorded_with_caller() {
    let f = setup();
    let payment_id = create_plain(&f, 1_000);
    f.client.cancel_payment(&f.merchant, &payment_id);

    let history = f.client.get_payment_status_history(&payment_id);
    assert_eq!(history.len(), 2);
    let cancelled = history.get(1).unwrap();
    assert_eq!(cancelled.status, PaymentStatus::Cancelled);
    assert_eq!(cancelled.actor, f.merchant);
}

#[test]
fn partial_refunds_record_only_actual_status_changes() {
    let f = setup();
    let payment_id = create_tipped(&f, 1_000, 0);
    f.client.partial_refund(&f.admin, &payment_id, &100);
    f.client.partial_refund(&f.admin, &payment_id, &100);
    f.client.partial_refund(&f.admin, &payment_id, &800);

    let history = f.client.get_payment_status_history(&payment_id);
    assert_eq!(history.len(), 3);
    assert_eq!(
        history.get(1).unwrap().status,
        PaymentStatus::PartialRefunded
    );
    assert_eq!(history.get(2).unwrap().status, PaymentStatus::Refunded);
}

#[test]
fn expiry_is_recorded_with_contract_as_actor() {
    let f = setup();
    let payment_id = f.client.create_payment(
        &f.customer,
        &f.merchant,
        &1_000,
        &f.token,
        &Currency::USDC,
        &100u64,
        &String::from_str(&f.env, ""),
    );
    f.env.ledger().set_timestamp(1_000 + 101);
    f.client.expire_payment(&payment_id);

    let history = f.client.get_payment_status_history(&payment_id);
    assert_eq!(history.len(), 2);
    let expired = history.get(1).unwrap();
    assert_eq!(expired.status, PaymentStatus::Cancelled);
    assert_eq!(expired.actor, f.contract_id);
}

#[test]
fn unknown_payment_has_empty_history() {
    let f = setup();
    assert_eq!(f.client.get_payment_status_history(&42).len(), 0);
}
