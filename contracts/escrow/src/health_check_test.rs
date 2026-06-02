#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let customer = Address::generate(env);
    token_admin.mint(&customer, &1_000_000);
    // fund contract directly so transfers succeed
    token_admin.mint(&contract_id, &1_000_000);

    (client, admin, customer, Address::generate(env), token_addr)
}

// inactivity_seconds = 10_000, near_expiry_buffer_seconds = 500
fn default_config() -> StaleThresholdConfig {
    StaleThresholdConfig {
        inactivity_seconds: 10_000,
        near_expiry_buffer_seconds: 500,
    }
}

#[test]
fn test_set_and_get_stale_threshold() {
    let env = Env::default();
    let (client, admin, _, _, _) = setup(&env);

    assert!(client.get_stale_threshold().is_none());

    client.set_stale_threshold(&admin, &default_config());

    let stored = client.get_stale_threshold().unwrap();
    assert_eq!(stored.inactivity_seconds, 10_000);
    assert_eq!(stored.near_expiry_buffer_seconds, 500);
}

#[test]
fn test_set_stale_threshold_non_admin_fails() {
    let env = Env::default();
    let (client, _admin, customer, _, _) = setup(&env);

    // `customer` is not part of the multisig admin set.
    let result = client.try_set_stale_threshold(&customer, &default_config());
    assert!(result.is_err());
}

#[test]
fn test_health_healthy() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    env.ledger().set_timestamp(1_000);
    // release=2000, expiry far in the future (100_000)
    let id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &100_000_u64, &true,
    );

    let report = client.get_escrow_health(&id);
    assert_eq!(report.health, EscrowHealth::Healthy);
    assert_eq!(report.last_activity, 1_000);
    assert_eq!(report.seconds_until_expiry, Some(99_000));
}

#[test]
fn test_health_near_expiry() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    env.ledger().set_timestamp(1_000);
    // release=2000, expiry=3000
    let id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &3_000_u64, &true,
    );

    // Within 500s of expiry (3000 - 2600 = 400 <= 500), still inactive < 10_000.
    env.ledger().set_timestamp(2_600);
    let report = client.get_escrow_health(&id);
    assert_eq!(report.health, EscrowHealth::NearExpiry);
    assert_eq!(report.seconds_until_expiry, Some(400));
}

#[test]
fn test_health_stale() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    env.ledger().set_timestamp(1_000);
    // expiry = 0 => no expiry, so it can only be Stale or Healthy.
    let id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &0_u64, &false,
    );

    // Advance past inactivity window (now - last_activity = 11_000 >= 10_000).
    env.ledger().set_timestamp(12_000);
    let report = client.get_escrow_health(&id);
    assert_eq!(report.health, EscrowHealth::Stale);
    assert_eq!(report.seconds_until_expiry, None);
}

#[test]
fn test_health_disputed() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    env.ledger().set_timestamp(1_000);
    let id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &100_000_u64, &true,
    );
    client.dispute_escrow(&customer, &id);

    let report = client.get_escrow_health(&id);
    assert_eq!(report.health, EscrowHealth::Disputed);
}

#[test]
fn test_health_expired() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    env.ledger().set_timestamp(1_000);
    // release=2000, expiry=3000
    let id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &3_000_u64, &true,
    );

    env.ledger().set_timestamp(3_001);
    let report = client.get_escrow_health(&id);
    assert_eq!(report.health, EscrowHealth::Expired);
    assert_eq!(report.seconds_until_expiry, Some(-1));
}

#[test]
fn test_get_stale_escrows_respects_limit() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    env.ledger().set_timestamp(1_000);
    // Create 5 escrows with no expiry.
    for _ in 0..5 {
        client.create_escrow(
            &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &0_u64, &false,
        );
    }

    // Before the inactivity window elapses, none are stale.
    let none_stale = client.get_stale_escrows(&10_u32);
    assert_eq!(none_stale.len(), 0);

    // Advance past inactivity window: all 5 become stale.
    env.ledger().set_timestamp(12_000);
    let all_stale = client.get_stale_escrows(&10_u32);
    assert_eq!(all_stale.len(), 5);

    // `limit` caps the number of returned IDs.
    let capped = client.get_stale_escrows(&3_u32);
    assert_eq!(capped.len(), 3);
}

#[test]
fn test_get_escrow_health_not_configured_fails() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1_000);
    let id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2_000_u64, &0_u64, &0_u64, &false,
    );

    // No threshold configured => StaleThresholdNotConfigured.
    let result = client.try_get_escrow_health(&id);
    assert!(result.is_err());
}

#[test]
fn test_get_stale_escrows_not_configured_fails() {
    let env = Env::default();
    let (client, _admin, _, _, _) = setup(&env);

    let result = client.try_get_stale_escrows(&10_u32);
    assert!(result.is_err());
}

#[test]
fn test_get_escrow_health_missing_escrow_fails() {
    let env = Env::default();
    let (client, admin, _, _, _) = setup(&env);
    client.set_stale_threshold(&admin, &default_config());

    let result = client.try_get_escrow_health(&999_u64);
    assert!(result.is_err());
}
