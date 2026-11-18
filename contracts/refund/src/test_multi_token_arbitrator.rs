#![cfg(test)]

use crate::{
    ArbitrationKey, ArbitrationStatus, CoreError, DataKey, Error, ExtError, RefundContract,
    RefundContractClient, RefundReasonCode, RefundStatus, SupportedRefundToken, TokenKey,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

fn setup_env() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    (env, admin, merchant, customer, token)
}

const ARBITRATION_FEE: i128 = 100;

/// Registers a Stellar asset to pay the arbitration fee and funds `payer` with it.
fn fee_token(env: &Env, admin: &Address, payer: &Address) -> Address {
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(env, &token).mint(payer, &10_000);
    token
}

fn deploy_contract<'a>(env: &'a Env, admin: &Address) -> RefundContractClient<'a> {
    let contract_id = env.register_contract(None, RefundContract);
    let client = RefundContractClient::new(env, &contract_id);
    client.initialize(admin);
    client
}

// ──────────────────────────────────────────────────────────────────────────────
// Multi-token registry tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_multi_token_registry_empty_allows_all() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);

    // When registry is empty (no tokens added), all tokens should be allowed
    // This is the "registry not configured yet" bypass
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Should succeed because registry is empty
    assert!(result.is_ok());
}

#[test]
fn test_multi_token_registry_allows_supported_token() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);

    // Add token to supported list
    client.register_refund_token(&admin, &token);

    // Request refund with supported token should succeed
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    assert!(result.is_ok());
}

#[test]
fn test_multi_token_registry_rejects_unsupported_token() {
    let (env, admin, merchant, customer, _) = setup_env();
    let client = deploy_contract(&env, &admin);

    let supported_token = Address::generate(&env);
    let unsupported_token = Address::generate(&env);

    // Add only one token to registry
    client.register_refund_token(&admin, &supported_token);

    // Try to request refund with unsupported token
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &unsupported_token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Should fail with UnsupportedRefundToken error
    assert!(result.is_err());
    match result {
        Err(Ok(Error::Ext(ExtError::UnsupportedRefundToken))) => (),
        _ => panic!("Expected UnsupportedRefundToken error"),
    }
}

#[test]
fn test_multi_token_registry_inactive_token_rejected() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);

    // Add token but mark it as inactive
    client.register_refund_token(&admin, &token);
    client.deregister_refund_token(&admin, &token);

    // Try to request refund with inactive token
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    // Should fail because token is inactive
    assert!(result.is_err());
    match result {
        Err(Ok(Error::Ext(ExtError::UnsupportedRefundToken))) => (),
        _ => panic!("Expected UnsupportedRefundToken error"),
    }
}

#[test]
fn test_multi_token_registry_reactivate_token() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);

    // Add token as inactive
    client.register_refund_token(&admin, &token);
    client.deregister_refund_token(&admin, &token);

    // Verify it's rejected
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );
    assert!(result.is_err());

    // Reactivate the token
    client.register_refund_token(&admin, &token);

    // Now it should work
    let result = client.try_request_refund(
        &merchant,
        &2u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );
    assert!(result.is_ok());
}

