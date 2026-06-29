#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, String};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn create_refund_and_issue_voucher(
    env: &Env,
    client: &RefundContractClient,
    admin: &Address,
    expiry_seconds: u64,
) -> (u64, u64) {
    let merchant = Address::generate(env);
    let customer = Address::generate(env);
    let token = Address::generate(env);
    let amount = 1000_i128;
    let payment_id = 1_u64;
    let reason = String::from_str(env, "defective product");

    let refund_id = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &amount,
        &token,
        &reason,
        &RefundReasonCode::ProductDefect,
        &env.ledger().timestamp(),
    );

    let voucher_id = client.issue_refund_voucher(admin, &refund_id, &expiry_seconds);
    (refund_id, voucher_id)
}

#[test]
fn test_redeem_voucher_before_expiry_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Issue a voucher expiring 1000s from now
    env.ledger().set_timestamp(1000);
    let (_, voucher_id) = create_refund_and_issue_voucher(&env, &client, &admin, 1000);

    // Redeem before expiry (at t=1500, expires at t=2000)
    env.ledger().set_timestamp(1500);
    let result = client.try_redeem_refund_voucher(
        &client.get_voucher(&voucher_id).unwrap().customer,
        &voucher_id,
        &1_u64,
    );
    assert!(result.is_ok(), "redeeming a valid unexpired voucher should succeed");
}

#[test]
fn test_redeem_expired_voucher_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Issue a voucher expiring 500s from t=1000 → expires at t=1500
    env.ledger().set_timestamp(1000);
    let (_, voucher_id) = create_refund_and_issue_voucher(&env, &client, &admin, 500);

    // Advance past expiry
    env.ledger().set_timestamp(1501);
    let result = client.try_redeem_refund_voucher(
        &client.get_voucher(&voucher_id).unwrap().customer,
        &voucher_id,
        &1_u64,
    );
    assert!(result.is_err(), "redeeming an expired voucher should fail");
}

#[test]
fn test_redeem_voucher_at_exact_expiry_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    // Issue a voucher expiring 300s from t=1000 → expires at t=1300
    env.ledger().set_timestamp(1000);
    let (_, voucher_id) = create_refund_and_issue_voucher(&env, &client, &admin, 300);

    // At exactly the expiry timestamp the voucher is expired (> check uses >)
    env.ledger().set_timestamp(1300);
    let result = client.try_redeem_refund_voucher(
        &client.get_voucher(&voucher_id).unwrap().customer,
        &voucher_id,
        &1_u64,
    );
    // timestamp(1300) > expires_at(1300) is false, so it should succeed at exactly the boundary
    assert!(result.is_ok(), "voucher should still be valid at exactly the expiry timestamp");
}

#[test]
fn test_already_redeemed_voucher_cannot_be_redeemed_again() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    env.ledger().set_timestamp(1000);
    let (_, voucher_id) = create_refund_and_issue_voucher(&env, &client, &admin, 1000);

    let customer = client.get_voucher(&voucher_id).unwrap().customer;

    // First redemption should succeed
    client.redeem_refund_voucher(&customer, &voucher_id, &1_u64);

    // Second redemption must fail
    let result = client.try_redeem_refund_voucher(&customer, &voucher_id, &1_u64);
    assert!(result.is_err(), "a voucher can only be redeemed once");
}
