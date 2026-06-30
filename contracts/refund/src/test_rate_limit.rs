#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, String};

#[test]
fn test_global_refund_rate_limit() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Abuse");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set global limit: 3 requests per 24 hours (86400 seconds)
    client.set_global_refund_rate_limit(&admin, &3, &86400);

    // Request 3 refunds - should succeed
    for i in 1..=3 {
        client.request_refund(
            &merchant,
            &(i as u64),
            &customer,
            &100,
            &100,
            &token,
            &reason,
            &RefundReasonCode::Other,
            &0,
        );
    }

    // 4th request should fail
    let res = client.try_request_refund(
        &merchant,
        &4,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );

    assert!(res.is_err());
}

#[test]
fn test_customer_override_refund_rate_limit() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Override");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set global limit: 1 request per 24 hours
    client.set_global_refund_rate_limit(&admin, &1, &86400);

    // Set customer override: 5 requests per 24 hours
    client.set_customer_rate_limit(&admin, &customer, &5, &86400);

    // Request 3 refunds - should succeed because of override
    for i in 1..=3 {
        client.request_refund(
            &merchant,
            &(i as u64),
            &customer,
            &100,
            &100,
            &token,
            &reason,
            &RefundReasonCode::Other,
            &0,
        );
    }

    let status = client.get_customer_rate_limit_status(&customer);
    assert_eq!(status.request_count, 3);
}

#[test]
fn test_rate_limit_window_reset() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Reset");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set global limit: 1 request per 1 hour (3600 seconds)
    client.set_global_refund_rate_limit(&admin, &1, &3600);

    // 1st request succeeds
    client.request_refund(
        &merchant,
        &1,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );

    // 2nd request fails
    let res = client.try_request_refund(
        &merchant,
        &2,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );
    assert!(res.is_err());

    // Jump time forward by 1 hour + 1 second
    env.ledger().set_timestamp(3601);

    // 3rd request succeeds because window reset
    client.request_refund(
        &merchant,
        &3,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );
}

#[test]
fn test_update_rate_limit_preserves_current_window_count() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Preserve count");

    env.mock_all_auths();
    client.initialize(&admin);
    client.set_global_refund_rate_limit(&admin, &3, &86400);

    for i in 1..=2 {
        client.request_refund(
            &merchant,
            &(i as u64),
            &customer,
            &100,
            &100,
            &token,
            &reason,
            &RefundReasonCode::Other,
            &0,
        );
    }

    // Tighten limits mid-window; in-progress window keeps prior max and count.
    client.update_rate_limit(&admin, &3600, &1);

    let status = client.get_customer_rate_limit_status(&customer);
    assert_eq!(status.request_count, 2);
    assert_eq!(status.max_requests_per_window, 3);
    assert_eq!(status.window_seconds, 86400);

    client.request_refund(
        &merchant,
        &3,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );

    let res = client.try_request_refund(
        &merchant,
        &4,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );
    assert!(res.is_err());
}

#[test]
fn test_update_rate_limit_applies_after_window_reset() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Deferred apply");

    env.mock_all_auths();
    client.initialize(&admin);
    client.set_global_refund_rate_limit(&admin, &3, &86400);

    for i in 1..=2 {
        client.request_refund(
            &merchant,
            &(i as u64),
            &customer,
            &100,
            &100,
            &token,
            &reason,
            &RefundReasonCode::Other,
            &0,
        );
    }

    env.ledger().set_timestamp(100);
    client.update_rate_limit(&admin, &3600, &1);

    // Still inside the original 24h window; old limits apply.
    client.request_refund(
        &merchant,
        &3,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );

    let res = client.try_request_refund(
        &merchant,
        &4,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );
    assert!(res.is_err());

    // After the original window expires, the updated config takes effect.
    env.ledger().set_timestamp(86401);

    client.request_refund(
        &merchant,
        &5,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );

    let status = client.get_customer_rate_limit_status(&customer);
    assert_eq!(status.request_count, 1);
    assert_eq!(status.max_requests_per_window, 1);
    assert_eq!(status.window_seconds, 3600);

    let res = client.try_request_refund(
        &merchant,
        &6,
        &customer,
        &100,
        &100,
        &token,
        &reason,
        &RefundReasonCode::Other,
        &0,
    );
    assert!(res.is_err());
}

#[test]
fn test_update_rate_limit_rejects_invalid_params() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    assert_eq!(
        client.try_update_rate_limit(&admin, &0, &5),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(
        client.try_update_rate_limit(&admin, &3600, &0),
        Err(Ok(Error::InvalidAmount))
    );
}