#[test]
fn test_multi_token_registry_multiple_tokens() {
    let (env, admin, merchant, customer, _) = setup_env();
    let client = deploy_contract(&env, &admin);

    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);
    let token3 = Address::generate(&env);

    // Add multiple tokens
    client.register_refund_token(&admin, &token1);
    client.register_refund_token(&admin, &token2);
    client.register_refund_token(&admin, &token3); // Inactive
    client.deregister_refund_token(&admin, &token3);

    // Test with token1 (active) - should work
    let result1 = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token1,
        &String::from_str(&env, "Test refund 1"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );
    assert!(result1.is_ok());

    // Test with token2 (active) - should work
    let result2 = client.try_request_refund(
        &merchant,
        &2u64,
        &customer,
        &1000i128,
        &1000i128,
        &token2,
        &String::from_str(&env, "Test refund 2"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );
    assert!(result2.is_ok());

    // Test with token3 (inactive) - should fail
    let result3 = client.try_request_refund(
        &merchant,
        &3u64,
        &customer,
        &1000i128,
        &1000i128,
        &token3,
        &String::from_str(&env, "Test refund 3"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );
    assert!(result3.is_err());
}

// ──────────────────────────────────────────────────────────────────────────────
// Arbitrator availability tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_arbitrator_availability_all_available() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);
    let fee_token = fee_token(&env, &admin, &customer);

    // Register 3 arbitrators
    let arb1 = Address::generate(&env);
    let arb2 = Address::generate(&env);
    let arb3 = Address::generate(&env);

    client.register_arbitrator(&admin, &arb1);
    client.register_arbitrator(&admin, &arb2);
    client.register_arbitrator(&admin, &arb3);

    // All should be available by default
    assert_eq!(client.is_arbitrator_available(&arb1), true);
    assert_eq!(client.is_arbitrator_available(&arb2), true);
    assert_eq!(client.is_arbitrator_available(&arb3), true);

    // Create a refund and escalate to arbitration
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Denied"));
    
    client.file_appeal(
        &customer,
        &refund_id,
        &String::from_str(&env, "I disagree"),
    );

    // Escalate to arbitration - should succeed with 3 available arbitrators
    let result = client.try_escalate_to_arbitration(&customer, &refund_id, &fee_token, &ARBITRATION_FEE);
    assert!(result.is_ok());
}

#[test]
fn test_arbitrator_availability_one_unavailable() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);
    let fee_token = fee_token(&env, &admin, &customer);

    // Register 3 arbitrators
    let arb1 = Address::generate(&env);
    let arb2 = Address::generate(&env);
    let arb3 = Address::generate(&env);

    client.register_arbitrator(&admin, &arb1);
    client.register_arbitrator(&admin, &arb2);
    client.register_arbitrator(&admin, &arb3);

    // Set one as unavailable
    client.set_arbitrator_availability(&arb2, &false);

    assert_eq!(client.is_arbitrator_available(&arb1), true);
    assert_eq!(client.is_arbitrator_available(&arb2), false);
    assert_eq!(client.is_arbitrator_available(&arb3), true);

    // Should still be able to create arbitration case with 2 available
    // (if system supports it)
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Denied"));
    
    client.file_appeal(
        &customer,
        &refund_id,
        &String::from_str(&env, "I disagree"),
    );

    // This might fail if minimum 3 arbitrators required, or succeed with 2
    let _ = client.try_escalate_to_arbitration(&customer, &refund_id, &fee_token, &ARBITRATION_FEE);
}

#[test]
fn test_arbitrator_availability_insufficient_available() {
    let (env, admin, merchant, customer, token) = setup_env();
    let client = deploy_contract(&env, &admin);
    let fee_token = fee_token(&env, &admin, &customer);

    // Register 3 arbitrators
    let arb1 = Address::generate(&env);
    let arb2 = Address::generate(&env);
    let arb3 = Address::generate(&env);

    client.register_arbitrator(&admin, &arb1);
    client.register_arbitrator(&admin, &arb2);
    client.register_arbitrator(&admin, &arb3);

    // Set two as unavailable, leaving only 1
    client.set_arbitrator_availability(&arb2, &false);
    client.set_arbitrator_availability(&arb3, &false);

    assert_eq!(client.is_arbitrator_available(&arb1), true);
    assert_eq!(client.is_arbitrator_available(&arb2), false);
    assert_eq!(client.is_arbitrator_available(&arb3), false);

    // Try to create arbitration case with insufficient arbitrators
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Denied"));
    
    client.file_appeal(
        &customer,
        &refund_id,
        &String::from_str(&env, "I disagree"),
    );

    // Should fail with QuorumNotReached error
    let result = client.try_escalate_to_arbitration(&customer, &refund_id, &fee_token, &ARBITRATION_FEE);
    assert!(result.is_err());
    match result {
        Err(Ok(Error::Core(CoreError::QuorumNotReached))) => (),
        _ => panic!("Expected QuorumNotReached error"),
    }
}

