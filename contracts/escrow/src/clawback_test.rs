#![cfg(test)]
use super::*;
use soroban_sdk::{Address, BytesN, Env, token};
use soroban_sdk::testutils::Address as TestAddress;

#[test]
fn test_initiate_clawback_success() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
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
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
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
fn test_initiate_clawback_already_initiated() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;

    client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    let result = client.try_initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);
    assert_eq!(result, Err(Ok(Error::ClawbackAlreadyInitiated)));
}

#[test]
fn test_execute_clawback_success() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    env.mock_all_auths();
    client.initialize(&admin);

    token_client.mint(&customer, &1000);
    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token_id,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    // Fast forward time
    env.ledger().set_timestamp(env.ledger().timestamp() + delay + 1);

    client.execute_clawback(&admin, &request_id);

    let request = client.get_clawback_request(&request_id).unwrap();
    assert!(request.executed);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);

    let user_token_client = token::Client::new(&env, &token_id);
    assert_eq!(user_token_client.balance(&admin), 1000);
}

#[test]
#[should_panic]
fn test_execute_clawback_before_delay() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    // Only move time forward slightly
    env.ledger().set_timestamp(env.ledger().timestamp() + 100);

    client.execute_clawback(&admin, &request_id);
}

#[test]
fn test_cancel_clawback_success() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    client.cancel_clawback(&admin, &request_id);

    let request = client.get_clawback_request(&request_id).unwrap();
    assert!(request.cancelled);
}

#[test]
#[should_panic]
fn test_execute_cancelled_clawback() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false
    );

    let reason_hash = BytesN::from_array(&env, &[1_u8; 32]);
    let delay = 86400_u64;
    let request_id = client.initiate_clawback(&admin, &escrow_id, &reason_hash, &delay);

    client.cancel_clawback(&admin, &request_id);

    env.ledger().set_timestamp(env.ledger().timestamp() + delay + 1);
    client.execute_clawback(&admin, &request_id);
}
