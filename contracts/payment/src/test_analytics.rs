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

#[test]
fn test_total_refunds_tracked_in_analytics() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(3600);
    
    // Create and complete two payments
    let payment_id1 = make_payment(&client, &env, &customer, &merchant, &token, 1000);
    client.complete_payment(&admin, &payment_id1);
    
    let payment_id2 = make_payment(&client, &env, &customer, &merchant, &token, 500);
    client.complete_payment(&admin, &payment_id2);

    // Initial state: no refunds
    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.total_refunds, 0);
    assert_eq!(analytics.total_volume, 1500);

    // Refund first payment
    client.refund_payment(&admin, &payment_id1);
    
    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.total_refunds, 1000);
    assert_eq!(analytics.total_volume, 1500); // Volume remains unchanged
}

#[test]
fn test_analytics_with_multiple_merchants() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(3600);
    
    // Pay different amounts to different merchants
    make_payment(&client, &env, &customer, &m1, &token, 100);
    make_payment(&client, &env, &customer, &m2, &token, 500);
    make_payment(&client, &env, &customer, &m3, &token, 300);
    make_payment(&client, &env, &customer, &m2, &token, 200); // Additional payment to m2

    let analytics = client.get_customer_analytics(&customer);
    
    // Total should be 4 payments, 1100 total volume
    assert_eq!(analytics.total_payments, 4);
    assert_eq!(analytics.total_volume, 1100);
    assert_eq!(analytics.avg_transaction_size, 275); // 1100 / 4
    
    // Top merchant should be m2 with 700 total
    assert_eq!(analytics.top_merchant, Some(m2.clone()));
    assert_eq!(analytics.top_merchant_volume, 700);
}

#[test]
fn test_peak_hour_updates_with_more_transactions() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Hour 10: 2 transactions
    env.ledger().set_timestamp(36000); // 10 * 3600
    make_payment(&client, &env, &customer, &merchant, &token, 100);
    make_payment(&client, &env, &customer, &merchant, &token, 200);

    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.peak_hour, 10);

    // Hour 15: 3 transactions (should become new peak)
    env.ledger().set_timestamp(54000); // 15 * 3600
    make_payment(&client, &env, &customer, &merchant, &token, 100);
    make_payment(&client, &env, &customer, &merchant, &token, 100);
    make_payment(&client, &env, &customer, &merchant, &token, 100);

    let analytics = client.get_customer_analytics(&customer);
    assert_eq!(analytics.peak_hour, 15);
    assert_eq!(analytics.total_payments, 5);
}

#[test]
fn test_monthly_volume_across_different_months() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // First month (month 1)
    let month1_ts = 2_592_000u64;
    let month1_bucket = (month1_ts / 2_592_000) * 2_592_000;
    
    env.ledger().set_timestamp(month1_ts);
    make_payment(&client, &env, &customer, &merchant, &token, 1000);
    make_payment(&client, &env, &customer, &merchant, &token, 500);

    // Second month (month 2)
    let month2_ts = 2_592_000u64 * 2 + 1000;
    let month2_bucket = (month2_ts / 2_592_000) * 2_592_000;
    
    env.ledger().set_timestamp(month2_ts);
    make_payment(&client, &env, &customer, &merchant, &token, 2000);
    make_payment(&client, &env, &customer, &merchant, &token, 300);

    // Verify each month's volume is tracked separately
    let vol1 = client.get_customer_monthly_volume(&customer, &month1_bucket);
    let vol2 = client.get_customer_monthly_volume(&customer, &month2_bucket);
    
    assert_eq!(vol1, 1500);
    assert_eq!(vol2, 2300);
}

#[test]
fn test_analytics_initial_state_for_new_customer() {
    let (env, client, _admin) = setup();
    let new_customer = Address::generate(&env);

    env.ledger().set_timestamp(3600);
    
    // Get analytics for customer with no payments
    let analytics = client.get_customer_analytics(&new_customer);
    
    assert_eq!(analytics.total_payments, 0);
    assert_eq!(analytics.total_volume, 0);
    assert_eq!(analytics.total_refunds, 0);
    assert_eq!(analytics.avg_transaction_size, 0);
    assert_eq!(analytics.peak_hour, 0);
    assert_eq!(analytics.top_merchant, None);
    assert_eq!(analytics.top_merchant_volume, 0);
    assert_eq!(analytics.first_payment_at, 0);
    assert_eq!(analytics.last_payment_at, 0);
}
