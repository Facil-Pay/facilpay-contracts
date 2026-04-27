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
