#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, Address, Env, Vec};

#[test]
fn test_multi_party_escrow_weight_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let token_user_client = token::Client::new(&env, &token_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    token_client.mint(&customer, &10000);

    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let mut participants = Vec::new(&env);
    // p1 has 60% share (6000 bps)
    participants.push_back(Participant {
        address: p1.clone(),
        role: ParticipantRole::Merchant,
        share_bps: 6000,
        weight_bps: 1000, // weight_bps is ignored now, initialized based on share_bps (6000)
        approved: false,
        approved_at: None,
    });
    // p2 has 30% share (3000 bps)
    participants.push_back(Participant {
        address: p2.clone(),
        role: ParticipantRole::ServiceProvider,
        share_bps: 3000,
        weight_bps: 1000, // will be initialized to 3000
        approved: false,
        approved_at: None,
    });
    // p3 has 10% share (1000 bps)
    participants.push_back(Participant {
        address: p3.clone(),
        role: ParticipantRole::Arbitrator,
        share_bps: 1000,
        weight_bps: 8000, // will be initialized to 1000
        approved: false,
        approved_at: None,
    });

    let release_timestamp = 1000_u64;
    env.ledger().set_timestamp(500);

    let escrow_id = client.create_multi_party_escrow(
        &customer,
        &participants,
        &10000,
        &token_id,
        &release_timestamp,
    );

    // Verify default threshold is 10000 bps
    let (approved_wt, threshold_bps) = client.get_approval_weight(&escrow_id);
    assert_eq!(approved_wt, 0);
    assert_eq!(threshold_bps, 10000);

    // p1 approves (60% weight, i.e. 6000 bps)
    client.approve_release(&p1, &escrow_id);
    let (approved_wt, _) = client.get_approval_weight(&escrow_id);
    assert_eq!(approved_wt, 6000);

    // Trying to release at 60% (6000 < 10000) should fail
    env.ledger().set_timestamp(1001);
    let release_res = client.try_release_multi_party_escrow(&escrow_id);
    assert_eq!(release_res, Err(Ok(Error::Action(ActionError::ApprovalsThresholdNotMet))));

    // Update threshold to 60% (6000 bps)
    client.update_approval_threshold_bps(&admin, &escrow_id, &6000);
    let (_, threshold_bps) = client.get_approval_weight(&escrow_id);
    assert_eq!(threshold_bps, 6000);

    // Now it should release successfully since approved weight (6000) >= threshold (6000)
    let release_res2 = client.try_release_multi_party_escrow(&escrow_id);
    assert!(release_res2.is_ok());

    let escrow = client.get_multi_party_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(token_user_client.balance(&p1), 6000);
    assert_eq!(token_user_client.balance(&p2), 3000);
    assert_eq!(token_user_client.balance(&p3), 1000);
}

#[test]
fn test_multi_party_escrow_set_participant_weight() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    token_client.mint(&customer, &10000);

    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);

    let mut participants = Vec::new(&env);
    participants.push_back(Participant {
        address: p1.clone(),
        role: ParticipantRole::Merchant,
        share_bps: 5000,
        weight_bps: 5000,
        approved: false,
        approved_at: None,
    });
    participants.push_back(Participant {
        address: p2.clone(),
        role: ParticipantRole::ServiceProvider,
        share_bps: 5000,
        weight_bps: 5000,
        approved: false,
        approved_at: None,
    });

    let release_timestamp = 1000_u64;
    let escrow_id = client.create_multi_party_escrow(
        &customer,
        &participants,
        &10000,
        &token_id,
        &release_timestamp,
    );

    // Initial weight of p1 = 5000, p2 = 5000.
    // We update p1's weight to 5000, which keeps the total at 10000.
    client.set_participant_weight(&admin, &escrow_id, &p1, &5000);

    // Set threshold to 5000 bps
    client.update_approval_threshold_bps(&admin, &escrow_id, &5000);

    // p1 approves (has 5000 weight)
    client.approve_release(&p1, &escrow_id);
    let (approved_wt, threshold_bps) = client.get_approval_weight(&escrow_id);
    assert_eq!(approved_wt, 5000);
    assert_eq!(threshold_bps, 5000);

    // Advance time and release
    env.ledger().set_timestamp(1001);
    let release_res = client.try_release_multi_party_escrow(&escrow_id);
    assert!(release_res.is_ok());
}
