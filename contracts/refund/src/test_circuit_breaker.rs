#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, RefundContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);
    env.ledger().set_timestamp(1000);
    (env, client, admin)
}

fn default_config(enabled: bool) -> CircuitBreakerConfig {
    CircuitBreakerConfig {
        max_refund_rate_bps: 1000, // 10%
        measurement_window_seconds: 3600,
        cooldown_seconds: 600,
        enabled,
    }
}

fn request(client: &RefundContractClient, merchant: &Address, customer: &Address, token: &Address, amount: i128, payment_amount: i128) -> u64 {
    let reason = soroban_sdk::String::from_str(&client.env, "test");
    client.request_refund(merchant, &1u64, customer, &amount, &payment_amount, token, &reason, &0u64)
}

#[test]
fn test_circuit_breaker_trips_when_rate_exceeded() {
    let (env, client, admin) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.set_circuit_breaker_config(&admin, &default_config(true));

    // 10% threshold: 1000/10000 = 10%. request 1100/10000 = 11% -> should trip
    let result = client.try_request_refund(
        &merchant, &1u64, &customer, &1100_i128, &10000_i128, &token,
        &String::from_str(&env, "test"), &0u64,
    );
    assert_eq!(result, Err(Ok(Error::CircuitBreakerTripped)));

    let state = client.get_circuit_breaker_state();
    assert!(state.tripped);
    assert!(state.trip_count > 0);
}

#[test]
fn test_tripped_breaker_blocks_new_requests() {
    let (env, client, admin) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.set_circuit_breaker_config(&admin, &default_config(true));

    // Trip the breaker
    let _ = client.try_request_refund(
        &merchant, &1u64, &customer, &1100_i128, &10000_i128, &token,
        &String::from_str(&env, "reason"), &0u64,
    );

    // Now a small refund should also be blocked
    let result = client.try_request_refund(
        &merchant, &2u64, &customer, &10_i128, &10000_i128, &token,
        &String::from_str(&env, "reason"), &0u64,
    );
    assert_eq!(result, Err(Ok(Error::CircuitBreakerTripped)));
}

#[test]
fn test_auto_reset_after_cooldown() {
    let (env, client, admin) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.set_circuit_breaker_config(&admin, &default_config(true));

    // Trip the breaker
    let _ = client.try_request_refund(
        &merchant, &1u64, &customer, &1100_i128, &10000_i128, &token,
        &String::from_str(&env, "reason"), &0u64,
    );

    // Advance past cooldown (600s from timestamp 1000)
    env.ledger().set_timestamp(1000 + 601);

    // A small refund should succeed now (new window, low rate)
    let result = client.try_request_refund(
        &merchant, &2u64, &customer, &10_i128, &10000_i128, &token,
        &String::from_str(&env, "reason"), &0u64,
    );
    assert!(result.is_ok());

    let state = client.get_circuit_breaker_state();
    assert!(!state.tripped);
}

#[test]
fn test_manual_reset_by_admin() {
    let (env, client, admin) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.set_circuit_breaker_config(&admin, &default_config(true));

    // Trip the breaker
    let _ = client.try_request_refund(
        &merchant, &1u64, &customer, &1100_i128, &10000_i128, &token,
        &String::from_str(&env, "reason"), &0u64,
    );

    assert!(client.get_circuit_breaker_state().tripped);

    client.reset_circuit_breaker(&admin);

    let state = client.get_circuit_breaker_state();
    assert!(!state.tripped);
}

#[test]
fn test_disabled_breaker_does_not_block() {
    let (env, client, admin) = setup();
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    client.set_circuit_breaker_config(&admin, &default_config(false));

    // High refund rate, but breaker disabled — should not block
    let result = client.try_request_refund(
        &merchant, &1u64, &customer, &9000_i128, &10000_i128, &token,
        &String::from_str(&env, "reason"), &0u64,
    );
    assert!(result.is_ok());

    let state = client.get_circuit_breaker_state();
    assert!(!state.tripped);
}
