#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, contract, contractimpl};

#[contract]
pub struct MockSwapOracle;

#[contractimpl]
impl MockSwapOracle {
    pub fn get_rate(env: Env) -> i128 {
        env.storage()
            .instance()
            .get::<u32, i128>(&0u32)
            .unwrap_or(10_000_000) // Default 1.0 rate (1e7)
    }

    pub fn set_rate(env: Env, rate: i128) {
        env.storage().instance().set(&0u32, &rate);
    }
}

#[test]
fn test_escrow_swap_successful() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let target_token = Address::generate(&env);
    let oracle = env.register(MockSwapOracle, ());
    let oracle_client = MockSwapOracleClient::new(&env, &oracle);

    client.initialize(&admin);

    // Create escrow (8 arguments as defined in lib.rs)
    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    // Set rate to 1.5 (15_000_000)
    oracle_client.set_rate(&15_000_000_i128);

    // Configure swap by merchant (min_output = 1400)
    client.configure_escrow_swap(
        &merchant,
        &escrow_id,
        &target_token,
        &1400_i128,
        &oracle,
    );

    // Verify config is retrieved correctly
    let config = client.get_swap_config(&escrow_id).unwrap();
    assert_eq!(config.escrow_id, escrow_id);
    assert_eq!(config.source_token, token);
    assert_eq!(config.target_token, target_token);
    assert_eq!(config.min_output_amount, 1400_i128);
    assert_eq!(config.oracle, oracle);
    assert_eq!(config.executed, false);

    // Execute swap
    let output = client.execute_escrow_swap(&merchant, &escrow_id);
    assert_eq!(output, 1500_i128); // 1000 * 15_000_000 / 10_000_000 = 1500

    // Verify updated escrow
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.token, target_token);
    assert_eq!(escrow.amount, 1500_i128);

    // Verify config executed
    let config = client.get_swap_config(&escrow_id).unwrap();
    assert_eq!(config.executed, true);
}

#[test]
fn test_escrow_swap_successful_by_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let target_token = Address::generate(&env);
    let oracle = env.register(MockSwapOracle, ());
    let oracle_client = MockSwapOracleClient::new(&env, &oracle);

    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    oracle_client.set_rate(&15_000_000_i128);

    // Configure swap by admin
    client.configure_escrow_swap(
        &admin,
        &escrow_id,
        &target_token,
        &1400_i128,
        &oracle,
    );

    // Execute swap by admin
    let output = client.execute_escrow_swap(&admin, &escrow_id);
    assert_eq!(output, 1500_i128);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.token, target_token);
    assert_eq!(escrow.amount, 1500_i128);
}

#[test]
fn test_escrow_swap_below_minimum_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let target_token = Address::generate(&env);
    let oracle = env.register(MockSwapOracle, ());
    let oracle_client = MockSwapOracleClient::new(&env, &oracle);

    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    // Set rate to 1.2 (12_000_000) -> output = 1200
    oracle_client.set_rate(&12_000_000_i128);

    // Configure swap (min_output = 1300)
    client.configure_escrow_swap(
        &merchant,
        &escrow_id,
        &target_token,
        &1300_i128,
        &oracle,
    );

    // Executing should fail with SwapOutputBelowMinimum
    let res = client.try_execute_escrow_swap(&merchant, &escrow_id);
    assert_eq!(res, Err(Ok(Error::SwapOutputBelowMinimum)));
}

#[test]
fn test_escrow_swap_double_execution_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let target_token = Address::generate(&env);
    let oracle = env.register(MockSwapOracle, ());
    let oracle_client = MockSwapOracleClient::new(&env, &oracle);

    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    oracle_client.set_rate(&15_000_000_i128);

    client.configure_escrow_swap(
        &merchant,
        &escrow_id,
        &target_token,
        &1400_i128,
        &oracle,
    );

    // Execute first time: success
    let output = client.execute_escrow_swap(&merchant, &escrow_id);
    assert_eq!(output, 1500_i128);

    // Execute second time: fails with SwapAlreadyExecuted
    let res = client.try_execute_escrow_swap(&merchant, &escrow_id);
    assert_eq!(res, Err(Ok(Error::SwapAlreadyExecuted)));
}

#[test]
fn test_escrow_swap_unauthorized_config_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let unauthorized_caller = Address::generate(&env);
    let token = Address::generate(&env);
    let target_token = Address::generate(&env);
    let oracle = env.register(MockSwapOracle, ());

    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    // Non-merchant, non-admin tries to configure swap: fails with Unauthorized
    let res = client.try_configure_escrow_swap(
        &unauthorized_caller,
        &escrow_id,
        &target_token,
        &1400_i128,
        &oracle,
    );
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_escrow_swap_unauthorized_execute_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let unauthorized_caller = Address::generate(&env);
    let token = Address::generate(&env);
    let target_token = Address::generate(&env);
    let oracle = env.register(MockSwapOracle, ());

    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    client.configure_escrow_swap(
        &merchant,
        &escrow_id,
        &target_token,
        &1400_i128,
        &oracle,
    );

    // Non-merchant, non-admin tries to execute swap: fails with Unauthorized
    let res = client.try_execute_escrow_swap(&unauthorized_caller, &escrow_id);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_escrow_swap_config_not_found_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &0_u64,
        &0_u64,
        &false,
    );

    // Execute swap directly without configuring first: fails with SwapConfigNotFound
    let res = client.try_execute_escrow_swap(&merchant, &escrow_id);
    assert_eq!(res, Err(Ok(Error::SwapConfigNotFound)));
}
