#![cfg(test)]

use super::*;
use crate::test_funded_token;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};

fn setup(env: &Env) -> (Address, Address, Address, Address, RefundContractClient<'_>) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let merchant = Address::generate(env);
    let customer = Address::generate(env);
    let token = test_funded_token(env, &customer);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    client.initialize(&admin);
    (admin, merchant, customer, token, client)
}

fn request(
    client: &RefundContractClient,
    env: &Env,
    merchant: &Address,
    customer: &Address,
    token: &Address,
    payment_id: u64,
) -> u64 {
    client.request_refund(
        merchant,
        &payment_id,
        customer,
        &100i128,
        &1000i128,
        token,
        &String::from_str(env, "sla test"),
        &RefundReasonCode::Other,
        &1000u64,
    )
}

#[test]
fn test_deadline_uses_global_config_when_no_override() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);

    let cfg = client.get_response_sla_config().unwrap();
    assert_eq!(cfg.response_sla_seconds, 3600u64);
    assert!(cfg.active);

    env.ledger().set_timestamp(1000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);

    assert_eq!(client.get_merchant_response_sla(&merchant), Some(3600u64));
    assert_eq!(
        client.get_refund_sla_deadline(&refund_id),
        Some(1000u64 + 3600u64)
    );
}

#[test]
fn test_deadline_uses_merchant_override_when_set() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);
    client.set_merchant_response_sla(&admin, &merchant, &900u64, &true);

    env.ledger().set_timestamp(2000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);

    assert_eq!(client.get_merchant_response_sla(&merchant), Some(900u64));
    assert_eq!(
        client.get_refund_sla_deadline(&refund_id),
        Some(2000u64 + 900u64)
    );
}

#[test]
fn test_remove_merchant_override_reverts_to_global() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);
    client.set_merchant_response_sla(&admin, &merchant, &900u64, &true);
    assert_eq!(client.get_merchant_response_sla(&merchant), Some(900u64));

    client.remove_merchant_response_sla(&admin, &merchant);
    assert_eq!(client.get_merchant_response_sla(&merchant), Some(3600u64));

    env.ledger().set_timestamp(5000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);
    assert_eq!(
        client.get_refund_sla_deadline(&refund_id),
        Some(5000u64 + 3600u64)
    );
}

#[test]
fn test_auto_approve_fails_before_deadline() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);

    env.ledger().set_timestamp(1000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);

    env.ledger().set_timestamp(1000 + 3600 - 1);
    let result = client.try_auto_approve_on_sla_breach(&refund_id);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::Ext(ExtError::SlaNotBreached)
    );
}

#[test]
fn test_auto_approve_succeeds_after_deadline() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);

    env.ledger().set_timestamp(1000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);

    env.ledger().set_timestamp(1000 + 3600);
    client.auto_approve_on_sla_breach(&refund_id);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

#[test]
fn test_already_reviewed_refund_is_never_auto_approved() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);

    env.ledger().set_timestamp(1000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);

    // Admin reviews (approves) the refund before the SLA deadline passes.
    client.approve_refund(&admin, &refund_id);

    env.ledger().set_timestamp(1000 + 3600);
    let result = client.try_auto_approve_on_sla_breach(&refund_id);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::Core(CoreError::InvalidStatus)
    );

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

#[test]
fn test_get_sla_breached_refunds_lists_breached_and_respects_limit() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);

    env.ledger().set_timestamp(1000);
    let refund_1 = request(&client, &env, &merchant, &customer, &token, 1u64);
    let refund_2 = request(&client, &env, &merchant, &customer, &token, 2u64);

    // Not breached yet.
    assert_eq!(client.get_sla_breached_refunds(&10u32).len(), 0);

    env.ledger().set_timestamp(1000 + 3600);
    let breached = client.get_sla_breached_refunds(&10u32);
    assert_eq!(breached.len(), 2);
    assert!(breached.contains(refund_1));
    assert!(breached.contains(refund_2));

    let limited = client.get_sla_breached_refunds(&1u32);
    assert_eq!(limited.len(), 1);
}

#[test]
fn test_get_sla_breached_refunds_excludes_reviewed() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    client.set_response_sla_config(&admin, &3600u64, &true);

    env.ledger().set_timestamp(1000);
    let refund_1 = request(&client, &env, &merchant, &customer, &token, 1u64);
    let refund_2 = request(&client, &env, &merchant, &customer, &token, 2u64);

    client.approve_refund(&admin, &refund_1);

    env.ledger().set_timestamp(1000 + 3600);
    let breached = client.get_sla_breached_refunds(&10u32);
    assert_eq!(breached.len(), 1);
    assert!(breached.contains(refund_2));
    assert!(!breached.contains(refund_1));
}

#[test]
fn test_no_deadline_recorded_when_sla_inactive() {
    let env = Env::default();
    let (admin, merchant, customer, token, client) = setup(&env);

    // SLA explicitly inactive globally, and no merchant override.
    client.set_response_sla_config(&admin, &3600u64, &false);

    env.ledger().set_timestamp(1000);
    let refund_id = request(&client, &env, &merchant, &customer, &token, 1u64);

    assert_eq!(client.get_merchant_response_sla(&merchant), None);
    assert_eq!(client.get_refund_sla_deadline(&refund_id), None);

    let result = client.try_auto_approve_on_sla_breach(&refund_id);
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::Ext(ExtError::SlaNotConfigured)
    );
}
