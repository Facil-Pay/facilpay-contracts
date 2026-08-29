#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env};

fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    token_admin.mint(&customer, &10_000);
    token_admin.mint(&contract_id, &10_000);

    (client, customer, merchant, token_addr)
}

/// Files an appeal and returns its id, leaving the escrow in the Appeal round.
fn file_appeal(
    env: &Env,
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> (u64, u64) {
    env.ledger().set_timestamp(1_000);
    let escrow_id = client.create_escrow(
        customer,
        merchant,
        &1_000_i128,
        token,
        &0_u64,
        &0_u64,
        &0_u64,
        &false,
    );
    client.dispute_escrow(customer, &escrow_id);

    // File the appeal one day into the 72-hour window.
    env.ledger().set_timestamp(1_000 + 86_400);
    let reason: BytesN<32> = BytesN::from_array(env, &[0u8; 32]);
    let appeal_id = client.file_dispute_appeal(customer, &escrow_id, &reason);
    (escrow_id, appeal_id)
}

#[test]
fn expire_appeal_after_deadline_forces_finality() {
    let env = Env::default();
    let (client, customer, merchant, token) = setup(&env);
    let (escrow_id, appeal_id) = file_appeal(&env, &client, &customer, &merchant, &token);

    // Deadline is filed_at + 72h. Move just past it.
    let appeal = client.get_appeal(&appeal_id).unwrap();
    env.ledger().set_timestamp(appeal.appeal_deadline + 1);

    client.expire_appeal(&appeal_id);

    let appeal = client.get_appeal(&appeal_id).unwrap();
    assert!(
        appeal.resolved,
        "appeal should be marked resolved after expiry"
    );
    assert_eq!(
        client.get_dispute_round(&escrow_id),
        DisputeRound::Final,
        "dispute round should advance to Final"
    );
}

#[test]
fn expire_appeal_before_deadline_fails() {
    let env = Env::default();
    let (client, customer, merchant, token) = setup(&env);
    let (_escrow_id, appeal_id) = file_appeal(&env, &client, &customer, &merchant, &token);

    // Still inside the appeal window.
    let appeal = client.get_appeal(&appeal_id).unwrap();
    env.ledger().set_timestamp(appeal.appeal_deadline);
    let result = client.try_expire_appeal(&appeal_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::TimeoutNotReached))),
        "expiry should be rejected before the deadline passes"
    );
}

#[test]
fn expire_appeal_twice_fails() {
    let env = Env::default();
    let (client, customer, merchant, token) = setup(&env);
    let (_escrow_id, appeal_id) = file_appeal(&env, &client, &customer, &merchant, &token);

    let appeal = client.get_appeal(&appeal_id).unwrap();
    env.ledger().set_timestamp(appeal.appeal_deadline + 1);
    client.expire_appeal(&appeal_id);

    let result = client.try_expire_appeal(&appeal_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::AlreadyProcessed))),
        "an already-expired appeal cannot be expired again"
    );
}

#[test]
fn expire_nonexistent_appeal_fails() {
    let env = Env::default();
    let (client, _customer, _merchant, _token) = setup(&env);

    let result = client.try_expire_appeal(&999);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::NotFound))));
}
