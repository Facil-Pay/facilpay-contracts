#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, token, Address, Env, Vec};

// Returns (client, admin, customer, merchant, token).
fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let customer = Address::generate(env);
    // The customer deposits the full amount on creation; the contract is funded
    // from that deposit, so no separate contract minting is required.
    token_admin.mint(&customer, &1_000_000);

    (client, admin, customer, Address::generate(env), token_addr)
}

// customer gets 60% share, merchant 40%; equal voting weight.
fn participants(env: &Env, customer: &Address, merchant: &Address) -> Vec<Participant> {
    Vec::from_array(
        env,
        [
            Participant {
                address: customer.clone(),
                role: ParticipantRole::Customer,
                share_bps: 6000,
                weight_bps: 5000,
                approved: false,
                approved_at: None,
            },
            Participant {
                address: merchant.clone(),
                role: ParticipantRole::Merchant,
                share_bps: 4000,
                weight_bps: 5000,
                approved: false,
                approved_at: None,
            },
        ],
    )
}

fn create(
    env: &Env,
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> u64 {
    let parts = participants(env, customer, merchant);
    client.create_multi_party_escrow(customer, &parts, &1_000_i128, token, &2_000_u64)
}

#[test]
fn test_full_acceptance_no_rollback() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);

    client.accept_multi_party_escrow(&customer, &id);
    client.accept_multi_party_escrow(&merchant, &id);

    let status = client.get_initiation_status(&id).unwrap();
    assert_eq!(status.accepted_by.len(), 2);
    assert!(!status.rolled_back);

    // After the deadline, a fully-accepted escrow cannot be rolled back.
    env.ledger().set_timestamp(2_000);
    let result = client.try_rollback_unaccepted_escrow(&id);
    assert_eq!(result, Err(Ok(Error::EscrowAlreadyFullyAccepted)));
}

#[test]
fn test_partial_acceptance_rollback_refunds_shares() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let token_client = token::Client::new(&env, &token);

    let id = create(&env, &client, &customer, &merchant, &token);
    // Deposit of 1000 has been taken from the customer.
    assert_eq!(token_client.balance(&customer), 999_000);

    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);

    // Only one of the two required parties accepts.
    client.accept_multi_party_escrow(&customer, &id);

    // After the deadline the rollback succeeds and refunds each share.
    env.ledger().set_timestamp(2_000);
    client.rollback_unaccepted_escrow(&id);

    // customer: 60% of 1000 = 600; merchant: 40% = 400.
    assert_eq!(token_client.balance(&customer), 999_000 + 600);
    assert_eq!(token_client.balance(&merchant), 400);

    let escrow = client.get_multi_party_escrow(&id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);

    let status = client.get_initiation_status(&id).unwrap();
    assert!(status.rolled_back);
}

#[test]
fn test_accept_after_deadline_fails() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);

    env.ledger().set_timestamp(1_600); // past deadline (1000 + 500)
    let result = client.try_accept_multi_party_escrow(&customer, &id);
    assert_eq!(result, Err(Ok(Error::InitiationDeadlinePassed)));
}

#[test]
fn test_rollback_before_deadline_fails() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);

    // now (1000) < deadline (1500)
    let result = client.try_rollback_unaccepted_escrow(&id);
    assert_eq!(result, Err(Ok(Error::RollbackNotYetAvailable)));
}

#[test]
fn test_idempotent_rollback_guard() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);
    client.accept_multi_party_escrow(&customer, &id);

    env.ledger().set_timestamp(2_000);
    client.rollback_unaccepted_escrow(&id); // first rollback succeeds

    let result = client.try_rollback_unaccepted_escrow(&id);
    assert_eq!(result, Err(Ok(Error::RollbackAlreadyExecuted)));
}

#[test]
fn test_non_participant_cannot_accept() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);

    let stranger = Address::generate(&env);
    let result = client.try_accept_multi_party_escrow(&stranger, &id);
    assert_eq!(result, Err(Ok(Error::ParticipantNotFound)));
}

#[test]
fn test_duplicate_acceptance_fails() {
    let env = Env::default();
    let (client, admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    client.set_initiation_deadline(&admin, &id, &500_u64, &2_u32);

    client.accept_multi_party_escrow(&customer, &id);
    let result = client.try_accept_multi_party_escrow(&customer, &id);
    assert_eq!(result, Err(Ok(Error::AlreadyApproved)));
}

#[test]
fn test_set_initiation_deadline_non_admin_fails() {
    let env = Env::default();
    let (client, _admin, customer, merchant, token) = setup(&env);
    env.ledger().set_timestamp(1_000);

    let id = create(&env, &client, &customer, &merchant, &token);
    // `customer` is not part of the multisig admin set.
    let result = client.try_set_initiation_deadline(&customer, &id, &500_u64, &2_u32);
    assert!(result.is_err());
}
