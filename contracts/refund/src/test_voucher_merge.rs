#![cfg(test)]

//! Issue #910: merging same-merchant, same-token refund vouchers.

use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{token, vec, Address, Env, String, Symbol, TryFromVal};

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

fn issue(
    s: &Setup,
    merchant: &Address,
    token: &Address,
    payment_id: u64,
    amount: i128,
    expiry: u64,
) -> u64 {
    let refund_id = s.client.request_refund(
        merchant,
        &payment_id,
        &s.customer,
        &amount,
        &amount,
        token,
        &String::from_str(&s.env, "store credit"),
        &RefundReasonCode::CustomerRequest,
        &s.env.ledger().timestamp(),
    );
    s.client.approve_refund(&s.admin, &refund_id);
    s.client.issue_refund_voucher(&s.admin, &refund_id, &expiry)
}

fn issue_voucher(s: &Setup, payment_id: u64, amount: i128, expiry: u64) -> u64 {
    issue(s, &s.merchant, &s.token, payment_id, amount, expiry)
}

fn merge_err(s: &Setup, ids: Vec<u64>) -> Error {
    s.client
        .try_merge_vouchers(&s.customer, &ids)
        .err()
        .and_then(|e| e.ok())
        .expect("merge should fail with a contract error")
}

#[test]
fn merges_balances_and_takes_earliest_expiry() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 2_000);
    let v3 = issue_voucher(&s, 3, 500, 9_000);
    s.client.redeem_voucher_amount(&s.customer, &v3, &100);

    let merged_id = s
        .client
        .merge_vouchers(&s.customer, &vec![&s.env, v1, v2, v3]);

    let merged = s.client.get_voucher(&merged_id).unwrap();
    assert_eq!(merged.amount, 900);
    assert_eq!(merged.customer, s.customer);
    assert_eq!(merged.merchant, s.merchant);
    assert_eq!(merged.token, s.token);
    assert_eq!(merged.refund_id, 0);
    assert_eq!(merged.expires_at, 1_000 + 2_000);
    assert!(!merged.redeemed);
    assert_eq!(s.client.get_voucher_balance(&merged_id), 900);
    assert!(s.client.is_voucher_transferable(&merged_id));
}

#[test]
fn source_vouchers_are_marked_consumed() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    let merged_id = s.client.merge_vouchers(&s.customer, &vec![&s.env, v1, v2]);

    for id in [v1, v2] {
        assert!(s.client.get_voucher(&id).unwrap().redeemed);
        assert_eq!(s.client.get_voucher_balance(&id), 0);
        assert_eq!(s.client.get_voucher_merged_into(&id), Some(merged_id));
        assert_eq!(
            s.client
                .try_redeem_refund_voucher(&s.customer, &id, &0)
                .err()
                .and_then(|e| e.ok()),
            Some(Error::Ext(ExtError::VoucherAlreadyRedeemed))
        );
    }
    assert_eq!(s.client.get_voucher_merged_into(&merged_id), None);

    // Consumed sources cannot be merged again.
    let v3 = issue_voucher(&s, 3, 100, 5_000);
    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v3]),
        Error::Ext(ExtError::VoucherAlreadyRedeemed)
    );
}

#[test]
fn merged_voucher_redeems_full_value() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    let merged_id = s.client.merge_vouchers(&s.customer, &vec![&s.env, v1, v2]);

    s.client.redeem_refund_voucher(&s.customer, &merged_id, &0);
    assert_eq!(
        token::Client::new(&s.env, &s.token).balance(&s.customer),
        500
    );
}

#[test]
fn merge_keeps_outstanding_liability_unchanged() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    assert_eq!(s.client.get_voucher_liability(&s.merchant, &s.token), 500);

    let merged_id = s.client.merge_vouchers(&s.customer, &vec![&s.env, v1, v2]);
    assert_eq!(s.client.get_voucher_liability(&s.merchant, &s.token), 500);

    s.client.redeem_refund_voucher(&s.customer, &merged_id, &0);
    assert_eq!(s.client.get_voucher_liability(&s.merchant, &s.token), 0);
}

#[test]
fn rejects_non_transferable_vouchers() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    s.client.set_vouchers_transferable(&s.merchant, &false);
    let v2 = issue_voucher(&s, 2, 200, 5_000);

    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v2]),
        Error::Ext(ExtError::VoucherNotTransferable)
    );
}

#[test]
fn rejects_expired_vouchers() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 100);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    s.env.ledger().set_timestamp(1_000 + 101);

    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v2]),
        Error::Ext(ExtError::VoucherExpired)
    );
}

#[test]
fn rejects_mixed_merchants() {
    let s = setup();
    let other_merchant = Address::generate(&s.env);
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue(&s, &other_merchant, &s.token, 2, 200, 5_000);

    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v2]),
        Error::Core(CoreError::VoucherMergeMismatch)
    );
}

#[test]
fn rejects_mixed_tokens() {
    let s = setup();
    let other_token = s
        .env
        .register_stellar_asset_contract_v2(s.admin.clone())
        .address();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue(&s, &s.merchant, &other_token, 2, 200, 5_000);

    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v2]),
        Error::Core(CoreError::VoucherMergeMismatch)
    );
}

#[test]
fn rejects_vouchers_owned_by_someone_else() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    s.client
        .transfer_voucher(&s.customer, &v2, &Address::generate(&s.env));

    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v2]),
        Error::Core(CoreError::Unauthorized)
    );
}

#[test]
fn rejects_revoked_vouchers() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    s.client.revoke_voucher(&s.admin, &v2);

    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v2]),
        Error::Core(CoreError::VoucherRevoked)
    );
}

#[test]
fn rejects_bad_id_lists() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);

    assert_eq!(
        merge_err(&s, vec![&s.env, v1]),
        Error::Core(CoreError::InvalidVoucherMerge)
    );
    assert_eq!(
        merge_err(&s, vec![&s.env, v1, v1]),
        Error::Core(CoreError::InvalidVoucherMerge)
    );
    assert_eq!(
        merge_err(&s, vec![&s.env, v1, 999]),
        Error::Ext(ExtError::VoucherNotFound)
    );

    let mut too_many = Vec::new(&s.env);
    for i in 0..=RefundContract::MAX_VOUCHER_MERGE as u64 {
        too_many.push_back(i + 1);
    }
    assert_eq!(
        merge_err(&s, too_many),
        Error::Core(CoreError::InvalidVoucherMerge)
    );
}

#[test]
fn failed_merge_leaves_sources_untouched() {
    let s = setup();
    let other_merchant = Address::generate(&s.env);
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue(&s, &other_merchant, &s.token, 2, 200, 5_000);
    merge_err(&s, vec![&s.env, v1, v2]);

    assert!(!s.client.get_voucher(&v1).unwrap().redeemed);
    assert_eq!(s.client.get_voucher_balance(&v1), 300);
    assert_eq!(s.client.get_voucher_merged_into(&v1), None);
}

#[test]
fn merge_emits_event() {
    let s = setup();
    let v1 = issue_voucher(&s, 1, 300, 5_000);
    let v2 = issue_voucher(&s, 2, 200, 5_000);
    s.client.merge_vouchers(&s.customer, &vec![&s.env, v1, v2]);

    let expected = Symbol::new(&s.env, "vouchers_merged");
    let found = s.env.events().all().iter().any(|(_, topics, _)| {
        topics
            .get(0)
            .and_then(|t| Symbol::try_from_val(&s.env, &t).ok())
            .is_some_and(|sym| sym == expected)
    });
    assert!(found);
}
