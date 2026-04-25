#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};

// ── REPUTATION SYSTEM TESTS ──────────────────────────────────────────────────

#[test]
fn test_new_address_starts_at_neutral_score() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let address = Address::generate(&env);
    env.mock_all_auths();

    let rep = client.get_reputation(&address);
    assert_eq!(rep.score, 5000);
    assert_eq!(rep.total_transactions, 0);
    assert_eq!(rep.disputes_won, 0);
    assert_eq!(rep.disputes_lost, 0);
}

#[test]
fn test_reputation_increases_on_escrow_completion() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Use default config (completion_reward = 100).
    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);
    client.release_escrow(&admin, &escrow_id, &true);

    let merchant_rep = client.get_reputation(&merchant);
    assert_eq!(merchant_rep.score, 5100); // 5000 + 100

    let customer_rep = client.get_reputation(&customer);
    assert_eq!(customer_rep.score, 5100);
    assert_eq!(customer_rep.total_transactions, 1);
}

#[test]
fn test_reputation_config_overrides_defaults() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    client.set_reputation_config(
        &admin,
        &ReputationConfig {
            win_reward: 300,
            loss_penalty: 400,
            completion_reward: 50,
            dispute_initiation_penalty: 0,
        },
    );

    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);
    client.release_escrow(&admin, &escrow_id, &true);

    // completion_reward is 50 now.
    let merchant_rep = client.get_reputation(&merchant);
    assert_eq!(merchant_rep.score, 5050);
}

#[test]
fn test_reputation_after_dispute_win() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Default config: win_reward=200, loss_penalty=200.
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    // Admin resolves in merchant's favour.
    client.resolve_dispute(&admin, &escrow_id, &true);

    let merchant_rep = client.get_reputation(&merchant);
    assert_eq!(merchant_rep.score, 5200); // +200 win_reward
    assert_eq!(merchant_rep.disputes_won, 1);

    let customer_rep = client.get_reputation(&customer);
    assert_eq!(customer_rep.score, 4800); // -200 loss_penalty
    assert_eq!(customer_rep.disputes_lost, 1);
}

#[test]
fn test_reputation_after_dispute_loss() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&merchant, &escrow_id);

    // Admin resolves in customer's favour.
    client.resolve_dispute(&admin, &escrow_id, &false);

    let customer_rep = client.get_reputation(&customer);
    assert_eq!(customer_rep.score, 5200); // +200 win_reward
    assert_eq!(customer_rep.disputes_won, 1);

    let merchant_rep = client.get_reputation(&merchant);
    assert_eq!(merchant_rep.score, 4800); // -200 loss_penalty
    assert_eq!(merchant_rep.disputes_lost, 1);
}

#[test]
fn test_score_clamped_at_10000() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    client.set_reputation_config(
        &admin,
        &ReputationConfig {
            win_reward: 6000, // large enough to push score above 10000
            loss_penalty: 200,
            completion_reward: 100,
            dispute_initiation_penalty: 0,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);
    client.resolve_dispute(&admin, &escrow_id, &true); // merchant wins

    let merchant_rep = client.get_reputation(&merchant);
    assert_eq!(merchant_rep.score, 10000); // clamped
}

#[test]
fn test_score_clamped_at_zero() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    client.set_reputation_config(
        &admin,
        &ReputationConfig {
            win_reward: 200,
            loss_penalty: 6000, // large enough to push score below 0
            completion_reward: 100,
            dispute_initiation_penalty: 0,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);
    client.resolve_dispute(&admin, &escrow_id, &true); // merchant wins, customer loses

    let customer_rep = client.get_reputation(&customer);
    assert_eq!(customer_rep.score, 0); // clamped
}

#[test]
fn test_weighted_auto_resolve_merchant_wins_higher_reputation() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Give merchant higher reputation than customer via a prior win.
    client.set_reputation_config(
        &admin,
        &ReputationConfig {
            win_reward: 3000,   // push merchant to 8000
            loss_penalty: 3000, // push customer to 2000
            completion_reward: 0,
            dispute_initiation_penalty: 0,
        },
    );

    // First escrow to establish reputation difference.
    let escrow_id1 =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id1);
    client.resolve_dispute(&admin, &escrow_id1, &true); // merchant wins → merchant=8000, customer=2000

    // Second escrow for the weighted auto-resolve test.
    env.ledger().set_timestamp(100);
    let escrow_id2 =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id2);

    // Each party submits one piece of evidence.
    env.ledger().set_timestamp(200);
    client.submit_evidence(
        &customer,
        &escrow_id2,
        &String::from_str(&env, "ipfs://cust"),
    );
    client.submit_evidence(
        &merchant,
        &escrow_id2,
        &String::from_str(&env, "ipfs://merch"),
    );

    // After timeout, auto-resolve should favour merchant (higher reputation).
    env.ledger().set_timestamp(800); // > 200 + 500 timeout
    client.auto_resolve_dispute(&escrow_id2);

    let escrow2 = client.get_escrow(&escrow_id2);
    // merchant reputation (8000) > customer reputation (2000) → merchant wins
    assert_eq!(escrow2.status, EscrowStatus::Released);
}

#[test]
fn test_weighted_auto_resolve_customer_wins_higher_reputation() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    client.set_reputation_config(
        &admin,
        &ReputationConfig {
            win_reward: 3000,
            loss_penalty: 3000,
            completion_reward: 0,
            dispute_initiation_penalty: 0,
        },
    );

    // First escrow: customer wins → customer=8000, merchant=2000.
    let escrow_id1 =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&merchant, &escrow_id1);
    client.resolve_dispute(&admin, &escrow_id1, &false); // customer wins

    // Second escrow for weighted auto-resolve.
    env.ledger().set_timestamp(100);
    let escrow_id2 =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&merchant, &escrow_id2);

    env.ledger().set_timestamp(200);
    client.submit_evidence(
        &customer,
        &escrow_id2,
        &String::from_str(&env, "ipfs://cust"),
    );
    client.submit_evidence(
        &merchant,
        &escrow_id2,
        &String::from_str(&env, "ipfs://merch"),
    );

    env.ledger().set_timestamp(800);
    client.auto_resolve_dispute(&escrow_id2);

    let escrow2 = client.get_escrow(&escrow_id2);
    // customer reputation (8000) > merchant reputation (2000) → customer wins → Resolved
    assert_eq!(escrow2.status, EscrowStatus::Resolved);
}

