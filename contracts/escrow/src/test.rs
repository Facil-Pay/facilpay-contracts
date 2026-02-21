#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_create_escrow() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);
    assert_eq!(escrow_id, 1);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.id, 1);
    assert_eq!(escrow.customer, customer);
    assert_eq!(escrow.merchant, merchant);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.release_timestamp, release_timestamp);
}

#[test]
fn test_get_escrow() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 5000_i128;
    let release_timestamp = 2000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    let escrow = client.get_escrow(&escrow_id);

    assert_eq!(escrow.id, escrow_id);
    assert_eq!(escrow.customer, customer);
    assert_eq!(escrow.merchant, merchant);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.release_timestamp, release_timestamp);
}

#[test]
#[should_panic(expected = "Escrow not found")]
fn test_get_escrow_not_found() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    client.get_escrow(&999);
}

#[test]
fn test_release_escrow_success() {
    let env = Env::default();
    env.ledger().set_timestamp(2000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Release the escrow
    client.release_escrow(&admin, &escrow_id);

    // Verify status changed to Released
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
#[should_panic]
fn test_release_escrow_before_release_timestamp() {
    let env = Env::default();
    env.ledger().set_timestamp(500);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Try to release before release timestamp - should fail
    client.release_escrow(&admin, &escrow_id);
}

#[test]
#[should_panic]
fn test_release_escrow_not_found() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.release_escrow(&admin, &999);
}

#[test]
#[should_panic]
fn test_release_already_released_escrow() {
    let env = Env::default();
    env.ledger().set_timestamp(2000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Release the escrow
    client.release_escrow(&admin, &escrow_id);

    // Try to release again - should fail
    client.release_escrow(&admin, &escrow_id);
}

#[test]
#[should_panic]
fn test_release_disputed_escrow() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Dispute the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Try to release a disputed escrow - should fail
    client.release_escrow(&admin, &escrow_id);
}

#[test]
fn test_dispute_escrow_by_customer() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Customer disputes the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Verify status changed to Disputed
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
fn test_dispute_escrow_by_merchant() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Merchant disputes the escrow
    client.dispute_escrow(&merchant, &escrow_id);

    // Verify status changed to Disputed
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
#[should_panic]
fn test_dispute_escrow_by_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Unauthorized user tries to dispute - should fail
    client.dispute_escrow(&other, &escrow_id);
}

#[test]
#[should_panic]
fn test_dispute_escrow_not_found() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);

    env.mock_all_auths();

    client.dispute_escrow(&customer, &999);
}

#[test]
#[should_panic]
fn test_dispute_already_disputed_escrow() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Dispute the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Try to dispute again - should fail
    client.dispute_escrow(&merchant, &escrow_id);
}

#[test]
fn test_resolve_dispute_release_to_merchant() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Dispute the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Resolve dispute - release to merchant
    client.resolve_dispute(&admin, &escrow_id, &true);

    // Verify status changed to Released
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_resolve_dispute_release_to_customer() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Dispute the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Resolve dispute - release to customer
    client.resolve_dispute(&admin, &escrow_id, &false);

    // Verify status changed to Resolved
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
#[should_panic]
fn test_resolve_dispute_not_found() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.resolve_dispute(&admin, &999, &true);
}

#[test]
#[should_panic]
fn test_resolve_dispute_not_disputed() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Try to resolve without dispute - should fail
    client.resolve_dispute(&admin, &escrow_id, &true);
}

#[test]
#[should_panic]
fn test_resolve_already_resolved_dispute() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &amount, &token, &release_timestamp);

    // Dispute the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Resolve dispute
    client.resolve_dispute(&admin, &escrow_id, &true);

    // Try to resolve again - should fail
    client.resolve_dispute(&admin, &escrow_id, &false);
}

#[test]
fn test_multiple_escrows() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let token = Address::generate(&env);
    let release_timestamp = 1000_u64;

    env.mock_all_auths();

    // Create first escrow
    let escrow_id1 = client.create_escrow(
        &customer,
        &merchant1,
        &1000_i128,
        &token,
        &release_timestamp,
    );
    assert_eq!(escrow_id1, 1);

    // Create second escrow
    let escrow_id2 = client.create_escrow(
        &customer,
        &merchant2,
        &2000_i128,
        &token,
        &release_timestamp,
    );
    assert_eq!(escrow_id2, 2);

    // Verify both escrows
    let escrow1 = client.get_escrow(&escrow_id1);
    assert_eq!(escrow1.merchant, merchant1);
    assert_eq!(escrow1.amount, 1000_i128);

    let escrow2 = client.get_escrow(&escrow_id2);
    assert_eq!(escrow2.merchant, merchant2);
    assert_eq!(escrow2.amount, 2000_i128);
}
