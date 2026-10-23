#![cfg(test)]

use crate::{
    ActionError, BasicError, Error, EscrowContract, EscrowContractClient, EscrowStatus,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

fn setup() -> (Env, EscrowContractClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Initialize token mock
    env.budget().reset_unlimited();

    (env, client, admin, customer, merchant, token)
}

fn create_disputed_escrow(
    env: &Env,
    client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
    amount: i128,
) -> u64 {
    env.ledger().set_timestamp(1000);
    
    let escrow_id = client.create_escrow(customer, merchant, &amount, token, &5000, &0);
    
    // Dispute the escrow
    client.dispute_escrow(customer, &escrow_id);
    
    escrow_id
}

#[test]
fn test_propose_settlement_offer_by_customer() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    // Customer proposes a settlement: 600 to customer, 400 to merchant
    env.ledger().set_timestamp(2000);
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400, // 1 day expiry
    );
    
    // Verify the escrow is still disputed
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
fn test_propose_settlement_offer_by_merchant() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    // Merchant proposes a settlement: 300 to customer, 700 to merchant
    env.ledger().set_timestamp(2000);
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &300,
        &700,
        &86400,
    );
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")] // NotDisputed
fn test_propose_settlement_on_non_disputed_escrow_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    env.ledger().set_timestamp(1000);
    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);
    
    // Try to propose settlement without disputing first
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")] // Unauthorized
fn test_propose_settlement_by_unauthorized_party_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    let unauthorized = Address::generate(&env);
    
    // Try to propose settlement as unauthorized party
    client.propose_settlement_offer(
        &unauthorized,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #333)")] // InvalidSettlementAmount
fn test_propose_settlement_with_invalid_amounts_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    // Amounts don't sum to escrow amount (600 + 300 = 900, not 1000)
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &300,
        &86400,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #329)")] // SettlementOfferAlreadyExists
fn test_propose_settlement_when_offer_already_exists_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // First proposal
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
    
    // Try to propose again before expiry
    env.ledger().set_timestamp(3000); // Still within 86400 seconds
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &500,
        &500,
        &86400,
    );
}

#[test]
fn test_propose_new_settlement_after_previous_expired() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // First proposal
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &1000, // 1000 seconds expiry
    );
    
    // Wait for expiry
    env.ledger().set_timestamp(4000); // Past expiry
    
    // New proposal should work
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &500,
        &500,
        &2000,
    );
}

#[test]
fn test_propose_new_settlement_after_previous_rejected() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
    
    // Merchant rejects
    client.reject_settlement_offer(&merchant, &escrow_id);
    
    // Customer can propose again
    env.ledger().set_timestamp(3000);
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &550,
        &450,
        &86400,
    );
}

#[test]
fn test_accept_settlement_offer_resolves_dispute() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes: 600 to customer, 400 to merchant
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
    
    // Merchant accepts
    env.ledger().set_timestamp(2500);
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    // Verify escrow is resolved
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_accept_settlement_offer_with_equal_split() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Merchant proposes equal split
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &500,
        &500,
        &86400,
    );
    
    // Customer accepts
    client.accept_settlement_offer(&customer, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")] // SettlementOfferNotFound
fn test_accept_nonexistent_settlement_offer_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    // Try to accept without any offer
    client.accept_settlement_offer(&merchant, &escrow_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #331)")] // SettlementOfferExpired
fn test_accept_expired_settlement_offer_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes with short expiry
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &1000, // 1000 seconds
    );
    
    // Wait past expiry
    env.ledger().set_timestamp(4000);
    
    // Try to accept expired offer
    client.accept_settlement_offer(&merchant, &escrow_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")] // Unauthorized
fn test_accept_settlement_offer_by_proposer_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
    
    // Customer tries to accept their own offer
    client.accept_settlement_offer(&customer, &escrow_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")] // Unauthorized
fn test_accept_settlement_offer_by_unauthorized_party_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &600,
        &400,
        &86400,
    );
    
    let unauthorized = Address::generate(&env);
    
    // Unauthorized party tries to accept
    client.accept_settlement_offer(&unauthorized, &escrow_id);
}

#[test]
fn test_reject_settlement_offer() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &700,
        &300,
        &86400,
    );
    
    // Merchant rejects
    client.reject_settlement_offer(&merchant, &escrow_id);
    
    // Escrow should still be disputed
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
#[should_panic(expected = "Error(Contract, #330)")] // SettlementOfferNotFound
fn test_reject_nonexistent_settlement_offer_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    // Try to reject without any offer
    client.reject_settlement_offer(&merchant, &escrow_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")] // Unauthorized
fn test_reject_settlement_offer_by_proposer_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &700,
        &300,
        &86400,
    );
    
    // Customer tries to reject their own offer
    client.reject_settlement_offer(&customer, &escrow_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")] // Unauthorized
fn test_reject_settlement_offer_by_unauthorized_party_fails() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &700,
        &300,
        &86400,
    );
    
    let unauthorized = Address::generate(&env);
    
    // Unauthorized party tries to reject
    client.reject_settlement_offer(&unauthorized, &escrow_id);
}

