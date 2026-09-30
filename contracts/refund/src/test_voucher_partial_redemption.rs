#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{token, Address, Env, String, Symbol, TryFromVal};

const EXPIRY: u64 = 1_000;
const VALUE: i128 = 1_000;

struct Setup<'a> {
    env: Env,
    client: RefundContractClient<'a>,
    admin: Address,
    merchant: Address,
    customer: Address,
    token: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&contract_id, &1_000_000);

    Setup {
        merchant: Address::generate(&env),
        customer: Address::generate(&env),
        env,
        client,
        admin,
        token,
    }
}

/// Requests and approves a refund on `payment_id`, then issues a voucher for it.
fn issue_voucher(s: &Setup, payment_id: u64, amount: i128) -> u64 {
    let refund_id = s.client.request_refund(
        &s.merchant,
        &payment_id,
        &s.customer,
        &amount,
        &amount,
        &s.token,
        &String::from_str(&s.env, "store credit"),
        &RefundReasonCode::CustomerRequest,
        &s.env.ledger().timestamp(),
    );
    s.client.approve_refund(&s.admin, &refund_id);
    s.client.issue_refund_voucher(&s.admin, &refund_id, &EXPIRY)
}

fn balance_of(s: &Setup, who: &Address) -> i128 {
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
fn unredeemed_voucher_balance_is_full_value() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    assert_eq!(s.client.get_voucher_balance(&voucher_id), VALUE);
}

#[test]
fn partial_redemption_transfers_amount_and_reduces_balance() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);

    let remaining = s
        .client
        .redeem_voucher_amount(&s.customer, &voucher_id, &300);
    assert!(has_event(&s.env, "voucher_amount_redeemed"));

    assert_eq!(remaining, 700);
    assert_eq!(s.client.get_voucher_balance(&voucher_id), 700);
    assert_eq!(balance_of(&s, &s.customer), 300);
    assert!(!s.client.get_voucher(&voucher_id).unwrap().redeemed);
}

#[test]
fn multiple_partial_redemptions_mark_voucher_redeemed_at_zero() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);

    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &250);
    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &250);
    let remaining = s
        .client
        .redeem_voucher_amount(&s.customer, &voucher_id, &500);

    assert_eq!(remaining, 0);
    assert_eq!(s.client.get_voucher_balance(&voucher_id), 0);
    assert!(s.client.get_voucher(&voucher_id).unwrap().redeemed);
    assert_eq!(balance_of(&s, &s.customer), VALUE);

    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&s.customer, &voucher_id, &1),
        Err(Ok(Error::Ext(ExtError::VoucherAlreadyRedeemed)))
    );
}

#[test]
fn redemption_above_remaining_balance_is_rejected() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &600);

    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&s.customer, &voucher_id, &401),
        Err(Ok(Error::Ext(ExtError::VoucherInsufficientBalance)))
    );
    // A rejected attempt leaves the balance untouched.
    assert_eq!(s.client.get_voucher_balance(&voucher_id), 400);
    assert_eq!(balance_of(&s, &s.customer), 600);
}

#[test]
fn redemption_above_voucher_value_is_rejected() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&s.customer, &voucher_id, &(VALUE + 1)),
        Err(Ok(Error::Ext(ExtError::VoucherInsufficientBalance)))
    );
}

#[test]
fn full_redeem_after_partial_pays_only_remaining_balance() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &400);

    s.client.redeem_refund_voucher(&s.customer, &voucher_id, &1);

    assert_eq!(balance_of(&s, &s.customer), VALUE);
    assert_eq!(s.client.get_voucher_balance(&voucher_id), 0);
    assert!(s.client.get_voucher(&voucher_id).unwrap().redeemed);
}

#[test]
fn non_positive_amount_is_rejected() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&s.customer, &voucher_id, &0),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&s.customer, &voucher_id, &-5),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
}

#[test]
fn expiry_applies_to_remaining_balance() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    let expires_at = s.client.get_voucher(&voucher_id).unwrap().expires_at;
    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &200);

    // Still redeemable at the exact expiry boundary.
    s.env.ledger().set_timestamp(expires_at);
    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &100);

    s.env.ledger().set_timestamp(expires_at + 1);
    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&s.customer, &voucher_id, &100),
        Err(Ok(Error::Ext(ExtError::VoucherExpired)))
    );
    assert_eq!(
        s.client
            .try_redeem_refund_voucher(&s.customer, &voucher_id, &1),
        Err(Ok(Error::Ext(ExtError::VoucherExpired)))
    );
    assert_eq!(s.client.get_voucher_balance(&voucher_id), 700);
    assert_eq!(balance_of(&s, &s.customer), 300);
}

#[test]
fn only_owner_can_redeem_amount() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_redeem_voucher_amount(&stranger, &voucher_id, &100),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
}

#[test]
fn remaining_balance_follows_voucher_on_transfer() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, VALUE);
    let new_owner = Address::generate(&s.env);
    s.client.redeem_voucher_amount(&s.customer, &voucher_id, &300);
    s.client
        .transfer_voucher(&s.customer, &voucher_id, &new_owner);

    assert_eq!(s.client.get_voucher_balance(&voucher_id), 700);
    s.client.redeem_refund_voucher(&new_owner, &voucher_id, &1);
    assert_eq!(balance_of(&s, &new_owner), 700);
}

#[test]
fn unknown_voucher_is_rejected() {
    let s = setup();
    assert_eq!(
        s.client.try_redeem_voucher_amount(&s.customer, &99, &100),
        Err(Ok(Error::Ext(ExtError::VoucherNotFound)))
    );
    assert_eq!(
        s.client.try_get_voucher_balance(&99),
        Err(Ok(Error::Ext(ExtError::VoucherNotFound)))
    );
}
