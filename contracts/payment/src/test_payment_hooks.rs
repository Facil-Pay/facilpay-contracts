#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
struct MockHookSubscriber;

#[contractimpl]
impl MockHookSubscriber {
    pub fn ping(_env: Env) {}

    pub fn on_payment_event(_env: Env, _event_type: PaymentEventType, _payment_id: u64) {}
}

fn setup() -> (Env, PaymentContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client)
}

fn mock_subscriber(env: &Env) -> Address {
    env.register(MockHookSubscriber, ())
}

#[test]
fn test_register_hooks_for_two_event_types_filters_correctly() {
    let (env, client) = setup();
    let sub_created = mock_subscriber(&env);
    let sub_completed = mock_subscriber(&env);

    let created_events = Vec::from_array(&env, [PaymentEventType::Created]);
    let completed_events = Vec::from_array(&env, [PaymentEventType::Completed]);

    let hook_id_created = client.register_payment_hook(&sub_created, &created_events);
    let hook_id_completed = client.register_payment_hook(&sub_completed, &completed_events);

    let created_hooks = client.get_payment_hooks(&PaymentEventType::Created);
    assert_eq!(created_hooks.len(), 1);
    assert_eq!(created_hooks.get(0).unwrap().hook_id, hook_id_created);
    assert_eq!(created_hooks.get(0).unwrap().subscriber, sub_created);

    let completed_hooks = client.get_payment_hooks(&PaymentEventType::Completed);
    assert_eq!(completed_hooks.len(), 1);
    assert_eq!(completed_hooks.get(0).unwrap().hook_id, hook_id_completed);
    assert_eq!(completed_hooks.get(0).unwrap().subscriber, sub_completed);

    // No cross-contamination between event types.
    assert!(client
        .get_payment_hooks(&PaymentEventType::Cancelled)
        .is_empty());
}

#[test]
fn test_deregister_removes_hook_from_listing() {
    let (env, client) = setup();
    let subscriber = mock_subscriber(&env);
    let events = Vec::from_array(&env, [PaymentEventType::Refunded]);

    let hook_id = client.register_payment_hook(&subscriber, &events);
    assert_eq!(
        client.get_payment_hooks(&PaymentEventType::Refunded).len(),
        1
    );

    client.deregister_payment_hook(&subscriber, &hook_id);

    assert!(client
        .get_payment_hooks(&PaymentEventType::Refunded)
        .is_empty());
}

#[test]
fn test_deregister_frees_per_event_cap() {
    let (env, client) = setup();
    let subscriber = mock_subscriber(&env);
    let events = Vec::from_array(&env, [PaymentEventType::Cancelled]);

    let mut hook_ids: Vec<u64> = Vec::new(&env);
    for _ in 0..PaymentContract::MAX_PAYMENT_HOOKS_PER_EVENT {
        hook_ids.push_back(client.register_payment_hook(&subscriber, &events));
    }
    assert_eq!(
        client.get_payment_hooks(&PaymentEventType::Cancelled).len(),
        PaymentContract::MAX_PAYMENT_HOOKS_PER_EVENT
    );

    // The cap is now reached: one more registration for this event type must fail.
    let result = client.try_register_payment_hook(&subscriber, &events);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::MaxHooksPerEventReached)))
    );

    // Deregister the most-recently-registered hook (LIFO) to free the cap without
    // tripping the non-compacting index's skip-over quirk for earlier slots.
    let last_hook_id = hook_ids.get(hook_ids.len() - 1).unwrap();
    client.deregister_payment_hook(&subscriber, &last_hook_id);

    assert_eq!(
        client.get_payment_hooks(&PaymentEventType::Cancelled).len(),
        PaymentContract::MAX_PAYMENT_HOOKS_PER_EVENT - 1
    );

    // The cap is freed: registration for this event type succeeds again.
    let new_hook_id = client.register_payment_hook(&subscriber, &events);
    assert!(!hook_ids.contains(&new_hook_id));
}

#[test]
fn test_deregister_requires_hook_owner() {
    let (env, client) = setup();
    let owner = mock_subscriber(&env);
    let impostor = mock_subscriber(&env);
    let events = Vec::from_array(&env, [PaymentEventType::Created]);

    let hook_id = client.register_payment_hook(&owner, &events);

    let result = client.try_deregister_payment_hook(&impostor, &hook_id);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::HookNotOwnedBySubscriber)))
    );

    // The hook is untouched by the failed attempt.
    assert_eq!(
        client.get_payment_hooks(&PaymentEventType::Created).len(),
        1
    );
}

#[test]
fn test_deregister_unknown_hook_fails() {
    let (env, client) = setup();
    let subscriber = mock_subscriber(&env);

    let result = client.try_deregister_payment_hook(&subscriber, &999u64);
    assert_eq!(result, Err(Ok(Error::Payment(PaymentError::HookNotFound))));
}

#[test]
fn test_register_payment_hook_requires_events() {
    let (env, client) = setup();
    let subscriber = mock_subscriber(&env);
    let no_events: Vec<PaymentEventType> = Vec::new(&env);

    let result = client.try_register_payment_hook(&subscriber, &no_events);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::NoEventsSpecified)))
    );
}

#[test]
fn test_register_payment_hook_requires_reachable_subscriber() {
    let (env, client) = setup();
    // A plain account address exposes no `ping()` entry point, so the
    // reachability check performed by `validate_payment_hook_subscriber`
    // must reject it.
    let not_a_contract = Address::generate(&env);
    let events = Vec::from_array(&env, [PaymentEventType::Created]);

    let result = client.try_register_payment_hook(&not_a_contract, &events);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::InvalidHookAddress)))
    );
}
