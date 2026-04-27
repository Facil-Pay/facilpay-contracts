#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env, String,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

fn setup_test_env() -> (Env, Address, Address, Address, Address, Address, Address, Address, token::Client<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let arbitrator1 = Address::generate(&env);
    let arbitrator2 = Address::generate(&env);
    let arbitrator3 = Address::generate(&env);
    let treasury = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&merchant, &10000);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Register arbitrators
    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, treasury, token_client)
}

#[test]
fn test_set_arbitration_fee_config_valid() {
    let (env, admin, _, _, _, _, _, treasury, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 7000, // 70%
        treasury_share_bps: 3000,   // 30%
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };

    client.set_arbitration_fee_config(&admin, &config);

    let retrieved_config = client.get_arbitration_fee_config();
    assert!(retrieved_config.is_some());
    let retrieved = retrieved_config.unwrap();
    assert_eq!(retrieved.arbitrator_share_bps, 7000);
    assert_eq!(retrieved.treasury_share_bps, 3000);
    assert_eq!(retrieved.treasury_address, treasury);
}

#[test]
#[should_panic(expected = "InvalidFeeConfig")]
fn test_set_arbitration_fee_config_invalid_sum() {
    let (env, admin, _, _, _, _, _, treasury, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 7000,
        treasury_share_bps: 2000, // Sum is 9000, not 10000
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };

    client.set_arbitration_fee_config(&admin, &config);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_set_arbitration_fee_config_unauthorized() {
    let (env, admin, _, customer, _, _, _, treasury, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 7000,
        treasury_share_bps: 3000,
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };

    // Try to set config with non-admin address
    client.set_arbitration_fee_config(&customer, &config);
}

#[test]
fn test_fee_distribution_equal_split() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, treasury, token_client) = setup_test_env();
    
    // Set fee configuration: 50/50 split
    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 5000, // 50%
        treasury_share_bps: 5000,   // 50%
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };
    
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);
    
    // Register arbitrators
    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);
    
    client.set_arbitration_fee_config(&admin, &config);

    // Create a refund request
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &10000i128,
        &token_client.address,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    // Reject it first so we can escalate
    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Rejected for testing"));

    // Escalate to arbitration with fee pool of 1000
    let fee_pool = 1000i128;
    token_client.transfer(&merchant, &contract_id, &fee_pool);
    
    let case_id = client.escalate_to_arbitration(
        &merchant,
        &refund_id,
        &token_client.address,
        &fee_pool,
    );

    // Cast votes - all three arbitrators vote for refund (majority)
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &hash);

    // Get initial balances
    let arb1_initial = token_client.balance(&arbitrator1);
    let arb2_initial = token_client.balance(&arbitrator2);
    let arb3_initial = token_client.balance(&arbitrator3);
    let treasury_initial = token_client.balance(&treasury);

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check balances after distribution
    let arb1_final = token_client.balance(&arbitrator1);
    let arb2_final = token_client.balance(&arbitrator2);
    let arb3_final = token_client.balance(&arbitrator3);
    let treasury_final = token_client.balance(&treasury);

    // Each arbitrator should get 500 / 3 = 166 (with rounding)
    let expected_per_arbitrator = 500 / 3;
    assert_eq!(arb1_final - arb1_initial, expected_per_arbitrator);
    assert_eq!(arb2_final - arb2_initial, expected_per_arbitrator);
    assert_eq!(arb3_final - arb3_initial, expected_per_arbitrator);

    // Treasury should get 500
    assert_eq!(treasury_final - treasury_initial, 500);

    // Check accumulated fees
    assert_eq!(client.get_accumulated_arbitration_fees(), 500);
}

#[test]
fn test_fee_distribution_majority_only() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, treasury, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Set fee configuration: 80/20 split
    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 8000, // 80%
        treasury_share_bps: 2000,   // 20%
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };
    client.set_arbitration_fee_config(&admin, &config);

    // Create a refund request
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &10000i128,
        &token_client.address,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    // Reject it first so we can escalate
    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Rejected for testing"));

    // Escalate to arbitration
    let fee_pool = 1000i128;
    token_client.transfer(&merchant, &contract_id, &fee_pool);
    
    let case_id = client.escalate_to_arbitration(
        &merchant,
        &refund_id,
        &token_client.address,
        &fee_pool,
    );

    // Cast votes - 2 for refund (majority), 1 against (minority)
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &false, &hash); // minority

    // Get initial balances
    let arb1_initial = token_client.balance(&arbitrator1);
    let arb2_initial = token_client.balance(&arbitrator2);
    let arb3_initial = token_client.balance(&arbitrator3);
    let treasury_initial = token_client.balance(&treasury);

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check balances after distribution
    let arb1_final = token_client.balance(&arbitrator1);
    let arb2_final = token_client.balance(&arbitrator2);
    let arb3_final = token_client.balance(&arbitrator3);
    let treasury_final = token_client.balance(&treasury);

    // Only majority voters (arb1 and arb2) should get fees
    // 800 / 2 = 400 each
    assert_eq!(arb1_final - arb1_initial, 400);
    assert_eq!(arb2_final - arb2_initial, 400);
    
    // Minority voter (arb3) should get nothing
    assert_eq!(arb3_final - arb3_initial, 0);

    // Treasury should get 200
    assert_eq!(treasury_final - treasury_initial, 200);
}

