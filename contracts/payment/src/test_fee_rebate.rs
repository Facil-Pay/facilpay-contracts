#![cfg(test)]
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String,
};

use crate::{Currency, Error, FeeConfig, FeeRebateConfig, PaymentContract, PaymentContractClient};

/// Sets up env, contract, admin, token, and fee config.
/// Returns (env, client, admin, token_addr, customer, merchant).
fn setup() -> (
    Env,
    PaymentContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PaymentContract);
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);

    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_sa = StellarAssetClient::new(&env, &token_addr);
    let token = TokenClient::new(&env, &token_addr);

    // Mint enough for several payments
    token_sa.mint(&customer, &1_000_000);
    token.approve(&customer, &contract_id, &1_000_000, &10_000);
    // Also mint to contract so it can pay out rebates
    token_sa.mint(&contract_id, &1_000_000);

    // Configure a 1% fee
    client.set_fee_config(
        &admin,
        &FeeConfig {
            fee_bps: 100,
            min_fee: 0,
            max_fee: 0,
            treasury: admin.clone(),
            fee_token: token_addr.clone(),
            active: true,
        },
    );

    (env, client, admin, token_addr, customer, merchant)
}

fn rebate_config(threshold: i128, rebate_bps: u32, period: u64) -> FeeRebateConfig {
    FeeRebateConfig {
        threshold_volume: threshold,
        rebate_bps,
        rebate_period_seconds: period,
        active: true,
    }
}

fn do_payment(
    env: &Env,
    client: &PaymentContractClient,
    admin: &Address,
    customer: &Address,
    merchant: &Address,
    token_addr: &Address,
    amount: i128,
) {
    let payment_id = client.create_payment(
        customer,
        merchant,
        &amount,
        token_addr,
        &Currency::USDC,
        &0,
        &String::from_str(env, ""),
    );
    client.complete_payment(admin, &payment_id);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// No rebate accrues when volume is below threshold.
#[test]
fn test_no_accrual_below_threshold() {
    let (env, client, admin, token_addr, customer, merchant) = setup();

    // Threshold = 5000; payment = 1000 (below threshold)
    client.configure_fee_rebate(&admin, &rebate_config(5_000, 2000, 86_400));

    do_payment(&env, &client, &admin, &customer, &merchant, &token_addr, 1_000);

    let accrual = client.get_rebate_accrual(&merchant);
    // Either None or zero accrued_rebate
    let accrued = accrual.map(|a| a.accrued_rebate).unwrap_or(0);
    assert_eq!(accrued, 0);
}

/// Rebate accrues only on volume above the threshold.
#[test]
fn test_accrual_above_threshold() {
    let (env, client, admin, token_addr, customer, merchant) = setup();

    // Threshold = 500; rebate = 20% of fee; fee = 1% of payment
    client.configure_fee_rebate(&admin, &rebate_config(500, 2000, 86_400));

    // Payment of 1000 → fee = 10 → rebate = 20% of 10 = 2
    do_payment(&env, &client, &admin, &customer, &merchant, &token_addr, 1_000);

    let accrual = client.get_rebate_accrual(&merchant).unwrap();
    assert!(accrual.accrued_rebate > 0);
    assert_eq!(accrual.period_volume, 1_000);
}

/// claim_fee_rebate transfers the accrued amount and resets accrual to zero.
#[test]
fn test_claim_rebate() {
    let (env, client, admin, token_addr, customer, merchant) = setup();

    client.configure_fee_rebate(&admin, &rebate_config(500, 2000, 86_400));
    do_payment(&env, &client, &admin, &customer, &merchant, &token_addr, 1_000);

    let accrual_before = client.get_rebate_accrual(&merchant).unwrap();
    assert!(accrual_before.accrued_rebate > 0);

    let claimed = client.claim_fee_rebate(&merchant);
    assert_eq!(claimed, accrual_before.accrued_rebate);

    // After claim, accrued_rebate is reset to 0
    let accrual_after = client.get_rebate_accrual(&merchant).unwrap();
    assert_eq!(accrual_after.accrued_rebate, 0);
}

/// Calling claim_fee_rebate again before new volume returns RebateAlreadyClaimed.
#[test]
fn test_double_claim_guard() {
    let (env, client, admin, token_addr, customer, merchant) = setup();

    client.configure_fee_rebate(&admin, &rebate_config(500, 2000, 86_400));
    do_payment(&env, &client, &admin, &customer, &merchant, &token_addr, 1_000);

    client.claim_fee_rebate(&merchant);

    let result = client.try_claim_fee_rebate(&merchant);
    assert_eq!(result, Err(Ok(Error::RebateAlreadyClaimed)));
}

/// Period resets automatically when rebate_period_seconds elapses on the next payment.
#[test]
fn test_period_reset() {
    let (env, client, admin, token_addr, customer, merchant) = setup();

    // 10-second period
    client.configure_fee_rebate(&admin, &rebate_config(500, 2000, 10));

    do_payment(&env, &client, &admin, &customer, &merchant, &token_addr, 1_000);

    let accrual_before = client.get_rebate_accrual(&merchant).unwrap();
    assert!(accrual_before.accrued_rebate > 0);

    // Advance time past the period
    env.ledger().with_mut(|l| l.timestamp += 20);

    // Mint more tokens for the second payment
    let token_sa = StellarAssetClient::new(&env, &token_addr);
    let token = TokenClient::new(&env, &token_addr);
    token_sa.mint(&customer, &1_000_000);
    token.approve(&customer, &contract_id_from_client(&client), &1_000_000, &10_000);

    do_payment(&env, &client, &admin, &customer, &merchant, &token_addr, 1_000);

    let accrual_after = client.get_rebate_accrual(&merchant).unwrap();
    // Period was reset: period_volume should equal only the new payment
    assert_eq!(accrual_after.period_volume, 1_000);
}

// Helper: extract contract address from client (not directly exposed, so we re-register)
fn contract_id_from_client(client: &PaymentContractClient) -> Address {
    client.address.clone()
}
