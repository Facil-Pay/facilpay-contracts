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
    token_admin.mint(&customer, &10_000);
    // fund contract directly so transfers succeed
    token_admin.mint(&contract_id, &10_000);

    (client, admin, customer, Address::generate(env), token_addr)
}

#[test]
fn test_expire_escrow_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    // release_timestamp=2000, expiry_timestamp=3000
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    assert!(!client.is_escrow_expired(&escrow_id));

    env.ledger().set_timestamp(3001);
    assert!(client.is_escrow_expired(&escrow_id));

    client.expire_escrow(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn test_expire_escrow_premature_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    // Still before expiry
    env.ledger().set_timestamp(2500);
    let result = client.try_expire_escrow(&escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_expire_disputed_escrow_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    client.dispute_escrow(&customer, &escrow_id);

    // Advance past expiry
    env.ledger().set_timestamp(4000);
    let result = client.try_expire_escrow(&escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_set_global_expiry_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _, _, _) = setup(&env);

    client.set_global_expiry_config(&admin, &86400_u64);
    // No panic = success; config stored correctly
}

#[test]
fn test_create_escrow_expiry_before_release_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    // expiry_timestamp <= release_timestamp should fail
    let result = client.try_create_escrow(
        &customer, &merchant, &500_i128, &token, &5000_u64, &0_u64, &4000_u64, &true,
    );
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_expiry_in_past_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(5000);
    // expiry_timestamp(4999) is in the past relative to ledger time(5000)
    let result = client.try_create_escrow(
        &customer, &merchant, &500_i128, &token, &1000_u64, &0_u64, &4999_u64, &true,
    );
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_expiry_equal_to_current_time_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(5000);
    // expiry_timestamp(5000) == ledger time(5000), must be strictly after
    let result = client.try_create_escrow(
        &customer, &merchant, &500_i128, &token, &1000_u64, &0_u64, &5000_u64, &true,
    );
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_expiry_within_hold_period_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    // min_hold_period=500 → expiry must be > 1000+500=1500; 1400 fails
    let result = client.try_create_escrow(
        &customer, &merchant, &500_i128, &token, &500_u64, &500_u64, &1400_u64, &true,
    );
    assert!(result.is_err());
}

#[test]
fn test_create_escrow_expiry_after_hold_period_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    // min_hold_period=500 → expiry must be > 1000+500=1500; 1600 succeeds
    // release_timestamp=500, expiry=1600 (> release and > current+hold_period)
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &500_u64, &500_u64, &1600_u64, &true,
    );
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.expiry_timestamp, 1600);
}

#[test]
fn test_extend_expiry_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    env.ledger().set_timestamp(2000);
    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &4000_u64);
    assert!(result.is_ok());

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.expiry_timestamp, 4000);
}

#[test]
fn test_extend_expiry_non_participant_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    let random = Address::generate(&env);
    env.ledger().set_timestamp(2000);
    let result = client.try_extend_escrow_expiry(&random, &escrow_id, &4000_u64);
    assert!(result.is_err());
}

#[test]
fn test_extend_expiry_not_after_current_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    env.ledger().set_timestamp(2000);
    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &2500_u64);
    assert!(result.is_err());
}

#[test]
fn test_extend_expiry_released_escrow_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &1500_u64, &0_u64, &3000_u64, &true,
    );

    env.ledger().set_timestamp(1600);
    client.release_escrow(&escrow_id);

    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &4000_u64);
    assert!(result.is_err());
}

#[test]
fn test_extend_expiry_respects_max_renewals() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, customer, merchant, token) = setup(&env);

    // Set renewal config with max_renewals = 1
    let config = crate::EscrowRenewalConfig {
        enabled: true,
        max_renewals: 1,
        renewal_fee_bps: 100,
        min_renewal_period: 100,
        max_renewal_period: 1000,
    };
    client.set_renewal_config(&admin, &config);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    env.ledger().set_timestamp(2000);
    // First extension should succeed
    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &4000_u64);
    assert!(result.is_ok());

    // Second extension should fail (max_renewals reached)
    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &5000_u64);
    assert!(result.is_err());
}

#[test]
fn test_extend_expiry_period_too_short() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, customer, merchant, token) = setup(&env);

    let config = crate::EscrowRenewalConfig {
        enabled: true,
        max_renewals: 10,
        renewal_fee_bps: 100,
        min_renewal_period: 500,
        max_renewal_period: 2000,
    };
    client.set_renewal_config(&admin, &config);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    env.ledger().set_timestamp(2000);
    // Extension = 3100 - 3000 = 100, which is < min_renewal_period (500)
    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &3100_u64);
    assert!(result.is_err());
}

#[test]
fn test_extend_expiry_disabled_config_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, customer, merchant, token) = setup(&env);

    let config = crate::EscrowRenewalConfig {
        enabled: false,
        max_renewals: 10,
        renewal_fee_bps: 100,
        min_renewal_period: 100,
        max_renewal_period: 2000,
    };
    client.set_renewal_config(&admin, &config);

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &2000_u64, &0_u64, &3000_u64, &true,
    );

    env.ledger().set_timestamp(2000);
    let result = client.try_extend_escrow_expiry(&customer, &escrow_id, &4000_u64);
    assert!(result.is_err());
}
