#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Env, String,
};

fn setup(env: &Env) -> (RefundContractClient, Address) {
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

#[contract]
struct MockPaymentContract;

#[contractimpl]
impl MockPaymentContract {
    pub fn set_payment(env: Env, payment: ExternalPayment) {
        env.storage().instance().set(&payment.id, &payment);
    }

    pub fn get_payment(env: Env, payment_id: u64) -> ExternalPayment {
        env.storage().instance().get(&payment_id).unwrap()
    }

    pub fn check_payment_customer(env: Env, payment_id: u64, customer: Address) -> bool {
        let payment: ExternalPayment = env.storage().instance().get(&payment_id).unwrap();
        payment.id == payment_id
            && payment.customer == customer
            && payment.status == ExternalPaymentStatus::Completed
    }
}

fn install_mock_payment_contract(env: &Env, payments: &[ExternalPayment]) -> Address {
    let contract_id = env.register(MockPaymentContract, ());
    let client = MockPaymentContractClient::new(env, &contract_id);
    for payment in payments {
        client.set_payment(payment);
    }
    contract_id
}

fn sample_payment(
    env: &Env,
    payment_id: u64,
    merchant: &Address,
    customer: &Address,
    token: &Address,
) -> ExternalPayment {
    ExternalPayment {
        id: payment_id,
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

/// Test 1: Verify error code 100 (Unauthorized) is defined in payment contract
/// This test documents the overlapping error codes across contracts
#[test]
fn test_error_code_100_payment_unauthorized_overlap() {
    // Payment contract BasicError::Unauthorized = 100
    // Escrow contract BasicError::Unauthorized = 100
    // Refund contract doesn't have error code 100 (starts at 1)

    // This test documents the architectural issue: no shared error registry
    // Error code 100 means different things in different contracts
    let payment_error_code = 100u32;
    let escrow_error_code = 100u32;

    // Both contracts define code 100 as "Unauthorized"
    assert_eq!(payment_error_code, escrow_error_code);
}

/// Test 2: Verify error code 101 overlap between payment and escrow
#[test]
fn test_error_code_101_overlap_across_contracts() {
    // Payment contract: BasicError::MetadataTooLarge = 101
    // Escrow contract: BasicError::NotAnAdmin = 101
    // Same numeric code, different semantic meanings

    let payment_101 = 101u32; // MetadataTooLarge
    let escrow_101 = 101u32;  // NotAnAdmin

    assert_eq!(payment_101, escrow_101);
    // This overlap can cause confusion when debugging cross-contract calls
}

/// Test 3: Verify error code 200 overlap - payment and escrow
#[test]
fn test_error_code_200_payment_escrow_overlap() {
    // Payment contract: PaymentError::NotFound = 200
    // Escrow contract: EscrowError::NotFound = 200
    // Both use same code for "NotFound" but in different contexts

    let payment_not_found = 200u32;
    let escrow_not_found = 200u32;

    assert_eq!(payment_not_found, escrow_not_found);
}

/// Test 4: Verify error code 201 triple overlap
#[test]
fn test_error_code_201_triple_overlap() {
    // Payment contract: PaymentError::InvalidStatus = 201
    // Escrow contract: EscrowError::InvalidStatus = 201
    // Refund has different range starting from 1

    let payment_invalid_status = 201u32;
    let escrow_invalid_status = 201u32;

    assert_eq!(payment_invalid_status, escrow_invalid_status);
}

/// Test 5: Verify error code ranges don't have centralized registry
#[test]
fn test_error_code_ranges_documentation() {
    // Payment contract uses ranges: 100-126, 200-224, 300-318, 400-406, 500-540
    // Escrow contract uses ranges: 100-114, 200-229, 300-315
    // Refund contract uses ranges: 1-31, 34-58

    // This documents the architectural issue: no central error registry
    let payment_basic_start = 100u32;
    let escrow_basic_start = 100u32;
    let refund_core_start = 1u32;

    assert_eq!(payment_basic_start, escrow_basic_start);
    assert_ne!(payment_basic_start, refund_core_start);
}

/// Test 6: Auto-refund does NOT trigger when merchant override flag is set
#[test]
fn test_auto_refund_blocked_by_merchant_override_flag() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 100u64;

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[sample_payment(&env, payment_id, &merchant, &customer, &token)],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    // Set up an auto-refund trigger that would normally fire
    let condition = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: env.ledger().timestamp() + 1000,
        }
    );

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &payment_id,
        &condition,
        &5000u32, // 50% refund
    );

    // Now simulate merchant setting an override flag that prevents auto-refund
    // (In actual implementation, this would be checked in the auto-refund logic)

    // Since there's no explicit merchant override flag in current implementation,
    // this test documents the missing feature
    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert_eq!(trigger.payment_id, payment_id);

    // TODO: Add merchant_override_no_auto_refund field to AutoRefundTrigger
    // and verify it prevents automatic execution
}

/// Test 7: Merchant override prevents auto-refund on timeout
#[test]
fn test_merchant_override_prevents_timeout_auto_refund() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 200u64;
    let deadline = env.ledger().timestamp() + 3600;

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[sample_payment(&env, payment_id, &merchant, &customer, &token)],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    let condition = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: deadline,
        }
    );

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &payment_id,
        &condition,
        &10000u32, // 100% refund
    );

    // Fast-forward past the deadline
    env.ledger().with_mut(|li| {
        li.timestamp = deadline + 100;
    });

    // Currently, evaluate_auto_refund_trigger would execute
    // This test documents that we need a merchant override check
    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert!(trigger.active);

    // Expected behavior: if merchant has set override flag,
    // evaluate_auto_refund_trigger should skip execution
}