#[test]
fn test_arbitrator_availability_toggle() {
    let (env, admin, _, _, _) = setup_env();
    let client = deploy_contract(&env, &admin);

    let arb = Address::generate(&env);
    client.register_arbitrator(&admin, &arb);

    // Available by default
    assert_eq!(client.is_arbitrator_available(&arb), true);

    // Toggle to unavailable
    client.set_arbitrator_availability(&arb, &false);
    assert_eq!(client.is_arbitrator_available(&arb), false);

    // Toggle back to available
    client.set_arbitrator_availability(&arb, &true);
    assert_eq!(client.is_arbitrator_available(&arb), true);
}

#[test]
fn test_arbitrator_availability_admin_override() {
    let (env, admin, _, _, _) = setup_env();
    let client = deploy_contract(&env, &admin);

    let arb = Address::generate(&env);
    client.register_arbitrator(&admin, &arb);

    // Admin sets arbitrator as unavailable (e.g., unresponsive arbitrator)
    client.admin_set_arbiter_availability(&admin, &arb, &false);
    assert_eq!(client.is_arbitrator_available(&arb), false);

    // Admin can restore availability
    client.admin_set_arbiter_availability(&admin, &arb, &true);
    assert_eq!(client.is_arbitrator_available(&arb), true);
}

#[test]
fn test_arbitrator_availability_non_registered_returns_false() {
    let (env, admin, _, _, _) = setup_env();
    let client = deploy_contract(&env, &admin);

    let non_registered_arb = Address::generate(&env);

    // Non-registered arbitrator should return false
    assert_eq!(client.is_arbitrator_available(&non_registered_arb), false);
}

// ──────────────────────────────────────────────────────────────────────────────
// Combined multi-token and arbitrator availability test
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_combined_multi_token_and_arbitrator_flow() {
    let (env, admin, merchant, customer, _) = setup_env();
    let client = deploy_contract(&env, &admin);
    let fee_token = fee_token(&env, &admin, &customer);

    // Setup multi-token registry
    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);
    client.register_refund_token(&admin, &token1);
    client.register_refund_token(&admin, &token2);

    // Setup arbitrators
    let arb1 = Address::generate(&env);
    let arb2 = Address::generate(&env);
    let arb3 = Address::generate(&env);
    client.register_arbitrator(&admin, &arb1);
    client.register_arbitrator(&admin, &arb2);
    client.register_arbitrator(&admin, &arb3);

    // Request refund with token1 (should work)
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token1,
        &String::from_str(&env, "Product defect"),
        &RefundReasonCode::ProductDefect,
        &env.ledger().timestamp(),
    );

    // Reject and escalate
    client.reject_refund(&admin, &refund_id, &String::from_str(&env, "Denied"));
    
    client.file_appeal(
        &customer,
        &refund_id,
        &String::from_str(&env, "I disagree"),
    );

    // All arbitrators available - should succeed
    let result = client.try_escalate_to_arbitration(&customer, &refund_id, &fee_token, &ARBITRATION_FEE);
    assert!(result.is_ok());

    // Now set one arbitrator as unavailable for future cases
    client.set_arbitrator_availability(&arb3, &false);
    assert_eq!(client.is_arbitrator_available(&arb3), false);

    // Request another refund with token2
    let refund_id2 = client.request_refund(
        &merchant,
        &2u64,
        &customer,
        &500i128,
        &500i128,
        &token2,
        &String::from_str(&env, "Non delivery"),
        &RefundReasonCode::NonDelivery,
        &env.ledger().timestamp(),
    );

    // Approve this one to show the flow works end-to-end
    client.approve_refund(&admin, &refund_id2);
    
    let refund = client.get_refund(&refund_id2);
    assert_eq!(refund.status, RefundStatus::Approved);
}
