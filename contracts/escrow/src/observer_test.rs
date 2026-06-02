#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup(
    env: &Env,
) -> (
    EscrowContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    let token = Address::generate(env);
    (client, admin, customer, merchant, token)
}

fn create_escrow(
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> u64 {
    client.create_escrow(
        customer,
        merchant,
        &1000_i128,
        token,
        &5000_u64,
        &0_u64,
        &0_u64,
        &false,
    )
}

#[test]
fn test_add_observer_grant_and_verify() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    client.add_observer(&customer, &escrow_id, &observer, &3600_u64);

    assert!(client.verify_observer_access(&escrow_id, &observer));
    let observers = client.get_observers(&escrow_id);
    assert_eq!(observers.len(), 1);
    assert_eq!(observers.get(0).unwrap().observer, observer);
}

#[test]
fn test_remove_observer() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    client.add_observer(&merchant, &escrow_id, &observer, &3600_u64);
    client.remove_observer(&merchant, &escrow_id, &observer);

    assert!(!client.verify_observer_access(&escrow_id, &observer));
    assert_eq!(client.get_observers(&escrow_id).len(), 0);
}

#[test]
fn test_expired_observer_not_removed_and_no_access() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    client.add_observer(&customer, &escrow_id, &observer, &100_u64);
    env.ledger().set_timestamp(env.ledger().timestamp() + 200);

    assert!(!client.verify_observer_access(&escrow_id, &observer));
    assert_eq!(client.get_observers(&escrow_id).len(), 1);
}

#[test]
fn test_duplicate_active_observer_rejected() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    client.add_observer(&customer, &escrow_id, &observer, &3600_u64);

    let result = client.try_add_observer(&customer, &escrow_id, &observer, &3600_u64);
    assert_eq!(result, Err(Ok(Error::ObserverAlreadyAdded)));
}

#[test]
fn test_unauthorized_grant_rejected() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);
    let stranger = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    let result = client.try_add_observer(&stranger, &escrow_id, &observer, &3600_u64);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_admin_can_grant_observer() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    client.add_observer(&admin, &escrow_id, &observer, &3600_u64);
    assert!(client.verify_observer_access(&escrow_id, &observer));
}

#[test]
fn test_remove_observer_not_found() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    let observer = Address::generate(&env);

    let escrow_id = create_escrow(&client, &customer, &merchant, &token);
    let result = client.try_remove_observer(&customer, &escrow_id, &observer);
    assert_eq!(result, Err(Ok(Error::ObserverNotFound)));
}
