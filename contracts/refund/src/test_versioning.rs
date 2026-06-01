#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

// Each set_refund_policy call increments the version counter
#[test]
fn test_policy_version_increments() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);

    let tiers1 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 1, max_refund_bps: 10000 }]);
    client.set_refund_policy(&merchant, &tiers1);

    let tiers2 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 2, max_refund_bps: 5000 }]);
    client.set_refund_policy(&merchant, &tiers2);

    let v1 = client.get_refund_policy_version(&merchant, &1u32).unwrap();
    let v2 = client.get_refund_policy_version(&merchant, &2u32).unwrap();

    assert_eq!(v1.version, 1);
    assert_eq!(v1.policy.tiers.get(0).unwrap().days_from_purchase, 1);
    assert_eq!(v2.version, 2);
    assert_eq!(v2.policy.tiers.get(0).unwrap().days_from_purchase, 2);
}

// get_refund_policy_at_time returns the version active at the given timestamp
#[test]
fn test_policy_at_time() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let tiers1 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 1, max_refund_bps: 10000 }]);
    client.set_refund_policy(&merchant, &tiers1);

    env.ledger().set_timestamp(2000);
    let tiers2 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 2, max_refund_bps: 5000 }]);
    client.set_refund_policy(&merchant, &tiers2);

    // At t=1500 only v1 existed
    let at_1500 = client.get_refund_policy_at_time(&merchant, &1500u64).unwrap();
    assert_eq!(at_1500.version, 1);

    // At t=2500 v2 is active
    let at_2500 = client.get_refund_policy_at_time(&merchant, &2500u64).unwrap();
    assert_eq!(at_2500.version, 2);
}

// Version history is append-only and immutable
#[test]
fn test_policy_history_append_only() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);

    let tiers1 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 1, max_refund_bps: 10000 }]);
    client.set_refund_policy(&merchant, &tiers1);

    let tiers2 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 2, max_refund_bps: 5000 }]);
    client.set_refund_policy(&merchant, &tiers2);

    let tiers3 = Vec::from_array(&env, &[RefundTier { days_from_purchase: 3, max_refund_bps: 2000 }]);
    client.set_refund_policy(&merchant, &tiers3);

    let history = client.get_refund_policy_history(&merchant);
    assert_eq!(history.len(), 3);
    assert_eq!(history.get(0).unwrap().version, 1);
    assert_eq!(history.get(1).unwrap().version, 2);
    assert_eq!(history.get(2).unwrap().version, 3);
}

// No history returns None
#[test]
fn test_policy_at_time_no_history() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);
    let result = client.get_refund_policy_at_time(&merchant, &9999u64);
    assert!(result.is_none());
}
