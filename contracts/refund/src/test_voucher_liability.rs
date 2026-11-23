#![cfg(test)]

//! Issue #914: outstanding voucher liability tracking and cap.

use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{token, vec, Address, Env, String, Symbol, TryFromVal};

const EXPIRY: u64 = 1_000;

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
    let sac = token::StellarAssetClient::new(&env, &token);
    sac.mint(&contract_id, &1_000_000);
    let merchant = Address::generate(&env);
    sac.mint(&merchant, &1_000_000);

    Setup {
        customer: Address::generate(&env),
        env,
        client,
        admin,
        merchant,
        token,
    }
}

fn approved_refund(s: &Setup, payment_id: u64, amount: i128) -> u64 {
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
    refund_id
}

fn issue_voucher(s: &Setup, payment_id: u64, amount: i128) -> u64 {
    let refund_id = approved_refund(s, payment_id, amount);
    s.client.issue_refund_voucher(&s.admin, &refund_id, &EXPIRY)
}

fn liability(s: &Setup) -> i128 {
    s.client.get_voucher_liability(&s.merchant, &s.token)
}

/// The contract error a `try_*` call failed with, if any.
fn contract_err<T, C, I>(result: Result<Result<T, C>, Result<Error, I>>) -> Option<Error> {
    result.err().and_then(|e| e.ok())
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

#[test]
fn liability_starts_at_zero_and_is_uncapped() {
    let s = setup();
    assert_eq!(liability(&s), 0);
    assert_eq!(
        s.client.get_voucher_liability_cap(&s.merchant, &s.token),
        None
    );
}

#[test]
fn issuance_increases_liability() {
    let s = setup();
    issue_voucher(&s, 1, 300);
    issue_voucher(&s, 2, 200);
    assert_eq!(liability(&s), 500);
}

#[test]
fn promotional_issuance_increases_liability() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &10_000);
    let alice = Address::generate(&s.env);
    let bob = Address::generate(&s.env);
    s.client.issue_promotional_vouchers(
        &s.merchant,
        &vec![&s.env, (alice, 300), (bob, 700)],
        &s.token,
        &EXPIRY,
    );
    assert_eq!(liability(&s), 1_000);
}

#[test]
fn full_and_partial_redemption_reduce_liability() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300);
    let v2 = issue_voucher(&s, 2, 500);
    assert_eq!(liability(&s), 800);

    s.client.redeem_voucher_amount(&s.customer, &v2, &200);
    assert_eq!(liability(&s), 600);

    s.client.redeem_refund_voucher(&s.customer, &v1, &0);
    assert_eq!(liability(&s), 300);

    s.client.redeem_voucher_amount(&s.customer, &v2, &300);
    assert_eq!(liability(&s), 0);
}

#[test]
fn revocation_reduces_liability_and_blocks_use() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, 400);
    s.client
        .redeem_voucher_amount(&s.customer, &voucher_id, &100);

    let released = s.client.revoke_voucher(&s.admin, &voucher_id);
    assert!(has_event(&s.env, "voucher_revoked"));
    assert_eq!(released, 300);
    assert_eq!(liability(&s), 0);
    assert!(s.client.is_voucher_revoked(&voucher_id));
    assert_eq!(s.client.get_voucher_balance(&voucher_id), 0);

    let revoked = Some(Error::Core(CoreError::VoucherRevoked));
    assert_eq!(
        contract_err(
            s.client
                .try_redeem_refund_voucher(&s.customer, &voucher_id, &0)
        ),
        revoked
    );
    assert_eq!(
        contract_err(
            s.client
                .try_redeem_voucher_amount(&s.customer, &voucher_id, &1)
        ),
        revoked
    );
    assert_eq!(
        contract_err(s.client.try_transfer_voucher(
            &s.customer,
            &voucher_id,
            &Address::generate(&s.env)
        )),
        revoked
    );
    assert_eq!(
        contract_err(s.client.try_revoke_voucher(&s.admin, &voucher_id)),
        revoked
    );
}

#[test]
fn only_admin_can_revoke() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, 400);
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client.try_revoke_voucher(&stranger, &voucher_id),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
}

#[test]
fn expiry_release_reduces_liability_once() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, 400);
    issue_voucher(&s, 2, 100);

    assert_eq!(
        s.client.try_expire_voucher(&voucher_id),
        Err(Ok(Error::Core(CoreError::VoucherNotExpired)))
    );

    s.env.ledger().set_timestamp(1_000 + EXPIRY + 1);
    assert_eq!(s.client.expire_voucher(&voucher_id), 400);
    assert!(has_event(&s.env, "expired_voucher_released"));
    assert_eq!(liability(&s), 100);

    // A second call releases nothing.
    assert_eq!(s.client.expire_voucher(&voucher_id), 0);
    assert_eq!(liability(&s), 100);
}

