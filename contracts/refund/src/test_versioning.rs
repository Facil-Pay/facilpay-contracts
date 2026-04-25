#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

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

    client.set_refund_policy(&merchant, &86400u64, &10000u32, &true, &0i128);
    client.set_refund_policy(&merchant, &172800u64, &5000u32, &false, &100i128);

    let v1 = client.get_refund_policy_version(&merchant, &1u32).unwrap();
    let v2 = client.get_refund_policy_version(&merchant, &2u32).unwrap();

    assert_eq!(v1.version, 1);
    assert_eq!(v1.policy.refund_window, 86400);
    assert_eq!(v2.version, 2);
    assert_eq!(v2.policy.refund_window, 172800);
}

// get_refund_policy_at_time returns the version active at the given timestamp
#[test]
fn test_policy_at_time() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let merchant = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.set_refund_policy(&merchant, &86400u64, &10000u32, &true, &0i128);

    env.ledger().set_timestamp(2000);
    client.set_refund_policy(&merchant, &172800u64, &5000u32, &false, &0i128);

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

    client.set_refund_policy(&merchant, &86400u64, &10000u32, &true, &0i128);
    client.set_refund_policy(&merchant, &172800u64, &5000u32, &false, &0i128);
    client.set_refund_policy(&merchant, &259200u64, &2000u32, &true, &50i128);

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
