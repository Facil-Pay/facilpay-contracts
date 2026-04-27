#![cfg(test)]

use super::*;
use soroban_sdk::{contract, contractimpl, testutils::Address as _, testutils::Ledger, Address, Bytes, BytesN, Env, String};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn install_mock_payment_contract(env: &Env, payment: ExternalPayment) -> Address {
    let contract_id = env.register(MockPaymentContract, ());
    let client = MockPaymentContractClient::new(env, &contract_id);
    client.set_payment(&payment);
    contract_id
}

#[contract]
struct MockPaymentContract;

#[contractimpl]
impl MockPaymentContract {
    pub fn set_payment(env: Env, payment: ExternalPayment) {
        env.storage().instance().set(&0u32, &payment);
    }

    pub fn get_payment(env: Env, payment_id: u64) -> ExternalPayment {
        let payment: ExternalPayment = env.storage().instance().get(&0u32).unwrap();
        assert_eq!(payment.id, payment_id);
        payment
    }

    pub fn check_payment_customer(env: Env, payment_id: u64, customer: Address) -> bool {
        let payment: ExternalPayment = env.storage().instance().get(&0u32).unwrap();
        payment.id == payment_id
            && payment.customer == customer
            && payment.status == ExternalPaymentStatus::Completed
    }
}

#[contract]
struct MockStateContract;

#[contractimpl]
impl MockStateContract {
    pub fn set_contract_state(env: Env, key: BytesN<32>, value: Bytes) {
        env.storage().instance().set(&key, &value);
    }

    pub fn get_contract_state(env: Env, key: BytesN<32>) -> Bytes {
        env.storage()
            .instance()
            .get(&key)
            .unwrap_or(Bytes::new(&env))
    }
}

fn sample_payment(env: &Env, merchant: &Address, customer: &Address, token: &Address) -> ExternalPayment {
    ExternalPayment {
        id: 7,
        customer: customer.clone(),
        merchant: merchant.clone(),
        amount: 10_000,
        token: token.clone(),
        currency: ExternalCurrency::USDC,
        status: ExternalPaymentStatus::Completed,
        created_at: 1_000,
        expires_at: 0,
        metadata: String::from_str(env, ""),
        notes: String::from_str(env, ""),
        refunded_amount: 0,
    }
}

#[test]
fn test_evaluate_auto_refund_triggers_on_timeout() {
    let env = Env::default();
    env.ledger().set_timestamp(10_000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_contract = install_mock_payment_contract(&env, sample_payment(&env, &merchant, &customer, &token));
    client.set_payment_contract_address(&admin, &payment_contract);

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &7u64,
        &AutoRefundCondition::FulfillmentTimeout(FulfillmentTimeoutCondition { fulfillment_deadline: 9_000 }),
        &2_500u32,
    );

    assert!(client.evaluate_auto_refund(&trigger_id));

    let refund = client.get_refund(&1u64);
    assert_eq!(refund.status, RefundStatus::Processed);
    assert_eq!(refund.amount, 2_500i128);

    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert!(!trigger.active);
}

#[test]
fn test_evaluate_auto_refund_holds_before_timeout() {
    let env = Env::default();
    env.ledger().set_timestamp(5_000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_contract = install_mock_payment_contract(&env, sample_payment(&env, &merchant, &customer, &token));
    client.set_payment_contract_address(&admin, &payment_contract);

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &7u64,
        &AutoRefundCondition::FulfillmentTimeout(FulfillmentTimeoutCondition { fulfillment_deadline: 9_000 }),
        &2_500u32,
    );

    assert!(!client.evaluate_auto_refund(&trigger_id));
    assert!(client.try_get_refund(&1u64).is_err());

    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert!(trigger.active);
}

#[test]
fn test_evaluate_auto_refund_triggers_on_contract_state_match() {
    let env = Env::default();
    env.ledger().set_timestamp(10_000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_contract = install_mock_payment_contract(&env, sample_payment(&env, &merchant, &customer, &token));
    client.set_payment_contract_address(&admin, &payment_contract);

    let state_contract_id = env.register(MockStateContract, ());
    let state_client = MockStateContractClient::new(&env, &state_contract_id);
    let key = BytesN::from_array(&env, &[1; 32]);
    let expected = Bytes::from_slice(&env, b"fulfilled");
    state_client.set_contract_state(&key, &expected);

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &7u64,
        &AutoRefundCondition::ContractStateMatch(ContractStateMatchCondition {
            contract: state_contract_id,
            key: key.clone(),
            expected: expected.clone(),
        }),
        &5_000u32,
    );

    assert!(client.evaluate_auto_refund(&trigger_id));

    let refund = client.get_refund(&1u64);
    assert_eq!(refund.status, RefundStatus::Processed);
    assert_eq!(refund.amount, 5_000i128);
}

#[test]
fn test_evaluate_auto_refund_cannot_retrigger_after_success() {
    let env = Env::default();
    env.ledger().set_timestamp(10_000);
    let (client, admin) = setup(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_contract = install_mock_payment_contract(&env, sample_payment(&env, &merchant, &customer, &token));
    client.set_payment_contract_address(&admin, &payment_contract);

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &7u64,
        &AutoRefundCondition::FulfillmentTimeout(FulfillmentTimeoutCondition { fulfillment_deadline: 9_000 }),
        &2_500u32,
    );

    assert!(client.evaluate_auto_refund(&trigger_id));
    assert!(!client.evaluate_auto_refund(&trigger_id));
    assert_eq!(client.get_refund(&1u64).status, RefundStatus::Processed);
    assert!(client.try_get_refund(&2u64).is_err());
}
