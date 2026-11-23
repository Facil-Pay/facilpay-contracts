#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as TestAddress, Ledger};
use soroban_sdk::{token, Address, BytesN, Env};

fn setup_token(env: &Env) -> (Address, token::StellarAssetClient<'_>, token::Client<'_>) {
    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let asset_client = token::StellarAssetClient::new(env, &token_id);
    let token_client = token::Client::new(env, &token_id);
    (token_id, asset_client, token_client)
}

#[test]
fn test_initiate_clawback_success() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, _token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;

    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);
    assert_eq!(request_id, 1);

    let request = client.get_clawback_request(&request_id).unwrap();
    assert_eq!(request.escrow_id, escrow_id);
    assert_eq!(request.initiated_by, admin);
    assert_eq!(request.reason_hash, reason_hash);
    assert_eq!(request.execute_after, env.ledger().timestamp() + delay);
    assert!(!request.executed);
    assert!(!request.cancelled);
}

#[test]
fn test_initiate_clawback_too_short_delay() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, _token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 0_u64;

    let result = client.try_initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::ClawbackDelayTooShort)))
    );
}

#[test]
fn test_initiate_clawback_already_pending() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, _token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;

    client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    let result = client.try_initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::AlreadyProcessed)))
    );
}

#[test]
fn test_initiate_clawback_rejected_for_released_escrow() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    // Release the escrow before the timelock window even exists.
    env.ledger().set_timestamp(env.ledger().timestamp() + 1000);
    client.release_escrow(&admin, &escrow_id, &false);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(token_client.balance(&merchant), 1000);

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let result = client.try_initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::InvalidStatus))));
}

#[test]
fn test_execute_clawback_success() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    asset_client.mint(&customer, &1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    // Fast forward time
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + delay + 1);

    client.execute_clawback(&admin, &request_id);

    let request = client.get_clawback_request(&request_id).unwrap();
    assert!(request.executed);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);

    assert_eq!(token_client.balance(&admin), 1000);
}

#[test]
fn test_execute_clawback_before_delay_fails() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, _token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    // Only move time forward slightly - still before execute_after.
    env.ledger().set_timestamp(env.ledger().timestamp() + 100);

    let result = client.try_execute_clawback(&admin, &request_id);
    assert_eq!(result, Err(Ok(Error::Action(ActionError::NotReady))));
}

#[test]
fn test_execute_clawback_exactly_at_timelock_boundary() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    let request = client.get_clawback_request(&request_id).unwrap();
    // Land exactly on `execute_after` (now < execute_after is the only rejection check).
    env.ledger().set_timestamp(request.execute_after);

    client.execute_clawback(&admin, &request_id);

    let request = client.get_clawback_request(&request_id).unwrap();
    assert!(request.executed);
    assert_eq!(token_client.balance(&admin), 1000);
}

#[test]
fn test_cancel_clawback_success() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, _token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    client.cancel_clawback(&admin, &request_id);

    let request = client.get_clawback_request(&request_id).unwrap();
    assert!(request.cancelled);
}

#[test]
fn test_execute_cancelled_clawback_fails() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let (token_id, asset_client, _token_client) = setup_token(&env);

    env.mock_all_auths();
    client.initialize(&admin);
    asset_client.mint(&customer, &1000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token_id, &1000_u64, &0_u64, &0_u64, &false,
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    client.cancel_clawback(&admin, &request_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + delay + 1);
    let result = client.try_execute_clawback(&admin, &request_id);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::InvalidStatus))));
}

#[test]
fn test_get_clawback_request_unknown_returns_none() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    assert!(client.get_clawback_request(&999_u64).is_none());
}
