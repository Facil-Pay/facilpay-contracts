#![cfg(test)]
//! Tests for #900: partial payments on payment requests.

use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, Address, Env, String};

use crate::{
    BasicError, Currency, Error, FeatureError, PaymentContract, PaymentContractClient,
    PaymentError, PaymentRequestStatus,
};

struct Fixture {
    env: Env,
    client: PaymentContractClient<'static>,
    contract_id: Address,
    admin: Address,
    merchant: Address,
    customer: Address,
    token: Address,
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
    let customer = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&customer, &10_000);
    Fixture {
        merchant: Address::generate(&env),
        customer,
        env,
        client,
        contract_id,
        admin,
        token,
    }
}

fn create_request(f: &Fixture) -> u64 {
    f.client.create_payment_request(
        &f.merchant,
        &500,
        &f.token,
        &Currency::USDC,
        &0,
        &String::from_str(&f.env, "invoice"),
        &None,
    )
}

fn balance(f: &Fixture, who: &Address) -> i128 {
    token::Client::new(&f.env, &f.token).balance(who)
}

#[test]
fn partial_payments_require_merchant_opt_in() {
    let f = setup();
    let id = create_request(&f);
    assert!(!f.client.get_request_allows_partial(&id));
    assert_eq!(
        f.client
            .try_pay_payment_request_partial(&f.customer, &id, &100),
        Err(Ok(Error::Feature(FeatureError::PartialPaymentNotAllowed)))
    );
    let stranger = Address::generate(&f.env);
    assert_eq!(
        f.client
            .try_set_payment_request_partial(&stranger, &id, &true),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    f.client
        .set_payment_request_partial(&f.merchant, &id, &true);
    assert!(f.client.get_request_allows_partial(&id));
}

#[test]
fn request_becomes_paid_once_paid_amount_reaches_amount() {
    let f = setup();
    let id = create_request(&f);
    f.client
        .set_payment_request_partial(&f.merchant, &id, &true);

    assert_eq!(
        f.client.pay_payment_request_partial(&f.customer, &id, &200),
        200
    );
    let req = f.client.get_payment_request(&id);
    assert_eq!(req.status, PaymentRequestStatus::Open);
    assert_eq!(req.payment_id, None);
    assert_eq!(f.client.get_payment_request_paid_amount(&id), 200);
    assert_eq!(balance(&f, &f.contract_id), 200);

    assert_eq!(
        f.client.pay_payment_request_partial(&f.customer, &id, &300),
        500
    );
    let req = f.client.get_payment_request(&id);
    assert_eq!(req.status, PaymentRequestStatus::Paid);
    let payment = f.client.get_payment(&req.payment_id.unwrap());
    assert_eq!(payment.amount, 500);
    assert_eq!(payment.customer, f.customer);
    assert_eq!(balance(&f, &f.contract_id), 500);
    assert_eq!(balance(&f, &f.customer), 9_500);

    assert_eq!(
        f.client
            .try_pay_payment_request_partial(&f.customer, &id, &1),
        Err(Ok(Error::Payment(PaymentError::RequestAlreadyPaid)))
    );
    // Completing the payment settles from the held funds; the customer is
    // not charged a second time.
    f.client
        .complete_payment(&f.admin, &req.payment_id.unwrap());
    assert_eq!(balance(&f, &f.customer), 9_500);
    assert_eq!(balance(&f, &f.merchant) + balance(&f, &f.contract_id), 500);
}

#[test]
fn overpayment_and_non_positive_amounts_are_rejected() {
    let f = setup();
    let id = create_request(&f);
    f.client
        .set_payment_request_partial(&f.merchant, &id, &true);
    f.client.pay_payment_request_partial(&f.customer, &id, &400);
    assert_eq!(
        f.client
            .try_pay_payment_request_partial(&f.customer, &id, &101),
        Err(Ok(Error::Feature(FeatureError::RequestOverpayment)))
    );
    assert_eq!(
        f.client
            .try_pay_payment_request_partial(&f.customer, &id, &0),
        Err(Ok(Error::Basic(BasicError::InvalidAmount)))
    );
    assert_eq!(f.client.get_payment_request_paid_amount(&id), 400);
}

#[test]
fn full_pay_settles_the_remainder_of_a_partially_paid_request() {
    let f = setup();
    let id = create_request(&f);
    f.client
        .set_payment_request_partial(&f.merchant, &id, &true);
    f.client.pay_payment_request_partial(&f.customer, &id, &150);
    let payment_id = f.client.pay_payment_request(&f.customer, &id);
    let req = f.client.get_payment_request(&id);
    assert_eq!(req.status, PaymentRequestStatus::Paid);
    assert_eq!(req.payment_id, Some(payment_id));
    assert_eq!(balance(&f, &f.customer), 9_500);
}

#[test]
fn cancelling_a_partially_paid_request_refunds_each_payer() {
    let f = setup();
    let id = create_request(&f);
    f.client
        .set_payment_request_partial(&f.merchant, &id, &true);
    let other = Address::generate(&f.env);
    token::StellarAssetClient::new(&f.env, &f.token).mint(&other, &1_000);

    f.client.pay_payment_request_partial(&f.customer, &id, &100);
    f.client.pay_payment_request_partial(&other, &id, &250);
    f.client.pay_payment_request_partial(&f.customer, &id, &50);
    assert_eq!(balance(&f, &f.contract_id), 400);

    // Cannot switch partial payments off once money has been taken.
    assert_eq!(
        f.client
            .try_set_payment_request_partial(&f.merchant, &id, &false),
        Err(Ok(Error::Payment(PaymentError::InvalidStatus)))
    );

    f.client.cancel_payment_request(&f.merchant, &id);
    assert_eq!(
        f.client.get_payment_request(&id).status,
        PaymentRequestStatus::Cancelled
    );
    assert_eq!(balance(&f, &f.contract_id), 0);
    assert_eq!(balance(&f, &f.customer), 10_000);
    assert_eq!(balance(&f, &other), 1_000);
    assert_eq!(f.client.get_payment_request_paid_amount(&id), 0);
}
