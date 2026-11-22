#![cfg(test)]

//! Issue #801: minimum purchase amount for voucher redemption.

use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, token, Address, Env, String};

/// Stand-in for the payment contract's `get_payment`.
#[contract]
pub struct MockPayment;

#[contractimpl]
impl MockPayment {
    pub fn get_payment(env: Env, payment_id: u64) -> ExternalPayment {
        ExternalPayment {
            id: payment_id,
            customer: Address::generate(&env),
            merchant: Address::generate(&env),
            amount: payment_id as i128, // test helper encodes the amount as the payment_id
            token: Address::generate(&env),
            currency: ExternalCurrency::XLM,
            status: ExternalPaymentStatus::Completed,
            created_at: 0,
            expires_at: 0,
            metadata: String::from_str(&env, ""),
            notes: String::from_str(&env, ""),
            refunded_amount: 0,
        }
    }
}

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
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&contract_id, &1_000_000);

    Setup {
        env,
        client,
        admin,
        merchant,
        customer,
        token,
    }
}

fn issue_voucher(s: &Setup, amount: i128) -> u64 {
    let payment_id = 1u64;
    let refund_id = s.client.request_refund(
        &s.merchant,
        &payment_id,
        &s.customer,
        &amount,
        &amount,
        &s.token,
        &String::from_str(&s.env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    );
    s.client.approve_refund(&s.admin, &refund_id);
    s.client
        .issue_refund_voucher(&s.admin, &refund_id, &1_000_000u64)
}

#[test]
fn test_set_and_get_voucher_min_purchase() {
    let s = setup();
    assert_eq!(s.client.get_voucher_min_purchase(&s.merchant), 0);
    s.client
        .set_voucher_min_purchase(&s.admin, &s.merchant, &500i128);
    assert_eq!(s.client.get_voucher_min_purchase(&s.merchant), 500i128);

    // 0 clears the minimum.
    s.client
        .set_voucher_min_purchase(&s.admin, &s.merchant, &0i128);
    assert_eq!(s.client.get_voucher_min_purchase(&s.merchant), 0);
}

#[test]
fn test_set_voucher_min_purchase_requires_admin() {
    let s = setup();
    let outsider = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_set_voucher_min_purchase(&outsider, &s.merchant, &500i128),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
}

#[test]
fn test_set_voucher_min_purchase_rejects_negative() {
    let s = setup();
    assert_eq!(
        s.client
            .try_set_voucher_min_purchase(&s.admin, &s.merchant, &-1i128),
        Err(Ok(Error::Core(CoreError::InvalidAmount)))
    );
}

#[test]
fn test_redeem_without_minimum_configured_still_works() {
    // Regression: no minimum configured, no payment contract wired up —
    // redemption must behave exactly as before #801.
    let s = setup();
    let voucher_id = issue_voucher(&s, 100i128);
    let result = s
        .client
        .try_redeem_refund_voucher(&s.customer, &voucher_id, &1u64);
    assert!(result.is_ok());
}

#[test]
fn test_redeem_below_minimum_is_rejected() {
    let s = setup();
    // Issue the voucher before wiring up the payment contract, since the mock
    // doesn't implement `check_payment_customer` used by `request_refund`.
    let voucher_id = issue_voucher(&s, 100i128);

    let payment_contract = s.env.register(MockPayment, ());
    s.client
        .set_payment_contract_address(&s.admin, &payment_contract);
    // MockPayment::get_payment encodes `amount` as the payment_id itself.
    s.client
        .set_voucher_min_purchase(&s.admin, &s.merchant, &500i128);

    let result = s
        .client
        .try_redeem_refund_voucher(&s.customer, &voucher_id, &100u64);
    assert_eq!(
        result,
        Err(Ok(Error::Core(CoreError::VoucherMinPurchaseNotMet)))
    );
}

#[test]
fn test_redeem_at_or_above_minimum_succeeds() {
    let s = setup();
    let voucher_id = issue_voucher(&s, 100i128);

    let payment_contract = s.env.register(MockPayment, ());
    s.client
        .set_payment_contract_address(&s.admin, &payment_contract);
    s.client
        .set_voucher_min_purchase(&s.admin, &s.merchant, &500i128);

    // payment_id 500 -> MockPayment reports amount == 500, meeting the minimum exactly.
    let result = s
        .client
        .try_redeem_refund_voucher(&s.customer, &voucher_id, &500u64);
    assert!(result.is_ok());
}
