#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn setup(env: &Env) -> (EscrowContractClient, Address, Address, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_addr = env.register_stellar_asset_contract(admin.clone());
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    let customer = Address::generate(env);
    token_admin.mint(&customer, &10_000);
    token_admin.mint(&contract_id, &10_000);

    (client, admin, customer, Address::generate(env), token_addr)
}

#[test]
fn escrow_refundable_after_hold_period_elapsed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    // Create escrow at t=1000 with hold period of 500s
    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &0_u64, &500_u64, &0_u64, &false,
    );

    // Advance past hold period: 1000 + 500 = 1500
    env.ledger().set_timestamp(1501);
    let result = client.try_refund_escrow(&customer, &escrow_id);
    assert!(result.is_ok(), "refund should succeed after hold period has elapsed");

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn escrow_not_refundable_before_hold_period_elapsed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    // Create escrow at t=1000 with hold period of 500s
    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &500_i128, &token, &0_u64, &500_u64, &0_u64, &false,
    );

    // Attempt refund before hold period expires (at exactly t=1000, hold period = 500s)
    env.ledger().set_timestamp(1400);
    let result = client.try_refund_escrow(&customer, &escrow_id);
    assert!(result.is_err(), "refund should fail before hold period has elapsed");
}

#[test]
fn refund_after_hold_period_transfers_correct_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, customer, merchant, token) = setup(&env);

    let token_client = token::Client::new(&env, &token);
    let amount = 750_i128;

    // Create escrow at t=2000 with hold period of 300s
    env.ledger().set_timestamp(2000);
    let escrow_id = client.create_escrow(
        &customer, &merchant, &amount, &token, &0_u64, &300_u64, &0_u64, &false,
    );

    let balance_before = token_client.balance(&customer);

    // Advance past hold period: 2000 + 300 = 2300
    env.ledger().set_timestamp(2301);
    client.refund_escrow(&customer, &escrow_id);

    let balance_after = token_client.balance(&customer);
    assert_eq!(
        balance_after - balance_before,
        amount,
        "customer should receive exactly the escrowed amount"
    );
}
