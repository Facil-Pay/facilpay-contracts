#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

#[test]
fn test_dispute_collateral_deposit_and_return() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    env.ledger().set_timestamp(1000);

    // Setup collateral config
    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 100,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    token_admin_client.mint(&customer, &1000);
    token_client.transfer(&customer, &contract_id, &1000);

    // Mint collateral to customer
    token_admin_client.mint(&customer, &100);

    // Dispute requires collateral
    client.dispute_escrow(&customer, &escrow_id);

    // Check balance - should be 0 (transferred to contract)
    assert_eq!(token_client.balance(&customer), 0);
    assert_eq!(token_client.balance(&contract_id), 1100); // 1000 escrow + 100 collateral

    let collateral = client.get_dispute_collateral(&escrow_id);
    assert_eq!(collateral.amount, 100);
    assert_eq!(collateral.disputing_party, customer);

    // Resolve in favor of customer -> collateral returned
    client.resolve_dispute(&admin, &escrow_id, &false);

    assert_eq!(token_client.balance(&customer), 1100); // 1000 escrow + 100 collateral returned
    assert_eq!(token_client.balance(&contract_id), 0);
}

#[test]
fn test_dispute_collateral_forfeiture() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    env.ledger().set_timestamp(1000);

    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 50,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    token_admin_client.mint(&customer, &1000);
    token_client.transfer(&customer, &contract_id, &1000);

    // Merchant disputes
    token_admin_client.mint(&merchant, &50);
    client.dispute_escrow(&merchant, &escrow_id);

    // Resolve in favor of customer -> merchant forfeits to customer
    client.resolve_dispute(&admin, &escrow_id, &false);

    assert_eq!(token_client.balance(&customer), 1050); // 1000 escrow + 50 forfeited collateral
    assert_eq!(token_client.balance(&merchant), 0);
}

#[test]
fn test_dispute_without_collateral_when_disabled() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);

    env.ledger().set_timestamp(1000);

    // Collateral disabled
    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 100,
            collateral_enabled: false,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &9999_u64, &0_u64);

    // Dispute without having any collateral token
    client.dispute_escrow(&customer, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);

    // get_dispute_collateral should fail
    let res = client.try_get_dispute_collateral(&escrow_id);
    assert!(res.is_err());
}

#[test]
fn test_collateral_different_token_than_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    
    // Different tokens for escrow and collateral
    let escrow_token = env.register_stellar_asset_contract(admin.clone());
    let collateral_token = env.register_stellar_asset_contract(admin.clone());
    
    let escrow_token_client = token::Client::new(&env, &escrow_token);
    let collateral_token_client = token::Client::new(&env, &collateral_token);
    let escrow_token_admin_client = token::StellarAssetClient::new(&env, &escrow_token);
    let collateral_token_admin_client = token::StellarAssetClient::new(&env, &collateral_token);

    env.ledger().set_timestamp(1000);

    // Setup collateral config with different token
    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: collateral_token.clone(),
            collateral_amount: 200,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &5000_i128, &escrow_token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    escrow_token_admin_client.mint(&customer, &5000);
    escrow_token_client.transfer(&customer, &contract_id, &5000);

    // Mint collateral in different token to customer
    collateral_token_admin_client.mint(&customer, &200);

    // Customer disputes and pays collateral
    client.dispute_escrow(&customer, &escrow_id);

    // Check balances - escrow and collateral in different tokens
    assert_eq!(escrow_token_client.balance(&customer), 0);
    assert_eq!(escrow_token_client.balance(&contract_id), 5000);
    assert_eq!(collateral_token_client.balance(&customer), 0);
    assert_eq!(collateral_token_client.balance(&contract_id), 200);

    let collateral = client.get_dispute_collateral(&escrow_id);
    assert_eq!(collateral.amount, 200);
    assert_eq!(collateral.disputing_party, customer);

    // Resolve in favor of merchant -> collateral forfeited
    client.resolve_dispute(&admin, &escrow_id, &true);

    // Merchant gets escrow + collateral
    assert_eq!(escrow_token_client.balance(&merchant), 5000);
    assert_eq!(collateral_token_client.balance(&merchant), 200);
}

#[test]
fn test_collateral_both_parties_dispute() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    env.ledger().set_timestamp(1000);

    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 150,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &2000_i128, &token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    token_admin_client.mint(&customer, &2000);
    token_client.transfer(&customer, &contract_id, &2000);

    // Customer disputes first
    token_admin_client.mint(&customer, &150);
    client.dispute_escrow(&customer, &escrow_id);

    let collateral = client.get_dispute_collateral(&escrow_id);
    assert_eq!(collateral.amount, 150);
    assert_eq!(collateral.disputing_party, customer);

    // Check only customer's collateral is locked
    assert_eq!(token_client.balance(&customer), 0);
    assert_eq!(token_client.balance(&contract_id), 2150); // 2000 escrow + 150 collateral

    // Merchant cannot dispute again (already disputed)
    // The contract should handle this appropriately
}

#[test]
fn test_collateral_insufficient_balance_should_fail() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    env.ledger().set_timestamp(1000);

    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 500,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    token_admin_client.mint(&customer, &1000);
    token_client.transfer(&customer, &contract_id, &1000);

    // Mint insufficient collateral (only 300 instead of 500)
    token_admin_client.mint(&customer, &300);

    // Dispute should fail due to insufficient collateral
    let res = client.try_dispute_escrow(&customer, &escrow_id);
    assert!(res.is_err());

    // Escrow should not be in disputed state
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Active);
}

#[test]
fn test_collateral_config_update_mid_dispute() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    env.ledger().set_timestamp(1000);

    // Initial config
    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 100,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    token_admin_client.mint(&customer, &1000);
    token_client.transfer(&customer, &contract_id, &1000);

    // Mint collateral and dispute
    token_admin_client.mint(&customer, &100);
    client.dispute_escrow(&customer, &escrow_id);

    let collateral = client.get_dispute_collateral(&escrow_id);
    assert_eq!(collateral.amount, 100);

    // Update collateral config (should not affect existing dispute)
    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 500,
            collateral_enabled: true,
        },
    );

    // Existing dispute collateral should remain at 100
    let collateral_after = client.get_dispute_collateral(&escrow_id);
    assert_eq!(collateral_after.amount, 100);

    // Resolve in favor of customer
    client.resolve_dispute(&admin, &escrow_id, &false);

    // Customer should get back original collateral amount (100)
    assert_eq!(token_client.balance(&customer), 1100);
}

#[test]
fn test_collateral_zero_amount_config() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token);
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    env.ledger().set_timestamp(1000);

    // Setup collateral with zero amount but enabled
    client.set_dispute_config(
        &admin,
        &DisputeConfig {
            collateral_token: token.clone(),
            collateral_amount: 0,
            collateral_enabled: true,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &9999_u64, &0_u64);

    // Mint and transfer escrow amount to contract
    token_admin_client.mint(&customer, &1000);
    token_client.transfer(&customer, &contract_id, &1000);

    // Don't mint any collateral since amount is 0
    
    // Dispute with zero collateral requirement
    client.dispute_escrow(&customer, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);

    // Should still have collateral record with zero amount
    let collateral = client.get_dispute_collateral(&escrow_id);
    assert_eq!(collateral.amount, 0);
    assert_eq!(collateral.disputing_party, customer);

    // Resolve in favor of merchant
    client.resolve_dispute(&admin, &escrow_id, &true);

    // Merchant gets escrow, no collateral to forfeit
    assert_eq!(token_client.balance(&merchant), 1000);
    assert_eq!(token_client.balance(&customer), 0);
}
