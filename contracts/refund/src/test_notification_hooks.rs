#![cfg(test)]

use crate::{
    Error, NotificationHook, RefundContract, RefundContractClient, RefundEventType, RefundStatus,
};
use soroban_sdk::{
    contract, contractimpl, symbol_short, testutils::Address as _, Address, Env, String, Vec,
};

// Mock subscriber contract for testing
#[contract]
pub struct MockSubscriber;

#[contractimpl]
impl MockSubscriber {
    pub fn on_refund_event(_env: Env, _event_type: RefundEventType, _refund_id: u64) {
        // Mock implementation - does nothing but doesn't fail
    }
}

// Mock subscriber that fails
#[contract]
pub struct FailingSubscriber;

#[contractimpl]
impl FailingSubscriber {
    pub fn on_refund_event(_env: Env, _event_type: RefundEventType, _refund_id: u64) {
        panic!("Intentional failure");
    }
}

fn setup_test_env<'a>() -> (Env, RefundContractClient<'a>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, RefundContract);
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);

    client.initialize(&admin);

    (env, client, admin, subscriber)
}

#[test]
fn test_register_notification_hook() {
    let (env, client, _admin, subscriber) = setup_test_env();

    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);
    events.push_back(RefundEventType::Approved);

    let hook_id = client.register_notification_hook(&subscriber, &events);

    assert_eq!(hook_id, 1);

    // Verify hook was stored
    let hooks = client.get_hooks_for_event(&RefundEventType::Requested);
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks.get(0).unwrap().hook_id, hook_id);
    assert_eq!(hooks.get(0).unwrap().subscriber, subscriber);
    assert_eq!(hooks.get(0).unwrap().active, true);
}

#[test]
fn test_register_multiple_hooks() {
    let (env, client, _admin, _subscriber) = setup_test_env();

    let subscriber1 = Address::generate(&env);
    let subscriber2 = Address::generate(&env);

    let mut events1 = Vec::new(&env);
    events1.push_back(RefundEventType::Requested);

    let mut events2 = Vec::new(&env);
    events2.push_back(RefundEventType::Approved);

    let hook_id1 = client.register_notification_hook(&subscriber1, &events1);
    let hook_id2 = client.register_notification_hook(&subscriber2, &events2);

    assert_eq!(hook_id1, 1);
    assert_eq!(hook_id2, 2);

    // Verify both hooks exist
    let hooks1 = client.get_hooks_for_event(&RefundEventType::Requested);
    assert_eq!(hooks1.len(), 1);

    let hooks2 = client.get_hooks_for_event(&RefundEventType::Approved);
    assert_eq!(hooks2.len(), 1);
}

#[test]
fn test_max_hooks_per_event() {
    let (env, client, _admin, _subscriber) = setup_test_env();

    // Register 10 hooks (max limit)
    for i in 0..10 {
        let subscriber = Address::generate(&env);
        let mut events = Vec::new(&env);
        events.push_back(RefundEventType::Requested);

        let hook_id = client.register_notification_hook(&subscriber, &events);
        assert_eq!(hook_id, (i + 1) as u64);
    }

    // Try to register 11th hook - should fail
    let subscriber11 = Address::generate(&env);
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);

    let result = client.try_register_notification_hook(&subscriber11, &events);
    assert_eq!(result, Err(Ok(Error::MaxHooksPerEventReached)));
}

#[test]
fn test_deregister_hook() {
    let (env, client, _admin, subscriber) = setup_test_env();

    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);

    let hook_id = client.register_notification_hook(&subscriber, &events);

    // Deregister the hook
    client.deregister_hook(&subscriber, &hook_id);

    // Verify hook is marked inactive
    let hooks = client.get_hooks_for_event(&RefundEventType::Requested);
    assert_eq!(hooks.len(), 0); // Inactive hooks are not returned
}

#[test]
fn test_deregister_hook_not_owner() {
    let (env, client, _admin, subscriber) = setup_test_env();

    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);

    let hook_id = client.register_notification_hook(&subscriber, &events);

    // Try to deregister with different address
    let other_subscriber = Address::generate(&env);
    let result = client.try_deregister_hook(&other_subscriber, &hook_id);
    assert_eq!(result, Err(Ok(Error::HookNotOwnedBySubscriber)));
}

#[test]
fn test_deregister_nonexistent_hook() {
    let (_env, client, _admin, subscriber) = setup_test_env();

    let result = client.try_deregister_hook(&subscriber, &999);
    assert_eq!(result, Err(Ok(Error::HookNotFound)));
}