#[test]
fn test_get_and_set_reputation_config() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    env.mock_all_auths();

    let config = ReputationConfig {
        win_reward: 500,
        loss_penalty: 300,
        completion_reward: 150,
        dispute_initiation_penalty: 75,
    };
    client.set_reputation_config(&admin, &config);

    let retrieved = client.get_reputation_config();
    assert_eq!(retrieved.win_reward, 500);
    assert_eq!(retrieved.loss_penalty, 300);
    assert_eq!(retrieved.completion_reward, 150);
    assert_eq!(retrieved.dispute_initiation_penalty, 75);
}

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
    let min_hold_period = 10_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );
    assert_eq!(escrow_id, 1);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.id, 1);
    assert_eq!(escrow.customer, customer);
    assert_eq!(escrow.merchant, merchant);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.release_timestamp, release_timestamp);
    assert_eq!(escrow.min_hold_period, min_hold_period);
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
    let min_hold_period = 10_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    let escrow = client.get_escrow(&escrow_id);

    assert_eq!(escrow.id, escrow_id);
    assert_eq!(escrow.customer, customer);
    assert_eq!(escrow.merchant, merchant);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.token, token);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.release_timestamp, release_timestamp);
    assert_eq!(escrow.min_hold_period, min_hold_period);
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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Release the escrow
    client.release_escrow(&admin, &escrow_id, &false);

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Try to release before release timestamp - should fail
    client.release_escrow(&admin, &escrow_id, &false);
}

#[test]
#[should_panic]
fn test_release_escrow_not_found() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.release_escrow(&admin, &999, &false);
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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Release the escrow
    client.release_escrow(&admin, &escrow_id, &false);

    // Try to release again - should fail
    client.release_escrow(&admin, &escrow_id, &false);
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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Dispute the escrow
    client.dispute_escrow(&customer, &escrow_id);

    // Try to release a disputed escrow - should fail
    client.release_escrow(&admin, &escrow_id, &false);
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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &amount,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

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
    let min_hold_period = 0_u64;

    env.mock_all_auths();

    // Create first escrow
    let escrow_id1 = client.create_escrow(
        &customer,
        &merchant1,
        &1000_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
    );
    assert_eq!(escrow_id1, 1);

    // Create second escrow
    let escrow_id2 = client.create_escrow(
        &customer,
        &merchant2,
        &2000_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
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

#[test]
fn test_submit_evidence_by_both_parties() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&customer, &escrow_id);
    env.ledger().set_timestamp(1200);
    client.submit_evidence(
        &customer,
        &escrow_id,
        &String::from_str(&env, "ipfs://hash1"),
    );
    env.ledger().set_timestamp(1300);
    client.submit_evidence(
        &merchant,
        &escrow_id,
        &String::from_str(&env, "ipfs://hash2"),
    );
    let count = client.get_evidence_count(&escrow_id);
    assert_eq!(count, 2);
    let items = client.get_evidence(&escrow_id, &10_u64, &0_u64);
    assert_eq!(items.len(), 2);
    assert_eq!(items.get(0).unwrap().submitter, customer);
    assert_eq!(items.get(1).unwrap().submitter, merchant);
}

#[test]
fn test_auto_resolve_to_customer_on_timeout() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&customer, &escrow_id);
    env.ledger().set_timestamp(1200);
    client.submit_evidence(
        &customer,
        &escrow_id,
        &String::from_str(&env, "ipfs://cust"),
    );
    env.ledger().set_timestamp(1801);
    client.auto_resolve_dispute(&escrow_id);
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_auto_resolve_to_merchant_on_timeout() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&merchant, &escrow_id);
    env.ledger().set_timestamp(1200);
    client.submit_evidence(
        &merchant,
        &escrow_id,
        &String::from_str(&env, "ipfs://merch"),
    );
    env.ledger().set_timestamp(1801);
    client.auto_resolve_dispute(&escrow_id);
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
#[should_panic]
fn test_release_blocked_by_min_hold_period() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let release_timestamp = 900_u64; // already passed
    let min_hold_period = 500_u64; // still active

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Try release before hold period ends → should fail
    client.release_escrow(&admin, &escrow_id, &false);
}

#[test]
fn test_early_release_by_admin() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let release_timestamp = 5000_u64; // future
    let min_hold_period = 5000_u64; // future

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &2000_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Admin forces early release
    client.release_escrow(&admin, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_release_after_min_hold_period() {
    let env = Env::default();

    // Created at = 1000
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let release_timestamp = 1100_u64;
    let min_hold_period = 200_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &3000_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // Move time forward past both locks
    env.ledger().set_timestamp(1300);

    client.release_escrow(&admin, &escrow_id, &false);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_release_exact_hold_period_boundary() {
    let env = Env::default();

    // Escrow created at 1000
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let release_timestamp = 900_u64; // already passed
    let min_hold_period = 500_u64;

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &1000_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    // EXACT boundary: created_at + hold
    env.ledger().set_timestamp(1500);

    client.release_escrow(&admin, &escrow_id, &false);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_escalate_dispute() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&customer, &escrow_id);
    client.escalate_dispute(&customer, &escrow_id);
    let mut escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.escalation_level, 1);
    client.escalate_dispute(&merchant, &escrow_id);
    escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.escalation_level, 2);
}

#[test]
#[should_panic]
fn test_release_when_only_release_timestamp_passed() {
    let env = Env::default();

    env.ledger().set_timestamp(2000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let release_timestamp = 1000_u64; // passed
    let min_hold_period = 3000_u64; // not passed

    env.mock_all_auths();

    let escrow_id = client.create_escrow(
        &customer,
        &merchant,
        &500_i128,
        &token,
        &release_timestamp,
        &min_hold_period,
    );

    client.release_escrow(&admin, &escrow_id, &false);
}

// ── VESTING SCHEDULE TESTS ───────────────────────────────────────────────────

#[test]
fn test_create_vesting_escrow_with_milestones() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create milestones that sum to total amount
    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 4000,
            released: false,
            description: String::from_str(&env, "Milestone 2"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 3,
            unlock_timestamp: 4000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 3"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &4000_u64,
        &milestones,
    );

    assert_eq!(escrow_id, 1);

    let vesting_schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(vesting_schedule.total_amount, 10000);
    assert_eq!(vesting_schedule.released_amount, 0);
    assert_eq!(vesting_schedule.cliff_timestamp, 1500);
    assert_eq!(vesting_schedule.end_timestamp, 4000);
    assert_eq!(vesting_schedule.milestones.len(), 3);
}

#[test]
fn test_create_vesting_escrow_time_linear() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create time-linear vesting (no milestones)
    let milestones = Vec::new(&env);
    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &2000_u64,
        &10000_u64,
        &milestones,
    );

    let vesting_schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(vesting_schedule.total_amount, 10000);
    assert_eq!(vesting_schedule.milestones.len(), 0);
}

