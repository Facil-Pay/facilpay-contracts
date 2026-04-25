#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, EscrowContractClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    env.ledger().set_timestamp(1000);
    client.create_escrow(&customer, &merchant, &1000_i128, &token, &2000_u64, &0_u64);

    (env, client, admin, customer, merchant, token)
}

#[test]
fn test_authorized_transfer_by_merchant() {
    let (env, client, _admin, _customer, merchant, _token) = setup();
    let new_merchant = Address::generate(&env);

    client.transfer_escrow_beneficiary(&merchant, &1u64, &new_merchant);

    let escrow = EscrowContract::get_escrow(&env, 1);
    assert_eq!(escrow.merchant, new_merchant);
}

#[test]
fn test_authorized_transfer_by_admin() {
    let (env, client, admin, _customer, _merchant, _token) = setup();
    let new_merchant = Address::generate(&env);

    client.transfer_escrow_beneficiary(&admin, &1u64, &new_merchant);

    let escrow = EscrowContract::get_escrow(&env, 1);
    assert_eq!(escrow.merchant, new_merchant);
}

#[test]
fn test_unauthorized_transfer_rejected() {
    let (env, client, _admin, customer, _merchant, _token) = setup();
    let new_merchant = Address::generate(&env);

    let result = client.try_transfer_escrow_beneficiary(&customer, &1u64, &new_merchant);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_transfer_blocked_on_disputed_escrow() {
    let (env, client, _admin, customer, merchant, _token) = setup();
    let new_merchant = Address::generate(&env);

    client.dispute_escrow(&customer, &1u64);

    let result = client.try_transfer_escrow_beneficiary(&merchant, &1u64, &new_merchant);
    assert_eq!(result, Err(Ok(Error::TransferNotAllowed)));
}

#[test]
fn test_transfer_blocked_on_resolved_escrow() {
    let (env, client, admin, customer, merchant, _token) = setup();
    let new_merchant = Address::generate(&env);

    client.dispute_escrow(&customer, &1u64);
    client.resolve_dispute(&admin, &1u64, &false);

    let result = client.try_transfer_escrow_beneficiary(&merchant, &1u64, &new_merchant);
    assert_eq!(result, Err(Ok(Error::TransferNotAllowed)));
}

#[test]
fn test_same_beneficiary_rejected() {
    let (_env, client, _admin, _customer, merchant, _token) = setup();

    let result = client.try_transfer_escrow_beneficiary(&merchant, &1u64, &merchant);
    assert_eq!(result, Err(Ok(Error::SameBeneficiary)));
}

#[test]
fn test_transfer_history_is_append_only() {
    let (env, client, _admin, _customer, merchant, _token) = setup();
    let new_merchant1 = Address::generate(&env);
    let new_merchant2 = Address::generate(&env);

    client.transfer_escrow_beneficiary(&merchant, &1u64, &new_merchant1);
    client.transfer_escrow_beneficiary(&new_merchant1, &1u64, &new_merchant2);

    let history = client.get_transfer_history(&1u64);
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().from, merchant);
    assert_eq!(history.get(0).unwrap().to, new_merchant1);
    assert_eq!(history.get(1).unwrap().from, new_merchant1);
    assert_eq!(history.get(1).unwrap().to, new_merchant2);
}

#[test]
fn test_transfer_history_empty_before_any_transfer() {
    let (_env, client, _admin, _customer, _merchant, _token) = setup();
    let history = client.get_transfer_history(&1u64);
    assert_eq!(history.len(), 0);
}
