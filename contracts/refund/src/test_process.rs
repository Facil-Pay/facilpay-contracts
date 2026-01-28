#![cfg(test)]
use super::{RefundContract, RefundContractClient, RefundStatus};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
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
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    // Approve
    client.approve_refund(&admin, &refund_id);

    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Approved);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // Unauthorized
fn test_approve_refund_not_admin() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let not_admin = Address::generate(&env);

    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "reason");

    env.mock_all_auths();
    let refund_id =
        client.request_refund(&merchant, &payment_id, &customer, &amount, &token, &reason);

    client.approve_refund(&not_admin, &refund_id);
}

#[test]
fn test_process_refund_success() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Setup Token
    let token_admin = Address::generate(&env);
    let token_contract = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(&env, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    // Mint tokens to merchant
    token_admin_client.mint(&merchant, &10000);

    // Merchant approves refund contract to spend tokens
    token_client.approve(&merchant, &contract_id, &1000, &20000);

    let payment_id = 1u64;
    let amount = 1000i128;
    let reason = String::from_str(&env, "reason");

    let refund_id = client.request_refund(
        &merchant,
        &payment_id,
        &customer,
        &amount,
        &token_contract,
        &reason,
    );

    client.approve_refund(&admin, &refund_id);

    client.process_refund(&admin, &refund_id);

    // Verify status
    let refund = client.get_refund(&refund_id);
    assert_eq!(refund.status, RefundStatus::Processed);

    // Verify balances
    assert_eq!(token_client.balance(&merchant), 9000);
    assert_eq!(token_client.balance(&customer), 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")] // NotApproved
fn test_process_refund_not_approved() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &token,
        &String::from_str(&env, "r"),
    );

    // Skip approval
    client.process_refund(&admin, &refund_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // TransferFailed
fn test_process_refund_transfer_failed() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let token_contract = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let refund_id = client.request_refund(
        &merchant,
        &1,
        &customer,
        &1000,
        &token_contract,
        &String::from_str(&env, "r"),
    );

    client.approve_refund(&admin, &refund_id);

    // Transfer should fail because merchant has no balance/allowance
    client.process_refund(&admin, &refund_id);
}
