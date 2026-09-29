#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};

fn setup(env: &Env) -> (EscrowContractClient, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin, contract_id)
}

fn setup_token(env: &Env, mint_to: &Address, contract_id: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = token::StellarAssetClient::new(env, &token_id);
    token_client.mint(mint_to, &amount);
    token_client.mint(contract_id, &amount);
    token_id
}

fn make_milestone(env: &Env, id: u64, amount: i128, deadline: Option<u64>) -> VestingMilestone {
    VestingMilestone {
        milestone_id: id,
        unlock_timestamp: 2000,
        amount,
        released: false,
        description: String::from_str(env, "M"),
        approved_by: None,
        approved_at: None,
        deadline,
        claimed_missed: false,
    }
}

// ── Issue #687 – Partial release ─────────────────────────────────────────────

#[test]
fn test_partial_release_basic() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );

    client.release_partial(&admin, &escrow_id, &300_i128);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.released_amount, 300);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_partial_release_full_marks_released() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );

    client.release_partial(&admin, &escrow_id, &500_i128);
    client.release_partial(&admin, &escrow_id, &500_i128);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.released_amount, 1000);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_partial_release_refund_uses_remainder() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );

    // Release 400 first; refund should succeed (refunds remaining 600)
    client.release_partial(&admin, &escrow_id, &400_i128);
    let result = client.try_refund_escrow(&customer, &escrow_id);
    assert!(result.is_ok());
}

#[test]
fn test_partial_release_exceeds_balance_fails() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );

    let result = client.try_release_partial(&admin, &escrow_id, &1001_i128);
    assert!(result.is_err());
}

// ── Issue #688 – Milestone deadlines / auto-refund ───────────────────────────

#[test]
fn test_claim_missed_milestone_after_deadline() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(1000);

    let milestones = vec![
        &env,
        make_milestone(&env, 1, 500, Some(3000)),
        make_milestone(&env, 2, 500, None),
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &5000_u64,
        &milestones,
    );

    env.ledger().set_timestamp(3001);
    let refunded = client.claim_missed_milestone(&customer, &escrow_id, &1_u64);
    assert_eq!(refunded, 500);
}

#[test]
fn test_claim_missed_milestone_before_deadline_fails() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(1000);

    let milestones = vec![
        &env,
        make_milestone(&env, 1, 500, Some(3000)),
        make_milestone(&env, 2, 500, None),
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &5000_u64,
        &milestones,
    );

    env.ledger().set_timestamp(2500); // before deadline 3000
    let result = client.try_claim_missed_milestone(&customer, &escrow_id, &1_u64);
    assert!(result.is_err());
}

#[test]
fn test_claim_already_approved_milestone_fails() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(1000);

    let milestones = vec![
        &env,
        make_milestone(&env, 1, 500, Some(3000)),
        make_milestone(&env, 2, 500, None),
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &1000_u64,
        &5000_u64,
        &milestones,
    );

    env.ledger().set_timestamp(2001);
    client.approve_milestone(&admin, &escrow_id, &1_u64);

    env.ledger().set_timestamp(3001);
    let result = client.try_claim_missed_milestone(&customer, &escrow_id, &1_u64);
    assert!(result.is_err());
}

// ── Issue #689 – Multiple beneficiaries ──────────────────────────────────────

#[test]
fn test_create_escrow_with_beneficiaries_and_release() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let b1 = Address::generate(&env);
    let b2 = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let shares = vec![
        &env,
        BeneficiaryShare {
            address: b1.clone(),
            bps: 6000,
        },
        BeneficiaryShare {
            address: b2.clone(),
            bps: 4000,
        },
    ];

    let escrow_id = client.create_escrow_with_beneficiaries(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false, &shares,
    );

    // Advance past release_timestamp so release_escrow doesn't require early_release admin bypass
    env.ledger().set_timestamp(2001);
    client.release_escrow(&admin, &escrow_id, &false);
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_beneficiary_shares_not_10000_fails() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let b1 = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let shares = vec![
        &env,
        BeneficiaryShare {
            address: b1.clone(),
            bps: 5000,
        },
    ];

    let result = client.try_create_escrow_with_beneficiaries(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false, &shares,
    );
    assert!(result.is_err());
}

#[test]
fn test_duplicate_beneficiary_fails() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let b1 = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let shares = vec![
        &env,
        BeneficiaryShare {
            address: b1.clone(),
            bps: 5000,
        },
        BeneficiaryShare {
            address: b1.clone(),
            bps: 5000,
        },
    ];

    let result = client.try_create_escrow_with_beneficiaries(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false, &shares,
    );
    assert!(result.is_err());
}

// ── Issue #690 – Dispute reason code ─────────────────────────────────────────

#[test]
fn test_dispute_with_reason_stores_and_counts() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &3000_u64, &0_u64, &0_u64, &false,
    );

    let details_hash: BytesN<32> = BytesN::from_array(&env, &[0u8; 32]);
    client.dispute_escrow_with_reason(
        &customer,
        &escrow_id,
        &DisputeReason::NonDelivery,
        &details_hash,
    );

    let count = client.get_dispute_reason_count(&DisputeReason::NonDelivery);
    assert_eq!(count, 1);

    let other_count = client.get_dispute_reason_count(&DisputeReason::Fraud);
    assert_eq!(other_count, 0);
}

#[test]
fn test_dispute_with_reason_marks_disputed() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = setup_token(&env, &customer, &contract_id, 5000);
    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &3000_u64, &0_u64, &0_u64, &false,
    );

    let details_hash: BytesN<32> = BytesN::from_array(&env, &[1u8; 32]);
    client.dispute_escrow_with_reason(
        &merchant,
        &escrow_id,
        &DisputeReason::Damaged,
        &details_hash,
    );

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}
