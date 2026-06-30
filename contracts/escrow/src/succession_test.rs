#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, String};

fn setup() -> (Env, EscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    (env, client, admin)
}

fn zero_address(env: &Env) -> Address {
    Address::from_string(
        env,
        &String::from_str(
            env,
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ),
    )
}

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
    assert_eq!(result, Err(Ok(Error::Action(ActionError::NotReady))));
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
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::SuccessionPlanExists)))
    );
}

#[test]
fn succession_to_zero_address_is_rejected() {
    let (env, client, admin) = setup();
    let zero = zero_address(&env);

    env.ledger().set_timestamp(12_000);
    let result = client.try_designate_successor(&admin, &zero, &60_u64);
    assert_eq!(
        result,
        Err(Ok(Error::Basic(BasicError::InvalidAddress)))
    );
    assert!(client.get_succession_plan().is_none());
}

#[test]
fn succession_to_self_is_rejected() {
    let (env, client, admin) = setup();

    env.ledger().set_timestamp(14_000);
    let result = client.try_designate_successor(&admin, &admin, &60_u64);
    assert_eq!(
        result,
        Err(Ok(Error::Action(ActionError::SameBeneficiary)))
    );
    assert!(client.get_succession_plan().is_none());
}

#[test]
fn valid_succession_transfer_completes() {
    let (env, client, admin) = setup();
    let successor = Address::generate(&env);

    env.ledger().set_timestamp(16_000);
    client.designate_successor(&admin, &successor, &120_u64);

    let plan = client.get_succession_plan().unwrap();
    assert_eq!(plan.successor, successor);
    assert!(!plan.activated);

    env.ledger().set_timestamp(16_120);
    client.activate_succession(&successor);

    let config = client.get_multisig_config();
    assert!(config.admins.contains(&successor));
    assert_eq!(config.total_admins, 2);

    let plan = client.get_succession_plan().unwrap();
    assert!(plan.activated);
}
