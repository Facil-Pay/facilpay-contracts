#![cfg(test)]

use super::*;
use escrow::{EscrowContract, EscrowContractClient, EscrowStatus};
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

// Demonstrates that an external contract (Payment) can use the new escrow
// state verification interface to inspect escrow state cross-contract.

fn setup_env() -> (
    Env,
    PaymentContractClient<'static>,
    EscrowContractClient<'static>,
    Address, // customer
    Address, // merchant
    Address, // admin
    Address, // token
) {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    let payment_contract_id = env.register(PaymentContract, ());
    let payment_client = PaymentContractClient::new(&env, &payment_contract_id);

    let escrow_contract_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);

    payment_client.initialize(&admin);
    token_admin_client.mint(&customer, &2000_i128);
    token::Client::new(&env, &token_contract_id).approve(
        &customer,
        &payment_contract_id,
        &2000_i128,
        &100_000,
    );

    (
        env,
        payment_client,
        escrow_client,
        customer,
        merchant,
        admin,
        token_contract_id,
    )
}

fn create_escrowed_payment_ids(
    env: &Env,
    payment_client: &PaymentContractClient,
    escrow_client: &EscrowContractClient,
    customer: &Address,
    merchant: &Address,
    token: &Address,
    amount: i128,
) -> (u64, u64) {
    let escrow_contract_id = escrow_client.address.clone();
    payment_client.create_escrowed_payment(
        customer,
        merchant,
        &amount,
        token,
        &Currency::USDC,
        &escrow_contract_id,
        &5000_u64,
        &0_u64,
        &String::from_str(env, "integration-test"),
        &true,
    )
}

// ── Cross-contract: is_escrow_released ────────────────────────────────────────

#[test]
fn test_cross_contract_is_escrow_released_false_while_locked() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    // Escrow was just created — must not appear released yet.
    assert!(!escrow_client.is_escrow_released(&escrow_id));
}

#[test]
fn test_cross_contract_is_escrow_released_true_after_complete() {
    let (env, payment_client, escrow_client, customer, merchant, admin, token) = setup_env();

    let (payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    // Complete the payment, which releases the escrow.
    payment_client.complete_escrowed_payment(&admin, &payment_id);

    assert!(escrow_client.is_escrow_released(&escrow_id));
}

// ── Cross-contract: get_escrow_status ─────────────────────────────────────────

#[test]
fn test_cross_contract_get_escrow_status_locked_after_creation() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    assert_eq!(
        escrow_client.get_escrow_status(&escrow_id),
        EscrowStatus::Locked
    );
}

#[test]
fn test_cross_contract_get_escrow_status_released_after_complete() {
    let (env, payment_client, escrow_client, customer, merchant, admin, token) = setup_env();

    let (payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    payment_client.complete_escrowed_payment(&admin, &payment_id);

    assert_eq!(
        escrow_client.get_escrow_status(&escrow_id),
        EscrowStatus::Released
    );
}

// ── Cross-contract: get_escrow_parties ───────────────────────────────────────

#[test]
fn test_cross_contract_get_escrow_parties_matches_payment_parties() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    let (returned_customer, returned_merchant) = escrow_client.get_escrow_parties(&escrow_id);
    assert_eq!(returned_customer, customer);
    assert_eq!(returned_merchant, merchant);
}

// ── Cross-contract: get_escrow_amount ────────────────────────────────────────

#[test]
fn test_cross_contract_get_escrow_amount_matches_payment_amount() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let amount = 750_i128;
    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        amount,
    );

    assert_eq!(escrow_client.get_escrow_amount(&escrow_id), amount);
}

// ── Cross-contract: verify_escrow_participant ─────────────────────────────────

#[test]
fn test_cross_contract_verify_customer_is_participant() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    assert!(escrow_client.verify_escrow_participant(&escrow_id, &customer));
}

#[test]
fn test_cross_contract_verify_merchant_is_participant() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    assert!(escrow_client.verify_escrow_participant(&escrow_id, &merchant));
}

#[test]
fn test_cross_contract_verify_unrelated_address_is_not_participant() {
    let (env, payment_client, escrow_client, customer, merchant, _admin, token) = setup_env();

    let (_payment_id, escrow_id) = create_escrowed_payment_ids(
        &env,
        &payment_client,
        &escrow_client,
        &customer,
        &merchant,
        &token,
        500_i128,
    );

    let unrelated = Address::generate(&env);
    assert!(!escrow_client.verify_escrow_participant(&escrow_id, &unrelated));
}
