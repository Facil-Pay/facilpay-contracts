#![cfg(test)]

use super::*;
use soroban_sdk::{ testutils::Address as _, testutils::Events, testutils::Ledger, Address, Env, String };

#[test]
fn test_set_refund_policy_successfully() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    let refund_window = 14u64 * 24u64 * 60u64 * 60u64; // 14 days
    let max_refund_percentage = 5000; // 50%
    let requires_admin_approval = false;
    let auto_approve_below = 1000i128;

    client.set_refund_policy(
        &merchant,
        &refund_window,
        &max_refund_percentage,
        &requires_admin_approval,
        &auto_approve_below
    );

    let policy = client.get_refund_policy(&merchant);
    assert!(policy.is_some());
    let policy = policy.unwrap();
    assert_eq!(policy.merchant, merchant);
    assert_eq!(policy.refund_window, refund_window);
    assert_eq!(policy.max_refund_percentage, max_refund_percentage);
    assert_eq!(policy.requires_admin_approval, requires_admin_approval);
    assert_eq!(policy.auto_approve_below, auto_approve_below);
    assert!(policy.active);
}

#[test]
#[should_panic]
fn test_set_refund_policy_with_invalid_percentage_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    let refund_window = 30u64 * 24u64 * 60u64 * 60u64;
    let max_refund_percentage = 15000u32; // Invalid: > 100%
    let requires_admin_approval = true;
    let auto_approve_below = 0i128;

    client.set_refund_policy(
        &merchant,
        &refund_window,
        &max_refund_percentage,
        &requires_admin_approval,
        &auto_approve_below
    );
}

#[test]
fn test_deactivate_refund_policy_successfully() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // First set a policy
    client.set_refund_policy(&merchant, &(30u64 * 24 * 60 * 60), &10000, &true, &0i128);

    // Then deactivate it
    client.deactivate_refund_policy(&merchant);

    let policy = client.get_refund_policy(&merchant);
    assert!(policy.is_some());
    assert!(!policy.unwrap().active);
}

#[test]
#[should_panic]
fn test_deactivate_nonexistent_policy_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    client.deactivate_refund_policy(&merchant);
}

#[test]
fn test_admin_override_policy_successfully() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // First create a refund
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp()
    );

    // Then admin overrides policy
    let reason = String::from_str(&env, "Manual override for special case");
    client.admin_override_policy(&admin, &refund_id, &reason);

    // Check that the override event was emitted
    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
#[should_panic]
fn test_admin_override_policy_by_non_admin_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // First create a refund
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Test"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp()
    );

    // Try to override with unauthorized user
    let reason = String::from_str(&env, "Unauthorized override");
    client.admin_override_policy(&unauthorized_user, &refund_id, &reason);
}

#[test]
fn test_refund_window_expired_should_fail() {
    let env = Env::default();
    env.ledger().set_timestamp(7 * 24 * 60 * 60); // Start at 7 days
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // Set a policy with 1 day refund window
    client.set_refund_policy(
        &merchant,
        &(24u64 * 60u64 * 60u64), // 1 day
        &10000u32,
        &true,
        &0i128
    );

    // Simulate payment created 2 days ago
    let payment_created_at = env.ledger().timestamp() - 2 * 24 * 60 * 60;

    // Try to request refund outside window
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Too late"),
        &RefundReasonCode::Other,
        &payment_created_at
    );

    assert_eq!(result, Err(Ok(Error::RefundWindowExpired)));
}

#[test]
fn test_refund_percentage_exceeds_policy_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // Set a policy with 50% max refund
    client.set_refund_policy(
        &merchant,
        &(30u64 * 24u64 * 60u64 * 60u64),
        &5000u32, // 50%
        &true,
        &0i128
    );

    // Try to request 75% refund
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &750i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Too much"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp()
    );

    assert_eq!(result, Err(Ok(Error::RefundExceedsPolicy)));
}

#[test]
fn test_auto_approve_below_threshold() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // Set policy with auto-approve for amounts <= 500
    client.set_refund_policy(
        &merchant,
        &(30u64 * 24u64 * 60u64 * 60u64),
        &10000u32,
        &false, // No admin approval required
        &500i128 // Auto-approve below 500
    );

    // Request refund for 300 (should be auto-approved)
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &300i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Small refund"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp()
    );

    // Check that AutoApproved event was emitted (before next contract call clears events)
    let events = env.events().all();
    assert!(events.len() > 0);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

#[test]
fn test_refund_with_inactive_policy_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // Set a policy
    client.set_refund_policy(&merchant, &(30u64 * 24u64 * 60u64 * 60u64), &10000u32, &true, &0i128);

    // Deactivate it
    client.deactivate_refund_policy(&merchant);

    // Try to request refund
    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Inactive policy"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp()
    );

    assert_eq!(result, Err(Ok(Error::PolicyInactive)));
}

