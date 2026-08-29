#![cfg(test)]

//! Regression tests for #557 — `refund_payment` and `cancel_payment` must return
//! any installment amounts the customer already paid via `pay_installment` before
//! marking the payment `Refunded` / `Cancelled`.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String,
};

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

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    let token_client = token::Client::new(&env, &token);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    token_admin_client.mint(&customer, &10_000i128);
    token_client.approve(&customer, &contract_id, &10_000i128, &10_000);

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

fn create_pending_payment(f: &Fixture, amount: i128) -> u64 {
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

#[test]
fn refund_returns_collected_installments_to_customer() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 1_000i128);

    f.client.pay_installment(&f.customer, &payment_id, &400i128);
    assert_eq!(f.token_client.balance(&f.contract_id), 400i128);

    let customer_before = f.token_client.balance(&f.customer);
    f.client.refund_payment(&f.admin, &payment_id);
    let customer_after = f.token_client.balance(&f.customer);

    assert_eq!(
        customer_after - customer_before,
        400i128,
        "installments already paid must be returned on refund"
    );
    assert_eq!(f.token_client.balance(&f.contract_id), 0i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Refunded
    );
}

#[test]
fn cancel_returns_collected_installments_to_customer() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 1_000i128);

    f.client.pay_installment(&f.customer, &payment_id, &250i128);
    f.client.pay_installment(&f.customer, &payment_id, &150i128);
    assert_eq!(f.token_client.balance(&f.contract_id), 400i128);

    let customer_before = f.token_client.balance(&f.customer);
    f.client.cancel_payment(&f.customer, &payment_id);
    let customer_after = f.token_client.balance(&f.customer);

    assert_eq!(
        customer_after - customer_before,
        400i128,
        "all installments already paid must be returned on cancel"
    );
    assert_eq!(f.token_client.balance(&f.contract_id), 0i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Cancelled
    );
}

#[test]
fn refund_without_installments_transfers_nothing() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 1_000i128);

    let customer_before = f.token_client.balance(&f.customer);
    let contract_before = f.token_client.balance(&f.contract_id);

    f.client.refund_payment(&f.admin, &payment_id);

    assert_eq!(f.token_client.balance(&f.customer), customer_before);
    assert_eq!(f.token_client.balance(&f.contract_id), contract_before);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Refunded
    );
}

#[test]
fn refund_only_returns_this_payments_installments_not_pool() {
    let f = setup();

    // A second, unrelated pending payment with its own installment sitting in
    // the shared contract balance.
    let other = create_pending_payment(&f, 1_000i128);
    f.client.pay_installment(&f.customer, &other, &600i128);

    let payment_id = create_pending_payment(&f, 1_000i128);
    f.client.pay_installment(&f.customer, &payment_id, &300i128);

    assert_eq!(f.token_client.balance(&f.contract_id), 900i128);

    let customer_before = f.token_client.balance(&f.customer);
    f.client.refund_payment(&f.admin, &payment_id);
    let customer_after = f.token_client.balance(&f.customer);

    assert_eq!(
        customer_after - customer_before,
        300i128,
        "only this payment's installments are returned"
    );
    // The other payment's 600 stays in the contract.
    assert_eq!(f.token_client.balance(&f.contract_id), 600i128);
}