#[test]
#[should_panic]
fn test_create_vesting_escrow_invalid_milestone_sum() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Milestones sum to 9000, but total amount is 10000 - should fail
    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 6000,
            released: false,
            description: String::from_str(&env, "Milestone 2"),
            approved_by: None,
            approved_at: None,
        },
    ];

    client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &4000_u64,
        &milestones,
    );
}

// ── MULTI-PARTY ESCROW TESTS ────────────────────────────────────────────────

#[test]
fn test_create_multi_party_escrow_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    let customer = Address::generate(&env);
    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let amount = 10000_i128;
    token_client.mint(&customer, &amount);

    let mut participants = Vec::new(&env);
    participants.push_back(Participant {
        address: p1.clone(),
        share_bps: 5000,
        role: ParticipantRole::Merchant,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p2.clone(),
        share_bps: 3000,
        role: ParticipantRole::ServiceProvider,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p3.clone(),
        share_bps: 2000,
        role: ParticipantRole::Arbitrator,
        required_approval: false,
    });

    let release_timestamp = 1000_u64;
    let escrow_id = client.create_multi_party_escrow(
        &customer,
        &participants,
        &amount,
        &token_id,
        &release_timestamp,
    );

    let escrow = client.get_multi_party_escrow(&escrow_id);
    assert_eq!(escrow.id, 1);
    assert_eq!(escrow.total_amount, amount);
    assert_eq!(escrow.required_approvals, 2);
    assert_eq!(escrow.status, EscrowStatus::Locked);

    // Verify tokens were transferred to contract
    let token_user_client = token::Client::new(&env, &token_id);
    assert_eq!(token_user_client.balance(&contract_id), amount);
}

#[test]
#[should_panic]
fn test_create_vesting_escrow_cliff_before_current_time() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Cliff timestamp is in the past - should fail
    let milestones = Vec::new(&env);
    client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &500_u64,
        &4000_u64,
        &milestones,
    );
}

#[test]
#[should_panic]
fn test_create_vesting_escrow_end_before_cliff() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // End timestamp is before cliff - should fail
    let milestones = Vec::new(&env);
    client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &5000_u64,
        &4000_u64,
        &milestones,
    );
}

#[test]
fn test_get_vested_amount_before_cliff() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = Vec::new(&env);
    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &2000_u64,
        &10000_u64,
        &milestones,
    );

    // Before cliff - should be 0
    env.ledger().set_timestamp(1500);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 0);
}

#[test]
fn test_get_vested_amount_after_cliff_linear() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = Vec::new(&env);
    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &2000_u64,
        &10000_u64,
        &milestones,
    );

    // At cliff - nothing vested yet in linear model (elapsed = 0)
    env.ledger().set_timestamp(2000);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 0);

    // Halfway through vesting period (at timestamp 6000)
    env.ledger().set_timestamp(6000);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 5000); // Half of 10000

    // After end timestamp - everything vested
    env.ledger().set_timestamp(11000);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 10000);
}

#[test]
fn test_get_vested_amount_milestone_based() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 4000,
            released: false,
            description: String::from_str(&env, "Milestone 2"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 3,
            unlock_timestamp: 4000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 3"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &4000_u64,
        &milestones,
    );

    // Before first milestone
    env.ledger().set_timestamp(1800);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 0);

    // After first milestone
    env.ledger().set_timestamp(2500);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 3000);

    // After second milestone
    env.ledger().set_timestamp(3500);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 7000);

    // After all milestones
    env.ledger().set_timestamp(4500);
    let vested_amount = client.get_vested_amount(&escrow_id);
    assert_eq!(vested_amount, 10000);
}

#[test]
fn test_get_releasable_amount() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 7000,
            released: false,
            description: String::from_str(&env, "Milestone 2"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &3000_u64,
        &milestones,
    );

    // After first milestone - releasable = vested
    env.ledger().set_timestamp(2500);
    let releasable = client.get_releasable_amount(&escrow_id);
    assert_eq!(releasable, 3000);

    // Release first milestone
    client.release_vested_amount(&admin, &escrow_id);

    // After release - releasable should be 0 until next milestone
    let releasable = client.get_releasable_amount(&escrow_id);
    assert_eq!(releasable, 0);

    // After second milestone
    env.ledger().set_timestamp(3500);
    let releasable = client.get_releasable_amount(&escrow_id);
    assert_eq!(releasable, 7000);
}

#[test]
fn test_release_vested_amount_milestone() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 3000,
            released: false,
            description: String::from_str(&env, "Milestone 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 7000,
            released: false,
            description: String::from_str(&env, "Milestone 2"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &3000_u64,
        &milestones,
    );

    // Try to release before cliff - should fail
    env.ledger().set_timestamp(1400);
    let result = client.try_release_vested_amount(&admin, &escrow_id);
    assert!(result.is_err());

    // After first milestone
    env.ledger().set_timestamp(2500);
    let released_amount = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released_amount, 3000);

    // Verify vesting schedule updated
    let vesting_schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(vesting_schedule.released_amount, 3000);

    // After second milestone
    env.ledger().set_timestamp(3500);
    let released_amount = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released_amount, 7000);

    // All released
    let vesting_schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(vesting_schedule.released_amount, 10000);
}

#[test]
#[should_panic]
fn test_release_vested_amount_before_cliff() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = Vec::new(&env);
    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &2000_u64,
        &10000_u64,
        &milestones,
    );

    // Try to release before cliff
    env.ledger().set_timestamp(1500);
    client.release_vested_amount(&admin, &escrow_id);
}

#[test]
#[should_panic]
fn test_create_multi_party_escrow_invalid_shares() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let customer = Address::generate(&env);
    let token_id = Address::generate(&env);

    let mut participants = Vec::new(&env);
    participants.push_back(Participant {
        address: Address::generate(&env),
        share_bps: 5000,
        role: ParticipantRole::Merchant,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: Address::generate(&env),
        share_bps: 4000, // Sum is 9000, should fail
        role: ParticipantRole::Merchant,
        required_approval: true,
    });

    client.create_multi_party_escrow(&customer, &participants, &1000, &token_id, &1000);
}

#[test]
fn test_approve_release_success() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let customer = Address::generate(&env);
    token_client.mint(&customer, &10000);

    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);

    let mut participants = Vec::new(&env);
    participants.push_back(Participant {
        address: p1.clone(),
        share_bps: 5000,
        role: ParticipantRole::Merchant,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p2.clone(),
        share_bps: 5000,
        role: ParticipantRole::ServiceProvider,
        required_approval: true,
    });

    let escrow_id =
        client.create_multi_party_escrow(&customer, &participants, &10000, &token_id, &1000);

    client.approve_release(&p1, &escrow_id);
    let escrow = client.get_multi_party_escrow(&escrow_id);
    assert_eq!(escrow.approvals.len(), 1);
    assert_eq!(escrow.approvals.get(0).unwrap(), p1);

    client.approve_release(&p2, &escrow_id);
    let escrow = client.get_multi_party_escrow(&escrow_id);
    assert_eq!(escrow.approvals.len(), 2);
}

