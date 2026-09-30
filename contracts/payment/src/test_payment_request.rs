#![cfg(test)]
//! Tests for #662: merchant-initiated payment requests.

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _},
    vec, Address, Env, Event, String,
};

use crate::{
    BasicError, Currency, Error, PaymentContract, PaymentContractClient, PaymentError,
    PaymentRequestCancelled, PaymentRequestCreated, PaymentRequestPaid, PaymentRequestStatus,
    PaymentStatus, SubscriptionError,
};

struct Fixture {
    env: Env,
    client: PaymentContractClient<'static>,
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

    Fixture {
        merchant: Address::generate(&env),
        customer: Address::generate(&env),
        env,
        client,
        admin,
        token,
    }
}

fn assert_last_event(f: &Fixture, event: &impl Event) {
    let all = f.env.events().all();
    assert_eq!(
        all.slice(all.len() - 1..),
        vec![
            &f.env,
            (
                f.client.address.clone(),
                event.topics(&f.env),
                event.data(&f.env)
            )
        ]
    );
}

fn create_request(f: &Fixture, expires_at: u64, customer: Option<Address>) -> u64 {
    f.client.create_payment_request(
        &f.merchant,
        &500,
        &f.token,
        &Currency::USDC,
        &expires_at,
        &String::from_str(&f.env, "invoice-42"),
        &customer,
    )
}

#[test]
fn test_create_payment_request_stores_open_request() {
    let f = setup();
    let request_id = create_request(&f, 5_000, None);
    assert_last_event(
        &f,
        &PaymentRequestCreated {
            request_id,
            merchant: f.merchant.clone(),
            customer: None,
            amount: 500,
            token: f.token.clone(),
            expires_at: 5_000,
        },
    );

    let request = f.client.get_payment_request(&request_id);
    assert_eq!(request_id, 1);
    assert_eq!(request.merchant, f.merchant);
    assert_eq!(request.customer, None);
    assert_eq!(request.amount, 500);
    assert_eq!(request.expires_at, 5_000);
    assert_eq!(request.status, PaymentRequestStatus::Open);
    assert_eq!(request.created_at, 1_000);
    assert_eq!(request.payment_id, None);

    assert_eq!(create_request(&f, 0, None), 2);
}

#[test]
fn test_create_payment_request_validation() {
    let f = setup();
    let metadata = String::from_str(&f.env, "");
    assert_eq!(
        f.client.try_create_payment_request(
            &f.merchant,
            &0,
            &f.token,
            &Currency::USDC,
            &0,
            &metadata,
            &None,
        ),
        Err(Ok(Error::Basic(BasicError::InvalidAmount)))
    );
    // An expiry that is not in the future can never be paid.
    assert_eq!(
        f.client.try_create_payment_request(
            &f.merchant,
            &100,
            &f.token,
            &Currency::USDC,
            &1_000,
            &metadata,
            &None,
        ),
        Err(Ok(Error::Payment(PaymentError::RequestExpired)))
    );

    f.client.pause_merchant(&f.admin, &f.merchant);
    assert_eq!(
        f.client.try_create_payment_request(
            &f.merchant,
            &100,
            &f.token,
            &Currency::USDC,
            &0,
            &metadata,
            &None,
        ),
        Err(Ok(Error::Subscription(SubscriptionError::MerchantPaused)))
    );
}

#[test]
fn test_pay_payment_request_creates_payment_on_request_terms() {
    let f = setup();
    let request_id = create_request(&f, 5_000, None);

    let payment_id = f.client.pay_payment_request(&f.customer, &request_id);
    assert_last_event(
        &f,
        &PaymentRequestPaid {
            request_id,
            payment_id,
            customer: f.customer.clone(),
        },
    );

    let payment = f.client.get_payment(&payment_id);
    assert_eq!(payment.customer, f.customer);
    assert_eq!(payment.merchant, f.merchant);
    assert_eq!(payment.amount, 500);
    assert_eq!(payment.token, f.token);
    assert_eq!(payment.currency, Currency::USDC);
    assert_eq!(payment.metadata, String::from_str(&f.env, "invoice-42"));
    assert_eq!(payment.status, PaymentStatus::Pending);

    let request = f.client.get_payment_request(&request_id);
    assert_eq!(request.status, PaymentRequestStatus::Paid);
    assert_eq!(request.payment_id, Some(payment_id));
}

