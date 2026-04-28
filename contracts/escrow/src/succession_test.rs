#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

fn setup() -> (Env, EscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    (env, client, admin)
}

#[test]
fn test_designate_successor_stores_plan() {
    let (env, client, admin) = setup();
    let successor = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.designate_successor(&admin, &successor, &300_u64);

    let plan = client.get_succession_plan().unwrap();
    assert_eq!(plan.successor, successor);
    assert_eq!(plan.designated_by, admin);
    assert_eq!(plan.designated_at, 1_000);
    assert_eq!(plan.activatable_after, 1_300);
    assert!(!plan.activated);
}

#[test]
fn test_activate_succession_adds_successor_as_admin() {
    let (env, client, admin) = setup();
    let successor = Address::generate(&env);

    env.ledger().set_timestamp(2_000);
    client.designate_successor(&admin, &successor, &60_u64);

    env.ledger().set_timestamp(2_060);
    client.activate_succession(&successor);

    let config = client.get_multisig_config();
    assert!(config.admins.contains(&successor));
    assert_eq!(config.total_admins, 2);

    let plan = client.get_succession_plan().unwrap();
    assert!(plan.activated);

    client.set_batch_limit(&successor, &75_u32);
    assert_eq!(client.get_batch_limit(), 75);
}

#[test]
fn test_activate_succession_rejects_premature_activation() {
    let (env, client, admin) = setup();
    let successor = Address::generate(&env);

    env.ledger().set_timestamp(5_000);
    client.designate_successor(&admin, &successor, &120_u64);

    env.ledger().set_timestamp(5_119);
    let result = client.try_activate_succession(&successor);
    assert_eq!(result, Err(Ok(Error::ActionNotReady)));
}

#[test]
fn test_any_admin_can_revoke_pending_succession() {
    let (env, client, admin) = setup();
    let second_admin = Address::generate(&env);
    let successor = Address::generate(&env);

    client.add_admin(&admin, &second_admin);
    env.ledger().set_timestamp(8_000);
    client.designate_successor(&admin, &successor, &600_u64);

    client.revoke_succession(&second_admin);

    assert!(client.get_succession_plan().is_none());
}

#[test]
fn test_only_one_pending_succession_plan_allowed() {
    let (env, client, admin) = setup();
    let first_successor = Address::generate(&env);
    let second_successor = Address::generate(&env);

    env.ledger().set_timestamp(10_000);
    client.designate_successor(&admin, &first_successor, &60_u64);

    let result = client.try_designate_successor(&admin, &second_successor, &120_u64);
    assert_eq!(result, Err(Ok(Error::SuccessionPlanExists)));
}