#[test]
fn test_withdraw_treasury_fees() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, treasury, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Set fee configuration
    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 7000, // 70%
        treasury_share_bps: 3000,   // 30%
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };
    client.set_arbitration_fee_config(&admin, &config);

    // Create and close an arbitration case
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &10000i128,
        &token_client.address,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    // Reject it first so we can escalate
    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Rejected for testing"));

    let fee_pool = 1000i128;
    token_client.transfer(&merchant, &contract_id, &fee_pool);
    
    let case_id = client.escalate_to_arbitration(
        &merchant,
        &refund_id,
        &token_client.address,
        &fee_pool,
    );

    let hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &hash);

    client.close_arbitration_case(&case_id);

    // Check accumulated fees (should be 300)
    let accumulated = client.get_accumulated_arbitration_fees();
    assert_eq!(accumulated, 300);

    // Withdraw fees
    let withdrawn = client.withdraw_treasury_fees(&admin);
    assert_eq!(withdrawn, 300);

    // Check that accumulated fees are now 0
    assert_eq!(client.get_accumulated_arbitration_fees(), 0);
}

#[test]
#[should_panic(expected = "InsufficientTreasuryFees")]
fn test_withdraw_treasury_fees_insufficient() {
    let (env, admin, _, _, _, _, _, _, _) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Try to withdraw when there are no accumulated fees
    client.withdraw_treasury_fees(&admin);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_withdraw_treasury_fees_unauthorized() {
    let (env, admin, _, customer, _, _, _, _, _) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Try to withdraw with non-admin address
    client.withdraw_treasury_fees(&customer);
}

#[test]
fn test_fee_distribution_without_config() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, _, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Don't set any fee configuration - should default to 100% arbitrators

    // Create a refund request
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &10000i128,
        &token_client.address,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    // Reject it first so we can escalate
    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Rejected for testing"));

    // Escalate to arbitration
    let fee_pool = 1000i128;
    token_client.transfer(&merchant, &contract_id, &fee_pool);
    
    let case_id = client.escalate_to_arbitration(
        &merchant,
        &refund_id,
        &token_client.address,
        &fee_pool,
    );

    // Cast votes
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &hash);

    // Get initial balances
    let arb1_initial = token_client.balance(&arbitrator1);
    let arb2_initial = token_client.balance(&arbitrator2);
    let arb3_initial = token_client.balance(&arbitrator3);

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check balances - all fees should go to arbitrators
    let arb1_final = token_client.balance(&arbitrator1);
    let arb2_final = token_client.balance(&arbitrator2);
    let arb3_final = token_client.balance(&arbitrator3);

    let expected_per_arbitrator = 1000 / 3;
    assert_eq!(arb1_final - arb1_initial, expected_per_arbitrator);
    assert_eq!(arb2_final - arb2_initial, expected_per_arbitrator);
    assert_eq!(arb3_final - arb3_initial, expected_per_arbitrator);

    // No treasury fees should be accumulated
    assert_eq!(client.get_accumulated_arbitration_fees(), 0);
}

#[test]
fn test_fee_distribution_100_percent_treasury() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, treasury, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Set fee configuration: 0% arbitrators, 100% treasury
    let config = ArbitrationFeeConfig {
        arbitrator_share_bps: 0,     // 0%
        treasury_share_bps: 10000,   // 100%
        treasury_address: treasury.clone(),
        fee_token: token_client.address.clone(),
        fee_per_case: 1000,
    };
    client.set_arbitration_fee_config(&admin, &config);

    // Create a refund request
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &10000i128,
        &token_client.address,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    // Reject it first so we can escalate
    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Rejected for testing"));

    // Escalate to arbitration
    let fee_pool = 1000i128;
    token_client.transfer(&merchant, &contract_id, &fee_pool);
    
    let case_id = client.escalate_to_arbitration(
        &merchant,
        &refund_id,
        &token_client.address,
        &fee_pool,
    );

    // Cast votes
    let hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &hash);

    // Get initial balances
    let arb1_initial = token_client.balance(&arbitrator1);
    let arb2_initial = token_client.balance(&arbitrator2);
    let arb3_initial = token_client.balance(&arbitrator3);
    let treasury_initial = token_client.balance(&treasury);

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check balances - arbitrators should get nothing
    let arb1_final = token_client.balance(&arbitrator1);
    let arb2_final = token_client.balance(&arbitrator2);
    let arb3_final = token_client.balance(&arbitrator3);
    let treasury_final = token_client.balance(&treasury);

    assert_eq!(arb1_final - arb1_initial, 0);
    assert_eq!(arb2_final - arb2_initial, 0);
    assert_eq!(arb3_final - arb3_initial, 0);

    // Treasury should get all 1000
    assert_eq!(treasury_final - treasury_initial, 1000);
    assert_eq!(client.get_accumulated_arbitration_fees(), 1000);
}
