#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_create_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token);
    assert_eq!(payment_id, 1);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.id, 1);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.merchant, merchant);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.token, token);
}

#[test]
fn test_get_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 5000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token);

    let payment = client.get_payment(&payment_id);

    assert_eq!(payment.id, payment_id);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.merchant, merchant);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.token, token);
    assert_eq!(payment.status, PaymentStatus::Pending);
}

#[test]
#[should_panic(expected = "Payment not found")]
fn test_get_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    client.get_payment(&999);
}