#[test]
fn test_release_multi_party_escrow_success() {
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

    let customer = Address::generate(&env);
    token_client.mint(&customer, &10000);

    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);
    let p3 = Address::generate(&env);

    let mut participants = Vec::new(&env);
    participants.push_back(Participant {
        address: p1.clone(),
        share_bps: 5000, // 5000
        role: ParticipantRole::Merchant,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p2.clone(),
        share_bps: 3000, // 3000
        role: ParticipantRole::ServiceProvider,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p3.clone(),
        share_bps: 2000, // 2000
        role: ParticipantRole::Arbitrator,
        required_approval: false,
    });

    env.ledger().set_timestamp(500);
    let escrow_id =
        client.create_multi_party_escrow(&customer, &participants, &10000, &token_id, &1000);

    client.approve_release(&p1, &escrow_id);
    client.approve_release(&p2, &escrow_id);

    env.ledger().set_timestamp(1001);
    client.release_multi_party_escrow(&escrow_id);

    let escrow = client.get_multi_party_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);

    assert_eq!(token_user_client.balance(&p1), 5000);
    assert_eq!(token_user_client.balance(&p2), 3000);
    assert_eq!(token_user_client.balance(&p3), 2000);
    assert_eq!(token_user_client.balance(&contract_id), 0);
}

#[test]
#[should_panic]
fn test_release_vested_amount_nothing_to_release() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 10000,
            released: false,
            description: String::from_str(&env, "Milestone 1"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &2000_u64,
        &milestones,
    );

    // Before milestone unlocks
    env.ledger().set_timestamp(1800);
    client.release_vested_amount(&admin, &escrow_id);
}

#[test]
fn test_full_vesting_completion() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 2500,
            released: false,
            description: String::from_str(&env, "Phase 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 2500,
            released: false,
            description: String::from_str(&env, "Phase 2"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 3,
            unlock_timestamp: 4000,
            amount: 2500,
            released: false,
            description: String::from_str(&env, "Phase 3"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 4,
            unlock_timestamp: 5000,
            amount: 2500,
            released: false,
            description: String::from_str(&env, "Phase 4"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &5000_u64,
        &milestones,
    );

    // Release each milestone as it unlocks
    env.ledger().set_timestamp(2500);
    let released = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released, 2500);

    env.ledger().set_timestamp(3500);
    let released = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released, 2500);

    env.ledger().set_timestamp(4500);
    let released = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released, 2500);

    env.ledger().set_timestamp(5500);
    let released = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released, 2500);

    // Verify all released
    let vesting_schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(vesting_schedule.released_amount, 10000);
    assert_eq!(vesting_schedule.total_amount, 10000);
}

#[test]
fn test_partial_milestone_release() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 5000,
            released: false,
            description: String::from_str(&env, "First half"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 5000,
            released: false,
            description: String::from_str(&env, "Second half"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &3000_u64,
        &milestones,
    );

    // Only first milestone unlocked
    env.ledger().set_timestamp(2500);
    let released = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released, 5000);

    // Try to release again before second milestone - should fail
    let result = client.try_release_vested_amount(&admin, &escrow_id);
    assert!(result.is_err());

    // Second milestone unlocks
    env.ledger().set_timestamp(3500);
    let released = client.release_vested_amount(&admin, &escrow_id);
    assert_eq!(released, 5000);
}

#[test]
#[should_panic]
fn test_release_multi_party_escrow_threshold_not_met() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    let customer = Address::generate(&env);
    token_client.mint(&customer, &10000);

    let p1 = Address::generate(&env);
    let p2 = Address::generate(&env);

    let mut participants = Vec::new(&env);
    participants.push_back(Participant {
        address: p1.clone(),
        share_bps: 5000,
        role: ParticipantRole::Merchant,
        required_approval: true,
    });
    participants.push_back(Participant {
        address: p2.clone(),
        share_bps: 5000,
        role: ParticipantRole::ServiceProvider,
        required_approval: true,
    });

    let escrow_id =
        client.create_multi_party_escrow(&customer, &participants, &10000, &token_id, &1000);

    client.approve_release(&p1, &escrow_id);
    // Only 1 approval, 2 required

    env.ledger().set_timestamp(1001);
    client.release_multi_party_escrow(&escrow_id);
}

// ── MULTI-SIG ADMIN TESTS ────────────────────────────────────────────────────

#[test]
fn test_multisig_initialize() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let config = client.get_multisig_config();
    assert_eq!(config.total_admins, 1);
    assert_eq!(config.required_signatures, 1);
    assert!(config.admins.contains(&admin));
}

#[test]
fn test_multisig_propose_release_escrow() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);

    // Encode escrow_id as 8 big-endian bytes + 1 byte for early_release=true
    let mut data_bytes = [0u8; 9];
    let id_bytes = escrow_id.to_be_bytes();
    for i in 0..8 {
        data_bytes[i] = id_bytes[i];
    }
    data_bytes[8] = 1u8; // early_release = true
    let data = soroban_sdk::Bytes::from_slice(&env, &data_bytes);

    let proposal_id = client.propose_action(&admin, &ActionType::ReleaseEscrow, &merchant, &data);

    // proposal id should be "1"
    assert_eq!(proposal_id, String::from_str(&env, "1"));
}

#[test]
fn test_multisig_approve_and_execute() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);

    // Encode escrow_id + early_release=true
    let mut data_bytes = [0u8; 9];
    let id_bytes = escrow_id.to_be_bytes();
    for i in 0..8 {
        data_bytes[i] = id_bytes[i];
    }
    data_bytes[8] = 1u8;
    let data = soroban_sdk::Bytes::from_slice(&env, &data_bytes);

    let proposal_id = client.propose_action(&admin, &ActionType::ReleaseEscrow, &merchant, &data);

    // With required_signatures=1 and 1 approval already from proposer, execute directly
    client.execute_action(&proposal_id);

    // Verify escrow was released by querying via client
    let escrow = env.as_contract(&contract_id, || EscrowContract::get_escrow(&env, escrow_id));
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
#[should_panic]
fn test_multisig_duplicate_approval_rejected() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let _token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2_u32);

    let data = soroban_sdk::Bytes::from_slice(&env, &[0u8; 9]);
    let proposal_id = client.propose_action(&admin, &ActionType::ReleaseEscrow, &admin2, &data);

    // admin already approved when proposing, approving again should panic
    client.approve_action(&admin, &proposal_id);
}