#[test]
fn test_payment_request_can_only_be_paid_once() {
    let f = setup();
    let request_id = create_request(&f, 0, None);
    f.client.pay_payment_request(&f.customer, &request_id);

    assert_eq!(
        f.client
            .try_pay_payment_request(&Address::generate(&f.env), &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestAlreadyPaid)))
    );
    assert_eq!(
        f.client.try_pay_payment_request(&f.customer, &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestAlreadyPaid)))
    );
    assert_eq!(
        f.client
            .try_cancel_payment_request(&f.merchant, &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestAlreadyPaid)))
    );
}

#[test]
fn test_expired_payment_request_cannot_be_paid() {
    let f = setup();
    let request_id = create_request(&f, 2_000, None);

    f.env.ledger().set_timestamp(2_000);
    assert_eq!(
        f.client.try_pay_payment_request(&f.customer, &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestExpired)))
    );
    assert_eq!(
        f.client.get_payment_request(&request_id).status,
        PaymentRequestStatus::Open
    );
}

#[test]
fn test_cancelled_payment_request_cannot_be_paid() {
    let f = setup();
    let request_id = create_request(&f, 0, None);

    f.client.cancel_payment_request(&f.merchant, &request_id);
    assert_last_event(
        &f,
        &PaymentRequestCancelled {
            request_id,
            merchant: f.merchant.clone(),
        },
    );
    assert_eq!(
        f.client.get_payment_request(&request_id).status,
        PaymentRequestStatus::Cancelled
    );

    assert_eq!(
        f.client.try_pay_payment_request(&f.customer, &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestCancelled)))
    );
    assert_eq!(
        f.client
            .try_cancel_payment_request(&f.merchant, &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestCancelled)))
    );
}

#[test]
fn test_only_issuing_merchant_can_cancel() {
    let f = setup();
    let request_id = create_request(&f, 0, None);
    assert_eq!(
        f.client
            .try_cancel_payment_request(&Address::generate(&f.env), &request_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
}

#[test]
fn test_restricted_payment_request_only_payable_by_named_customer() {
    let f = setup();
    let request_id = create_request(&f, 0, Some(f.customer.clone()));
    assert_eq!(
        f.client.get_payment_request(&request_id).customer,
        Some(f.customer.clone())
    );

    assert_eq!(
        f.client
            .try_pay_payment_request(&Address::generate(&f.env), &request_id),
        Err(Ok(Error::Payment(PaymentError::RequestCustomerMismatch)))
    );

    let payment_id = f.client.pay_payment_request(&f.customer, &request_id);
    assert_eq!(f.client.get_payment(&payment_id).customer, f.customer);
}

#[test]
fn test_unknown_payment_request() {
    let f = setup();
    assert_eq!(
        f.client.try_get_payment_request(&7),
        Err(Ok(Error::Payment(PaymentError::RequestNotFound)))
    );
    assert_eq!(
        f.client.try_pay_payment_request(&f.customer, &7),
        Err(Ok(Error::Payment(PaymentError::RequestNotFound)))
    );
    assert_eq!(
        f.client.try_cancel_payment_request(&f.merchant, &7),
        Err(Ok(Error::Payment(PaymentError::RequestNotFound)))
    );
}

#[test]
fn test_pay_payment_request_respects_create_payment_pause() {
    let f = setup();
    let request_id = create_request(&f, 0, None);
    f.client.pause_function(
        &f.admin,
        &String::from_str(&f.env, "create_payment"),
        &String::from_str(&f.env, "maintenance"),
    );

    assert_eq!(
        f.client.try_pay_payment_request(&f.customer, &request_id),
        Err(Ok(Error::Basic(BasicError::FunctionPaused)))
    );
}
