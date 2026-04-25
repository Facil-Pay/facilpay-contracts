#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn make_3_party_escrow(
    env: &Env,
    client: &EscrowContractClient,
    token: &Address,
    p1: &Address,
    p2: &Address,
    p3: &Address,
) -> u64 {
    let mut participants = soroban_sdk::Vec::new(env);
    participants.push_back(Participant {
        address: p1.clone(),
        share_bps: 4000,
        role: ParticipantRole::Customer,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p2.clone(),
        share_bps: 3000,
        role: ParticipantRole::Merchant,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p3.clone(),
        share_bps: 3000,
        role: ParticipantRole::ServiceProvider,
        required_approval: false,
    });
    client.create_multi_party_escrow(p1, &participants, &1000_i128, token, &9999_u64)
}

fn setup() -> (Env, EscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);
    env.ledger().set_timestamp(1000);
    (env, client, admin)
}

#[test]
fn test_dispute_raised_by_participant() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let escrow_id = make_3_party_escrow(&env, &client, &token, &p1, &p2, &p3);
    client.dispute_multi_party_escrow(&p1, &escrow_id);

    let dispute = client.get_multi_party_dispute(&escrow_id);
    assert_eq!(dispute.escrow_id, escrow_id);
    assert!(!dispute.resolved);
    assert_eq!(dispute.quorum_required, 2); // majority of 3
}

#[test]
fn test_non_participant_cannot_dispute() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);
    let outsider = Address::generate(&env);

    let escrow_id = make_3_party_escrow(&env, &client, &token, &p1, &p2, &p3);

    let result = client.try_dispute_multi_party_escrow(&outsider, &escrow_id);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_quorum_met_resolves_for_merchant() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let escrow_id = make_3_party_escrow(&env, &client, &token, &p1, &p2, &p3);
    client.dispute_multi_party_escrow(&p1, &escrow_id);

    client.vote_on_multi_party_dispute(&p2, &escrow_id, &true);
    client.vote_on_multi_party_dispute(&p3, &escrow_id, &true);

    client.resolve_multi_party_dispute(&escrow_id);

    let dispute = client.get_multi_party_dispute(&escrow_id);
    assert!(dispute.resolved);
}

#[test]
fn test_deadline_expiry_defaults_to_customer_refund() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let escrow_id = make_3_party_escrow(&env, &client, &token, &p1, &p2, &p3);
    client.dispute_multi_party_escrow(&p1, &escrow_id);

    // Only one vote for merchant — no quorum
    client.vote_on_multi_party_dispute(&p2, &escrow_id, &true);

    // Advance time past resolution_deadline (7 days from timestamp 1000)
    env.ledger().set_timestamp(1000 + 7 * 24 * 3600 + 1);

    client.resolve_multi_party_dispute(&escrow_id);

    let dispute = client.get_multi_party_dispute(&escrow_id);
    assert!(dispute.resolved);
}

#[test]
fn test_duplicate_vote_rejected() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let escrow_id = make_3_party_escrow(&env, &client, &token, &p1, &p2, &p3);
    client.dispute_multi_party_escrow(&p1, &escrow_id);
    client.vote_on_multi_party_dispute(&p1, &escrow_id, &false);

    let result = client.try_vote_on_multi_party_dispute(&p1, &escrow_id, &false);
    assert_eq!(result, Err(Ok(Error::DuplicateApproval)));
}

#[test]
fn test_no_quorum_before_deadline_blocked() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let escrow_id = make_3_party_escrow(&env, &client, &token, &p1, &p2, &p3);
    client.dispute_multi_party_escrow(&p1, &escrow_id);

    // Only 1 vote, quorum is 2
    client.vote_on_multi_party_dispute(&p1, &escrow_id, &false);

    let result = client.try_resolve_multi_party_dispute(&escrow_id);
    assert_eq!(result, Err(Ok(Error::ApprovalsThresholdNotMet)));
}
