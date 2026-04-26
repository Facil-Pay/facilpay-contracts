#![cfg(test)]
use super::{RefundContract, RefundContractClient, RefundReasonCode, RefundStatus};
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String,
};

#[test]
fn test_initialize() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_initialize_twice_should_fail() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_approve_refund() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "reason");

    env.mock_all_auths();
    let refund_id =
        client.request_refund(
            &merchant,
            &payment_id,
            &customer,
            &amount,
            &1000,
            &token,
            &reason,
            &RefundReasonCode::Other,
            &0_u64,
        );

    // Approve
    client.approve_refund(&admin, &refund_id);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}
