#![cfg(test)]

//! Tests for scheduled (future-dated) refund policy changes:
//! `schedule_refund_policy_change`, `cancel_scheduled_refund_policy`,
//! `get_scheduled_refund_policy` and `activate_scheduled_refund_policy`.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, Vec,
};

fn setup(env: &Env) -> (RefundContractClient<'_>, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let merchant = Address::generate(env);
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    client.initialize(&admin);
    env.ledger().set_timestamp(1_000);
    (client, merchant)
}

fn initial_tiers(env: &Env) -> Vec<RefundTier> {
    Vec::from_array(
        env,
        [RefundTier {
            days_from_purchase: 30,
            max_refund_bps: 10_000,
        }],
    )
}

fn scheduled_tiers(env: &Env) -> Vec<RefundTier> {
    Vec::from_array(
        env,
        [
            RefundTier {
                days_from_purchase: 14,
                max_refund_bps: 5_000,
            },
            RefundTier {
                days_from_purchase: 7,
                max_refund_bps: 10_000,
            },
        ],
    )
}

#[test]
fn test_schedule_stores_pending_change() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    assert!(client.get_scheduled_refund_policy(&merchant).is_none());

    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);

    let scheduled = client.get_scheduled_refund_policy(&merchant).unwrap();
    assert_eq!(scheduled.merchant, merchant);
    assert_eq!(scheduled.effective_at, 5_000);
    assert_eq!(scheduled.scheduled_at, 1_000);
    assert_eq!(scheduled.tiers.len(), 2);
    // Tiers are stored sorted by days_from_purchase.
    assert_eq!(scheduled.tiers.get(0).unwrap().days_from_purchase, 7);
    assert_eq!(scheduled.tiers.get(1).unwrap().days_from_purchase, 14);
}

#[test]
fn test_schedule_rejects_effective_time_not_in_future() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    let now = client.try_schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &1_000);
    assert_eq!(now, Err(Ok(Error::Ext(ExtError::InvalidEffectiveTime))));

    let past = client.try_schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &500);
    assert_eq!(past, Err(Ok(Error::Ext(ExtError::InvalidEffectiveTime))));

    assert!(client.get_scheduled_refund_policy(&merchant).is_none());
}

#[test]
fn test_schedule_rejects_invalid_bps() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    let tiers = Vec::from_array(
        &env,
        [RefundTier {
            days_from_purchase: 7,
            max_refund_bps: 10_001,
        }],
    );
    let result = client.try_schedule_refund_policy_change(&merchant, &tiers, &5_000);
    assert_eq!(result, Err(Ok(Error::Core(CoreError::RefundExceedsPolicy))));
    assert!(client.get_scheduled_refund_policy(&merchant).is_none());
}

#[test]
fn test_activation_fails_before_effective_time_and_succeeds_after() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    client.set_refund_policy(&merchant, &initial_tiers(&env));
    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);

    // Before effective_at the change is not due.
    let early = client.try_activate_scheduled_refund_policy(&merchant);
    assert_eq!(early, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotDue))));

    env.ledger().set_timestamp(4_999);
    let still_early = client.try_activate_scheduled_refund_policy(&merchant);
    assert_eq!(still_early, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotDue))));

    // The pending change is untouched and the old policy still applies.
    assert!(client.get_scheduled_refund_policy(&merchant).is_some());
    let current = client.get_refund_policy(&merchant).unwrap();
    assert_eq!(current.tiers.len(), 1);

    // At effective_at activation succeeds.
    env.ledger().set_timestamp(5_000);
    let version = client.activate_scheduled_refund_policy(&merchant);
    assert_eq!(version, 2);

    assert!(client.get_scheduled_refund_policy(&merchant).is_none());
    let current = client.get_refund_policy(&merchant).unwrap();
    assert_eq!(current.tiers.len(), 2);
    assert_eq!(current.updated_at, 5_000);
    assert!(current.active);

    // Nothing is left to activate.
    let again = client.try_activate_scheduled_refund_policy(&merchant);
    assert_eq!(again, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotFound))));
}

#[test]
fn test_activation_without_schedule_fails() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    let result = client.try_activate_scheduled_refund_policy(&merchant);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotFound))));
}