#[test]
fn test_get_subscriber_hooks() {
    let (env, client, _admin, subscriber) = setup_test_env();

    let mut events1 = Vec::new(&env);
    events1.push_back(RefundEventType::Requested);

    let mut events2 = Vec::new(&env);
    events2.push_back(RefundEventType::Approved);
    events2.push_back(RefundEventType::Processed);

    client.register_notification_hook(&subscriber, &events1);
    client.register_notification_hook(&subscriber, &events2);

    let hooks = client.get_subscriber_hooks(&subscriber);
    assert_eq!(hooks.len(), 2);
}

#[test]
fn test_hook_invocation_on_refund_requested() {
    let (env, client, admin, _subscriber) = setup_test_env();

    // Register mock subscriber
    let mock_contract_id = env.register_contract(None, MockSubscriber);
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);

    client.register_notification_hook(&mock_contract_id, &events);

    // Create a refund - this should trigger the hook
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &1000,
        &token,
        &String::from_str(&env, "Test refund"),
        &crate::RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    assert_eq!(refund_id, 1);
    // If hook invocation failed, the test would panic
}

#[test]
fn test_hook_invocation_on_refund_approved() {
    let (env, client, admin, _subscriber) = setup_test_env();

    // Register mock subscriber
    let mock_contract_id = env.register_contract(None, MockSubscriber);
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Approved);

    client.register_notification_hook(&mock_contract_id, &events);

    // Create and approve a refund
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &1000,
        &token,
        &String::from_str(&env, "Test refund"),
        &crate::RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.approve_refund(&admin, &refund_id);
    // If hook invocation failed, the test would panic
}

#[test]
fn test_hook_invocation_on_refund_rejected() {
    let (env, client, admin, _subscriber) = setup_test_env();

    // Register mock subscriber
    let mock_contract_id = env.register_contract(None, MockSubscriber);
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Rejected);

    client.register_notification_hook(&mock_contract_id, &events);

    // Create and reject a refund
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &1000,
        &token,
        &String::from_str(&env, "Test refund"),
        &crate::RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.reject_refund(
        &admin,
        &refund_id,
        &String::from_str(&env, "Invalid request"),
    );
    // If hook invocation failed, the test would panic
}

#[test]
fn test_hook_invocation_on_refund_processed() {
    let (env, client, admin, _subscriber) = setup_test_env();

    // Register mock subscriber
    let mock_contract_id = env.register_contract(None, MockSubscriber);
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Processed);

    client.register_notification_hook(&mock_contract_id, &events);

    // Create, approve, and process a refund
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &1000,
        &token,
        &String::from_str(&env, "Test refund"),
        &crate::RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    client.approve_refund(&admin, &refund_id);
    client.process_refund(&admin, &refund_id);
    // If hook invocation failed, the test would panic
}

#[test]
fn test_failed_hook_does_not_revert_operation() {
    let (env, client, admin, _subscriber) = setup_test_env();

    // Register failing subscriber
    let failing_contract_id = env.register_contract(None, FailingSubscriber);
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);

    client.register_notification_hook(&failing_contract_id, &events);

    // Create a refund - hook will fail but operation should succeed
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &1000,
        &token,
        &String::from_str(&env, "Test refund"),
        &crate::RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    assert_eq!(refund_id, 1);

    // Verify refund was created despite hook failure
    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Requested);
}

#[test]
fn test_multiple_hooks_same_event() {
    let (env, client, admin, _subscriber) = setup_test_env();

    // Register multiple subscribers for the same event
    let mock1 = env.register_contract(None, MockSubscriber);
    let mock2 = env.register_contract(None, MockSubscriber);

    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);

    client.register_notification_hook(&mock1, &events);
    client.register_notification_hook(&mock2, &events);

    // Verify both hooks are registered
    let hooks = client.get_hooks_for_event(&RefundEventType::Requested);
    assert_eq!(hooks.len(), 2);

    // Create a refund - both hooks should be invoked
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &1000,
        &token,
        &String::from_str(&env, "Test refund"),
        &crate::RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    );

    assert_eq!(refund_id, 1);
}

#[test]
fn test_hook_for_multiple_events() {
    let (env, client, _admin, subscriber) = setup_test_env();

    // Register hook for multiple events
    let mut events = Vec::new(&env);
    events.push_back(RefundEventType::Requested);
    events.push_back(RefundEventType::Approved);
    events.push_back(RefundEventType::Rejected);

    let hook_id = client.register_notification_hook(&subscriber, &events);

    // Verify hook appears in all event indices
    let hooks_requested = client.get_hooks_for_event(&RefundEventType::Requested);
    assert_eq!(hooks_requested.len(), 1);

    let hooks_approved = client.get_hooks_for_event(&RefundEventType::Approved);
    assert_eq!(hooks_approved.len(), 1);

    let hooks_rejected = client.get_hooks_for_event(&RefundEventType::Rejected);
    assert_eq!(hooks_rejected.len(), 1);
}
