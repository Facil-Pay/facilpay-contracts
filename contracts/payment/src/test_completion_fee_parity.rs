#![cfg(test)]
//! Regression tests for issues #560, #561 and #562: every payment-completion
//! entry point must run the same fee deduction and post-completion logic as
//! `do_complete_payment`, and the public fee preview must match what is
//! actually deducted.

use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec};

use crate::{
    BatchPaymentEntry, Currency, FeeConfig, FinalityConfig, LoyaltyConfig, PaymentContract,
    PaymentContractClient, PaymentStatus, RiskFeeConfig,
};

fn setup() -> (
    Env,
    PaymentContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.initialize(&admin);
    (env, client, admin, token_addr, contract_id)
}

fn mint_and_approve(
    env: &Env,
    token_addr: &Address,
    spender: &Address,
    who: &Address,
    amount: i128,
) {
    token::StellarAssetClient::new(env, token_addr).mint(who, &amount);
    token::Client::new(env, token_addr).approve(who, spender, &amount, &100_000);
}

fn standard_fee_config(token_addr: &Address, admin: &Address) -> FeeConfig {
    FeeConfig {
        fee_bps: 100, // 1%
        min_fee: 0,
        max_fee: 0,
        treasury: admin.clone(),
        fee_token: token_addr.clone(),
        active: true,
    }
}

// ── #562 ──────────────────────────────────────────────────────────────────────

#[test]
fn execute_large_payment_deducts_platform_fee() {
    let (env, client, admin, token_addr, contract_id) = setup();
    let admin2 = Address::generate(&env);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2);

    client.set_fee_config(&admin, &standard_fee_config(&token_addr, &admin));
    client.set_large_payment_threshold(&admin, &1_000);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    mint_and_approve(&env, &token_addr, &contract_id, &customer, 10_000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &10_000,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Route through the multisig flow.
    client.propose_large_payment(&merchant, &payment_id);
    client.approve_large_payment(&admin, &payment_id);
    client.execute_large_payment(&payment_id);

    let token_client = token::Client::new(&env, &token_addr);
    // 1% fee on 10_000 = 100; merchant receives the net.
    assert_eq!(token_client.balance(&merchant), 9_900);
    assert_eq!(client.get_accumulated_fees(), 100);
    assert_eq!(
        client.get_payment(&payment_id).status,
        PaymentStatus::Completed
    );
    assert!(client.get_large_payment_proposal(&payment_id).executed);
}

#[test]
fn execute_large_payment_respects_finality_delay() {
    let (env, client, admin, token_addr, contract_id) = setup();
    let admin2 = Address::generate(&env);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2);

    client.set_large_payment_threshold(&admin, &1_000);
    client.configure_finality_delay(
        &admin,
        &FinalityConfig {
            delay_seconds: 3_600,
            min_amount_threshold: 1_000,
            active: true,
        },
    );

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    mint_and_approve(&env, &token_addr, &contract_id, &customer, 10_000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &10_000,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.propose_large_payment(&merchant, &payment_id);
    client.approve_large_payment(&admin, &payment_id);
    client.execute_large_payment(&payment_id);

    // Funds are held, not settled instantly.
    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&merchant), 0);
    let settlements = client.get_pending_settlements(&merchant);
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements.get(0).unwrap().payment_id, payment_id);
}

// ── #561 ──────────────────────────────────────────────────────────────────────

#[test]
fn batch_optimized_respects_finality_delay_and_accrues_loyalty() {
    let (env, client, admin, token_addr, contract_id) = setup();

    client.set_fee_config(&admin, &standard_fee_config(&token_addr, &admin));
    client.configure_finality_delay(
        &admin,
        &FinalityConfig {
            delay_seconds: 3_600,
            min_amount_threshold: 5_000,
            active: true,
        },
    );
    client.configure_loyalty(
        &admin,
        &LoyaltyConfig {
            points_per_unit: 100,
            redemption_rate: 1,
            expiry_seconds: 1_000_000,
            active: true,
        },
    );

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    mint_and_approve(&env, &token_addr, &contract_id, &customer, 20_000);

    // One large entry (held by finality delay) and one small entry (settles now).
    // Kept below the 10_000 volume tier-upgrade threshold so the fee stays 1%.
    let entries = Vec::from_array(
        &env,
        [
            BatchPaymentEntry {
                customer: customer.clone(),
                merchant: merchant.clone(),
                amount: 6_000,
                token: token_addr.clone(),
                currency: Currency::USDC,
                expiration_duration: 0,
                metadata: String::from_str(&env, ""),
            },
            BatchPaymentEntry {
                customer: customer.clone(),
                merchant: merchant.clone(),
                amount: 1_000,
                token: token_addr.clone(),
                currency: Currency::USDC,
                expiration_duration: 0,
                metadata: String::from_str(&env, ""),
            },
        ],
    );

    let results = client.create_payment_batch_optimized(&admin, &entries);
    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(results.get(1).unwrap().success);

    // The large payment is held as a pending settlement (finality delay honored).
    let settlements = client.get_pending_settlements(&merchant);
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements.get(0).unwrap().amount, 5_940); // 6_000 - 1% fee

    // The small payment settled immediately, net of the 1% fee.
    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&merchant), 990);

    // Loyalty points accrued for the settled payment (1_000 / 100 = 10).
    let balance = client.get_loyalty_balance(&customer).unwrap();
    assert_eq!(balance.points, 10);

    // Fees from both entries were collected (60 + 10).
    assert_eq!(client.get_accumulated_fees(), 70);
}

// ── #560 ──────────────────────────────────────────────────────────────────────

#[test]
fn calculate_fee_matches_deducted_fee_including_risk_surcharge() {
    let (env, client, admin, token_addr, contract_id) = setup();

    client.set_fee_config(&admin, &standard_fee_config(&token_addr, &admin));
    client.set_risk_fee_config(
        &admin,
        &RiskFeeConfig {
            base_fee_bps: 100,
            large_amount_threshold: 1_000,
            large_amount_surcharge_bps: 50,  // 0.5%
            new_customer_surcharge_bps: 100, // 1%
            high_risk_currency_surcharge: 200,
        },
    );

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    mint_and_approve(&env, &token_addr, &contract_id, &customer, 10_000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &10_000,
        &token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Preview: base 1% + large-amount 0.5% + new-customer 1% = 2.5% of 10_000.
    let quoted = client.calculate_fee(&10_000, &merchant, &customer, &Currency::USDC);
    assert_eq!(quoted, 250);

    client.complete_payment(&admin, &payment_id);

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&merchant), 10_000 - quoted);
    assert_eq!(client.get_accumulated_fees(), quoted);
}

#[test]
fn calculate_fee_has_no_risk_surcharge_when_unconfigured() {
    let (env, client, admin, token_addr, _contract_id) = setup();
    client.set_fee_config(&admin, &standard_fee_config(&token_addr, &admin));

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    // No RiskFeeConfig set → only the 1% base fee is quoted.
    let quoted = client.calculate_fee(&10_000, &merchant, &customer, &Currency::BTC);
    assert_eq!(quoted, 100);
}
