#![cfg(test)]
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String};

use crate::{Currency, Error, FinalityConfig, PaymentContract, PaymentContractClient};

fn setup() -> (Env, PaymentContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PaymentContract);
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&contract_id, &10_000_000);
    client.initialize(&admin);
    (env, client, admin, token_addr)
}

#[test]
fn test_configure_finality_delay() {
    let (_, client, admin, _) = setup();
    client.configure_finality_delay(&admin, &FinalityConfig {
        delay_seconds: 86400,
        min_amount_threshold: 100,
        active: true,
    });
    let cfg = client.get_finality_config().unwrap();
    assert_eq!(cfg.delay_seconds, 86400);
    assert!(cfg.active);
}

#[test]
fn test_complete_payment_creates_pending_settlement() {
    let (env, client, admin, token_addr) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&customer, &10_000);

    client.configure_finality_delay(&admin, &FinalityConfig {
        delay_seconds: 3600,
        min_amount_threshold: 100,
        active: true,
    });

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &500,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let settlements = client.get_pending_settlements(&merchant);
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements.get(0).unwrap().payment_id, payment_id);
}

#[test]
fn test_finalize_before_release_at_fails() {
    let (env, client, admin, token_addr) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&customer, &10_000);

    client.configure_finality_delay(&admin, &FinalityConfig {
        delay_seconds: 3600,
        min_amount_threshold: 100,
        active: true,
    });

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &500,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let result = client.try_finalize_pending_settlement(&payment_id);
    assert_eq!(result, Err(Ok(Error::SettlementNotReady)));
}

#[test]
fn test_finalize_after_delay_succeeds() {
    let (env, client, admin, token_addr) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&customer, &10_000);

    client.configure_finality_delay(&admin, &FinalityConfig {
        delay_seconds: 3600,
        min_amount_threshold: 100,
        active: true,
    });

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &500,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // Advance time past the delay
    env.ledger().with_mut(|l| l.timestamp += 7200);

    client.finalize_pending_settlement(&payment_id);

    // Double finalization should fail
    let result = client.try_finalize_pending_settlement(&payment_id);
    assert_eq!(result, Err(Ok(Error::SettlementAlreadyFinalized)));
}

#[test]
fn test_threshold_bypass_settles_immediately() {
    let (env, client, admin, token_addr) = setup();
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = soroban_sdk::token::StellarAssetClient::new(&env, &token_addr);
    token.mint(&customer, &10_000);
    soroban_sdk::token::Client::new(&env, &token_addr).approve(&customer, &client.address, &50, &10_000);

    // min_amount_threshold = 1000, payment = 50 → bypass
    client.configure_finality_delay(&admin, &FinalityConfig {
        delay_seconds: 3600,
        min_amount_threshold: 1000,
        active: true,
    });

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &50,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // No pending settlement created for below-threshold payment
    let settlements = client.get_pending_settlements(&merchant);
    assert_eq!(settlements.len(), 0);
}
