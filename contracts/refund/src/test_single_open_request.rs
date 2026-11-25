#![cfg(test)]

//! Issue #915: optional one-open-request-per-payment rule.

use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{token, Address, Env, String, Symbol, TryFromVal};

struct Setup<'a> {
    env: Env,
    client: RefundContractClient<'a>,
    admin: Address,
    merchant: Address,
    customer: Address,
    token: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&contract_id, &1_000_000);

    Setup {
        merchant: Address::generate(&env),
        customer: Address::generate(&env),
        env,
        client,
        admin,
        token,
    }
}

fn try_request(s: &Setup, payment_id: u64, amount: i128) -> Result<u64, Error> {
    match s.client.try_request_refund(
        &s.merchant,
        &payment_id,
        &s.customer,
        &amount,
        &10_000,
        &s.token,
        &String::from_str(&s.env, "refund"),
        &RefundReasonCode::CustomerRequest,
        &s.env.ledger().timestamp(),
    ) {
        Ok(Ok(id)) => Ok(id),
        Err(Ok(e)) => Err(e),
        other => panic!("unexpected result: {:?}", other.is_ok()),
    }
}

fn request(s: &Setup, payment_id: u64, amount: i128) -> u64 {
    try_request(s, payment_id, amount).unwrap()
}

const BLOCKED: Result<u64, Error> = Err(Error::Core(CoreError::OpenRefundRequestExists));

#[test]
fn rule_is_off_by_default_and_requests_can_stack() {
    let s = setup();
    assert!(!s.client.get_single_open_request(&s.merchant));

    let first = request(&s, 1, 100);
    let second = request(&s, 1, 100);
    assert_ne!(first, second);
    assert_eq!(s.client.get_refund(&first).status, RefundStatus::Requested);
}

#[test]
fn enabled_rule_rejects_second_request_while_first_is_requested() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);
    assert!(s.client.get_single_open_request(&s.merchant));

    request(&s, 1, 100);
    assert_eq!(try_request(&s, 1, 100), BLOCKED);
}

#[test]
fn approved_but_unprocessed_request_blocks_until_processed() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);

    let refund_id = request(&s, 1, 100);
    s.client.approve_refund(&s.admin, &refund_id);
    assert_eq!(try_request(&s, 1, 100), BLOCKED);

    s.client.process_refund(&s.admin, &refund_id);
    assert!(try_request(&s, 1, 100).is_ok());
}

#[test]
fn rejected_request_blocks_during_appeal_window_then_frees_payment() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);

    let refund_id = request(&s, 1, 100);
    s.client
        .reject_refund(&s.admin, &refund_id, &String::from_str(&s.env, "no"));
    assert_eq!(
        s.client.get_refund(&refund_id).status,
        RefundStatus::PendingAppeal
    );
    assert_eq!(try_request(&s, 1, 100), BLOCKED);

    let deadline = s.client.get_refund(&refund_id).appeal_deadline.unwrap();
    s.env.ledger().set_timestamp(deadline + 1);
    s.client.finalize_denial(&refund_id);
    assert!(try_request(&s, 1, 100).is_ok());
}

#[test]
fn withdrawn_request_frees_payment() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);

    let refund_id = request(&s, 1, 100);
    s.client.withdraw_refund_request(&s.customer, &refund_id);
    assert!(try_request(&s, 1, 100).is_ok());
}

#[test]
fn rule_is_per_payment() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);

    request(&s, 1, 100);
    assert!(try_request(&s, 2, 100).is_ok());
}

#[test]
fn rule_is_per_merchant() {
    let s = setup();
    let other_merchant = Address::generate(&s.env);
    s.client.set_single_open_request(&other_merchant, &true);

    request(&s, 1, 100);
    assert!(try_request(&s, 1, 100).is_ok());
}

#[test]
fn disabling_rule_allows_stacking_again() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);
    request(&s, 1, 100);
    assert_eq!(try_request(&s, 1, 100), BLOCKED);

    s.client.set_single_open_request(&s.merchant, &false);
    assert!(!s.client.get_single_open_request(&s.merchant));
    assert!(try_request(&s, 1, 100).is_ok());
}

#[test]
fn toggling_rule_emits_event() {
    let s = setup();
    s.client.set_single_open_request(&s.merchant, &true);
    let expected = Symbol::new(&s.env, "single_open_request_set");
    let found = s.env.events().all().iter().any(|(_, topics, _)| {
        topics
            .get(0)
            .and_then(|t| Symbol::try_from_val(&s.env, &t).ok())
            .is_some_and(|sym| sym == expected)
    });
    assert!(found);
}
