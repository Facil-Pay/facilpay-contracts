#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String,
};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

fn setup_test_env() -> (Env, Address, Address, Address, Address, Address, Address, token::Client<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let arbitrator1 = Address::generate(&env);
    let arbitrator2 = Address::generate(&env);
    let arbitrator3 = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&merchant, &100000);
    token_admin.mint(&customer, &100000);

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Register arbitrators
    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client)
}

fn create_and_escalate_refund(
    env: &Env,
    client: &RefundContractClient,
    merchant: &Address,
    customer: &Address,
    admin: &Address,
    token_client: &token::Client,
) -> u64 {
    // Set initial ledger timestamp to avoid underflow
    env.ledger().with_mut(|li| {
        if li.timestamp < 1000 {
            li.timestamp = 1000;
        }
    });

    // Request refund
    let refund_id = client.request_refund(
        merchant,
        &1,
        customer,
        &1000,
        &5000,
        &token_client.address,
        &String::from_str(env, "Test refund"),
        &RefundReasonCode::Other,
        &(env.ledger().timestamp() - 100),
    );

    // Reject refund
    client.reject_refund(admin, &refund_id, &String::from_str(env, "Rejected for testing"));

    // Escalate to arbitration
    let case_id = client.escalate_to_arbitration(
        customer,
        &refund_id,
        &token_client.address,
        &3000,
    );

    case_id
}

#[test]
fn test_register_arbitrator_initializes_reputation() {
    let (env, admin, _, _, arbitrator1, _, _, _) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);

    let reputation = client.get_arbitrator_reputation(&arbitrator1);
    assert!(reputation.is_some());
    
    let rep = reputation.unwrap();
    assert_eq!(rep.arbitrator, arbitrator1);
    assert_eq!(rep.total_cases, 0);
    assert_eq!(rep.majority_votes, 0);
    assert_eq!(rep.minority_votes, 0);
    assert_eq!(rep.avg_resolution_time, 0);
    assert_eq!(rep.score, 100); // Starting score
}

#[test]
fn test_score_increases_on_majority_vote() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    let case_id = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);

    // All three arbitrators vote for refund (majority)
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &reasoning_hash);

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check that all arbitrators' scores increased
    let rep1 = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let rep2 = client.get_arbitrator_reputation(&arbitrator2).unwrap();
    let rep3 = client.get_arbitrator_reputation(&arbitrator3).unwrap();

    assert_eq!(rep1.score, 110); // 100 + 10
    assert_eq!(rep2.score, 110);
    assert_eq!(rep3.score, 110);
    assert_eq!(rep1.majority_votes, 1);
    assert_eq!(rep1.minority_votes, 0);
    assert_eq!(rep1.total_cases, 1);
}

#[test]
fn test_score_decreases_on_minority_vote() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    let case_id = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);

    // Two arbitrators vote for refund, one against (minority)
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &false, &reasoning_hash); // Minority

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check scores
    let rep1 = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let rep2 = client.get_arbitrator_reputation(&arbitrator2).unwrap();
    let rep3 = client.get_arbitrator_reputation(&arbitrator3).unwrap();

    assert_eq!(rep1.score, 110); // Majority: 100 + 10
    assert_eq!(rep2.score, 110); // Majority: 100 + 10
    assert_eq!(rep3.score, 95);  // Minority: 100 - 5
    assert_eq!(rep3.majority_votes, 0);
    assert_eq!(rep3.minority_votes, 1);
}

#[test]
fn test_avg_resolution_time_updated() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    let case_id = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);

    // Advance time by 1000 seconds
    env.ledger().with_mut(|li| {
        li.timestamp += 1000;
    });

    // Cast votes
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &reasoning_hash);

    // Close the case
    client.close_arbitration_case(&case_id);

    // Check that avg_resolution_time is set
    let rep1 = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    assert!(rep1.avg_resolution_time > 0);
    assert!(rep1.avg_resolution_time >= 1000); // At least 1000 seconds
}

#[test]
fn test_multiple_cases_update_average_resolution_time() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // First case - 1000 seconds
    let case_id1 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    env.ledger().with_mut(|li| {
        li.timestamp += 1000;
    });
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id1, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id1);

    let rep_after_first = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let first_avg = rep_after_first.avg_resolution_time;

    // Second case - 2000 seconds
    let case_id2 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    env.ledger().with_mut(|li| {
        li.timestamp += 2000;
    });
    client.cast_arbitration_vote(&arbitrator1, &case_id2, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id2, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id2, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id2);

    let rep_after_second = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let second_avg = rep_after_second.avg_resolution_time;

    // Average should be between first and second case durations
    assert!(second_avg > first_avg);
    assert_eq!(rep_after_second.total_cases, 2);
}

#[test]
fn test_get_top_arbitrators_sorted_by_score() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // Create multiple cases to differentiate scores
    // Case 1: arbitrator1 and arbitrator2 in majority, arbitrator3 in minority
    let case_id1 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id1, &false, &reasoning_hash);
    client.close_arbitration_case(&case_id1);

    // Case 2: arbitrator1 in majority, arbitrator2 and arbitrator3 in minority
    let case_id2 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    client.cast_arbitration_vote(&arbitrator1, &case_id2, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id2, &false, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id2, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id2);

    // Get top arbitrators
    let top_arbitrators = client.get_top_arbitrators(&3);
    
    assert_eq!(top_arbitrators.len(), 3);
    
    // arbitrator1 should be first (2 majority votes: 100 + 10 + 10 = 120)
    assert_eq!(top_arbitrators.get(0).unwrap().arbitrator, arbitrator1);
    assert_eq!(top_arbitrators.get(0).unwrap().score, 120);
    
    // arbitrator2 and arbitrator3 should have lower scores
    // arbitrator2: 1 majority, 1 minority = 100 + 10 - 5 = 105
    // arbitrator3: 1 majority, 1 minority = 100 + 10 - 5 = 105
    let second_score = top_arbitrators.get(1).unwrap().score;
    let third_score = top_arbitrators.get(2).unwrap().score;
    assert_eq!(second_score, 105);
    assert_eq!(third_score, 105);
}

