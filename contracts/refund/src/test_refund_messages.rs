#![cfg(test)]

//! Issue #802: message thread on refund requests.

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, String};

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
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&contract_id, &1_000_000);
    token::StellarAssetClient::new(&env, &token).mint(&merchant, &1_000_000);

    Setup {
        env,
        client,
        admin,
        merchant,
        customer,
        token,
    }
}

fn hash(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

fn open_refund(s: &Setup) -> u64 {
    s.client.request_refund(
        &s.merchant,
        &1u64,
        &s.customer,
        &100i128,
        &1000i128,
        &s.token,
        &String::from_str(&s.env, "reason"),
        &RefundReasonCode::Other,
        &0u64,
    )
}

#[test]
fn test_customer_and_merchant_can_post() {
    let s = setup();
    let refund_id = open_refund(&s);

    let id1 = s
        .client
        .post_refund_message(&s.customer, &refund_id, &hash(&s.env, 1));
    let id2 = s
        .client
        .post_refund_message(&s.merchant, &refund_id, &hash(&s.env, 2));
    assert_eq!(id1, 0);
    assert_eq!(id2, 1);

    let messages = s.client.get_refund_messages(&refund_id, &0u64, &10u64);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages.get(0).unwrap().sender, s.customer);
    assert_eq!(messages.get(1).unwrap().sender, s.merchant);
}

#[test]
fn test_assigned_arbitrator_can_post() {
    let s = setup();
    // escalate_to_arbitration requires a pool of at least 3 available arbitrators.
    let arbitrator = Address::generate(&s.env);
    s.client.register_arbitrator(&s.admin, &arbitrator);
    s.client
        .register_arbitrator(&s.admin, &Address::generate(&s.env));
    s.client
        .register_arbitrator(&s.admin, &Address::generate(&s.env));

    let refund_id = open_refund(&s);
    s.client
        .reject_refund(&s.admin, &refund_id, &String::from_str(&s.env, "rejected"));
    let case_id = s
        .client
        .escalate_to_arbitration(&s.merchant, &refund_id, &s.token, &10i128);
    s.client.assign_arbitrator(&s.admin, &case_id, &arbitrator);

    let result = s
        .client
        .try_post_refund_message(&arbitrator, &refund_id, &hash(&s.env, 3));
    assert!(
        result.is_ok(),
        "an assigned arbitrator should be able to post"
    );
}

#[test]
fn test_third_party_cannot_post() {
    let s = setup();
    let refund_id = open_refund(&s);
    let stranger = Address::generate(&s.env);

    let result = s
        .client
        .try_post_refund_message(&stranger, &refund_id, &hash(&s.env, 4));
    assert_eq!(result, Err(Ok(Error::Core(CoreError::Unauthorized))));
}

#[test]
fn test_unassigned_arbitrator_cannot_post() {
    let s = setup();
    let arbitrator = Address::generate(&s.env);
    s.client.register_arbitrator(&s.admin, &arbitrator);

    // No case escalated for this refund, so the arbitrator isn't "assigned".
    let refund_id = open_refund(&s);
    let result = s
        .client
        .try_post_refund_message(&arbitrator, &refund_id, &hash(&s.env, 5));
    assert_eq!(result, Err(Ok(Error::Core(CoreError::Unauthorized))));
}

#[test]
fn test_message_cap_is_enforced() {
    let s = setup();
    let refund_id = open_refund(&s);

    for i in 0..RefundContract::MAX_REFUND_MESSAGES {
        let id =
            s.client
                .post_refund_message(&s.customer, &refund_id, &hash(&s.env, (i % 255) as u8));
        assert_eq!(id, i as u64);
    }

    let result = s
        .client
        .try_post_refund_message(&s.customer, &refund_id, &hash(&s.env, 9));
    assert_eq!(
        result,
        Err(Ok(Error::Core(CoreError::RefundMessageLimitReached)))
    );
}

#[test]
fn test_get_refund_messages_pagination() {
    let s = setup();
    let refund_id = open_refund(&s);

    for i in 0..5u8 {
        s.client
            .post_refund_message(&s.customer, &refund_id, &hash(&s.env, i));
    }

    let page1 = s.client.get_refund_messages(&refund_id, &0u64, &2u64);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().message_id, 0);
    assert_eq!(page1.get(1).unwrap().message_id, 1);

    let page2 = s.client.get_refund_messages(&refund_id, &2u64, &2u64);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().message_id, 2);
    assert_eq!(page2.get(1).unwrap().message_id, 3);

    let page3 = s.client.get_refund_messages(&refund_id, &4u64, &2u64);
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().message_id, 4);
}

#[test]
fn test_unknown_refund_is_rejected() {
    let s = setup();
    let result = s
        .client
        .try_post_refund_message(&s.customer, &999u64, &hash(&s.env, 1));
    assert_eq!(result, Err(Ok(Error::Core(CoreError::RefundNotFound))));
}
