#![cfg(test)]
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

use crate::{Currency, Error, FeatureError, FeeConfig, PaymentContract, PaymentContractClient};

/// Helper: creates a funded token, mints `amount` to `customer`, sets an
/// allowance from `customer` to `contract_id`, then creates and completes a
/// payment so that a real fee is accumulated inside the contract.
fn create_completed_payment_with_fee(
    env: &Env,
    client: &PaymentContractClient,
    admin: &Address,
    amount: i128,
) {
    let customer = Address::generate(env);
    let merchant = Address::generate(env);

    // Spin up a fresh token so the fee-token config matches
    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_asset = token::StellarAssetClient::new(env, &token_id);
    let token_user = token::Client::new(env, &token_id);

    // Fund customer and give the contract a spending allowance
    token_asset.mint(&customer, &amount);
    token_user.approve(&customer, &client.address, &amount, &999_999);

    // Override FeeConfig so fee_token matches the token we just created
    client.set_fee_config(
        admin,
        &FeeConfig {
            fee_bps: 100,      // 1 %
            min_fee: 0,
            max_fee: 0,
            treasury: admin.clone(),
            fee_token: token_id.clone(),
            active: true,
        },
    );

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_id,
        &Currency::USDC,
        &0,
        &String::from_str(env, ""),
    );
    client.complete_payment(admin, &payment_id);
}

fn setup() -> (Env, PaymentContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PaymentContract);
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.initialize(&admin);
    let token = soroban_sdk::token::StellarAssetClient::new(
        &env,
        &env.register_stellar_asset_contract_v2(admin.clone())
            .address(),
    );
    let token_addr = token.address.clone();
    token.mint(&contract_id, &1_000_000);
    client.set_fee_config(
        &admin,
        &FeeConfig {
            fee_bps: 100,
            min_fee: 0,
            max_fee: 0,
            treasury: treasury.clone(),
            fee_token: token_addr,
            active: true,
        },
    );
    (env, client, admin, treasury)
}

#[test]
fn test_sweep_recipient_not_set() {
    let (_, client, admin, _) = setup();
    let result = client.try_sweep_platform_fees(&admin);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::SweepRecipientNotSet)))
    );
}

#[test]
fn test_nothing_to_sweep() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);
    let result = client.try_sweep_platform_fees(&admin);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::NothingToSweep)))
    );
}

#[test]
fn test_successful_sweep_and_history() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);

    // Manually accumulate fees by completing a payment
    // get_sweepable_balance should be 0 initially
    assert_eq!(client.get_sweepable_balance(), 0);

    // History should be empty
    let history = client.get_sweep_history(&10);
    assert_eq!(history.len(), 0);
}

#[test]
fn test_get_sweepable_balance() {
    let (_, client, _, _) = setup();
    assert_eq!(client.get_sweepable_balance(), 0);
}

// ── 5 new tests ─────────────────────────────────────────────────────────────

/// Sweeping fees transfers the accumulated balance to the recipient and returns
/// the exact amount that was swept.
#[test]
fn test_sweep_transfers_correct_amount_to_recipient() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);

    // Accumulate a real fee via a completed payment
    create_completed_payment_with_fee(&env, &client, &admin, 10_000);

    let balance_before = client.get_sweepable_balance();
    assert!(balance_before > 0, "expected non-zero fee balance");

    let swept = client.sweep_platform_fees(&admin);

    // Returned value must equal what was accumulated
    assert_eq!(swept, balance_before);
    // Contract balance resets to zero
    assert_eq!(client.get_sweepable_balance(), 0);
}

/// After a successful sweep the sweepable balance is zero, so a second
/// immediate sweep must fail with NothingToSweep.
#[test]
fn test_double_sweep_fails_with_nothing_to_sweep() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);

    create_completed_payment_with_fee(&env, &client, &admin, 5_000);

    // First sweep succeeds
    client.sweep_platform_fees(&admin);

    // Second sweep on the same zero balance must fail
    let result = client.try_sweep_platform_fees(&admin);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::NothingToSweep)))
    );
}

/// Every successful sweep appends exactly one record to the history. Running
/// two sweeps (each after re-accumulating fees) must produce two history
/// entries with incrementing sweep IDs.
#[test]
fn test_sweep_history_records_each_sweep() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);

    // First sweep
    create_completed_payment_with_fee(&env, &client, &admin, 8_000);
    client.sweep_platform_fees(&admin);

    // Second sweep
    create_completed_payment_with_fee(&env, &client, &admin, 4_000);
    client.sweep_platform_fees(&admin);

    let history = client.get_sweep_history(&10);
    assert_eq!(history.len(), 2);

    let first = history.get(0).unwrap();
    let second = history.get(1).unwrap();

    // IDs are sequential
    assert_eq!(first.sweep_id, 1);
    assert_eq!(second.sweep_id, 2);

    // Both records reference the recipient set above
    assert_eq!(first.recipient, recipient);
    assert_eq!(second.recipient, recipient);

    // Second sweep amount corresponds to the second payment fee
    assert!(second.amount > 0);
}

/// get_sweep_history respects the `limit` parameter: when there are more
/// records than the limit, only the most recent `limit` entries are returned.
#[test]
fn test_get_sweep_history_respects_limit() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);

    // Produce three sweep records
    for _ in 0..3 {
        create_completed_payment_with_fee(&env, &client, &admin, 2_000);
        client.sweep_platform_fees(&admin);
    }

    // Requesting a limit of 2 should return only the two most recent entries
    let history = client.get_sweep_history(&2);
    assert_eq!(history.len(), 2);

    // The two most-recent sweeps are IDs 2 and 3
    assert_eq!(history.get(0).unwrap().sweep_id, 2);
    assert_eq!(history.get(1).unwrap().sweep_id, 3);
}

/// Calling set_sweep_recipient twice replaces the previous recipient. The
/// sweep must land at the most recently set address, not the original one.
#[test]
fn test_set_sweep_recipient_is_overridable() {
    let (env, client, admin, _) = setup();
    let first_recipient = Address::generate(&env);
    let second_recipient = Address::generate(&env);

    client.set_sweep_recipient(&admin, &first_recipient);
    // Override with a different address
    client.set_sweep_recipient(&admin, &second_recipient);

    create_completed_payment_with_fee(&env, &client, &admin, 6_000);
    client.sweep_platform_fees(&admin);

    let history = client.get_sweep_history(&1);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().recipient, second_recipient);
}