#[test]
fn test_cancelled_schedule_cannot_activate() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    client.set_refund_policy(&merchant, &initial_tiers(&env));
    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);
    client.cancel_scheduled_refund_policy(&merchant);

    assert!(client.get_scheduled_refund_policy(&merchant).is_none());

    env.ledger().set_timestamp(6_000);
    let result = client.try_activate_scheduled_refund_policy(&merchant);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotFound))));

    // The original policy is still in force and no new version was written.
    let current = client.get_refund_policy(&merchant).unwrap();
    assert_eq!(current.tiers.len(), 1);
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 1);
}

#[test]
fn test_cancel_without_schedule_fails() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    let result = client.try_cancel_scheduled_refund_policy(&merchant);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotFound))));
}

#[test]
fn test_cancel_after_effective_time_fails_and_applies_change() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    client.set_refund_policy(&merchant, &initial_tiers(&env));
    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);

    env.ledger().set_timestamp(5_000);
    let result = client.try_cancel_scheduled_refund_policy(&merchant);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::ScheduledPolicyNotFound))));

    // A due change cannot be cancelled; it is already the policy in effect.
    let current = client.get_refund_policy(&merchant).unwrap();
    assert_eq!(current.tiers.len(), 2);
}

#[test]
fn test_rescheduling_replaces_pending_change() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);
    client.schedule_refund_policy_change(&merchant, &initial_tiers(&env), &8_000);

    let scheduled = client.get_scheduled_refund_policy(&merchant).unwrap();
    assert_eq!(scheduled.effective_at, 8_000);
    assert_eq!(scheduled.tiers.len(), 1);
}

#[test]
fn test_activation_increments_version_and_history() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    client.set_refund_policy(&merchant, &initial_tiers(&env));
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 1);

    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);
    // Scheduling alone does not add a version.
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 1);

    // Activate lazily, after effective_at has passed.
    env.ledger().set_timestamp(7_000);
    let version = client.activate_scheduled_refund_policy(&merchant);
    assert_eq!(version, 2);

    let history = client.get_refund_policy_history(&merchant);
    assert_eq!(history.len(), 2);

    let v1 = history.get(0).unwrap();
    assert_eq!(v1.version, 1);
    assert_eq!(v1.created_at, 1_000);
    assert_eq!(v1.policy.tiers.len(), 1);

    let v2 = history.get(1).unwrap();
    assert_eq!(v2.version, 2);
    // The version is dated at effective_at, not at the activation call.
    assert_eq!(v2.created_at, 5_000);
    assert_eq!(v2.created_by, merchant);
    assert_eq!(v2.policy.tiers.len(), 2);
    // The existing policy's creation time is kept.
    assert_eq!(v2.policy.created_at, 1_000);

    let stored = client.get_refund_policy_version(&merchant, &2).unwrap();
    assert_eq!(stored.created_at, 5_000);

    // Point-in-time lookups reflect when the change really applied.
    assert_eq!(client.get_refund_policy_at_time(&merchant, &4_999).unwrap().version, 1);
    assert_eq!(client.get_refund_policy_at_time(&merchant, &5_000).unwrap().version, 2);

    // A second scheduled change bumps the version again.
    client.schedule_refund_policy_change(&merchant, &initial_tiers(&env), &9_000);
    env.ledger().set_timestamp(9_000);
    assert_eq!(client.activate_scheduled_refund_policy(&merchant), 3);
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 3);
}

#[test]
fn test_activation_without_existing_policy_creates_version_one() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    assert!(client.get_refund_policy(&merchant).is_none());
    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);

    env.ledger().set_timestamp(5_000);
    assert_eq!(client.activate_scheduled_refund_policy(&merchant), 1);

    let policy = client.get_refund_policy(&merchant).unwrap();
    assert_eq!(policy.created_at, 5_000);
    assert_eq!(policy.default_window_seconds, 30 * 24 * 60 * 60);
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 1);
}

#[test]
fn test_due_change_visible_before_activation() {
    let env = Env::default();
    let (client, merchant) = setup(&env);

    client.set_refund_policy(&merchant, &initial_tiers(&env));
    client.schedule_refund_policy_change(&merchant, &scheduled_tiers(&env), &5_000);

    env.ledger().set_timestamp(5_000);
    // get_refund_policy reflects the due change even before it is written.
    assert_eq!(client.get_refund_policy(&merchant).unwrap().tiers.len(), 2);
    // ...but the history is only updated once it is activated.
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 1);

    client.activate_scheduled_refund_policy(&merchant);
    assert_eq!(client.get_refund_policy_history(&merchant).len(), 2);
}
