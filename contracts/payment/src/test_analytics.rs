#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, PaymentContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);
    (env, client, admin)
}

fn make_payment(
    client: &PaymentContractClient,
    env: &Env,
    customer: &Address,
    merchant: &Address,
    token: &Address,
    amount: i128,
) -> u64 {
    client.create_payment(
        customer,
        merchant,
        &amount,
        token,
        &Currency::XLM,
        &0u64,
        &String::from_str(env, ""),
    )
}

#[test]
fn test_avg_transaction_size_updated_on_payment() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(3600); // hour 1

    make_payment(&client, &env, &customer, &merchant, &token, 1000);
    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.avg_transaction_size, 1000);

    make_payment(&client, &env, &customer, &merchant, &token, 2000);
    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.avg_transaction_size, 1500); // (1000 + 2000) / 2
}

#[test]
fn test_peak_hour_tracked() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Hour 14 (50400 seconds / 3600 = 14)
    env.ledger().set_timestamp(50400);
    make_payment(&client, &env, &customer, &merchant, &token, 1000);
    make_payment(&client, &env, &customer, &merchant, &token, 1000);

    // Hour 5 (18000 seconds / 3600 = 5)
    env.ledger().set_timestamp(18000);
    make_payment(&client, &env, &customer, &merchant, &token, 1000);

    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.peak_hour, 14); // 2 payments vs 1 payment at hour 5
}

#[test]
fn test_first_and_last_payment_at_tracked() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    make_payment(&client, &env, &customer, &merchant, &token, 500);

    env.ledger().set_timestamp(5000);
    make_payment(&client, &env, &customer, &merchant, &token, 500);

    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.first_payment_at, 1000);
    assert_eq!(analytics.last_payment_at, 5000);
}

#[test]
fn test_top_merchant_ranking() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(3600);
    make_payment(&client, &env, &customer, &m1, &token, 1000);
    make_payment(&client, &env, &customer, &m2, &token, 5000);

    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.top_merchant, Some(m2.clone()));
    assert_eq!(analytics.top_merchant_volume, 5000);

    let top = client.get_customer_top_merchants(&customer, &2u32);
    assert_eq!(top.len(), 2);
    // First entry should be m2 (highest volume)
    assert_eq!(top.get(0).unwrap().1, 5000);
    assert_eq!(top.get(1).unwrap().1, 1000);
}

#[test]
fn test_top_merchants_respects_limit() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(3600);
    make_payment(&client, &env, &customer, &m1, &token, 100);
    make_payment(&client, &env, &customer, &m2, &token, 200);
    make_payment(&client, &env, &customer, &m3, &token, 300);

    let top = client.get_customer_top_merchants(&customer, &2u32);
    assert_eq!(top.len(), 2);
}

#[test]
fn test_monthly_volume_tracked() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Month bucket = (ts / 2592000) * 2592000
    let ts = 2_592_000u64; // exactly 1 month from epoch
    let month_bucket = (ts / 2_592_000) * 2_592_000;

    env.ledger().set_timestamp(ts);
    make_payment(&client, &env, &customer, &merchant, &token, 500);
    make_payment(&client, &env, &customer, &merchant, &token, 300);

    let vol = client.get_customer_monthly_volume(&customer, &month_bucket);
    assert_eq!(vol, 800);
}