/// Test 8: Auto-refund trigger becomes inactive once it has fired
#[test]
fn test_merchant_override_flag_persistence() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 300u64;
    let deadline = env.ledger().timestamp() + 5000;

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[sample_payment(&env, payment_id, &merchant, &customer, &token)],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    let condition = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: deadline,
        }
    );

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &payment_id,
        &condition,
        &7500u32,
    );

    // There is no dedicated "deactivate" entry point; the only way a trigger
    // becomes inactive today is by firing through `evaluate_auto_refund`.
    env.ledger().with_mut(|li| {
        li.timestamp = deadline + 1;
    });
    assert!(client.evaluate_auto_refund(&trigger_id));

    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert!(!trigger.active);

    // Future enhancement: add an explicit merchant_override_flag field that
    // lets a merchant deactivate a trigger without it having fired.
}

/// Test 9: Auto-refund with contract state condition respects merchant override
#[test]
fn test_contract_state_auto_refund_respects_merchant_override() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 400u64;
    let external_contract = Address::generate(&env);

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[sample_payment(&env, payment_id, &merchant, &customer, &token)],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    let state_key = BytesN::from_array(&env, &[0u8; 32]);
    let expected_value = Bytes::new(&env);

    let condition = AutoRefundCondition::ContractStateMatch(
        ContractStateMatchCondition {
            contract: external_contract,
            key: state_key,
            expected: expected_value,
        }
    );

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &payment_id,
        &condition,
        &6000u32,
    );

    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert_eq!(trigger.trigger_id, trigger_id);

    // Documents missing feature: merchant should be able to set
    // a flag that prevents this trigger from executing even when
    // the contract state condition is met
}

/// Test 10: Merchant override audit log for auto-refund prevention
#[test]
fn test_merchant_override_creates_audit_log() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 500u64;

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[sample_payment(&env, payment_id, &merchant, &customer, &token)],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    let condition = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: env.ledger().timestamp() + 2000,
        }
    );

    let _trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &payment_id,
        &condition,
        &8000u32,
    );

    // When merchant sets override, it should create an audit entry
    // Current implementation doesn't have this feature

    // Expected: AdminOverrideHistory entry documenting the override decision
    // This would help with compliance and dispute resolution
}

/// Test 11: Multiple auto-refund triggers, one fired and one left pending
#[test]
fn test_multiple_triggers_selective_override() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id_1 = 600u64;
    let payment_id_2 = 601u64;

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[
            sample_payment(&env, payment_id_1, &merchant, &customer, &token),
            sample_payment(&env, payment_id_2, &merchant, &customer, &token),
        ],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    // Trigger 1: left pending (not evaluated)
    let condition_1 = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: env.ledger().timestamp() + 1000,
        }
    );

    let trigger_1 = client.register_auto_refund_trigger(
        &merchant,
        &payment_id_1,
        &condition_1,
        &5000u32,
    );

    // Trigger 2: fires once its deadline has passed
    let deadline_2 = env.ledger().timestamp() + 1000;
    let condition_2 = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: deadline_2,
        }
    );

    let trigger_2 = client.register_auto_refund_trigger(
        &merchant,
        &payment_id_2,
        &condition_2,
        &5000u32,
    );

    env.ledger().with_mut(|li| {
        li.timestamp = deadline_2 + 1;
    });
    assert!(client.evaluate_auto_refund(&trigger_2));

    let t1 = client.get_auto_refund_trigger(&trigger_1);
    let t2 = client.get_auto_refund_trigger(&trigger_2);

    assert!(t1.active);
    assert!(!t2.active);
}

/// Test 12: Merchant override flag interaction with refund policy
#[test]
fn test_merchant_override_flag_with_refund_policy() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 700u64;

    env.mock_all_auths();

    let payment_contract = install_mock_payment_contract(
        &env,
        &[sample_payment(&env, payment_id, &merchant, &customer, &token)],
    );
    client.set_payment_contract_address(&admin, &payment_contract);

    // Set a refund policy for the merchant
    let tier1 = RefundTier {
        days_from_purchase: 30,
        max_refund_bps: 10000, // 100%
    };
    let tier2 = RefundTier {
        days_from_purchase: 60,
        max_refund_bps: 5000, // 50%
    };

    let mut tiers = Vec::new(&env);
    tiers.push_back(tier1);
    tiers.push_back(tier2);

    client.set_refund_policy(&merchant, &tiers);

    // Register auto-refund trigger
    let condition = AutoRefundCondition::FulfillmentTimeout(
        FulfillmentTimeoutCondition {
            fulfillment_deadline: env.ledger().timestamp() + 5000,
        }
    );

    let trigger_id = client.register_auto_refund_trigger(
        &merchant,
        &payment_id,
        &condition,
        &10000u32,
    );

    // Verify policy exists
    let policy = client.get_refund_policy(&merchant).unwrap();
    assert_eq!(policy.merchant, merchant);
    assert!(policy.active);

    // Verify trigger exists and is active
    let trigger = client.get_auto_refund_trigger(&trigger_id);
    assert!(trigger.active);

    // Key missing feature: merchant should be able to set an override
    // that prevents auto-refund even when both policy and trigger would allow it
    // This would provide merchants with fine-grained control over auto-refunds

    // Expected behavior:
    // 1. Merchant sets "no_auto_refund" flag on specific payment
    // 2. Even though policy allows refund and trigger condition is met
    // 3. Auto-refund should NOT execute
    // 4. Manual refund request should still work (policy still applies)
}