#[test]
fn test_multisig_proposal_expires() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2_u32);

    env.ledger().set_timestamp(1000);
    let data = soroban_sdk::Bytes::from_slice(&env, &[0u8; 9]);
    let proposal_id = client.propose_action(&admin, &ActionType::ReleaseEscrow, &admin2, &data);

    // Advance past TTL (604800 seconds = 7 days)
    env.ledger().set_timestamp(1000 + 604801);

    // Approving an expired proposal should fail
    let result = client.try_approve_action(&admin2, &proposal_id);
    assert!(result.is_err());
}

#[test]
#[should_panic]
fn test_multisig_threshold_not_met() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2_u32);

    let data = soroban_sdk::Bytes::from_slice(&env, &[0u8; 9]);
    let proposal_id = client.propose_action(&admin, &ActionType::ReleaseEscrow, &admin2, &data);

    // Only 1 approval (from proposer), threshold is 2 — should panic
    client.execute_action(&proposal_id);
}

#[test]
fn test_multisig_add_admin() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &new_admin);

    let config = client.get_multisig_config();
    assert_eq!(config.total_admins, 2);
    assert!(config.admins.contains(&new_admin));
}

#[test]
#[should_panic]
fn test_multisig_remove_admin_drops_below_threshold() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    // With 1 admin and required_signatures=1, removing admin drops below threshold
    client.remove_admin(&admin, &admin);
}

#[test]
fn test_multisig_reject_action() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2_u32);

    let data = soroban_sdk::Bytes::from_slice(&env, &[0u8; 9]);
    let proposal_id = client.propose_action(&admin, &ActionType::ReleaseEscrow, &admin2, &data);

    client.reject_action(&admin2, &proposal_id);

    // After rejection, execute should fail
    let result = client.try_execute_action(&proposal_id);
    assert!(result.is_err());
}

#[test]
fn test_multisig_resolve_dispute_via_proposal() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    env.ledger().set_timestamp(2000);
    let escrow_id =
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    // Encode escrow_id + release_to_merchant=false (0)
    let mut data_bytes = [0u8; 9];
    let id_bytes = escrow_id.to_be_bytes();
    for i in 0..8 {
        data_bytes[i] = id_bytes[i];
    }
    data_bytes[8] = 0u8; // release to customer
    let data = soroban_sdk::Bytes::from_slice(&env, &data_bytes);

    let proposal_id = client.propose_action(&admin, &ActionType::ResolveDispute, &customer, &data);

    client.execute_action(&proposal_id);

    let escrow = env.as_contract(&contract_id, || EscrowContract::get_escrow(&env, escrow_id));
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_insurance_system() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let config = InsuranceConfig {
        premium_bps: 100, // 1%
        max_coverage_bps: 5000, // 50%
        enabled: true,
    };
    client.set_insurance_config(&admin, &config);

    let escrow_id = client.create_escrow(&customer, &merchant, &10000_i128, &token, &2000_u64, &0_u64);
    client.opt_into_insurance(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.amount, 9900);

    let pool = client.get_insurance_pool();
    assert_eq!(pool.balance, 100);

    client.refund_escrow(&customer, &escrow_id);
    
    let claim_id = client.file_insurance_claim(&admin, &escrow_id, &100_i128);
    assert_eq!(claim_id, 1);

    client.approve_claim(&admin, &claim_id);
    
    let final_pool = client.get_insurance_pool();
    assert_eq!(final_pool.balance, 0);
    assert_eq!(final_pool.total_claims_paid, 100);
}

// ── #74 TIMELOCK TESTS ───────────────────────────────────────────────────────

#[test]
fn test_timelock_execute_after_expiry_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    // Set a short timelock (1 hour delay, 1 hour grace)
    client.set_timelock_config(&admin, &TimeLockConfig { delay: 3600, grace_period: 3600 });

    let escrow_id = client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    let action_id = client.queue_action(
        &admin,
        &escrow_id,
        &EscrowActionType::ResolveDispute(true),
        &soroban_sdk::Bytes::new(&env),
    );

    // Advance past grace period (delay 3600 + grace 3600 = 7200 seconds)
    env.ledger().set_timestamp(1000 + 7201);

    let result = client.try_execute_queued_action(&action_id);
    assert_eq!(result, Err(Ok(Error::ActionExpired)));
}

#[test]
fn test_timelock_execute_after_delay_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);


    // Set a 1-hour timelock, 24-hour grace period
    client.set_timelock_config(&admin, &TimeLockConfig { delay: 3600, grace_period: 86400 });

    let escrow_id = client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    let action_id = client.queue_action(
        &admin,
        &escrow_id,
        &EscrowActionType::ResolveDispute(true),
        &soroban_sdk::Bytes::new(&env),
    );

    // Advance past delay but within grace period
    env.ledger().set_timestamp(1000 + 3601);

    client.execute_queued_action(&action_id);

    let escrow = env.as_contract(&contract_id, || EscrowContract::get_escrow(&env, escrow_id));
    assert_eq!(escrow.status, EscrowStatus::Released);
}



#[test]
fn test_timelock_cancel_by_any_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin1);
    client.add_admin(&admin1, &admin2);

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);

    // admin1 queues the action
    let action_id = client.queue_action(
        &admin1,
        &escrow_id,
        &EscrowActionType::ForceRelease,
        &soroban_sdk::Bytes::new(&env),
    );

    // admin2 (not the proposer) can cancel it
    client.cancel_queued_action(&admin2, &action_id);

    let action = client.get_queued_action(&action_id);
    assert!(action.cancelled);
}

// ── #75 REPUTATION DECAY TESTS ───────────────────────────────────────────────

#[test]
fn test_update_decay_config() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let config = ReputationDecayConfig {
        decay_rate_bps: 200,
        decay_threshold_days: 7,
        min_score: 1000,
        max_score: 9000,
    };
    client.update_decay_config(&admin, &config);
}

#[test]
fn test_get_effective_reputation_no_decay_within_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let merchant = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    // Set 30-day threshold
    client.update_decay_config(
        &admin,
        &ReputationDecayConfig {
            decay_rate_bps: 100,
            decay_threshold_days: 30,
            min_score: 0,
            max_score: 10000,
        },
    );

    // Create and release escrow to give user a score update at t=1000
    let escrow_id = client.create_escrow(&user, &merchant, &100_i128, &token, &500_u64, &0_u64);
    let _ = escrow_id; // Score updated at t=1000, default 5000+completion_reward

    // Advance only 10 days — below the 30-day threshold
    env.ledger().set_timestamp(1000 + 10 * 86400);

    let rep = client.get_reputation(&user);
    let effective = client.get_effective_reputation(&user);

    // score hasn't changed because no reputation was explicitly set; but effective should match
    assert_eq!(effective, rep.score as i128);
}

