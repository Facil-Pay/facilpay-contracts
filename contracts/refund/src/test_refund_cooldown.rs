#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};

#[test]
fn test_refund_cooldown_blocks_second_request() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.set_refund_cooldown_config(
        &admin,
        &RefundCooldownConfig {
            cooldown_seconds: 3600,
            enabled: true,
        },
    );

    env.ledger().set_timestamp(1000);
    client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &100i128,
        &1000i128,
        &token,
        &String::from_str(&env, "first"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    env.ledger().set_timestamp(2000);
    let result = client.try_request_refund(
        &merchant,
        &2u64,
        &customer,
        &100i128,
        &1000i128,
        &token,
        &String::from_str(&env, "second"),
        &RefundReasonCode::Other,
        &1000u64,
    );
    assert_eq!(
        result.unwrap_err().unwrap(),
        Error::Core(CoreError::RefundCooldownActive)
    );
}

#[test]
fn test_refund_cooldown_allows_after_window() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.set_refund_cooldown_config(
        &admin,
        &RefundCooldownConfig {
            cooldown_seconds: 3600,
            enabled: true,
        },
    );

    env.ledger().set_timestamp(1000);
    client.request_refund(
        &merchant,
        &1u64,
        &customer,
        &100i128,
        &1000i128,
        &token,
        &String::from_str(&env, "first"),
        &RefundReasonCode::Other,
        &1000u64,
    );

    env.ledger().set_timestamp(5000);
    client.request_refund(
        &merchant,
        &2u64,
        &customer,
        &100i128,
        &1000i128,
        &token,
        &String::from_str(&env, "second"),
        &RefundReasonCode::Other,
        &1000u64,
    );
}
