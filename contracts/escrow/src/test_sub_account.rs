#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, BytesN, Env};

fn setup(env: &Env) -> (EscrowContractClient<'_>, Address, Address, Address, Address) {
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    let token = Address::generate(env);
    env.mock_all_auths();
    (client, admin, customer, merchant, token)
}

fn make_escrow(
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
    amount: i128,
) -> u64 {
    client.create_escrow(customer, merchant, &amount, token, &9999999_u64, &0_u64, &0_u64, &false)
}

fn label(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

#[test]
fn test_create_sub_account() {
    let env = Env::default();
    let (client, _, customer, merchant, token) = setup(&env);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token, 1000);

    let sub_id = client.create_sub_account(&merchant, &escrow_id, &label(&env), &400, &None);
    assert_eq!(sub_id, 1);

    let sub = client.get_sub_account(&escrow_id, &sub_id).unwrap();
    assert_eq!(sub.amount, 400);
    assert!(!sub.released);
}

#[test]
fn test_funding_sub_account() {
    let env = Env::default();
    let (client, _, customer, merchant, token) = setup(&env);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token, 1000);
    let sub_id = client.create_sub_account(&merchant, &escrow_id, &label(&env), &200, &None);

    client.fund_sub_account(&merchant, &escrow_id, &sub_id, &300);

    let sub = client.get_sub_account(&escrow_id, &sub_id).unwrap();
    assert_eq!(sub.amount, 500);
}

#[test]
fn test_release_sub_account() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);

    let token_id = env.register(soroban_sdk::token::StellarAssetContract, (&admin,));
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&customer, &1000);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token_id, 1000);
    let sub_id = client.create_sub_account(&merchant, &escrow_id, &label(&env), &500, &None);

    client.release_sub_account(&admin, &escrow_id, &sub_id);

    let sub = client.get_sub_account(&escrow_id, &sub_id).unwrap();
    assert!(sub.released);
}

#[test]
fn test_double_release_rejected() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);

    let token_id = env.register(soroban_sdk::token::StellarAssetContract, (&admin,));
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&customer, &1000);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token_id, 1000);
    let sub_id = client.create_sub_account(&merchant, &escrow_id, &label(&env), &500, &None);

    client.release_sub_account(&admin, &escrow_id, &sub_id);

    let result = client.try_release_sub_account(&admin, &escrow_id, &sub_id);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::SubAccountAlreadyReleased))));
}

#[test]
fn test_funding_exceeds_escrow_rejected() {
    let env = Env::default();
    let (client, _, customer, merchant, token) = setup(&env);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token, 1000);

    // Try to allocate more than the escrow holds
    let result = client.try_create_sub_account(&merchant, &escrow_id, &label(&env), &1001, &None);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::SubAccountFundingExceedsEscrow))));
}

#[test]
fn test_parent_guard_blocks_release_with_unreleased_sub_accounts() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1000);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token, 1000);
    client.create_sub_account(&merchant, &escrow_id, &label(&env), &500, &None);

    // Parent release should fail because sub-account is not yet released
    let result = client.try_release_escrow(&admin, &escrow_id, &true);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::InvalidStatus))));
}

#[test]
fn test_list_sub_accounts() {
    let env = Env::default();
    let (client, _, customer, merchant, token) = setup(&env);

    let escrow_id = make_escrow(&client, &customer, &merchant, &token, 1000);
    client.create_sub_account(&merchant, &escrow_id, &label(&env), &200, &None);
    client.create_sub_account(
        &merchant,
        &escrow_id,
        &BytesN::from_array(&env, &[2u8; 32]),
        &300,
        &None,
    );

    let subs = client.list_sub_accounts(&escrow_id);
    assert_eq!(subs.len(), 2);
    assert_eq!(subs.get(0).unwrap().amount, 200);
    assert_eq!(subs.get(1).unwrap().amount, 300);
}

#[test]
fn test_sub_account_fee_bps_override() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);

    client.set_escrow_fee_config(
        &admin,
        &EscrowFeeConfig {
            fee_bps: 500,
            fee_recipient: admin.clone(),
            enabled: true,
        },
    );

    let escrow_id = make_escrow(&client, &customer, &merchant, &token, 1000);

    let fee_free_id = client.create_sub_account(
        &merchant,
        &escrow_id,
        &label(&env),
        &200,
        &Some(0),
    );
    let premium_id = client.create_sub_account(
        &merchant,
        &escrow_id,
        &BytesN::from_array(&env, &[2u8; 32]),
        &300,
        &Some(1000),
    );
    let inherited_id = client.create_sub_account(
        &merchant,
        &escrow_id,
        &BytesN::from_array(&env, &[3u8; 32]),
        &100,
        &None,
    );

    assert_eq!(
        client
            .get_sub_account(&escrow_id, &fee_free_id)
            .unwrap()
            .fee_bps_override,
        Some(0)
    );
    assert_eq!(
        client
            .get_sub_account(&escrow_id, &premium_id)
            .unwrap()
            .fee_bps_override,
        Some(1000)
    );
    assert!(
        client
            .get_sub_account(&escrow_id, &inherited_id)
            .unwrap()
            .fee_bps_override
            .is_none()
    );

    client.set_sub_account_fee_override(&merchant, &escrow_id, &inherited_id, &Some(250));
    assert_eq!(
        client
            .get_sub_account(&escrow_id, &inherited_id)
            .unwrap()
            .fee_bps_override,
        Some(250)
    );
}