#[test]
fn test_get_effective_reputation_decays_after_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Set last_updated at t=0
    env.ledger().set_timestamp(0);
    client.initialize(&admin);

    // Set threshold to 1 day and rate of 1% per day
    client.update_decay_config(
        &admin,
        &ReputationDecayConfig {
            decay_rate_bps: 100,
            decay_threshold_days: 1,
            min_score: 0,
            max_score: 10000,
        },
    );

    // user starts with neutral score (last_updated=0)
    let user = customer.clone();

    // Advance 11 days past threshold (10 days of decay)
    env.ledger().set_timestamp(11 * 86400);

    let rep = client.get_reputation(&user); // score=5000, last_updated=0
    let effective = client.get_effective_reputation(&user);

    // 10 days * 1% per day = 10% of 5000 = 500 decay
    let expected_decay = (rep.score as i128) * 100 * 10 / 10000;
    let expected_score = rep.score as i128 - expected_decay;
    assert_eq!(effective, expected_score);
    let _ = (merchant, token); // silence unused warnings
}

#[test]
fn test_apply_reputation_decay_persists() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let user = Address::generate(&env);

    env.ledger().set_timestamp(0);
    client.update_decay_config(
        &admin,
        &ReputationDecayConfig {
            decay_rate_bps: 100,
            decay_threshold_days: 1,
            min_score: 0,
            max_score: 10000,
        },
    );

    // Advance 11 days — 10 days of decay at 1% per day
    env.ledger().set_timestamp(11 * 86400);

    let effective_before = client.get_effective_reputation(&user);
    client.apply_reputation_decay(&user);
    let rep_after = client.get_reputation(&user);

    assert_eq!(rep_after.score as i128, effective_before);
}

#[test]
fn test_reputation_floor_enforcement() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let user = Address::generate(&env);

    env.ledger().set_timestamp(0);
    // Very high decay rate + long time → floor should kick in
    client.update_decay_config(
        &admin,
        &ReputationDecayConfig {
            decay_rate_bps: 10000, // 100% per day
            decay_threshold_days: 1,
            min_score: 1000, // floor at 1000
            max_score: 10000,
        },
    );

    // Advance 10 days — would decay by 1000% but floor is 1000
    env.ledger().set_timestamp(10 * 86400);

    let effective = client.get_effective_reputation(&user);
    assert_eq!(effective, 1000); // clamped to min_score
}

// ── #85 ORACLE CONDITION TESTS ───────────────────────────────────────────────

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(env: Env, _feed_id: BytesN<32>) -> OraclePriceData {
        env.storage()
            .instance()
            .get::<u32, OraclePriceData>(&0u32)
            .expect("price not set")
    }

    pub fn set_price(env: Env, data: OraclePriceData) {
        env.storage().instance().set(&0u32, &data);
    }
}

#[test]
fn test_attach_and_get_oracle_condition() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle_id = env.register(MockOracle, ());

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);

    let condition = OracleCondition {
        escrow_id,
        oracle: OracleConfig {
            oracle_address: oracle_id.clone(),
            price_feed_id: BytesN::from_array(&env, &[0u8; 32]),
            staleness_threshold: 3600,
        },
        target_price: 1000,
        comparison: PriceComparison::GreaterThan,
        release_to_merchant_if_met: true,
    };

    client.attach_oracle_condition(&admin, &escrow_id, &condition);

    let stored = client.get_oracle_condition(&escrow_id);
    assert_eq!(stored.target_price, 1000);
    assert_eq!(stored.release_to_merchant_if_met, true);
}

#[test]
fn test_oracle_auto_resolve_condition_met() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle_id = env.register(MockOracle, ());
    let oracle_client = MockOracleClient::new(&env, &oracle_id);

    env.ledger().set_timestamp(5000);
    client.initialize(&admin);

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);

    // Attach condition while escrow is Locked (before dispute)
    oracle_client.set_price(&OraclePriceData {
        price: 2000,
        timestamp: 4500,
    });
    let condition = OracleCondition {
        escrow_id,
        oracle: OracleConfig {
            oracle_address: oracle_id,
            price_feed_id: BytesN::from_array(&env, &[0u8; 32]),
            staleness_threshold: 3600,
        },
        target_price: 1000,
        comparison: PriceComparison::GreaterThan, // 2000 > 1000 → met
        release_to_merchant_if_met: true,
    };
    client.attach_oracle_condition(&admin, &escrow_id, &condition);

    client.dispute_escrow(&customer, &escrow_id);
    client.auto_resolve_with_oracle(&escrow_id);

    let escrow = env.as_contract(&contract_id, || EscrowContract::get_escrow(&env, escrow_id));
    assert_eq!(escrow.status, EscrowStatus::Released); // condition met → merchant
}

#[test]
fn test_oracle_auto_resolve_stale_price_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle_id = env.register(MockOracle, ());
    let oracle_client = MockOracleClient::new(&env, &oracle_id);

    env.ledger().set_timestamp(10000);
    client.initialize(&admin);

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);

    // Set stale price and attach condition before dispute
    oracle_client.set_price(&OraclePriceData {
        price: 2000,
        timestamp: 0,
    });
    let condition = OracleCondition {
        escrow_id,
        oracle: OracleConfig {
            oracle_address: oracle_id,
            price_feed_id: BytesN::from_array(&env, &[0u8; 32]),
            staleness_threshold: 3600,
        },
        target_price: 1000,
        comparison: PriceComparison::GreaterThan,
        release_to_merchant_if_met: true,
    };
    client.attach_oracle_condition(&admin, &escrow_id, &condition);

    client.dispute_escrow(&customer, &escrow_id);

    // current time 10000, price timestamp 0, threshold 3600 → stale
    let result = client.try_auto_resolve_with_oracle(&escrow_id);
    assert_eq!(result, Err(Ok(Error::OracleStalePriceFeed)));
}

#[test]
fn test_oracle_auto_resolve_condition_not_met_releases_to_customer() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle_id = env.register(MockOracle, ());
    let oracle_client = MockOracleClient::new(&env, &oracle_id);

    env.ledger().set_timestamp(5000);
    client.initialize(&admin);

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);

    // Attach condition while Locked, then dispute
    oracle_client.set_price(&OraclePriceData {
        price: 500,
        timestamp: 4800,
    });
    let condition = OracleCondition {
        escrow_id,
        oracle: OracleConfig {
            oracle_address: oracle_id,
            price_feed_id: BytesN::from_array(&env, &[0u8; 32]),
            staleness_threshold: 3600,
        },
        target_price: 1000,
        comparison: PriceComparison::GreaterThan, // 500 > 1000 → NOT met
        release_to_merchant_if_met: true,         // not met → customer
    };
    client.attach_oracle_condition(&admin, &escrow_id, &condition);

    client.dispute_escrow(&customer, &escrow_id);
    client.auto_resolve_with_oracle(&escrow_id);

    let escrow = env.as_contract(&contract_id, || EscrowContract::get_escrow(&env, escrow_id));
    assert_eq!(escrow.status, EscrowStatus::Resolved); // condition NOT met → customer wins
}

