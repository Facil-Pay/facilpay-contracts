#![cfg(test)]

// Issue #389: two-step admin rotation for the refund contract.

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

#[test]
fn test_propose_and_accept_admin_rotates_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.get_pending_admin(), None);

    client.propose_admin(&admin, &new_admin);
    assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

    client.accept_admin(&new_admin);
    assert_eq!(client.get_pending_admin(), None);

    // The rotated-in admin can now perform an admin-gated action.
    let target_version = client.get_schema_version() + 1;
    client.migrate_schema(&new_admin, &target_version);
    assert_eq!(client.get_schema_version(), target_version);
}

#[test]
fn test_propose_admin_rejects_non_admin_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let not_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_propose_admin(&not_admin, &new_admin);
    assert_eq!(result, Err(Ok(Error::Core(CoreError::Unauthorized))));
}

#[test]
fn test_accept_admin_rejects_wrong_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let imposter = Address::generate(&env);
    client.initialize(&admin);

    client.propose_admin(&admin, &new_admin);

    let result = client.try_accept_admin(&imposter);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::NotPendingAdmin))));

    // Old admin still retains control since rotation never completed.
    let target_version = client.get_schema_version() + 1;
    client.migrate_schema(&admin, &target_version);
    assert_eq!(client.get_schema_version(), target_version);
}

#[test]
fn test_accept_admin_without_proposal_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);
    client.initialize(&admin);

    let result = client.try_accept_admin(&stranger);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::NoPendingAdmin))));
}

#[test]
fn test_compromised_admin_key_cannot_be_used_after_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let compromised_admin = Address::generate(&env);
    let safe_admin = Address::generate(&env);
    client.initialize(&compromised_admin);

    client.propose_admin(&compromised_admin, &safe_admin);
    client.accept_admin(&safe_admin);

    // The old (compromised) admin address can no longer perform admin actions.
    let result = client.try_migrate_schema(&compromised_admin, &2);
    assert_eq!(result, Err(Ok(Error::Core(CoreError::Unauthorized))));
}
