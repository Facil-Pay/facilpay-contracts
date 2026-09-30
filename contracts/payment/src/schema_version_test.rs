#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_schema_version_initialized_to_current() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // v2 (#648): per-record data lives in persistent storage.
    assert_eq!(client.get_schema_version(), 2);
}

#[test]
fn test_migrate_schema_rejects_already_at_target() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    client.migrate_schema(&admin, &3);
    assert_eq!(client.get_schema_version(), 3);

    let result = client.try_migrate_schema(&admin, &3);
    assert_eq!(
        result,
        Err(Ok(Error::Basic(BasicError::SchemaAlreadyAtTarget)))
    );
}