// ── #86 ANALYTICS TESTS ───────────────────────────────────────────────────────

#[test]
fn test_analytics_increments_on_create() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);
    client.create_escrow(&customer, &merchant, &300_i128, &token, &9999_u64, &0_u64);

    let analytics = client.get_escrow_analytics();
    assert_eq!(analytics.total_escrows_created, 2);
    assert_eq!(analytics.total_value_locked, 800);
}

#[test]
fn test_analytics_dispute_rate_bps() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    let e1 = client.create_escrow(&customer, &merchant, &100_i128, &token, &9999_u64, &0_u64);
    let e2 = client.create_escrow(&customer, &merchant, &100_i128, &token, &9999_u64, &0_u64);
    let _ = e2;
    client.dispute_escrow(&customer, &e1); // 1 dispute out of 2 escrows

    let analytics = client.get_escrow_analytics();
    assert_eq!(analytics.total_disputes, 1);
    assert_eq!(analytics.dispute_rate_bps, 5000); // 1/2 * 10000
}

#[test]
fn test_per_address_merchant_analytics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other_merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    // 2 escrows with merchant, 1 with other_merchant
    client.create_escrow(&customer, &merchant, &200_i128, &token, &9999_u64, &0_u64);
    client.create_escrow(&customer, &merchant, &300_i128, &token, &9999_u64, &0_u64);
    client.create_escrow(
        &customer,
        &other_merchant,
        &400_i128,
        &token,
        &9999_u64,
        &0_u64,
    );

    let m_analytics = client.get_merchant_analytics(&merchant);
    assert_eq!(m_analytics.total_escrows_created, 2);
    assert_eq!(m_analytics.total_value_locked, 500);

    let other_analytics = client.get_merchant_analytics(&other_merchant);
    assert_eq!(other_analytics.total_escrows_created, 1);
    assert_eq!(other_analytics.total_value_locked, 400);
}

#[test]
fn test_per_address_customer_analytics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    client.create_escrow(&customer, &merchant, &100_i128, &token, &9999_u64, &0_u64);
    let e2 = client.create_escrow(&customer, &merchant, &200_i128, &token, &9999_u64, &0_u64);
    client.dispute_escrow(&customer, &e2);

    let c_analytics = client.get_customer_analytics(&customer);
    assert_eq!(c_analytics.total_escrows_created, 2);
    assert_eq!(c_analytics.total_value_locked, 300);
    assert_eq!(c_analytics.total_disputes, 1);
}

#[test]
fn test_reset_analytics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    client.create_escrow(&customer, &merchant, &500_i128, &token, &9999_u64, &0_u64);
    let before = client.get_escrow_analytics();
    assert_eq!(before.total_escrows_created, 1);

    client.reset_analytics(&admin);

    let after = client.get_escrow_analytics();
    assert_eq!(after.total_escrows_created, 0);
    assert_eq!(after.total_value_locked, 0);
}

#[test]
fn test_analytics_avg_duration_updated_on_release() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.initialize(&admin);

    // Set short timelock so we can release via timelock
    client.set_timelock_config(
        &admin,
        &TimeLockConfig {
            delay: 3600,
            grace_period: 86400,
        },
    );

    let escrow_id =
        client.create_escrow(&customer, &merchant, &500_i128, &token, &1001_u64, &0_u64);

    let action_id = client.queue_action(
        &admin,
        &escrow_id,
        &EscrowActionType::ForceRelease,
        &soroban_sdk::Bytes::new(&env),
    );

    // Advance 4000 seconds past the creation time (1000): total 5000
    env.ledger().set_timestamp(5000);
    client.execute_queued_action(&action_id);

    let analytics = client.get_escrow_analytics();
    assert_eq!(analytics.total_escrows_released, 1);
    assert_eq!(analytics.total_value_released, 500);
    // duration = 5000 - 1000 = 4000 seconds
    assert_eq!(analytics.avg_escrow_duration_seconds, 4000);
}

// ── PARTIAL MILESTONE RELEASE TESTS (#80) ────────────────────────────────────

fn setup_vesting_with_milestones(
    env: &Env,
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
) -> u64 {
    let milestones = vec![
        env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 4000,
            released: false,
            description: String::from_str(env, "Deliverable 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 6000,
            released: false,
            description: String::from_str(env, "Deliverable 2"),
            approved_by: None,
            approved_at: None,
        },
    ];
    client.create_vesting_escrow(
        customer,
        merchant,
        &10000_i128,
        token,
        &1500_u64,
        &3000_u64,
        &milestones,
    )
}

#[test]
fn test_approve_milestone_success() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
#[test]
fn test_insurance_underfunded() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    client.set_insurance_config(&admin, &InsuranceConfig {
        premium_bps: 10,
        max_coverage_bps: 1000,
        enabled: true,
    });

    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &2000_u64, &0_u64);
    client.opt_into_insurance(&escrow_id);

    client.refund_escrow(&customer, &escrow_id);
    
    let claim_id = client.file_insurance_claim(&admin, &escrow_id, &50_i128);
    let result = client.try_approve_claim(&admin, &claim_id);
    
    assert!(result.is_err());
}

// ── #92 WATCHDOG TESTS ───────────────────────────────────────────────────────

#[test]
fn test_watchdog_release_eligible() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    // Admin approves milestone 1
    env.ledger().set_timestamp(1800);
    client.approve_milestone(&admin, &escrow_id, &1_u64);

    let schedule = client.get_vesting_schedule(&escrow_id);
    let m = schedule.milestones.get(0).unwrap();
    assert_eq!(m.milestone_id, 1);
    assert!(m.approved_by.is_some());
    assert_eq!(m.approved_by.unwrap(), admin);
    assert!(m.approved_at.is_some());
}

