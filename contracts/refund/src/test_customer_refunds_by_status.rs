#![cfg(test)]
//! Tests for #913: filtering a customer's refund history by status.

use crate::{FraudConfig, RefundContract, RefundContractClient, RefundReasonCode, RefundStatus};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

fn setup<'a>() -> (Env, RefundContractClient<'a>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.set_fraud_config(
        &admin,
        &FraudConfig {
            enabled: false,
            max_refund_rate_bps: 10_000,
            min_transactions_for_check: 100,
        },
    );
    (
        env.clone(),
        client,
        admin,
        Address::generate(&env),
        Address::generate(&env),
    )
}

fn request(
    env: &Env,
    client: &RefundContractClient,
    merchant: &Address,
    customer: &Address,
    i: u64,
) -> u64 {
    env.ledger().set_timestamp(i * 100);
    client.request_refund(
        merchant,
        &i,
        customer,
        &(i as i128 * 100),
        &10_000,
        &Address::generate(env),
        &String::from_str(env, "refund"),
        &RefundReasonCode::CustomerRequest,
        &env.ledger().timestamp(),
    )
}

#[test]
fn filters_by_status_newest_first_with_pagination() {
    let (env, client, admin, merchant, customer) = setup();
    let mut ids = soroban_sdk::Vec::<u64>::new(&env);
    for i in 1..=6 {
        ids.push_back(request(&env, &client, &merchant, &customer, i));
    }
    // Reject refunds 2 and 4 (they enter PendingAppeal); the rest stay Requested.
    let reason = String::from_str(&env, "no");
    client.reject_refund(&admin, &ids.get(1).unwrap(), &reason);
    client.reject_refund(&admin, &ids.get(3).unwrap(), &reason);

    let requested =
        client.get_customer_refunds_by_status(&customer, &RefundStatus::Requested, &0, &10);
    assert_eq!(requested.len(), 4);
    let got: std::vec::Vec<u64> = requested.iter().map(|r| r.id).collect();
    assert_eq!(
        got,
        std::vec![
            ids.get(5).unwrap(),
            ids.get(4).unwrap(),
            ids.get(2).unwrap(),
            ids.get(0).unwrap()
        ]
    );

    let rejected =
        client.get_customer_refunds_by_status(&customer, &RefundStatus::PendingAppeal, &0, &10);
    assert_eq!(rejected.len(), 2);
    assert_eq!(rejected.get(0).unwrap().id, ids.get(3).unwrap());
    assert_eq!(rejected.get(1).unwrap().id, ids.get(1).unwrap());

    // Pagination: offset skips matching refunds, limit caps the page.
    let page1 = client.get_customer_refunds_by_status(&customer, &RefundStatus::Requested, &0, &2);
    let page2 = client.get_customer_refunds_by_status(&customer, &RefundStatus::Requested, &2, &2);
    let page3 = client.get_customer_refunds_by_status(&customer, &RefundStatus::Requested, &4, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().id, ids.get(5).unwrap());
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(1).unwrap().id, ids.get(0).unwrap());
    assert_eq!(page3.len(), 0);
}

#[test]
fn empty_cases() {
    let (env, client, _admin, merchant, customer) = setup();
    assert_eq!(
        client
            .get_customer_refunds_by_status(&customer, &RefundStatus::Requested, &0, &10)
            .len(),
        0
    );
    request(&env, &client, &merchant, &customer, 1);
    assert_eq!(
        client
            .get_customer_refunds_by_status(&customer, &RefundStatus::Requested, &0, &0)
            .len(),
        0
    );
    assert_eq!(
        client
            .get_customer_refunds_by_status(&customer, &RefundStatus::Processed, &0, &10)
            .len(),
        0
    );
}
