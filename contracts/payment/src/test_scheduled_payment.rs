#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

fn setup(
    env: &Env,
) -> (
    PaymentContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    let id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);

    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();
    let asset_client = token::StellarAssetClient::new(env, &token_address);

    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    asset_client.mint(&customer, &1_000_000i128);
    token::Client::new(env, &token_address).approve(&customer, &id, &1_000_000i128, &100_000);

    (client, admin, merchant, customer, token_address)
}

/// schedule_payment with a timestamp in the past must return InvalidScheduleTime.
#[test]
fn test_schedule_payment_with_past_timestamp_rejected() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, _admin, merchant, customer, token) = setup(&env);

    let result = client.try_schedule_payment(&customer, &merchant, &token, &100i128, &500u64);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::InvalidScheduleTime)))
    );
}

/// schedule_payment with timestamp equal to current ledger time must also be rejected.
#[test]
fn test_schedule_payment_at_current_time_rejected() {
    let env = Env::default();
    env.ledger().set_timestamp(5_000);
    let (client, _admin, merchant, customer, token) = setup(&env);

    let result = client.try_schedule_payment(&customer, &merchant, &token, &100i128, &5_000u64);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::InvalidScheduleTime)))
    );
}

/// schedule_payment with a future timestamp must succeed and the payment must not
/// execute until that time is reached.
#[test]
fn test_schedule_payment_with_future_timestamp_succeeds() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, _admin, merchant, customer, token) = setup(&env);

    let future = 10_000u64;
    let payment_id = client.schedule_payment(&customer, &merchant, &token, &100i128, &future);
    assert_eq!(payment_id, 1);

    let scheduled = client.get_scheduled_payment(&payment_id);
    assert_eq!(scheduled.scheduled_at, future);
    assert!(!scheduled.executed);
}

/// Attempting to execute a scheduled payment before its time must return NotYetDue.
#[test]
fn test_execute_scheduled_payment_before_time_rejected() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, _admin, merchant, customer, token) = setup(&env);

    let payment_id = client.schedule_payment(&customer, &merchant, &token, &100i128, &10_000u64);

    // Still before execute_at
    env.ledger().set_timestamp(5_000);
    let result = client.try_execute_scheduled_payment(&payment_id);
    assert_eq!(result, Err(Ok(Error::Payment(PaymentError::NotYetDue))));
}

/// A scheduled payment executes successfully once the ledger time reaches execute_at.
#[test]
fn test_execute_scheduled_payment_at_due_time_succeeds() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, _admin, merchant, customer, token) = setup(&env);

    let token_client = token::Client::new(&env, &token);
    let payment_id = client.schedule_payment(&customer, &merchant, &token, &200i128, &10_000u64);

    env.ledger().set_timestamp(10_000);
    client.execute_scheduled_payment(&payment_id);

    let scheduled = client.get_scheduled_payment(&payment_id);
    assert!(scheduled.executed);

    assert!(token_client.balance(&merchant) > 0);
}

/// A scheduled payment with a timestamp 10 years in the past must be rejected at
/// creation time, not executed on the next trigger call.
#[test]
fn test_schedule_payment_ten_years_in_past_rejected() {
    let env = Env::default();
    env.ledger().set_timestamp(315_360_000); // ~10 years in seconds
    let (client, _admin, merchant, customer, token) = setup(&env);

    // 1_000 is far in the past relative to the ledger
    let result = client.try_schedule_payment(&customer, &merchant, &token, &100i128, &1_000u64);
    assert_eq!(
        result,
        Err(Ok(Error::Payment(PaymentError::InvalidScheduleTime)))
    );
}