#[test]
fn test_get_top_arbitrators_with_limit() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // Get top 2 arbitrators
    let top_arbitrators = client.get_top_arbitrators(&2);
    
    assert_eq!(top_arbitrators.len(), 2);
}

#[test]
fn test_deregister_low_performing_arbitrators() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // Create cases to lower arbitrator3's score
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    
    // Case 1: arbitrator3 in minority
    let case_id1 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    client.cast_arbitration_vote(&arbitrator1, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id1, &false, &reasoning_hash);
    client.close_arbitration_case(&case_id1);

    // Case 2: arbitrator3 in minority again
    let case_id2 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    client.cast_arbitration_vote(&arbitrator1, &case_id2, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id2, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id2, &false, &reasoning_hash);
    client.close_arbitration_case(&case_id2);

    // arbitrator3 should now have score: 100 - 5 - 5 = 90
    let rep3 = client.get_arbitrator_reputation(&arbitrator3).unwrap();
    assert_eq!(rep3.score, 90);

    // Deregister arbitrators with score below 100
    let removed_count = client.deregister_low_performers(&admin, &100);
    assert_eq!(removed_count, 1);

    // arbitrator3 should no longer have reputation
    let rep3_after = client.get_arbitrator_reputation(&arbitrator3);
    assert!(rep3_after.is_none());

    // arbitrator1 and arbitrator2 should still exist
    let rep1_after = client.get_arbitrator_reputation(&arbitrator1);
    let rep2_after = client.get_arbitrator_reputation(&arbitrator2);
    assert!(rep1_after.is_some());
    assert!(rep2_after.is_some());
}

#[test]
fn test_deregister_multiple_low_performers() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // Create cases where arbitrator2 and arbitrator3 are in minority
    // arbitrator1 and arbitrator2 vote FOR refund (majority)
    // arbitrator3 votes AGAINST refund (minority)
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    
    // Case 1
    let case_id1 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    client.cast_arbitration_vote(&arbitrator1, &case_id1, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id1, &false, &reasoning_hash); // minority
    client.cast_arbitration_vote(&arbitrator3, &case_id1, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id1);

    // Case 2
    let case_id2 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    client.cast_arbitration_vote(&arbitrator1, &case_id2, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id2, &false, &reasoning_hash); // minority
    client.cast_arbitration_vote(&arbitrator3, &case_id2, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id2);

    // Case 3
    let case_id3 = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    client.cast_arbitration_vote(&arbitrator1, &case_id3, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id3, &false, &reasoning_hash); // minority
    client.cast_arbitration_vote(&arbitrator3, &case_id3, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id3);

    // Check scores before deregistration
    let rep1 = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let rep2 = client.get_arbitrator_reputation(&arbitrator2).unwrap();
    let rep3 = client.get_arbitrator_reputation(&arbitrator3).unwrap();
    
    // arbitrator1: 3 majority votes = 100 + 10 + 10 + 10 = 130
    assert_eq!(rep1.score, 130);
    // arbitrator2: 3 minority votes = 100 - 5 - 5 - 5 = 85
    assert_eq!(rep2.score, 85);
    // arbitrator3: 3 majority votes = 100 + 10 + 10 + 10 = 130
    assert_eq!(rep3.score, 130);
    
    // Deregister arbitrators with score below 90
    let removed_count = client.deregister_low_performers(&admin, &90);
    assert_eq!(removed_count, 1); // Only arbitrator2 should be removed

    // arbitrator1 and arbitrator3 should remain
    let top_arbitrators = client.get_top_arbitrators(&10);
    assert_eq!(top_arbitrators.len(), 2);
}

#[test]
#[should_panic]
fn test_deregister_low_performers_unauthorized() {
    let (env, admin, _, customer, arbitrator1, arbitrator2, arbitrator3, _) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // Non-admin tries to deregister
    client.deregister_low_performers(&customer, &100);
}

#[test]
#[should_panic]
fn test_deregister_with_negative_threshold() {
    let (env, admin, _, _, arbitrator1, arbitrator2, arbitrator3, _) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    // Try to deregister with negative threshold
    client.deregister_low_performers(&admin, &-10);
}

#[test]
fn test_last_active_timestamp_updated() {
    let (env, admin, merchant, customer, arbitrator1, arbitrator2, arbitrator3, token_client) = setup_test_env();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client.register_arbitrator(&admin, &arbitrator1);
    client.register_arbitrator(&admin, &arbitrator2);
    client.register_arbitrator(&admin, &arbitrator3);

    let initial_rep = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let initial_timestamp = initial_rep.last_active;

    // Advance time
    env.ledger().with_mut(|li| {
        li.timestamp += 5000;
    });

    // Create and close a case
    let case_id = create_and_escalate_refund(&env, &client, &merchant, &customer, &admin, &token_client);
    let reasoning_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.cast_arbitration_vote(&arbitrator1, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator2, &case_id, &true, &reasoning_hash);
    client.cast_arbitration_vote(&arbitrator3, &case_id, &true, &reasoning_hash);
    client.close_arbitration_case(&case_id);

    let updated_rep = client.get_arbitrator_reputation(&arbitrator1).unwrap();
    let updated_timestamp = updated_rep.last_active;

    // Timestamp should be updated
    assert!(updated_timestamp > initial_timestamp);
}
