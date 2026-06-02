#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_set_payment_refund_cap() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let payment_id = 1u64;

    env.mock_all_auths();
    client.initialize(&admin);

    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 5,
        max_total_amount: 5000i128,
    };

    let res = client.try_set_payment_refund_cap(&admin, &cap);
    assert!(res.is_ok());

    let retrieved_cap = client.get_payment_refund_cap(&payment_id);
    assert!(retrieved_cap.is_some());
    let retrieved = retrieved_cap.unwrap();
    assert_eq!(retrieved.payment_id, payment_id);
    assert_eq!(retrieved.max_refund_count, 5);
    assert_eq!(retrieved.max_total_amount, 5000i128);
}

#[test]
fn test_get_payment_refund_usage_default() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    let payment_id = 1u64;
    let (count, amount) = client.get_payment_refund_usage(&payment_id);

    assert_eq!(count, 0);
    assert_eq!(amount, 0);
}

#[test]
fn test_refund_count_cap_exceeded() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set cap: max 2 refunds for this payment
    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 2,
        max_total_amount: 10000i128,
    };
    client.set_payment_refund_cap(&admin, &cap);

    // First refund should succeed
    let res1 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res1.is_ok());

    // Second refund should succeed
    let res2 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res2.is_ok());

    // Third refund should fail with RefundCountCapExceeded
    let res3 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res3.is_err());
    assert_eq!(res3.unwrap_err().unwrap(), Error::RefundCountCapExceeded);
}

#[test]
fn test_refund_amount_cap_exceeded() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set cap: max 3 refunds, max 500 total amount for this payment
    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 3,
        max_total_amount: 500i128,
    };
    client.set_payment_refund_cap(&admin, &cap);

    // First refund: 200 (total: 200, within 500 limit)
    let res1 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &200,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res1.is_ok());

    // Second refund: 200 (total: 400, within 500 limit)
    let res2 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &200,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res2.is_ok());

    // Third refund: 150 (total would be 550, exceeds 500 limit)
    let res3 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &150,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res3.is_err());
    assert_eq!(res3.unwrap_err().unwrap(), Error::RefundAmountCapExceeded);
}

#[test]
fn test_cumulative_amount_enforcement() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set cap: max 1000 total amount
    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 10,
        max_total_amount: 1000i128,
    };
    client.set_payment_refund_cap(&admin, &cap);

    // Add refund 1: 400
    let res1 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &400,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res1.is_ok());

    let (count1, amount1) = client.get_payment_refund_usage(&payment_id);
    assert_eq!(count1, 1);
    assert_eq!(amount1, 400i128);

    // Add refund 2: 350 (cumulative: 750)
    let res2 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &350,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res2.is_ok());

    let (count2, amount2) = client.get_payment_refund_usage(&payment_id);
    assert_eq!(count2, 2);
    assert_eq!(amount2, 750i128);

    // Add refund 3: 300 (cumulative would be 1050, exceeds 1000)
    let res3 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &300,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res3.is_err());
    assert_eq!(res3.unwrap_err().unwrap(), Error::RefundAmountCapExceeded);

    // But 250 should succeed (cumulative: 1000)
    let res4 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &250,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res4.is_ok());

    let (count4, amount4) = client.get_payment_refund_usage(&payment_id);
    assert_eq!(count4, 3);
    assert_eq!(amount4, 1000i128);
}

#[test]
fn test_no_cap_allows_unlimited() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // No cap is set for this payment, so unlimited refunds should be allowed
    
    // Request multiple refunds without a cap
    for i in 0..5 {
        let res = client.try_request_refund(
            &merchant,
            &payment_id,
            &customer,
            &100,
            &1000,
            &token,
            &reason,
            &RefundReasonCode::CustomerRequest,
            &0,
        );
        assert!(res.is_ok(), "Refund {} should succeed without cap", i + 1);
    }

    let (count, amount) = client.get_payment_refund_usage(&payment_id);
    assert_eq!(count, 5);
    assert_eq!(amount, 500i128);
}

