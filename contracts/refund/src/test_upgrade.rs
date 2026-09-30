#![cfg(test)]

// Issue #643: authorized in-place WASM upgrade of the refund contract.

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{vec, Env, Event};

mod v2 {
    soroban_sdk::contractimport!(file = "test_fixtures/refund_upgrade_v2.wasm");
}

fn setup() -> (Env, RefundContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn test_upgrade_keeps_address_and_storage_and_changes_behaviour() {
    let (env, client, admin) = setup();
    let contract_id = client.address.clone();
    let upgraded = v2::Client::new(&env, &contract_id);

    // The current code has no `version` function.
    assert!(upgraded.try_version().is_err());

    let new_wasm_hash = env.deployer().upload_contract_wasm(v2::WASM);
    client.upgrade(&admin, &new_wasm_hash);

    // Same address, new code: `version` now exists and the refund entry points
    // are gone, while the admin stored by the old code is still readable.
    assert_eq!(upgraded.address, contract_id);
    assert_eq!(upgraded.version(), 2);
    assert_eq!(upgraded.get_admin(), Some(admin));
    assert!(client.try_get_schema_version().is_err());
}

#[test]
fn test_upgrade_emits_contract_upgraded_event() {
    let (env, client, admin) = setup();
    let old_schema_version = client.get_schema_version();
    let new_wasm_hash = env.deployer().upload_contract_wasm(v2::WASM);

    client.upgrade(&admin, &new_wasm_hash);

    let event = ContractUpgraded {
        old_schema_version,
        new_wasm_hash,
        upgraded_by: admin,
    };
    let all = env.events().all();
    assert_eq!(
        all.slice(all.len() - 1..),
        vec![
            &env,
            (client.address.clone(), event.topics(&env), event.data(&env))
        ]
    );
}

#[test]
fn test_upgrade_rejects_non_admin_caller() {
    let (env, client, _admin) = setup();
    let new_wasm_hash = env.deployer().upload_contract_wasm(v2::WASM);
    let attacker = Address::generate(&env);

    let result = client.try_upgrade(&attacker, &new_wasm_hash);
    assert_eq!(result, Err(Ok(Error::Core(CoreError::Unauthorized))));

    // The original code is still in place.
    assert_eq!(client.get_schema_version(), 2);
    assert!(v2::Client::new(&env, &client.address)
        .try_version()
        .is_err());
}

#[test]
fn test_upgrade_rejects_pending_admin_until_accepted() {
    let (env, client, admin) = setup();
    let new_admin = Address::generate(&env);
    let new_wasm_hash = env.deployer().upload_contract_wasm(v2::WASM);

    client.propose_admin(&admin, &new_admin);
    assert_eq!(
        client.try_upgrade(&new_admin, &new_wasm_hash),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );

    client.accept_admin(&new_admin);
    assert_eq!(
        client.try_upgrade(&admin, &new_wasm_hash),
        Err(Ok(Error::Core(CoreError::Unauthorized)))
    );
    client.upgrade(&new_admin, &new_wasm_hash);
    assert_eq!(v2::Client::new(&env, &client.address).version(), 2);
}