#[test]
fn expire_rejects_redeemed_voucher() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, 400);
    s.client.redeem_refund_voucher(&s.customer, &voucher_id, &0);
    s.env.ledger().set_timestamp(1_000 + EXPIRY + 1);
    assert_eq!(
        s.client.try_expire_voucher(&voucher_id),
        Err(Ok(Error::Ext(ExtError::VoucherAlreadyRedeemed)))
    );
}

#[test]
fn cap_rejects_issuance_that_would_exceed_it() {
    let s = setup();
    s.client
        .set_voucher_liability_cap(&s.admin, &s.merchant, &s.token, &500);
    assert!(has_event(&s.env, "voucher_liability_cap_set"));
    assert_eq!(
        s.client.get_voucher_liability_cap(&s.merchant, &s.token),
        Some(500)
    );

    issue_voucher(&s, 1, 300);
    issue_voucher(&s, 2, 200); // exactly at the cap

    let refund_id = approved_refund(&s, 3, 1);
    assert_eq!(
        s.client
            .try_issue_refund_voucher(&s.admin, &refund_id, &EXPIRY),
        Err(Ok(Error::Core(CoreError::VoucherLiabilityCapExceeded)))
    );
    assert_eq!(liability(&s), 500);
}

#[test]
fn cap_frees_up_after_redemption() {
    let s = setup();
    s.client
        .set_voucher_liability_cap(&s.admin, &s.merchant, &s.token, &300);
    let voucher_id = issue_voucher(&s, 1, 300);
    let refund_id = approved_refund(&s, 2, 100);
    assert!(s
        .client
        .try_issue_refund_voucher(&s.admin, &refund_id, &EXPIRY)
        .is_err());

    s.client
        .redeem_voucher_amount(&s.customer, &voucher_id, &100);
    assert!(s
        .client
        .try_issue_refund_voucher(&s.admin, &refund_id, &EXPIRY)
        .is_ok());
}

#[test]
fn cap_applies_to_whole_promotional_batch() {
    let s = setup();
    s.client
        .deposit_refund_reserve(&s.merchant, &s.token, &10_000);
    s.client
        .set_voucher_liability_cap(&s.admin, &s.merchant, &s.token, &900);
    let alice = Address::generate(&s.env);
    let bob = Address::generate(&s.env);

    assert_eq!(
        s.client.try_issue_promotional_vouchers(
            &s.merchant,
            &vec![&s.env, (alice, 300), (bob, 700)],
            &s.token,
            &EXPIRY,
        ),
        Err(Ok(Error::Core(CoreError::VoucherLiabilityCapExceeded)))
    );
    assert_eq!(liability(&s), 0);
    // The reserve was not debited.
    assert_eq!(s.client.get_refund_reserve(&s.merchant, &s.token), 10_000);
}

#[test]
fn cap_is_per_token() {
    let s = setup();
    let other_token = s
        .env
        .register_stellar_asset_contract_v2(s.admin.clone())
        .address();
    s.client
        .set_voucher_liability_cap(&s.admin, &s.merchant, &other_token, &0);
    issue_voucher(&s, 1, 300);
    assert_eq!(liability(&s), 300);
}

#[test]
fn clearing_cap_removes_limit() {
    let s = setup();
    s.client
        .set_voucher_liability_cap(&s.admin, &s.merchant, &s.token, &0);
    let refund_id = approved_refund(&s, 1, 100);
    assert!(s
        .client
        .try_issue_refund_voucher(&s.admin, &refund_id, &EXPIRY)
        .is_err());

    s.client
        .clear_voucher_liability_cap(&s.admin, &s.merchant, &s.token);
    assert!(has_event(&s.env, "voucher_liability_cap_cleared"));
    assert_eq!(
        s.client.get_voucher_liability_cap(&s.merchant, &s.token),
        None
    );
    s.client.issue_refund_voucher(&s.admin, &refund_id, &EXPIRY);
    assert_eq!(liability(&s), 100);
}

#[test]
fn cap_rejects_negative_and_non_admin() {
    let s = setup();
    assert_eq!(
        s.client
            .try_set_voucher_liability_cap(&s.admin, &s.merchant, &s.token, &-1),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_set_voucher_liability_cap(&stranger, &s.merchant, &s.token, &10),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
}

#[test]
fn transfer_keeps_liability_unchanged() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 1, 300);
    let friend = Address::generate(&s.env);
    s.client.transfer_voucher(&s.customer, &voucher_id, &friend);
    assert_eq!(liability(&s), 300);
    s.client.redeem_refund_voucher(&friend, &voucher_id, &0);
    assert_eq!(liability(&s), 0);
}
