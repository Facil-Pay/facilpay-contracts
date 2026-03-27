#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{testutils::Address as _, Address, Env};

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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);
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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64);
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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
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

    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
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

    let escrow_id = client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
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

    let escrow_id = client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
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
            win_reward: 3000, // push merchant to 8000
            loss_penalty: 3000, // push customer to 2000
            completion_reward: 0,
            dispute_initiation_penalty: 0,
        },
    );

    // First escrow to establish reputation difference.
    let escrow_id1 = client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id1);
    client.resolve_dispute(&admin, &escrow_id1, &true); // merchant wins → merchant=8000, customer=2000

    // Second escrow for the weighted auto-resolve test.
    env.ledger().set_timestamp(100);
    let escrow_id2 = client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&customer, &escrow_id2);

    // Each party submits one piece of evidence.
    env.ledger().set_timestamp(200);
    client.submit_evidence(&customer, &escrow_id2, &String::from_str(&env, "ipfs://cust"));
    client.submit_evidence(&merchant, &escrow_id2, &String::from_str(&env, "ipfs://merch"));

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
    let escrow_id1 = client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&merchant, &escrow_id1);
    client.resolve_dispute(&admin, &escrow_id1, &false); // customer wins

    // Second escrow for weighted auto-resolve.
    env.ledger().set_timestamp(100);
    let escrow_id2 = client.create_escrow(&customer, &merchant, &500_i128, &token, &5000_u64, &0_u64);
    client.dispute_escrow(&merchant, &escrow_id2);

    env.ledger().set_timestamp(200);
    client.submit_evidence(&customer, &escrow_id2, &String::from_str(&env, "ipfs://cust"));
    client.submit_evidence(&merchant, &escrow_id2, &String::from_str(&env, "ipfs://merch"));

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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&customer, &escrow_id);
    env.ledger().set_timestamp(1200);
    client.submit_evidence(&customer, &escrow_id, &String::from_str(&env, "ipfs://hash1"));
    env.ledger().set_timestamp(1300);
    client.submit_evidence(&merchant, &escrow_id, &String::from_str(&env, "ipfs://hash2"));
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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&customer, &escrow_id);
    env.ledger().set_timestamp(1200);
    client.submit_evidence(&customer, &escrow_id, &String::from_str(&env, "ipfs://cust"));
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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
    env.ledger().set_timestamp(1000);
    client.dispute_escrow(&merchant, &escrow_id);
    env.ledger().set_timestamp(1200);
    client.submit_evidence(&merchant, &escrow_id, &String::from_str(&env, "ipfs://merch"));
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
    let escrow_id = client.create_escrow(&customer, &merchant, &1000_i128, &token, &1500_u64, &0_u64);
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

// ── MULTI-PARTY ESCROW TESTS ────────────────────────────────────────────────

#[test]
fn test_create_multi_party_escrow_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
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
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
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

    let escrow_id = client.create_multi_party_escrow(&customer, &participants, &10000, &token_id, &1000);

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
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
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
    let escrow_id = client.create_multi_party_escrow(&customer, &participants, &10000, &token_id, &1000);

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
fn test_release_multi_party_escrow_threshold_not_met() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
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

    let escrow_id = client.create_multi_party_escrow(&customer, &participants, &10000, &token_id, &1000);

    client.approve_release(&p1, &escrow_id);
    // Only 1 approval, 2 required

    env.ledger().set_timestamp(1001);
    client.release_multi_party_escrow(&escrow_id);
}