#[test]
fn test_refund_without_merchant_policy_uses_default() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // Don't set any merchant policy - should use default

    // Request refund (should work with default policy)
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &1000i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Default policy"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp()
    );

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Requested); // Default requires admin approval
}

#[test]
fn test_refund_policy_set_event_emitted() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    let refund_window = 7u64 * 24 * 60 * 60; // 7 days

    client.set_refund_policy(&merchant, &refund_window, &10000, &true, &0i128);

    // Check that RefundPolicySet event was emitted
    let events = env.events().all();
    assert!(events.len() > 0);
}

#[test]
fn test_refund_policy_deactivated_event_emitted() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    env.mock_all_auths();
    // Set and then deactivate policy
    client.set_refund_policy(&merchant, &(30u64 * 24 * 60 * 60), &10000, &true, &0i128);

    client.deactivate_refund_policy(&merchant);

    // Check that RefundPolicyDeactivated event was emitted
    let events = env.events().all();
    assert!(events.len() > 0);
}

// ── Issue #93: Default refund policy tests ────────────────────────────────

#[test]
fn test_set_default_refund_policy_by_admin_succeeds() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);
    env.mock_all_auths();

    let policy = RefundPolicy {
        merchant: admin.clone(),
        refund_window: 7 * 24 * 60 * 60, // 7 days
        max_refund_percentage: 5000,       // 50 %
        requires_admin_approval: true,
        auto_approve_below: 0,
        active: true,
    };

    client.set_default_refund_policy(&admin, &policy);

    let stored = client.get_default_refund_policy();
    assert!(stored.is_some());
    let stored = stored.unwrap();
    assert_eq!(stored.refund_window, 7 * 24 * 60 * 60);
    assert_eq!(stored.max_refund_percentage, 5000);
}

#[test]
#[should_panic]
fn test_set_default_refund_policy_by_non_admin_fails() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.initialize(&admin);
    env.mock_all_auths();

    let policy = RefundPolicy {
        merchant: attacker.clone(),
        refund_window: 7 * 24 * 60 * 60,
        max_refund_percentage: 10000,
        requires_admin_approval: true,
        auto_approve_below: 0,
        active: true,
    };

    // attacker != stored admin → should panic with Unauthorized
    client.set_default_refund_policy(&attacker, &policy);
}

#[test]
fn test_remove_default_refund_policy_by_admin_succeeds() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);
    env.mock_all_auths();

    // A default policy is set by initialize(); remove it
    client.remove_default_refund_policy(&admin);

    let stored = client.get_default_refund_policy();
    assert!(stored.is_none());
}

#[test]
#[should_panic]
fn test_remove_default_refund_policy_by_non_admin_fails() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.initialize(&admin);
    env.mock_all_auths();

    client.remove_default_refund_policy(&attacker);
}

#[test]
fn test_request_refund_uses_global_default_when_no_merchant_policy() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    // Replace the default with a custom global policy (no admin approval, auto-approve <= 200)
    let default_policy = RefundPolicy {
        merchant: admin.clone(),
        refund_window: 30 * 24 * 60 * 60,
        max_refund_percentage: 10000,
        requires_admin_approval: false,
        auto_approve_below: 200,
        active: true,
    };
    client.set_default_refund_policy(&admin, &default_policy);

    // No merchant-specific policy set; amount (100) <= auto_approve_below (200) → auto-approved
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &100i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Uses global default"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp(),
    );

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

#[test]
fn test_request_refund_returns_policy_not_found_when_no_policy_at_all() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    // Remove the default policy that initialize() set, and set NO merchant policy
    client.remove_default_refund_policy(&admin);

    let result = client.try_request_refund(
        &merchant,
        &1u64,
        &customer,
        &500i128,
        &1000i128,
        &token,
        &String::from_str(&env, "No policy at all"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp(),
    );

    assert_eq!(result, Err(Ok(Error::PolicyNotFound)));
}

#[test]
fn test_default_policy_change_does_not_affect_pending_refunds() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);
    env.mock_all_auths();

    // Submit a refund using the current default (set by initialize())
    let refund_id = client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &500i128,
        &1000i128,
        &token,
        &String::from_str(&env, "Pending refund"),
        &RefundReasonCode::Other,
        &env.ledger().timestamp(),
    );

    // Verify it is in Requested state (default requires admin approval)
    let refund_before = client.get_refund(&refund_id);
    assert_eq!(refund_before.status, RefundStatus::Requested);

    // Now admin changes the global default policy
    let new_default = RefundPolicy {
        merchant: admin.clone(),
        refund_window: 7 * 24 * 60 * 60,
        max_refund_percentage: 5000,
        requires_admin_approval: false,
        auto_approve_below: 1000,
        active: true,
    };
    client.set_default_refund_policy(&admin, &new_default);

    // The already-stored refund must NOT be retroactively changed
    let refund_after = client.get_refund(&refund_id);
    assert_eq!(refund_after.status, RefundStatus::Requested);
}