#[test]
fn test_multiple_payments_independent_caps() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set cap for payment 1: max 2 refunds, 500 total
    let cap1 = PaymentRefundCap {
        payment_id: 1,
        max_refund_count: 2,
        max_total_amount: 500i128,
    };
    client.set_payment_refund_cap(&admin, &cap1);

    // Set cap for payment 2: max 3 refunds, 1000 total
    let cap2 = PaymentRefundCap {
        payment_id: 2,
        max_refund_count: 3,
        max_total_amount: 1000i128,
    };
    client.set_payment_refund_cap(&admin, &cap2);

    // Request refunds for payment 1
    for i in 0..2 {
        let res = client.try_request_refund(
            &merchant,
            &1,
            &customer,
            &250,
            &1000,
            &token,
            &reason,
            &RefundReasonCode::CustomerRequest,
            &0,
        );
        assert!(res.is_ok(), "Payment 1 refund {} should succeed", i + 1);
    }

    // Third refund for payment 1 should fail
    let res_fail = client.try_request_refund(
        &merchant,
        &1,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res_fail.is_err());

    // Request refunds for payment 2 (should succeed since it has higher cap)
    for i in 0..3 {
        let res = client.try_request_refund(
            &merchant,
            &2,
            &customer,
            &300,
            &1000,
            &token,
            &reason,
            &RefundReasonCode::CustomerRequest,
            &0,
        );
        assert!(res.is_ok(), "Payment 2 refund {} should succeed", i + 1);
    }

    let (count1, amount1) = client.get_payment_refund_usage(&1);
    assert_eq!(count1, 2);
    assert_eq!(amount1, 500i128);

    let (count2, amount2) = client.get_payment_refund_usage(&2);
    assert_eq!(count2, 3);
    assert_eq!(amount2, 900i128);
}

#[test]
fn test_cap_update() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let payment_id = 1u64;

    env.mock_all_auths();
    client.initialize(&admin);

    // Set initial cap
    let cap1 = PaymentRefundCap {
        payment_id,
        max_refund_count: 3,
        max_total_amount: 300i128,
    };
    client.set_payment_refund_cap(&admin, &cap1);

    let retrieved1 = client.get_payment_refund_cap(&payment_id).unwrap();
    assert_eq!(retrieved1.max_refund_count, 3);
    assert_eq!(retrieved1.max_total_amount, 300i128);

    // Update cap with higher limits
    let cap2 = PaymentRefundCap {
        payment_id,
        max_refund_count: 5,
        max_total_amount: 1000i128,
    };
    client.set_payment_refund_cap(&admin, &cap2);

    let retrieved2 = client.get_payment_refund_cap(&payment_id).unwrap();
    assert_eq!(retrieved2.max_refund_count, 5);
    assert_eq!(retrieved2.max_total_amount, 1000i128);
}

#[test]
fn test_unauthorized_set_cap() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let payment_id = 1u64;

    env.mock_all_auths();
    client.initialize(&admin);

    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 3,
        max_total_amount: 300i128,
    };

    // Attempt to set cap as non-admin
    let res = client.try_set_payment_refund_cap(&unauthorized, &cap);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_invalid_payment_id_cap() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin);

    // Try to set cap with payment_id = 0
    let cap = PaymentRefundCap {
        payment_id: 0,
        max_refund_count: 3,
        max_total_amount: 300i128,
    };

    let res = client.try_set_payment_refund_cap(&admin, &cap);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().unwrap(), Error::InvalidPaymentId);
}

#[test]
fn test_exact_boundary_amount() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set cap: exactly 500 total amount
    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 10,
        max_total_amount: 500i128,
    };
    client.set_payment_refund_cap(&admin, &cap);

    // Refund exactly 500 (should succeed)
    let res1 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &500,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res1.is_ok());

    // Any additional amount should fail
    let res2 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &1,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res2.is_err());
    assert_eq!(res2.unwrap_err().unwrap(), Error::RefundAmountCapExceeded);
}

#[test]
fn test_exact_boundary_count() {
    let env = Env::default();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let token = Address::generate(&env);
    let payment_id = 1u64;
    let reason = String::from_str(&env, "Customer requested");

    env.mock_all_auths();
    client.initialize(&admin);

    // Set cap: exactly 2 refunds
    let cap = PaymentRefundCap {
        payment_id,
        max_refund_count: 2,
        max_total_amount: 10000i128,
    };
    client.set_payment_refund_cap(&admin, &cap);

    // First refund succeeds
    let res1 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res1.is_ok());

    // Second refund succeeds
    let res2 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res2.is_ok());

    // Third refund fails
    let res3 = client.try_request_refund(
        &merchant,
        &payment_id,
        &customer,
        &100,
        &1000,
        &token,
        &reason,
        &RefundReasonCode::CustomerRequest,
        &0,
    );
    assert!(res3.is_err());
    assert_eq!(res3.unwrap_err().unwrap(), Error::RefundCountCapExceeded);
}