#[test]
fn test_release_milestone_success() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    client.set_watchdog_config(&admin, &WatchdogConfig {
        inactivity_release_seconds: 100,
        enabled: true,
        favor_customer_on_release: false, // releases to merchant
    });

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1100_u64, &0_u64);

    // After release_timestamp (1100) + inactivity (100) = 1200
    env.ledger().set_timestamp(1201);
    
    assert!(client.is_watchdog_eligible(&escrow_id));
    client.trigger_watchdog_release(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_watchdog_ineligible_dispute() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    // Approve then release milestone 1
    env.ledger().set_timestamp(2500);
    client.approve_milestone(&admin, &escrow_id, &1_u64);
    let released = client.release_milestone(&escrow_id, &1_u64);
    assert_eq!(released, 4000);

    // Verify schedule updated
    let schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(schedule.released_amount, 4000);

    // Milestone 1 should be marked released
    let m = schedule.milestones.get(0).unwrap();
    assert!(m.released);
}

#[test]
fn test_release_milestone_transfers_exact_amount() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Use a real token so we can verify balances
    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_asset_client = token::StellarAssetClient::new(&env, &token_id);
    let token_client = token::Client::new(&env, &token_id);

    env.mock_all_auths();
    client.initialize(&admin);

    token_asset_client.mint(&customer, &10000);

    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 4000,
            released: false,
            description: String::from_str(&env, "Deliverable 1"),
            approved_by: None,
            approved_at: None,
        },
        VestingMilestone {
            milestone_id: 2,
            unlock_timestamp: 3000,
            amount: 6000,
            released: false,
            description: String::from_str(&env, "Deliverable 2"),
            approved_by: None,
            approved_at: None,
        },
    ];

    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token_id,
        &1500_u64,
        &3000_u64,
        &milestones,
    );

    env.ledger().set_timestamp(2500);
    client.approve_milestone(&admin, &escrow_id, &1_u64);
    client.release_milestone(&escrow_id, &1_u64);

    // Merchant receives exactly 4000
    assert_eq!(token_client.balance(&merchant), 4000);

    // Release second milestone
    env.ledger().set_timestamp(3500);
    client.approve_milestone(&admin, &escrow_id, &2_u64);
    client.release_milestone(&escrow_id, &2_u64);

    assert_eq!(token_client.balance(&merchant), 10000);
}

#[test]
fn test_release_unapproved_milestone_fails() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    // Attempt release without approval
    env.ledger().set_timestamp(2500);
    let result = client.try_release_milestone(&escrow_id, &1_u64);
    client.set_watchdog_config(&admin, &WatchdogConfig {
        inactivity_release_seconds: 100,
        enabled: true,
        favor_customer_on_release: false,
    });

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1100_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id);

    env.ledger().set_timestamp(1201);
    
    assert!(!client.is_watchdog_eligible(&escrow_id));
    let result = client.try_trigger_watchdog_release(&escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_release_already_released_milestone_fails() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
fn test_watchdog_premature_trigger() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(2500);
    client.approve_milestone(&admin, &escrow_id, &1_u64);
    client.release_milestone(&escrow_id, &1_u64);

    // Duplicate release attempt must fail
    let result = client.try_release_milestone(&escrow_id, &1_u64);
    client.set_watchdog_config(&admin, &WatchdogConfig {
        inactivity_release_seconds: 1000,
        enabled: true,
        favor_customer_on_release: false,
    });

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &2000_u64, &0_u64);

    // Before inactivity window (2000 + 1000 = 3000)
    env.ledger().set_timestamp(2500);
    
    assert!(!client.is_watchdog_eligible(&escrow_id));
    let result = client.try_trigger_watchdog_release(&escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_approve_already_released_milestone_fails() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
fn test_watchdog_favor_customer() {
    let env = Env::default();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    env.ledger().set_timestamp(2500);
    client.approve_milestone(&admin, &escrow_id, &1_u64);
    client.release_milestone(&escrow_id, &1_u64);

    // Re-approving a released milestone must fail
    let result = client.try_approve_milestone(&admin, &escrow_id, &1_u64);
    assert!(result.is_err());
}

#[test]
fn test_release_milestone_overflow_guard() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    // Create a schedule where milestones sum exactly to total_amount
    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 10000,
            released: false,
            description: String::from_str(&env, "Full amount"),
            approved_by: None,
            approved_at: None,
        },
    ];
    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &2000_u64,
        &milestones,
    );

    // Manually try to add a second milestone that would overflow via add_milestone
    let overflow_milestone = VestingMilestone {
        milestone_id: 2,
        unlock_timestamp: 2000,
        amount: 1, // even 1 extra would overflow
        released: false,
        description: String::from_str(&env, "Overflow"),
        approved_by: None,
        approved_at: None,
    };
    let result = client.try_add_milestone(&admin, &escrow_id, &overflow_milestone);
    assert!(result.is_err());
}

#[test]
fn test_get_pending_milestones() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    // Both milestones pending initially
    let pending = client.get_pending_milestones(&escrow_id);
    assert_eq!(pending.len(), 2);

    // Release milestone 1
    env.ledger().set_timestamp(2500);
    client.approve_milestone(&admin, &escrow_id, &1_u64);
    client.release_milestone(&escrow_id, &1_u64);

    // Only milestone 2 should remain pending
    let pending = client.get_pending_milestones(&escrow_id);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending.get(0).unwrap().milestone_id, 2);
}

#[test]
fn test_add_milestone_success() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    // Create with milestones summing to 8000 (leaving 2000 headroom)
    let milestones = vec![
        &env,
        VestingMilestone {
            milestone_id: 1,
            unlock_timestamp: 2000,
            amount: 8000,
            released: false,
            description: String::from_str(&env, "Main deliverable"),
            approved_by: None,
            approved_at: None,
        },
    ];
    let escrow_id = client.create_vesting_escrow(
        &customer,
        &merchant,
        &10000_i128,
        &token,
        &1500_u64,
        &3000_u64,
        &milestones,
    );

    // Add a new milestone for the remaining 2000
    let new_milestone = VestingMilestone {
        milestone_id: 0, // auto-assigned
        unlock_timestamp: 3000,
        amount: 2000,
        released: false,
        description: String::from_str(&env, "Bonus deliverable"),
        approved_by: None,
        approved_at: None,
    };
    client.add_milestone(&admin, &escrow_id, &new_milestone);

    let schedule = client.get_vesting_schedule(&escrow_id);
    assert_eq!(schedule.milestones.len(), 2);
}

#[test]
fn test_non_admin_cannot_approve_milestone() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let escrow_id = setup_vesting_with_milestones(&env, &client, &customer, &merchant, &token);

    let result = client.try_approve_milestone(&non_admin, &escrow_id, &1_u64);
    assert!(result.is_err());
    client.set_watchdog_config(&admin, &WatchdogConfig {
        inactivity_release_seconds: 100,
        enabled: true,
        favor_customer_on_release: true, // releases to customer
    });

    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1100_u64, &0_u64);

    env.ledger().set_timestamp(1201);
    
    client.trigger_watchdog_release(&escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}
