#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, String};

fn setup() -> (Env, PaymentContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);
    (env, client, admin)
}

#[contract]
struct MockOracleContract;

#[contractimpl]
impl MockOracleContract {
    pub fn get_price(_env: Env, _feed_id: BytesN<32>) -> (i128, u64) {
        (123_0000000, 1_000)
    }
}

#[contract]
struct FreshStateContract;

#[contractimpl]
impl FreshStateContract {
    pub fn get_state_hash(_env: Env) -> BytesN<32> {
        BytesN::from_array(&_env, &[7; 32])
    }
}

#[test]
fn test_schedule_payment_flow_and_guards() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(&env, &token_address);
    let token_asset = token::StellarAssetClient::new(&env, &token_address);
    token_asset.mint(&customer, &5_000);

    env.ledger().set_timestamp(100);
    let payment_id = client
        .schedule_payment(&customer, &merchant, &token_address, &1_000, &150)
        .unwrap();
    assert_eq!(token_client.balance(&customer), 4_000);
    assert_eq!(token_client.balance(&client.address), 1_000);

    let early = client.try_execute_scheduled_payment(&payment_id);
    assert_eq!(early.err(), Some(Ok(Error::PaymentNotYetDue)));

    env.ledger().set_timestamp(151);
    client.execute_scheduled_payment(&payment_id).unwrap();
    assert_eq!(token_client.balance(&merchant), 1_000);

    let second = client.try_execute_scheduled_payment(&payment_id);
    assert_eq!(second.err(), Some(Ok(Error::AlreadyProcessed)));
}

#[test]
fn test_cancel_scheduled_payment_refunds_customer() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(&env, &token_address);
    let token_asset = token::StellarAssetClient::new(&env, &token_address);
    token_asset.mint(&customer, &2_000);

    env.ledger().set_timestamp(50);
    let payment_id = client
        .schedule_payment(&customer, &merchant, &token_address, &1_200, &90)
        .unwrap();
    client
        .cancel_scheduled_payment(&customer, &payment_id)
        .unwrap();

    let scheduled = client.get_scheduled_payment(&payment_id).unwrap();
    assert!(scheduled.cancelled);
    assert_eq!(token_client.balance(&customer), 2_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_oracle_refresh_and_manual_fallback() {
    let (env, client, admin) = setup();
    env.ledger().set_timestamp(1_005);
    client
        .set_conversion_rate(&admin, &Currency::BTC, &90_0000000)
        .unwrap();

    let oracle_id = env.register(MockOracleContract, ());
    let feed = BytesN::from_array(&env, &[1; 32]);
    let cfg = OracleRateConfig {
        oracle_address: oracle_id,
        currency: Currency::BTC,
        price_feed_id: feed,
        max_staleness_seconds: 20,
        enabled: true,
    };
    client.set_oracle_rate_config(&admin, &cfg).unwrap();
    let refreshed = client.refresh_conversion_rate(&Currency::BTC).unwrap();
    assert_eq!(refreshed, 123_0000000);

    let stale_cfg = OracleRateConfig {
        oracle_address: cfg.oracle_address.clone(),
        currency: Currency::ETH,
        price_feed_id: cfg.price_feed_id.clone(),
        max_staleness_seconds: 1,
        enabled: true,
    };
    client.set_oracle_rate_config(&admin, &stale_cfg).unwrap();
    let stale = client.try_refresh_conversion_rate(&Currency::ETH);
    assert_eq!(stale.err(), Some(Ok(Error::OracleFeedStale)));

    let disabled_cfg = OracleRateConfig {
        oracle_address: cfg.oracle_address,
        currency: Currency::USDC,
        price_feed_id: cfg.price_feed_id,
        max_staleness_seconds: 100,
        enabled: false,
    };
    client
        .set_conversion_rate(&admin, &Currency::USDC, &1_0000000)
        .unwrap();
    client
        .set_oracle_rate_config(&admin, &disabled_cfg)
        .unwrap();
    let fallback = client.refresh_conversion_rate(&Currency::USDC).unwrap();
    assert_eq!(fallback, 1_0000000);
}

#[test]
fn test_cross_contract_condition_success_and_failure() {
    let (env, client, admin) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "cond");

    let good_id = env.register(FreshStateContract, ());
    let good_hash = BytesN::from_array(&env, &[7; 32]);
    let condition = ConditionType::CrossContractState(good_id, good_hash);
    let payment_id = client
        .create_conditional_payment(
            &customer,
            &merchant,
            &100,
            &token,
            &Currency::USDC,
            &300,
            &meta,
            &condition,
        )
        .unwrap();
    client
        .execute_if_condition_met(&payment_id)
        .expect("should execute when state matches");
    client
        .execute_if_condition_met(&payment_id)
        .expect("idempotent re-execution should succeed");

    let bad_target = Address::generate(&env);
    let bad_cond =
        ConditionType::CrossContractState(bad_target, BytesN::from_array(&env, &[9; 32]));
    let payment_id2 = client
        .create_conditional_payment(
            &customer,
            &merchant,
            &50,
            &token,
            &Currency::USDC,
            &300,
            &meta,
            &bad_cond,
        )
        .unwrap();
    let eval = client.try_evaluate_condition(&payment_id2);
    assert_eq!(eval.err(), Some(Ok(Error::ConditionEvaluationFailed)));

    let _ = admin;
}

#[test]
fn test_analytics_range_and_top_merchants() {
    let (env, client, _admin) = setup();
    let customer = Address::generate(&env);
    let merchant_a = Address::generate(&env);
    let merchant_b = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    env.ledger().set_timestamp(3_600);
    let p1 = client
        .create_payment(
            &customer,
            &merchant_a,
            &1_000,
            &token,
            &Currency::USDC,
            &0,
            &meta,
        )
        .unwrap();
    let _ = client.cancel_payment(&customer, &p1);

    env.ledger().set_timestamp(7_200);
    let _ = client
        .create_payment(
            &customer,
            &merchant_b,
            &5_000,
            &token,
            &Currency::USDC,
            &0,
            &meta,
        )
        .unwrap();

    let range = client
        .get_merchant_analytics_range(&merchant_a, &3_600, &10_800)
        .unwrap();
    assert_eq!(range.len(), 1);
    assert_eq!(range.get(0).unwrap().total_volume, 1_000);
    assert_eq!(range.get(0).unwrap().failed_count, 1);

    let reversed = client.try_get_merchant_analytics_range(&merchant_a, &10_800, &3_600);
    assert!(reversed.is_err());

    let top = client.get_top_merchants_by_volume(&1);
    assert_eq!(top.len(), 1);
    assert_eq!(top.get(0).unwrap().0, merchant_b);
    assert_eq!(top.get(0).unwrap().1, 5_000);

    let daily = client.get_platform_analytics_daily(&7_250);
    assert!(daily.total_payments >= 1);
    assert!(daily.total_volume >= 5_000);
}
