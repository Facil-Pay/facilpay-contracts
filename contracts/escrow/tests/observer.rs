use super::super::*;
use soroban_sdk::{Env, Address};

// Integration-style tests for observer feature
#[test]
fn observer_grant_remove_and_expiry() {
    let env = Env::default();
    let contract_id = env.register(super::EscrowContract, ());
    let client = super::EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let observer = Address::generate(&env);

    env.mock_all_auths();

    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false);

    // grant
    client.add_observer(&customer, &escrow_id, &observer, &3600_u64);
    assert!(client.verify_observer_access(&escrow_id, &observer));

    // remove
    client.remove_observer(&customer, &escrow_id, &observer).unwrap();
    assert!(!client.verify_observer_access(&escrow_id, &observer));

    // grant short and expire
    let now = env.ledger().timestamp();
    client.add_observer(&customer, &escrow_id, &observer, &1_u64);
    assert!(client.verify_observer_access(&escrow_id, &observer));
    env.ledger().set_timestamp(now + 10);
    assert!(!client.verify_observer_access(&escrow_id, &observer));
}