#[test]
fn test_settlement_negotiation_workflow() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Round 1: Customer offers 70/30 split
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &700,
        &300,
        &86400,
    );
    
    // Merchant rejects
    client.reject_settlement_offer(&merchant, &escrow_id);
    
    env.ledger().set_timestamp(3000);
    
    // Round 2: Merchant counter-offers 40/60 split
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &400,
        &600,
        &86400,
    );
    
    // Customer rejects
    client.reject_settlement_offer(&customer, &escrow_id);
    
    env.ledger().set_timestamp(4000);
    
    // Round 3: Customer offers 55/45 split (compromise)
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &550,
        &450,
        &86400,
    );
    
    // Merchant accepts
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    // Verify resolved
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_settlement_with_zero_amount_to_one_party() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Merchant offers all funds to customer
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &1000,
        &0,
        &86400,
    );
    
    // Customer accepts
    client.accept_settlement_offer(&customer, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_settlement_with_zero_amount_to_other_party() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer offers all funds to merchant
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &0,
        &1000,
        &86400,
    );
    
    // Merchant accepts
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_settlement_offer_expiry_allows_new_proposal() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Customer proposes with 1 hour expiry
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &700,
        &300,
        &3600,
    );
    
    // Wait past expiry
    env.ledger().set_timestamp(6000); // 2000 + 3600 = 5600, so 6000 > 5600
    
    // Merchant should be able to propose a new offer
    client.propose_settlement_offer(
        &merchant,
        &escrow_id,
        &350,
        &650,
        &3600,
    );
    
    // Customer accepts
    client.accept_settlement_offer(&customer, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_settlement_with_large_amounts() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let large_amount = 1_000_000_000_i128; // 1 billion
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, large_amount);
    
    env.ledger().set_timestamp(2000);
    
    // 60/40 split of large amount
    let customer_share = 600_000_000_i128;
    let merchant_share = 400_000_000_i128;
    
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &customer_share,
        &merchant_share,
        &86400,
    );
    
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")] // NotDisputed
fn test_accept_settlement_on_resolved_escrow_fails() {
    let (env, client, admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Propose and accept settlement
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &500,
        &500,
        &86400,
    );
    
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    // Try to accept again after it's already resolved
    client.accept_settlement_offer(&merchant, &escrow_id);
}

#[test]
fn test_settlement_offer_with_minimal_expiry() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Propose with minimal expiry (1 second)
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &500,
        &500,
        &1,
    );
    
    // Accept immediately (within 1 second)
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_settlement_offer_with_maximum_expiry() {
    let (env, client, _admin, customer, merchant, token) = setup();
    
    let escrow_id = create_disputed_escrow(&env, &client, &customer, &merchant, &token, 1000);
    
    env.ledger().set_timestamp(2000);
    
    // Propose with very long expiry (1 year in seconds)
    let one_year = 365 * 24 * 60 * 60_u64;
    client.propose_settlement_offer(
        &customer,
        &escrow_id,
        &500,
        &500,
        &one_year,
    );
    
    // Accept before expiry
    env.ledger().set_timestamp(3000);
    client.accept_settlement_offer(&merchant, &escrow_id);
    
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}
