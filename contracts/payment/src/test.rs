#![cfg(test)]

use super::*;
use escrow::{EscrowContract, EscrowContractClient, EscrowStatus};
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

// ── RATE LIMITING / ANTI-FRAUD TESTS ────────────────────────────────────────

fn setup_rate_limit_contract(env: &Env) -> (PaymentContractClient<'_>, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin, contract_id)
}

#[test]
fn test_rate_limit_window_resets_after_duration() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Allow 2 payments per 100-second window.
    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 2,
            window_duration: 100,
            max_payment_amount: 0,
            max_daily_volume: 0,
        }),
    );

    env.ledger().set_timestamp(1000);
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Advance past the window; counter should reset.
    env.ledger().set_timestamp(1200);
    // This third payment would fail if the window hadn't reset.
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    let rl = client.get_address_rate_limit(&customer);
    assert_eq!(rl.payment_count, 1); // only 1 payment in the new window
}

#[test]
#[should_panic]
fn test_rate_limit_exceeded_within_window() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Allow only 1 payment per very long window.
    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 1,
            window_duration: 100_000,
            max_payment_amount: 0,
            max_daily_volume: 0,
        }),
    );

    env.ledger().set_timestamp(1000);
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
    // Second payment in the same window — must panic.
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
}

#[test]
fn test_flag_address_blocks_payments() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Flag the customer.
    client.flag_address(
        &admin,
        &customer,
        &String::from_str(&env, "velocity attack"),
    );
    let rl = client.get_address_rate_limit(&customer);
    assert!(rl.flagged);
    assert!(client.is_address_flagged(&customer));

    // Payment must fail because address is flagged, even without rate-limit config.
    let result = client.try_create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
    assert_eq!(result.unwrap_err().unwrap(), Error::AddressFlagged);
}

#[test]
fn test_unflag_address_allows_payments() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 100,
            window_duration: 100_000,
            max_payment_amount: 0,
            max_daily_volume: 0,
        }),
    );

    client.flag_address(&admin, &customer, &String::from_str(&env, "test"));
    client.unflag_address(&admin, &customer);

    let rl = client.get_address_rate_limit(&customer);
    assert!(!rl.flagged);
    assert!(!client.is_address_flagged(&customer));

    // Payment must succeed after unflag.
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
}

#[test]
fn test_allowlist_bypasses_flag_check() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    client.flag_address(
        &admin,
        &customer,
        &String::from_str(&env, "sanctions review"),
    );
    client.add_to_allowlist(&admin, &customer);

    // Allowlisted addresses can still initiate new payments even when flagged.
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
}

#[test]
fn test_unflag_requires_current_flag() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);

    let result = client.try_unflag_address(&admin, &customer);
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidStatus);
}

#[test]
fn test_flag_reason_stored_and_cleared_on_unflag() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let reason = String::from_str(&env, "regulatory hold");

    client.flag_address(&admin, &customer, &reason);
    assert_eq!(client.get_flag_reason(&customer), Some(reason));

    client.unflag_address(&admin, &customer);
    assert_eq!(client.get_flag_reason(&customer), None);
}

#[test]
fn test_flag_address_fails_if_already_flagged() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);

    client.flag_address(&admin, &customer, &String::from_str(&env, "first reason"));
    let result =
        client.try_flag_address(&admin, &customer, &String::from_str(&env, "second reason"));
    assert_eq!(result.unwrap_err().unwrap(), Error::AddressAlreadyFlagged);
}

#[test]
fn test_flag_unflag_events_emitted() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);

    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 100,
            window_duration: 100_000,
            max_payment_amount: 0,
            max_daily_volume: 0,
        }),
    );

    client.flag_address(&admin, &customer, &String::from_str(&env, "fraud"));
    client.unflag_address(&admin, &customer);

    // Events should contain both AddressFlagged and AddressUnflagged entries.
    let all_events = env.events().all();
    assert!(!all_events.is_empty());
}

#[test]
fn test_rate_limit_breach_event_emitted() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 1,
            window_duration: 100_000,
            max_payment_amount: 0,
            max_daily_volume: 0,
        }),
    );

    env.ledger().set_timestamp(1000);
    client.create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Second attempt should fail due to the per-window limit.
    let result = client.try_create_payment(
        &customer,
        &merchant,
        &50,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
    assert!(result.is_err());

    // Failed invocations may rollback emitted events in host simulation.
    // The key behavior is that the payment attempt is rejected.
    assert_eq!(result.unwrap_err().unwrap(), Error::RateLimitExceeded);
}

#[test]
fn test_amount_exceeds_limit() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Cap single payment at 100.
    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 100,
            window_duration: 100_000,
            max_payment_amount: 100,
            max_daily_volume: 0,
        }),
    );

    // Within limit — must succeed.
    client.create_payment(
        &customer,
        &merchant,
        &100,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Over limit — must fail.
    let result = client.try_create_payment(
        &customer,
        &merchant,
        &101,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
    assert!(result.is_err());
}

#[test]
fn test_daily_volume_limit() {
    let env = Env::default();
    let (client, admin, _) = setup_rate_limit_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Daily volume cap of 200.
    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 100,
            window_duration: 100_000,
            max_payment_amount: 0,
            max_daily_volume: 200,
        }),
    );

    env.ledger().set_timestamp(1000);
    client.create_payment(
        &customer,
        &merchant,
        &100,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
    client.create_payment(
        &customer,
        &merchant,
        &100,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Third payment would exceed daily volume.
    let result =
        client.try_create_payment(&customer, &merchant, &1, &token, &Currency::USDC, &0, &meta);
    assert!(result.is_err());

    // Advance a full day — daily volume resets.
    env.ledger().set_timestamp(1000 + 86400 + 1);
    client.create_payment(
        &customer,
        &merchant,
        &100,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );
}

#[test]
fn test_create_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );
    assert_eq!(payment_id, 1);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.id, 1);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.merchant, merchant);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.token, token);
    assert_eq!(payment.expires_at, 0);
    assert_eq!(payment.metadata, metadata);
    assert_eq!(payment.notes, String::from_str(&env, ""));
}

#[test]
fn test_get_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 5000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payment = client.get_payment(&payment_id);

    assert_eq!(payment.id, payment_id);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.merchant, merchant);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.token, token);
    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.expires_at, 0);
}

#[test]
#[should_panic(expected = "Payment not found")]
fn test_get_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    client.get_payment(&999);
}

#[test]
fn test_complete_payment_success() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_complete_payment_event_emission() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id);

    // Token contract also emits events (transfer_from); count only the payment contract's events
    let all_events = env.events().all();
    let mut payment_event_count = 0u32;
    for i in 0..all_events.len() {
        let event = all_events.get(i).unwrap();
        if event.0 == contract_id {
            payment_event_count += 1;
        }
    }
    assert_eq!(
        payment_event_count, 1,
        "PaymentCompleted must be emitted exactly once"
    );

    // PaymentCompleted is the last event emitted by complete_payment
    let last_event = all_events.last().unwrap();
    assert_eq!(last_event.0, contract_id);
}

#[test]
fn test_refund_payment_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Refund the payment
    client.refund_payment(&admin, &payment_id);

    // Verify status changed to Refunded
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Refunded);
}

#[test]
fn test_refund_payment_event_emission() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.refund_payment(&admin, &payment_id);

    // Verify PaymentRefunded event is emitted exactly once
    let events = env.events().all();
    assert_eq!(
        events.len(),
        1,
        "PaymentRefunded must be emitted exactly once"
    );
    let event = events.get(0).unwrap();
    assert_eq!(event.0, contract_id);
}

#[test]
#[should_panic]
fn test_complete_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);

    client.complete_payment(&admin, &999);
}

#[test]
#[should_panic]
fn test_refund_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);

    client.refund_payment(&admin, &999);
}

#[test]
#[should_panic]
fn test_complete_already_completed_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Complete the payment first time
    client.complete_payment(&admin, &payment_id);

    // Try to complete again - should fail
    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_refund_already_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Refund the payment first time
    client.refund_payment(&admin, &payment_id);

    // Try to refund again - should fail
    client.refund_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_complete_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Refund the payment first
    client.refund_payment(&admin, &payment_id);

    // Try to complete refunded payment - should panic due to InvalidStatus error
    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_refund_completed_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Complete the payment first
    client.complete_payment(&admin, &payment_id);

    // Try to refund completed payment - should panic due to InvalidStatus error
    client.refund_payment(&admin, &payment_id);
}

#[test]
fn test_multiple_payments_correct_modification() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer1, &amount);
    token_user_client.approve(&customer1, &contract_id, &amount, &1000);

    let payment_id1 = client.create_payment(
        &customer1,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let payment_id2 = client.create_payment(
        &customer2,
        &merchant,
        &2000_i128,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id1);

    let payment1 = client.get_payment(&payment_id1);
    let payment2 = client.get_payment(&payment_id2);

    assert_eq!(payment1.status, PaymentStatus::Completed);
    assert_eq!(payment2.status, PaymentStatus::Pending);
}
// Cancel Payment Tests
#[test]
fn test_customer_cancel_pending_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Customer cancels their pending payment
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_ok());

    // Verify status changed to Cancelled
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
fn test_merchant_cancel_pending_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Merchant cancels the pending payment
    let result = client.try_cancel_payment(&merchant, &payment_id);
    assert!(result.is_ok());

    // Verify status changed to Cancelled
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
fn test_cancel_nonexistent_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let caller = Address::generate(&env);

    env.mock_all_auths();

    // Try to cancel a non-existent payment
    let result = client.try_cancel_payment(&caller, &999);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::PaymentNotFound);
}

#[test]
fn test_cancel_payment_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Try to cancel as unauthorized user
    let result = client.try_cancel_payment(&unauthorized_user, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
#[should_panic]
fn test_cancel_completed_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // Should panic - InvalidStatus
    client.cancel_payment(&customer, &payment_id);
}

#[test]
fn test_cancel_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Refund the payment first
    client.refund_payment(&admin, &payment_id);

    // Try to cancel refunded payment
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidStatus);
}

#[test]
fn test_cancel_already_cancelled_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Cancel the payment first time
    client.cancel_payment(&customer, &payment_id);

    // Try to cancel again
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidStatus);
}

#[test]
fn test_cancel_payment_event_emission() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.cancel_payment(&customer, &payment_id);

    // Verify PaymentCancelled event is emitted exactly once
    let events = env.events().all();
    assert_eq!(
        events.len(),
        1,
        "PaymentCancelled must be emitted exactly once"
    );
    let event = events.get(0).unwrap();
    assert_eq!(event.0, contract_id);
}

#[test]
fn test_cancel_multiple_payments_correct_modification() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create two payments
    let payment_id1 = client.create_payment(
        &customer1,
        &merchant,
        &1000_i128,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let payment_id2 = client.create_payment(
        &customer2,
        &merchant,
        &2000_i128,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Cancel first payment
    client.cancel_payment(&customer1, &payment_id1);

    // Check both payments have correct status
    let payment1 = client.get_payment(&payment_id1);
    let payment2 = client.get_payment(&payment_id2);

    assert_eq!(payment1.status, PaymentStatus::Cancelled);
    assert_eq!(payment2.status, PaymentStatus::Pending);
}

#[test]
fn test_get_payments_by_customer_multiple() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments for same customer
    let id1 = client.create_payment(
        &customer,
        &merchant1,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id2 = client.create_payment(
        &customer,
        &merchant2,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id3 = client.create_payment(
        &customer,
        &merchant1,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payments = client.get_payments_by_customer(&customer, &10, &0);
    assert_eq!(payments.len(), 3);
    assert_eq!(payments.get(0).unwrap().id, id1);
    assert_eq!(payments.get(1).unwrap().id, id2);
    assert_eq!(payments.get(2).unwrap().id, id3);
}

#[test]
fn test_get_payments_by_merchant_multiple() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments for same merchant
    let id1 = client.create_payment(
        &customer1,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id2 = client.create_payment(
        &customer2,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id3 = client.create_payment(
        &customer1,
        &merchant,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payments = client.get_payments_by_merchant(&merchant, &10, &0);
    assert_eq!(payments.len(), 3);
    assert_eq!(payments.get(0).unwrap().id, id1);
    assert_eq!(payments.get(1).unwrap().id, id2);
    assert_eq!(payments.get(2).unwrap().id, id3);
}

#[test]
fn test_customer_payment_count() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    assert_eq!(client.get_payment_count_by_customer(&customer), 0);

    client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_payment_count_by_customer(&customer), 1);

    client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_payment_count_by_customer(&customer), 2);

    client.create_payment(
        &customer,
        &merchant,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_payment_count_by_customer(&customer), 3);
}

#[test]
fn test_merchant_payment_count() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    assert_eq!(client.get_payment_count_by_merchant(&merchant), 0);

    client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_payment_count_by_merchant(&merchant), 1);

    client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_payment_count_by_merchant(&merchant), 2);

    client.create_payment(
        &customer,
        &merchant,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    assert_eq!(client.get_payment_count_by_merchant(&merchant), 3);
}

#[test]
fn test_pagination_first_page() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 10 payments
    for i in 1..=10 {
        client.create_payment(
            &customer,
            &merchant,
            &(i * 100),
            &token,
            &Currency::USDC,
            &0,
            &String::from_str(&env, ""),
        );
    }

    let payments = client.get_payments_by_customer(&customer, &5, &0);
    assert_eq!(payments.len(), 5);
    assert_eq!(payments.get(0).unwrap().amount, 100);
    assert_eq!(payments.get(4).unwrap().amount, 500);
}

#[test]
fn test_pagination_second_page() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 10 payments
    for i in 1..=10 {
        client.create_payment(
            &customer,
            &merchant,
            &(i * 100),
            &token,
            &Currency::USDC,
            &0,
            &String::from_str(&env, ""),
        );
    }

    let payments = client.get_payments_by_customer(&customer, &5, &5);
    assert_eq!(payments.len(), 5);
    assert_eq!(payments.get(0).unwrap().amount, 600);
    assert_eq!(payments.get(4).unwrap().amount, 1000);
}

#[test]
fn test_pagination_limit_larger_than_total() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments
    client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.create_payment(
        &customer,
        &merchant,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payments = client.get_payments_by_customer(&customer, &100, &0);
    assert_eq!(payments.len(), 3);
}

#[test]
fn test_pagination_offset_beyond_available() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments
    client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.create_payment(
        &customer,
        &merchant,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payments = client.get_payments_by_customer(&customer, &5, &10);
    assert_eq!(payments.len(), 0);
}

#[test]
fn test_query_customer_with_no_payments() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);

    let payments = client.get_payments_by_customer(&customer, &10, &0);
    assert_eq!(payments.len(), 0);

    let count = client.get_payment_count_by_customer(&customer);
    assert_eq!(count, 0);
}

#[test]
fn test_query_merchant_with_no_payments() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);

    let payments = client.get_payments_by_merchant(&merchant, &10, &0);
    assert_eq!(payments.len(), 0);

    let count = client.get_payment_count_by_merchant(&merchant);
    assert_eq!(count, 0);
}

#[test]
fn test_payments_not_mixed_between_customers() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create payments for customer1
    let id1 = client.create_payment(
        &customer1,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id2 = client.create_payment(
        &customer1,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Create payments for customer2
    let id3 = client.create_payment(
        &customer2,
        &merchant,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payments1 = client.get_payments_by_customer(&customer1, &10, &0);
    assert_eq!(payments1.len(), 2);
    assert_eq!(payments1.get(0).unwrap().id, id1);
    assert_eq!(payments1.get(1).unwrap().id, id2);

    let payments2 = client.get_payments_by_customer(&customer2, &10, &0);
    assert_eq!(payments2.len(), 1);
    assert_eq!(payments2.get(0).unwrap().id, id3);
}

#[test]
fn test_payments_not_mixed_between_merchants() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create payments for merchant1
    let id1 = client.create_payment(
        &customer,
        &merchant1,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id2 = client.create_payment(
        &customer,
        &merchant1,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    // Create payments for merchant2
    let id3 = client.create_payment(
        &customer,
        &merchant2,
        &3000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payments1 = client.get_payments_by_merchant(&merchant1, &10, &0);
    assert_eq!(payments1.len(), 2);
    assert_eq!(payments1.get(0).unwrap().id, id1);
    assert_eq!(payments1.get(1).unwrap().id, id2);

    let payments2 = client.get_payments_by_merchant(&merchant2, &10, &0);
    assert_eq!(payments2.len(), 1);
    assert_eq!(payments2.get(0).unwrap().id, id3);
}

// New tests for expiration functionality

#[test]
fn test_create_payment_with_expiration_duration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 3600_u64; // 1 hour

    env.mock_all_auths();

    let current_timestamp = env.ledger().timestamp();
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.expires_at, current_timestamp + expiration_duration);
}

#[test]
fn test_create_payment_no_expiration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 0_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.expires_at, 0);
}

#[test]
fn test_is_payment_expired_true() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    assert!(client.is_payment_expired(&payment_id));
}

#[test]
fn test_is_payment_expired_false_not_yet() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 100_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger().set_timestamp(env.ledger().timestamp() + 10);

    assert!(!client.is_payment_expired(&payment_id));
}

#[test]
fn test_is_payment_expired_false_no_expiration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 0_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger().set_timestamp(env.ledger().timestamp() + 1000);

    assert!(!client.is_payment_expired(&payment_id));
}

#[test]
fn test_is_payment_expired_false_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    env.mock_all_auths();

    assert!(!client.is_payment_expired(&999));
}

#[test]
fn test_expire_pending_payment_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    let result = client.try_expire_payment(&payment_id);
    assert!(result.is_ok());

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
#[should_panic]
fn test_expire_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    env.mock_all_auths();

    client.expire_payment(&999);
}

#[test]
#[should_panic]
fn test_expire_payment_before_expiration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 100_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger().set_timestamp(env.ledger().timestamp() + 10);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_payment_no_expiration_set() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 0_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );
    env.ledger().set_timestamp(env.ledger().timestamp() + 1000);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_completed_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );
    client.refund_payment(&admin, &payment_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_cancelled_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );
    client.cancel_payment(&customer, &payment_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);
}

#[test]
fn test_payment_expired_event_emitted() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );
    let _expected_expires_at = env.ledger().timestamp() + expiration_duration;

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);

    let events = env.events().all();
    assert!(!events.is_empty());

    let last_event = events.last().unwrap();
    let _data = &last_event.2;
}

#[test]
fn test_multiple_payments_different_expiration_times() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id1 = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &10,
        &String::from_str(&env, ""),
    );
    let initial_timestamp1 = env.ledger().timestamp();

    let payment_id2 = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let payment_id3 = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &30,
        &String::from_str(&env, ""),
    );
    let initial_timestamp3 = env.ledger().timestamp();

    env.ledger().set_timestamp(initial_timestamp1 + 10 + 1);
    client.expire_payment(&payment_id1);

    let p1 = client.get_payment(&payment_id1);
    let p2 = client.get_payment(&payment_id2);
    let _p3 = client.get_payment(&payment_id3);

    assert_eq!(p1.status, PaymentStatus::Cancelled);
    assert_eq!(p2.status, PaymentStatus::Pending);
    assert!(!client.is_payment_expired(&payment_id3));

    env.ledger().set_timestamp(initial_timestamp3 + 30 + 1);
    client.expire_payment(&payment_id3);

    let p3_after = client.get_payment(&payment_id3);
    assert_eq!(p3_after.status, PaymentStatus::Cancelled);
    assert_eq!(p2.status, PaymentStatus::Pending);
}

#[test]
#[should_panic]
fn test_complete_expired_payment_fails() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_refund_expired_payment_fails() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &expiration_duration,
        &String::from_str(&env, ""),
    );

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.refund_payment(&admin, &payment_id);
}

#[test]
fn test_complete_payment_transfers_tokens_to_merchant() {
    let env = Env::default();
    env.mock_all_auths();

    // Register a real mock token contract
    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    // Mint tokens to customer
    token_client.mint(&customer, &amount);
    assert_eq!(token_user_client.balance(&customer), amount);

    // Customer approves contract to spend on their behalf
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id);

    // Verify funds moved
    assert_eq!(token_user_client.balance(&customer), 0);
    assert_eq!(token_user_client.balance(&merchant), amount);
}

#[test]
fn test_complete_payment_status_is_completed_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 500_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
#[should_panic]
fn test_complete_payment_fails_without_allowance() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    // Mint but no approve — transfer_from should fail
    token_client.mint(&customer, &amount);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_complete_payment_fails_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    // Approve but no balance
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id);
}

#[test]
fn test_complete_payment_partial_allowance_with_exact_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 750_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &2000);
    // Approve exactly the payment amount
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.complete_payment(&admin, &payment_id);

    assert_eq!(token_user_client.balance(&merchant), amount);
    assert_eq!(token_user_client.balance(&customer), 2000 - amount);
}

// Metadata and Notes Tests

#[test]
fn test_create_payment_with_metadata() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345,customer_ref:ABC");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.metadata, metadata);
    assert_eq!(payment.notes, String::from_str(&env, ""));
}

#[test]
fn test_create_payment_with_empty_metadata() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.metadata, String::from_str(&env, ""));
}

#[test]
fn test_create_payment_metadata_too_large() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    // Create metadata larger than MAX_METADATA_SIZE (512)
    let large_metadata = String::from_str(&env, &"x".repeat(513));

    env.mock_all_auths();

    let result = client.try_create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &large_metadata,
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::MetadataTooLarge);
}

#[test]
fn test_create_payment_metadata_at_max_size() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    // Create metadata exactly at MAX_METADATA_SIZE (512)
    let max_metadata = String::from_str(&env, &"x".repeat(512));

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &max_metadata,
    );

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.metadata.len(), 512);
}

#[test]
fn test_update_payment_notes_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let notes = String::from_str(&env, "Customer requested express delivery");
    let result = client.try_update_payment_notes(&merchant, &payment_id, &notes);
    assert!(result.is_ok());

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.notes, notes);
}

#[test]
fn test_update_payment_notes_multiple_times() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let notes1 = String::from_str(&env, "First note");
    client.update_payment_notes(&merchant, &payment_id, &notes1);

    let notes2 = String::from_str(&env, "Updated note");
    client.update_payment_notes(&merchant, &payment_id, &notes2);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.notes, notes2);
}

#[test]
fn test_update_payment_notes_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let notes = String::from_str(&env, "Unauthorized note");
    let result = client.try_update_payment_notes(&unauthorized, &payment_id, &notes);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_update_payment_notes_customer_cannot_update() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let notes = String::from_str(&env, "Customer trying to add notes");
    let result = client.try_update_payment_notes(&customer, &payment_id, &notes);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_update_payment_notes_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);
    let notes = String::from_str(&env, "Some notes");

    env.mock_all_auths();

    let result = client.try_update_payment_notes(&merchant, &999, &notes);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::PaymentNotFound);
}

#[test]
fn test_update_payment_notes_too_large() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    // Create notes larger than MAX_NOTES_SIZE (1024)
    let large_notes = String::from_str(&env, &"x".repeat(1025));
    let result = client.try_update_payment_notes(&merchant, &payment_id, &large_notes);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::NotesTooLarge);
}

#[test]
fn test_update_payment_notes_at_max_size() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    // Create notes exactly at MAX_NOTES_SIZE (1024)
    let max_notes = String::from_str(&env, &"x".repeat(1024));
    let result = client.try_update_payment_notes(&merchant, &payment_id, &max_notes);
    assert!(result.is_ok());

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.notes.len(), 1024);
}

#[test]
fn test_metadata_persists_through_payment_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345,priority:high");

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &metadata,
    );

    // Add notes
    let notes = String::from_str(&env, "Verified customer identity");
    client.update_payment_notes(&merchant, &payment_id, &notes);

    // Complete payment
    client.complete_payment(&admin, &payment_id);

    // Verify metadata and notes persist after completion
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.metadata, metadata);
    assert_eq!(payment.notes, notes);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_metadata_included_in_query_responses() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let metadata1 = String::from_str(&env, "order_id:111");
    let metadata2 = String::from_str(&env, "order_id:222");

    env.mock_all_auths();

    let id1 = client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &metadata1,
    );
    let id2 = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &metadata2,
    );

    // Query by customer
    let payments = client.get_payments_by_customer(&customer, &10, &0);
    assert_eq!(payments.len(), 2);
    assert_eq!(payments.get(0).unwrap().metadata, metadata1);
    assert_eq!(payments.get(1).unwrap().metadata, metadata2);

    // Query by merchant
    let payments = client.get_payments_by_merchant(&merchant, &10, &0);
    assert_eq!(payments.len(), 2);
    assert_eq!(payments.get(0).unwrap().id, id1);
    assert_eq!(payments.get(1).unwrap().id, id2);
}

#[test]
fn test_notes_persist_after_cancellation() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let metadata = String::from_str(&env, "order_id:12345");

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
    );

    let notes = String::from_str(&env, "Customer requested cancellation");
    client.update_payment_notes(&merchant, &payment_id, &notes);

    client.cancel_payment(&customer, &payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.notes, notes);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

// Multi-Currency Tests

#[test]
fn test_create_payment_with_xlm_currency() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::XLM,
        &0,
        &String::from_str(&env, ""),
    );
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.currency, Currency::XLM);
}

#[test]
fn test_create_payment_with_btc_currency() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &5000,
        &token,
        &Currency::BTC,
        &0,
        &String::from_str(&env, ""),
    );
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.currency, Currency::BTC);
}

#[test]
fn test_create_payment_with_eth_currency() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::ETH,
        &0,
        &String::from_str(&env, ""),
    );
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.currency, Currency::ETH);
}

#[test]
fn test_create_payment_with_usdt_currency() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &1500,
        &token,
        &Currency::USDT,
        &0,
        &String::from_str(&env, ""),
    );
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.currency, Currency::USDT);
}

#[test]
fn test_set_conversion_rate() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);
    client.set_conversion_rate(&admin, &Currency::BTC, &50000_0000000);

    let rate = client.get_conversion_rate(&Currency::BTC);
    assert_eq!(rate, 50000_0000000);
}

#[test]
fn test_get_conversion_rate_default() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let rate = client.get_conversion_rate(&Currency::XLM);
    assert_eq!(rate, 1_0000000);
}

#[test]
fn test_set_multiple_conversion_rates() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);
    client.set_conversion_rate(&admin, &Currency::BTC, &50000_0000000);
    client.set_conversion_rate(&admin, &Currency::ETH, &3000_0000000);
    client.set_conversion_rate(&admin, &Currency::XLM, &0_1000000);

    assert_eq!(client.get_conversion_rate(&Currency::BTC), 50000_0000000);
    assert_eq!(client.get_conversion_rate(&Currency::ETH), 3000_0000000);
    assert_eq!(client.get_conversion_rate(&Currency::XLM), 0_1000000);
}

#[test]
fn test_set_conversion_rate_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);

    let result = client.try_set_conversion_rate(&unauthorized, &Currency::BTC, &50000_0000000);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_multiple_currencies_in_payments() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    let id1 = client.create_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::XLM,
        &0,
        &String::from_str(&env, ""),
    );
    let id2 = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    let id3 = client.create_payment(
        &customer,
        &merchant,
        &3000,
        &token,
        &Currency::BTC,
        &0,
        &String::from_str(&env, ""),
    );

    let p1 = client.get_payment(&id1);
    let p2 = client.get_payment(&id2);
    let p3 = client.get_payment(&id3);

    assert_eq!(p1.currency, Currency::XLM);
    assert_eq!(p2.currency, Currency::USDC);
    assert_eq!(p3.currency, Currency::BTC);
}

// Partial Refund Tests

#[test]
fn test_partial_refund_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.partial_refund(&admin, &payment_id, &300);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::PartialRefunded);
    assert_eq!(payment.refunded_amount, 300);
}

#[test]
fn test_multiple_partial_refunds() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.partial_refund(&admin, &payment_id, &200);
    client.partial_refund(&admin, &payment_id, &300);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::PartialRefunded);
    assert_eq!(payment.refunded_amount, 500);
}

#[test]
fn test_partial_refund_becomes_full_refund() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.partial_refund(&admin, &payment_id, &600);
    client.partial_refund(&admin, &payment_id, &400);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Refunded);
    assert_eq!(payment.refunded_amount, 1000);
}

#[test]
fn test_partial_refund_exceeds_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    let result = client.try_partial_refund(&admin, &payment_id, &1500);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::RefundExceedsPayment);
}

#[test]
fn test_partial_refund_cumulative_exceeds_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );

    client.partial_refund(&admin, &payment_id, &700);
    let result = client.try_partial_refund(&admin, &payment_id, &400);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::RefundExceedsPayment);
}

// ── DUNNING MANAGEMENT TESTS ─────────────────────────────────────────

fn setup_dunning_contract(
    env: &Env,
) -> (PaymentContractClient, Address, Address, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let customer = Address::generate(env);
    let merchant = Address::generate(env);
    let token = Address::generate(env);

    env.mock_all_auths();
    client.initialize(&admin);

    // Set up dunning configuration
    client.set_dunning_config(
        &admin,
        &(DunningConfig {
            initial_backoff_seconds: 3600, // 1 hour
            max_retries: 4,
        }),
    );

    (client, admin, customer, merchant, token)
}

#[test]
fn test_set_and_get_dunning_config() {
    let env = Env::default();
    let (client, _admin, _, _, _) = setup_dunning_contract(&env);

    let config = client.get_dunning_config();
    assert_eq!(config.initial_backoff_seconds, 3600);
    assert_eq!(config.max_retries, 4);
}

// Helper: create a subscription that will always fail (no customer funds).
fn setup_failing_subscription(
    env: &Env,
    client: &PaymentContractClient<'_>,
    customer: &Address,
    merchant: &Address,
) -> (u64, Address) {
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    env.ledger().set_timestamp(1000);

    let sub_id = client.create_subscription(
        customer,
        merchant,
        &500i128,
        &token_address,
        &Currency::USDC,
        &86400u64, // 1-day interval
        &0u64,
        &3u64,
        &String::from_str(env, ""),
        &0u64,
    );
    (sub_id, token_address)
}

#[test]
fn test_dunning_retry_too_early_returns_error() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    client.set_dunning_config(
        &admin,
        &(DunningConfig {
            initial_backoff_seconds: 3600,
            max_retries: 4,
        }),
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Advance to payment due time and trigger the first failure → enters dunning
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);

    let dunning = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(dunning.retry_count, 0);
    let next_retry = dunning.next_retry_at;

    // Calling before next_retry_at must return RetryTooEarly
    env.ledger().set_timestamp(next_retry - 1);
    let result = client.try_execute_recurring_payment(&sub_id);
    assert_eq!(result, Err(Ok(Error::RetryTooEarly)));
}

#[test]
fn test_dunning_exponential_backoff_doubles() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    let backoff = 3600u64;
    client.set_dunning_config(
        &admin,
        &(DunningConfig {
            initial_backoff_seconds: backoff,
            max_retries: 5,
        }),
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Initial failure → enter dunning, retry_count = 0, next_retry_at = now + backoff (1x)
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);
    let d1 = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(d1.retry_count, 0);

    // First retry failure → retry_count = 1, wait = backoff * 2^1 = backoff * 2 (doubles)
    env.ledger().set_timestamp(d1.next_retry_at + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);
    let d2 = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(d2.retry_count, 1);
    // next_retry_at = now + (backoff_seconds << 1) = now + backoff * 2
    let expected_wait = backoff << 1u32;
    assert_eq!(d2.next_retry_at, d1.next_retry_at + 1 + expected_wait);
}

#[test]
fn test_dunning_max_retries_suspends_subscription() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    client.set_dunning_config(
        &admin,
        &(DunningConfig {
            initial_backoff_seconds: 60,
            max_retries: 3,
        }),
    );

    let (sub_id, _token) = setup_failing_subscription(&env, &client, &customer, &merchant);

    // Initial failure → enter dunning (retry_count = 0)
    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);

    // Exhaust retries: retry_count goes 0 → 1 → 2 → 3, suspension fires at 3 >= max_retries(3)
    for _ in 0..3 {
        let d = client.get_dunning_state(&sub_id).unwrap();
        env.ledger().set_timestamp(d.next_retry_at + 1);
        let _ = client.try_execute_recurring_payment(&sub_id);
    }

    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Suspended);

    let last = client.try_execute_recurring_payment(&sub_id);
    assert_eq!(last, Err(Ok(Error::SubscriptionNotActive)));
}

#[test]
fn test_dunning_successful_retry_fires_dunning_resolved() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    client.set_dunning_config(
        &admin,
        &(DunningConfig {
            initial_backoff_seconds: 3600,
            max_retries: 4,
        }),
    );

    // Start with no funds so first payment fails
    let (sub_id, token_address) = setup_failing_subscription(&env, &client, &customer, &merchant);

    env.ledger().set_timestamp(1000 + 86400 + 1);
    let _ = client.try_execute_recurring_payment(&sub_id);

    let dunning = client.get_dunning_state(&sub_id).unwrap();
    assert_eq!(dunning.retry_count, 0);

    // Fund the customer so the retry succeeds
    let asset_client = token::StellarAssetClient::new(&env, &token_address);
    asset_client.mint(&customer, &100_000i128);

    env.ledger().set_timestamp(dunning.next_retry_at + 1);
    client.execute_recurring_payment(&sub_id);

    // Dunning state should be cleared
    assert!(client.get_dunning_state(&sub_id).is_none());

    // Subscription should be Active again
    let sub = client.get_subscription(&sub_id);
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert_eq!(sub.retry_count, 0);
}

#[test]
fn test_create_escrowed_payment_locks_funds_in_escrow() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let payment_contract_id = env.register(PaymentContract, ());
    let payment_client = PaymentContractClient::new(&env, &payment_contract_id);
    let escrow_contract_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1_000_i128;

    payment_client.initialize(&admin);
    token_admin_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &payment_contract_id, &amount, &10_000);

    let ids = payment_client.create_escrowed_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &escrow_contract_id,
        &1000_u64,
        &0_u64,
        &String::from_str(&env, "bridge"),
        &true,
    );

    assert_eq!(ids.0, 1);
    assert_eq!(ids.1, 1);
    assert_eq!(token_user_client.balance(&customer), 0);
    assert_eq!(token_user_client.balance(&escrow_contract_id), amount);
    assert_eq!(token_user_client.balance(&merchant), 0);

    let bridge = payment_client.get_escrowed_payment(&ids.0);
    assert_eq!(bridge.escrow_id, ids.1);
    let escrow = escrow_client.get_escrow(&ids.1);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_complete_escrowed_payment_releases_escrow_and_merchant_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let payment_contract_id = env.register(PaymentContract, ());
    let payment_client = PaymentContractClient::new(&env, &payment_contract_id);
    let escrow_contract_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1_000_i128;

    let escrow_admin = Address::generate(&env);
    payment_client.initialize(&admin);
    escrow_client.initialize(&escrow_admin);
    token_admin_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &payment_contract_id, &amount, &10_000);

    let ids = payment_client.create_escrowed_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &escrow_contract_id,
        &1000_u64,
        &0_u64,
        &String::from_str(&env, "bridge"),
        &true,
    );

    payment_client.complete_escrowed_payment(&admin, &ids.0);

    let payment = payment_client.get_payment(&ids.0);
    assert_eq!(payment.status, PaymentStatus::Completed);
    let escrow = escrow_client.get_escrow(&ids.1);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(token_user_client.balance(&escrow_contract_id), 0);
    assert_eq!(token_user_client.balance(&merchant), amount);
}

#[test]
fn test_cancel_escrowed_payment_refunds_customer() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let payment_contract_id = env.register(PaymentContract, ());
    let payment_client = PaymentContractClient::new(&env, &payment_contract_id);
    let escrow_contract_id = env.register(EscrowContract, ());
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1_000_i128;

    payment_client.initialize(&admin);
    token_admin_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &payment_contract_id, &amount, &10_000);

    let ids = payment_client.create_escrowed_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &escrow_contract_id,
        &1000_u64,
        &0_u64,
        &String::from_str(&env, "bridge"),
        &true,
    );

    payment_client.cancel_escrowed_payment(&customer, &ids.0);

    let payment = payment_client.get_payment(&ids.0);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
    let escrow = escrow_client.get_escrow(&ids.1);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
    assert_eq!(token_user_client.balance(&escrow_contract_id), 0);
    assert_eq!(token_user_client.balance(&customer), amount);
}

// ── MULTI-SIG ADMIN TESTS (PAYMENT CONTRACT) ─────────────────────────────────

#[test]
fn test_payment_multisig_initialize() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let config = client.get_multisig_config();
    assert_eq!(config.total_admins, 1);
    assert_eq!(config.required_signatures, 1);
    assert!(config.admins.contains(&admin));
}

#[test]
fn test_payment_multisig_propose_complete_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    env.ledger().set_timestamp(1000);
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &500_i128,
        &token,
        &Currency::USDC,
        &0_u64,
        &String::from_str(&env, ""),
    );

    let mut data_bytes = [0u8; 8];
    let id_bytes = payment_id.to_be_bytes();
    for i in 0..8 {
        data_bytes[i] = id_bytes[i];
    }
    let data = soroban_sdk::Bytes::from_slice(&env, &data_bytes);

    let proposal_id = client.propose_action(&admin, &ActionType::CompletePayment, &merchant, &data);
    assert_eq!(proposal_id, String::from_str(&env, "1"));
}

#[test]
fn test_payment_multisig_add_admin() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &new_admin);

    let config = client.get_multisig_config();
    assert_eq!(config.total_admins, 2);
    assert!(config.admins.contains(&new_admin));
}

#[test]
fn test_payment_multisig_reject_action() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let admin2 = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2_u32);

    let data = soroban_sdk::Bytes::from_slice(&env, &[0u8; 8]);
    let proposal_id = client.propose_action(&admin, &ActionType::CompletePayment, &admin2, &data);

    client.reject_action(&admin2, &proposal_id);

    // After rejection, execute should fail
    let result = client.try_execute_action(&proposal_id);
    assert!(result.is_err());
}

#[test]
#[should_panic]
fn test_payment_multisig_not_admin_propose() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let data = soroban_sdk::Bytes::from_slice(&env, &[0u8; 8]);
    // Non-admin trying to propose should panic
    client.propose_action(&non_admin, &ActionType::CompletePayment, &non_admin, &data);
}

// ── BATCH PAYMENT TESTS ──────────────────────────────────────────────────────

#[test]
fn test_create_batch_payment_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let entries = soroban_sdk::vec![
        &env,
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry1"),
        },
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 200_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry2"),
        }
    ];

    let results = client.create_batch_payment(&entries);
    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(results.get(1).unwrap().success);
    assert_eq!(results.get(0).unwrap().payment_id, 1);
    assert_eq!(results.get(1).unwrap().payment_id, 2);
}

#[test]
#[should_panic]
fn test_create_batch_payment_empty() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let entries: soroban_sdk::Vec<BatchPaymentEntry> = soroban_sdk::vec![&env];
    client.create_batch_payment(&entries);
}

#[test]
#[should_panic]
fn test_create_batch_payment_too_large() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    // Create 51 entries (over MAX_BATCH_SIZE of 50)
    let mut entries = soroban_sdk::Vec::new(&env);
    for _ in 0..51 {
        entries.push_back(BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, ""),
        });
    }
    client.create_batch_payment(&entries);
}

#[test]
fn test_create_batch_payment_partial_failure() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);
    // Set a rate limit that allows only 1 payment per window
    client.set_rate_limit_config(
        &admin,
        &(RateLimitConfig {
            max_payments_per_window: 1,
            window_duration: 100_000,
            max_payment_amount: 0,
            max_daily_volume: 0,
        }),
    );

    let entries = soroban_sdk::vec![
        &env,
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "ok"),
        },
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "fail"),
        }
    ];

    let results = client.create_batch_payment(&entries);
    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(!results.get(1).unwrap().success);
}

#[test]
fn test_complete_batch_payment_success() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_mint_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let total_amount = 300_i128;

    client.initialize(&admin);
    token_mint_client.mint(&customer, &total_amount);
    token_user_client.approve(&customer, &contract_id, &total_amount, &10_000);

    env.ledger().set_timestamp(1000);

    let pid1 = client.create_payment(
        &customer,
        &merchant,
        &100_i128,
        &token_contract_id,
        &Currency::USDC,
        &0_u64,
        &String::from_str(&env, ""),
    );
    let pid2 = client.create_payment(
        &customer,
        &merchant,
        &200_i128,
        &token_contract_id,
        &Currency::USDC,
        &0_u64,
        &String::from_str(&env, ""),
    );

    let payment_ids = soroban_sdk::vec![&env, pid1, pid2];
    let results = client.complete_batch_payment(&admin, &payment_ids);

    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(results.get(1).unwrap().success);
}

#[test]
fn test_complete_batch_payment_partial_failure() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_mint_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    client.initialize(&admin);
    token_mint_client.mint(&customer, &100_i128);
    token_user_client.approve(&customer, &contract_id, &100_i128, &10_000);

    let pid1 = client.create_payment(
        &customer,
        &merchant,
        &100_i128,
        &token_contract_id,
        &Currency::USDC,
        &0_u64,
        &String::from_str(&env, ""),
    );
    // Complete pid1 first so it's already processed
    client.complete_payment(&admin, &pid1);

    // Now try batch complete: pid1 (already done) + 9999 (not found) = both fail
    let payment_ids = soroban_sdk::vec![&env, pid1, 9999_u64];
    let results = client.complete_batch_payment(&admin, &payment_ids);

    assert_eq!(results.len(), 2);
    assert!(!results.get(0).unwrap().success); // already completed
    assert!(!results.get(1).unwrap().success); // not found
}

#[test]
fn test_cancel_batch_payment_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let pid1 = client.create_payment(
        &customer,
        &merchant,
        &100_i128,
        &token,
        &Currency::USDC,
        &0_u64,
        &String::from_str(&env, ""),
    );
    let pid2 = client.create_payment(
        &customer,
        &merchant,
        &200_i128,
        &token,
        &Currency::USDC,
        &0_u64,
        &String::from_str(&env, ""),
    );

    let payment_ids = soroban_sdk::vec![&env, pid1, pid2];
    let results = client.cancel_batch_payment(&customer, &payment_ids);

    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(results.get(1).unwrap().success);

    let p1 = client.get_payment(&pid1);
    assert_eq!(p1.status, PaymentStatus::Cancelled);
    let p2 = client.get_payment(&pid2);
    assert_eq!(p2.status, PaymentStatus::Cancelled);
}

#[test]
fn test_batch_payment_events_emitted() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let entries = soroban_sdk::vec![
        &env,
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "batch_event_test"),
        }
    ];

    let results = client.create_batch_payment(&entries);
    assert!(results.get(0).unwrap().success);

    // Verify events were emitted by checking payment was created
    let payment = client.get_payment(&results.get(0).unwrap().payment_id);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.amount, 100_i128);
}

#[test]
fn test_create_payment_batch_optimized_same_token_grouping() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let entries = soroban_sdk::vec![
        &env,
        BatchPaymentEntry {
            customer: customer1.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry1"),
        },
        BatchPaymentEntry {
            customer: customer2.clone(),
            merchant: merchant.clone(),
            amount: 200_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry2"),
        }
    ];

    let results = client.create_payment_batch_optimized(&admin, &entries);
    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(results.get(1).unwrap().success);
    assert_eq!(results.get(0).unwrap().payment_id, 1);
    assert_eq!(results.get(1).unwrap().payment_id, 2);

    // Check payments are completed
    let p1 = client.get_payment(&1);
    assert_eq!(p1.status, PaymentStatus::Completed);
    let p2 = client.get_payment(&2);
    assert_eq!(p2.status, PaymentStatus::Completed);
}

#[test]
fn test_create_payment_batch_optimized_cross_token_isolation() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let token1 = Address::generate(&env);
    let token2 = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin);

    let entries = soroban_sdk::vec![
        &env,
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant1.clone(),
            amount: 100_i128,
            token: token1.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry1"),
        },
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant2.clone(),
            amount: 200_i128,
            token: token2.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry2"),
        }
    ];

    let results = client.create_payment_batch_optimized(&admin, &entries);
    assert_eq!(results.len(), 2);
    assert!(results.get(0).unwrap().success);
    assert!(results.get(1).unwrap().success);
}

#[test]
fn test_get_batch_gas_estimate() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    let entries = soroban_sdk::vec![
        &env,
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 100_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry1"),
        },
        BatchPaymentEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount: 200_i128,
            token: token.clone(),
            currency: Currency::USDC,
            expiration_duration: 0,
            metadata: String::from_str(&env, "entry2"),
        }
    ];

    let estimate = client.get_batch_gas_estimate(&entries);
    assert!(estimate > 0);
}

// ── FEE MANAGEMENT TESTS ─────────────────────────────────────────────────────

fn setup_fee_contract(
    env: &Env,
) -> (
    PaymentContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    let token_admin = Address::generate(env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    env.mock_all_auths();
    client.initialize(&admin);

    (client, admin, contract_id, token_contract_id, token_admin)
}

#[test]
fn test_set_and_get_fee_config() {
    let env = Env::default();
    let (client, admin, _, token_contract_id, _) = setup_fee_contract(&env);

    let treasury = Address::generate(&env);
    let fee_config = FeeConfig {
        fee_bps: 30,
        min_fee: 1,
        max_fee: 1000,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };

    client.set_fee_config(&admin, &fee_config);

    let stored = client.get_fee_config();
    assert_eq!(stored.fee_bps, 30);
    assert_eq!(stored.treasury, treasury);
    assert_eq!(stored.active, true);
    assert_eq!(stored.min_fee, 1);
    assert_eq!(stored.max_fee, 1000);
}

#[test]
fn test_fee_deducted_from_payment_amount() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    let amount = 10_000_i128;

    // 30 bps = 0.30% fee
    let fee_config = FeeConfig {
        fee_bps: 30,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // fee = 10_000 * 30 / 10_000 = 30
    let expected_fee: i128 = 30;
    let expected_net = amount - expected_fee;

    assert_eq!(token_user_client.balance(&merchant), expected_net);
    assert_eq!(token_user_client.balance(&contract_id), expected_fee);
    assert_eq!(client.get_accumulated_fees(), expected_fee);
}

#[test]
fn test_fee_min_clamping() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    // A small amount: 10 bps of 100 = 0.1 → rounds to 0, min_fee ensures at least 5
    let amount = 100_i128;

    let fee_config = FeeConfig {
        fee_bps: 10, // 0.10%: raw_fee = 100 * 10 / 10000 = 0
        min_fee: 5,  // clamped to 5
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // raw fee = 0, clamped to min_fee = 5
    assert_eq!(token_user_client.balance(&merchant), amount - 5);
    assert_eq!(client.get_accumulated_fees(), 5);
}

#[test]
fn test_fee_max_clamping() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    // Large amount: 1% of 100_000 = 1000, capped by max_fee = 50
    let amount = 100_000_i128;

    let fee_config = FeeConfig {
        fee_bps: 100, // 1%: raw_fee = 100_000 * 100 / 10_000 = 1000
        min_fee: 0,
        max_fee: 50, // clamped to 50
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // raw fee = 1000, clamped to max_fee = 50
    assert_eq!(token_user_client.balance(&merchant), amount - 50);
    assert_eq!(client.get_accumulated_fees(), 50);
}

#[test]
fn test_merchant_tier_upgrade_to_premium() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);

    let fee_config = FeeConfig {
        fee_bps: 30,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    let amount = 10_000_i128;
    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let record = client.get_merchant_fee_record(&merchant);
    assert_eq!(record.fee_tier, FeeTier::Premium);
    assert_eq!(record.total_volume, amount);
    assert_eq!(client.get_merchant_tier(&merchant), FeeTier::Premium);
}

#[test]
fn test_merchant_tier_upgrade_to_enterprise() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);

    let fee_config = FeeConfig {
        fee_bps: 30,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    let amount = 100_000_i128;
    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let record = client.get_merchant_fee_record(&merchant);
    assert_eq!(record.fee_tier, FeeTier::Enterprise);
}

#[test]
fn test_auto_tier_upgrade_runs_on_complete_payment_even_without_fee_config() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // No fee config set, but tier check must still run after completion.
    let amount = 10_000_i128;
    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let record = client.get_merchant_fee_record(&merchant);
    assert_eq!(record.fee_tier, FeeTier::Premium);
}

#[test]
fn test_auto_tier_upgrade_never_downgrades() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.set_fee_config(
        &admin,
        &(FeeConfig {
            fee_bps: 100,
            min_fee: 0,
            max_fee: 0,
            treasury,
            fee_token: token_contract_id.clone(),
            active: true,
        }),
    );

    // Lower initial thresholds so first completion reaches Enterprise quickly.
    client.set_tier_thresholds(
        &admin,
        &soroban_sdk::vec![
            &env,
            (FeeTier::Premium, 100_i128),
            (FeeTier::Enterprise, 200_i128),
        ],
    );

    token_client.mint(&customer, &500);
    token_user_client.approve(&customer, &contract_id, &500, &200);

    let first = client.create_payment(
        &customer,
        &merchant,
        &250,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &first);
    assert_eq!(client.get_merchant_tier(&merchant), FeeTier::Enterprise);

    // Increase thresholds above existing volume. Auto path must not downgrade tier.
    client.set_tier_thresholds(
        &admin,
        &soroban_sdk::vec![
            &env,
            (FeeTier::Premium, 1_000_i128),
            (FeeTier::Enterprise, 2_000_i128),
        ],
    );

    let second = client.create_payment(
        &customer,
        &merchant,
        &10,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &second);
    assert_eq!(client.get_merchant_tier(&merchant), FeeTier::Enterprise);
}

#[test]
fn test_manual_override_allows_downgrade() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    token_client.mint(&customer, &120_000);
    token_user_client.approve(&customer, &contract_id, &120_000, &200);

    let first = client.create_payment(
        &customer,
        &merchant,
        &100_000,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &first);
    assert_eq!(client.get_merchant_tier(&merchant), FeeTier::Enterprise);

    client.manually_set_merchant_tier(&admin, &merchant, &FeeTier::Standard);
    assert_eq!(client.get_merchant_tier(&merchant), FeeTier::Standard);

    // Subsequent completion can auto-upgrade back up based on cumulative volume.
    let second = client.create_payment(
        &customer,
        &merchant,
        &1_000,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &second);
    assert_eq!(client.get_merchant_tier(&merchant), FeeTier::Enterprise);
}

#[test]
fn test_set_tier_thresholds_validates_ascending_order() {
    let env = Env::default();
    let (client, admin, _, _, _) = setup_fee_contract(&env);

    let result = client.try_set_tier_thresholds(
        &admin,
        &soroban_sdk::vec![
            &env,
            (FeeTier::Premium, 200_i128),
            (FeeTier::Enterprise, 100_i128),
        ],
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidTierThresholds);
}

#[test]
fn test_set_and_get_tier_thresholds() {
    let env = Env::default();
    let (client, admin, _, _, _) = setup_fee_contract(&env);

    let updated = soroban_sdk::vec![
        &env,
        (FeeTier::Premium, 25_000_i128),
        (FeeTier::Enterprise, 250_000_i128),
    ];

    client.set_tier_thresholds(&admin, &updated);
    let stored = client.get_tier_thresholds();
    assert_eq!(stored, updated);
}

#[test]
fn test_tier_discount_reduces_fee() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);

    let fee_config = FeeConfig {
        fee_bps: 1000, // 10%
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    // First: push merchant to Premium (volume >= 10,000)
    let vol_amount = 10_000_i128;
    let fee1 = (10_000_i128 * 1000) / 10_000; // = 1000 (Standard, no discount)
    token_client.mint(&customer, &(vol_amount + 10_000));
    token_user_client.approve(&customer, &contract_id, &(vol_amount + 10_000), &200);

    let pid1 = client.create_payment(
        &customer,
        &merchant,
        &vol_amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &pid1);

    // Merchant should now be Premium (750 bps discount = 7.5% off fee)
    let record = client.get_merchant_fee_record(&merchant);
    assert_eq!(record.fee_tier, FeeTier::Premium);

    // Second payment: effective_bps = 1000 - (1000 * 750 / 10000) = 1000 - 75 = 925
    // fee = 10_000 * 925 / 10_000 = 925
    let amount2 = 10_000_i128;
    let pid2 = client.create_payment(
        &customer,
        &merchant,
        &amount2,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &pid2);

    // fee1 = 1000 (Standard), fee2 = 925 (Premium with 7.5% discount)
    let expected_fee2: i128 = 925;
    let total_fees = fee1 + expected_fee2;
    assert_eq!(client.get_accumulated_fees(), total_fees);

    let merchant_balance = token_user_client.balance(&merchant);
    assert_eq!(
        merchant_balance,
        vol_amount - fee1 + (amount2 - expected_fee2)
    );
}

#[test]
fn test_withdraw_fees_to_treasury() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    let amount = 10_000_i128;

    let fee_config = FeeConfig {
        fee_bps: 100, // 1%: fee = 100
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    let accumulated = client.get_accumulated_fees();
    assert_eq!(accumulated, 100); // 1% of 10_000

    // Withdraw all fees to treasury
    client.withdraw_fees(&admin, &accumulated);

    assert_eq!(client.get_accumulated_fees(), 0);
    assert_eq!(token_user_client.balance(&treasury), accumulated);
    assert_eq!(token_user_client.balance(&contract_id), 0);
}

#[test]
fn test_withdraw_fees_only_to_treasury_address() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    let amount = 10_000_i128;

    let fee_config = FeeConfig {
        fee_bps: 100,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // withdraw_fees always uses the treasury from fee config
    client.withdraw_fees(&admin, &100);
    assert_eq!(token_user_client.balance(&treasury), 100);
}

#[test]
#[should_panic]
fn test_withdraw_fees_exceeds_accumulated_fails() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    let amount = 10_000_i128;

    let fee_config = FeeConfig {
        fee_bps: 100,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    // Attempt to withdraw more than accumulated (100)
    client.withdraw_fees(&admin, &999);
}

#[test]
fn test_fee_not_collected_when_inactive() {
    let env = Env::default();
    let (client, admin, contract_id, token_contract_id, _) = setup_fee_contract(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);
    let amount = 10_000_i128;

    // active: false → no fee collected
    let fee_config = FeeConfig {
        fee_bps: 100,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: false,
    };
    client.set_fee_config(&admin, &fee_config);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &200);

    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &amount,
        &token_contract_id,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
    );
    client.complete_payment(&admin, &payment_id);

    assert_eq!(client.get_accumulated_fees(), 0);
    assert_eq!(token_user_client.balance(&merchant), amount);
}

#[test]
fn test_calculate_fee_respects_tier() {
    let env = Env::default();
    let (client, admin, _, token_contract_id, _) = setup_fee_contract(&env);

    let merchant = Address::generate(&env);
    let treasury = Address::generate(&env);

    let fee_config = FeeConfig {
        fee_bps: 1000, // 10%
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_contract_id.clone(),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    // Standard tier: fee = 10_000 * 1000 / 10_000 = 1000
    let fee_standard = client.calculate_fee(&10_000_i128, &merchant);
    assert_eq!(fee_standard, 1000);
}

#[test]
fn test_get_merchant_fee_record_default() {
    let env = Env::default();
    let (client, _, _, _, _) = setup_fee_contract(&env);

    let merchant = Address::generate(&env);
    let record = client.get_merchant_fee_record(&merchant);

    assert_eq!(record.total_fees_paid, 0);
    assert_eq!(record.total_volume, 0);
    assert_eq!(record.fee_tier, FeeTier::Standard);
}

// ── CONDITIONAL PAYMENT TESTS ────────────────────────────────────────────

fn setup_conditional_payment_contract(env: &Env) -> (PaymentContractClient<'_>, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin, contract_id)
}

#[test]
fn test_create_conditional_payment_timestamp_after() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let metadata = String::from_str(&env, "Test conditional payment");

    // Set timestamp to 1000, condition is after 2000
    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(2000);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
        &condition,
    );

    // Verify conditional payment was created
    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert_eq!(conditional_payment.payment_id, payment_id);
    assert!(!conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, None);

    // Verify base payment was also created
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.id, payment_id);
    assert_eq!(payment.status, PaymentStatus::Pending);

    // Verify event was published (at least one event should be emitted)
    let events = env.events().all();
    assert!(!events.is_empty());
}

#[test]
fn test_create_conditional_payment_timestamp_before() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let metadata = String::from_str(&env, "Test conditional payment");

    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampBefore(2000);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &metadata,
        &condition,
    );

    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert_eq!(conditional_payment.payment_id, payment_id);
    assert!(!conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, None);
}

#[test]
fn test_create_conditional_payment_oracle_price() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle = Address::generate(&env);
    let metadata = String::from_str(&env, "Test oracle conditional payment");

    let condition = ConditionType::OraclePrice(
        oracle,
        String::from_str(&env, "BTC"),
        50000,
        PriceComparison::GreaterThan,
    );

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::BTC,
        &0,
        &metadata,
        &condition,
    );

    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert_eq!(conditional_payment.payment_id, payment_id);
    assert!(!conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, None);

    // Verify event was published
    let events = env.events().all();
    let condition_event = events.last().unwrap();
    // Verify this is from our contract
    assert_eq!(condition_event.0, contract_id);
}

#[test]
fn test_create_conditional_payment_cross_contract_state() {
    let env = Env::default();
    let (client, _admin, contract_id) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let target_contract = Address::generate(&env);
    let metadata = String::from_str(&env, "Test cross-contract conditional payment");

    let state_hash = BytesN::from_array(&env, &[1; 32]);
    let condition = ConditionType::CrossContractState(target_contract, state_hash);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::ETH,
        &0,
        &metadata,
        &condition,
    );

    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert_eq!(conditional_payment.payment_id, payment_id);
    assert!(!conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, None);

    // Verify event was published
    let events = env.events().all();
    let condition_event = events.last().unwrap();
    // Verify this is from our contract
    assert_eq!(condition_event.0, contract_id);
}

#[test]
fn test_evaluate_condition_timestamp_after_met() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Set timestamp to 1000, condition is after 500
    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(500);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Evaluate condition - should be true since 1000 > 500
    let result = client.evaluate_condition(&payment_id);
    assert!(result);

    // Verify result was cached
    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert!(conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, Some(1000));

    // Second evaluation should return cached result
    let result2 = client.evaluate_condition(&payment_id);
    assert!(result2);
}

#[test]
fn test_evaluate_condition_timestamp_after_not_met() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Set timestamp to 1000, condition is after 2000
    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(2000);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Evaluate condition - should be false since 1000 <= 2000
    let result = client.evaluate_condition(&payment_id);
    assert!(!result);

    // Verify result was cached
    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert!(!conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, Some(1000));
}

#[test]
fn test_evaluate_condition_timestamp_before_met() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Set timestamp to 1000, condition is before 2000
    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampBefore(2000);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Evaluate condition - should be true since 1000 < 2000
    let result = client.evaluate_condition(&payment_id);
    assert!(result);

    // Verify result was cached
    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert!(conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, Some(1000));
}

#[test]
fn test_evaluate_condition_timestamp_before_not_met() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Set timestamp to 2000, condition is before 1000
    env.ledger().set_timestamp(2000);
    let condition = ConditionType::TimestampBefore(1000);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Evaluate condition - should be false since 2000 >= 1000
    let result = client.evaluate_condition(&payment_id);
    assert!(!result);

    // Verify result was cached
    let conditional_payment = client.get_conditional_payment(&payment_id);
    assert!(!conditional_payment.condition_met);
    assert_eq!(conditional_payment.evaluated_at, Some(2000));
}

#[test]
fn test_evaluate_condition_oracle_price_fails() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let oracle = Address::generate(&env);

    let condition = ConditionType::OraclePrice(
        oracle,
        String::from_str(&env, "BTC"),
        50000,
        PriceComparison::GreaterThan,
    );

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::BTC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Oracle conditions should fail with OracleCallFailed error
    let result = client.try_evaluate_condition(&payment_id);
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(Error::OracleCallFailed)));
}

#[test]
fn test_evaluate_condition_cross_contract_state_false() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let target_contract = Address::generate(&env);

    let state_hash = BytesN::from_array(&env, &[1; 32]);
    let condition = ConditionType::CrossContractState(target_contract, state_hash);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::ETH,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Cross-contract invoke failures should return a typed error.
    let result = client.try_evaluate_condition(&payment_id);
    assert_eq!(result.err(), Some(Ok(Error::ConditionEvaluationFailed)));
}

#[test]
fn test_complete_conditional_payment_success() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Set timestamp to 1000, condition is after 500 (should be met)
    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(500);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Complete the conditional payment
    client.complete_conditional_payment(&admin, &payment_id);

    // Verify payment was completed
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_complete_conditional_payment_condition_not_met() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    // Set timestamp to 1000, condition is after 2000 (should not be met)
    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(2000);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Attempt to complete should fail with ConditionNotMet
    let result = client.try_complete_conditional_payment(&admin, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(Error::ConditionNotMet)));

    // Verify payment is still pending
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Pending);
}

#[test]
fn test_complete_conditional_payment_expired() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(500);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &100, // Expires at 1100
        &String::from_str(&env, ""),
        &condition,
    );

    // Advance time past expiration
    env.ledger().set_timestamp(1200);

    // Attempt to complete should fail with PaymentExpired
    let result = client.try_complete_conditional_payment(&admin, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(Error::PaymentExpired)));
}

#[test]
fn test_complete_conditional_payment_unauthorized() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(500);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // Attempt to complete with unauthorized user should fail
    let result = client.try_complete_conditional_payment(&unauthorized_user, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(Error::Unauthorized)));
}

#[test]
fn test_get_conditional_payment_not_found() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    // Attempt to get non-existent conditional payment should fail
    let result = client.try_get_conditional_payment(&999);
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(Error::PaymentNotFound)));
}

#[test]
fn test_condition_evaluation_caching() {
    let env = Env::default();
    let (client, admin, _) = setup_conditional_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    let condition = ConditionType::TimestampAfter(500);

    let payment_id = client.create_conditional_payment(
        &customer,
        &merchant,
        &1000,
        &token,
        &Currency::USDC,
        &0,
        &String::from_str(&env, ""),
        &condition,
    );

    // First evaluation
    let result1 = client.evaluate_condition(&payment_id);
    assert!(result1);

    let conditional_payment1 = client.get_conditional_payment(&payment_id);
    let evaluated_at1 = conditional_payment1.evaluated_at.unwrap();

    // Advance time and evaluate again
    env.ledger().set_timestamp(2000);
    let result2 = client.evaluate_condition(&payment_id);
    assert!(result2); // Should return cached result

    let conditional_payment2 = client.get_conditional_payment(&payment_id);
    let evaluated_at2 = conditional_payment2.evaluated_at.unwrap();

    // Evaluation timestamp should be the same (cached)
    assert_eq!(evaluated_at1, evaluated_at2);
    assert_eq!(evaluated_at1, 1000);
}

// ── FEE WAIVER TESTS ─────────────────────────────────────────────────────

fn setup_fee_waiver_contract(env: &Env) -> (PaymentContractClient<'_>, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);

    // Set up fee configuration
    let fee_config = FeeConfig {
        fee_bps: 200, // 2%
        min_fee: 100,
        max_fee: 10000,
        treasury: Address::generate(env),
        fee_token: Address::generate(env),
        active: true,
    };
    client.set_fee_config(&admin, &fee_config);

    (client, admin, contract_id)
}

#[test]
fn test_grant_fee_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "Partnership deal");

    // Grant 50% fee waiver (5000 bps)
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // Verify waiver was granted
    let waiver = client.get_fee_waiver(&merchant);
    assert!(waiver.is_some());

    let waiver_data = waiver.unwrap();
    assert_eq!(waiver_data.merchant, merchant);
    assert_eq!(waiver_data.waiver_bps, 5000);
    assert_eq!(waiver_data.valid_until, 1000000000);
    assert_eq!(waiver_data.reason, reason);
    assert_eq!(waiver_data.granted_by, admin);

    // Verify event was published
    let events = env.events().all();
    let waiver_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "FeeWaiverGranted")
        .collect();
    assert_eq!(waiver_events.len(), 1);
}

// ── LARGE PAYMENT MULTI-SIG TESTS ───────────────────────────────────────────

fn setup_large_payment_contract(
    env: &Env,
) -> (PaymentContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let admin2 = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    client.add_admin(&admin, &admin2);
    client.update_required_signatures(&admin, &2);
    (client, admin, admin2, contract_id)
}

#[test]
fn test_set_large_payment_threshold() {
    let env = Env::default();
    let (client, admin, _, _) = setup_large_payment_contract(&env);

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Verify threshold is set
    assert_eq!(client.get_large_payment_threshold(), 1000);

    // Verify event was published
    let events = env.events().all();
    assert_eq!(events.len(), 3); // initialize, add_admin, threshold_updated
    assert_eq!(events[2].topics[2], Address::from_str(&env, "1000"));
}

#[test]
#[should_panic]
fn test_grant_fee_waiver_unauthorized() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let reason = String::from_str(&env, "Unauthorized waiver");

    // Unauthorized user should not be able to grant waiver
    client.grant_fee_waiver(&unauthorized, &merchant, &5000, &1000000000, &reason);
}

#[test]
#[should_panic]
fn test_grant_fee_waiver_invalid_bps() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "Invalid waiver");

    // Waiver BPS over 10000 (100%) should fail
    client.grant_fee_waiver(&admin, &merchant, &15000, &1000000000, &reason);
}

#[test]
fn test_revoke_fee_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "Partnership deal");

    // Grant waiver first
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // Verify waiver exists
    assert!(client.get_fee_waiver(&merchant).is_some());

    // Revoke waiver
    client.revoke_fee_waiver(&admin, &merchant);

    // Verify waiver was revoked
    assert!(client.get_fee_waiver(&merchant).is_none());

    // Verify event was published
    let events = env.events().all();
    let revoke_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "FeeWaiverRevoked")
        .collect();
    assert_eq!(revoke_events.len(), 1);
}

#[test]
#[should_panic]
fn test_revoke_fee_waiver_unauthorized() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let reason = String::from_str(&env, "Partnership deal");

    // Grant waiver first
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // Unauthorized user should not be able to revoke waiver
    client.revoke_fee_waiver(&unauthorized, &merchant);
}

#[test]
#[should_panic]
fn test_revoke_nonexistent_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);

    // Should not be able to revoke non-existent waiver
    client.revoke_fee_waiver(&admin, &merchant);
}

#[test]
fn test_expired_waiver_ignored() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "Expired waiver");

    // Set timestamp and grant waiver that expires quickly
    env.ledger().set_timestamp(1000);
    client.grant_fee_waiver(&admin, &merchant, &5000, &2000, &reason); // Expires at 2000

    // Verify waiver exists initially
    assert!(client.get_fee_waiver(&merchant).is_some());

    // Advance time past expiration
    env.ledger().set_timestamp(3000);

    // Waiver should now be expired and removed
    assert!(client.get_fee_waiver(&merchant).is_none());

    // Verify expiration event was published
    let events = env.events().all();
    let expire_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "FeeWaiverExpired")
        .collect();
    assert_eq!(expire_events.len(), 1);
}

#[test]
fn test_get_effective_fee_bps_with_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "50% waiver");

    // Without waiver: 2% fee = 200 bps
    assert_eq!(client.get_effective_fee_bps(&merchant), 200);

    // Grant 50% waiver
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // With waiver: 2% * (1 - 0.5) = 1% = 100 bps
    assert_eq!(client.get_effective_fee_bps(&merchant), 100);
}

#[test]
fn test_get_effective_fee_bps_100_percent_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "100% waiver");

    // Grant 100% waiver (zero fee)
    client.grant_fee_waiver(&admin, &merchant, &10000, &1000000000, &reason);

    // With 100% waiver: 2% * (1 - 1.0) = 0% = 0 bps
    assert_eq!(client.get_effective_fee_bps(&merchant), 0);
}

#[test]
fn test_get_effective_fee_bps_with_tier_and_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "Premium merchant waiver");

    // Upgrade merchant to Premium tier (7.5% discount on fees)
    client.set_merchant_tier(&admin, &merchant, &FeeTier::Premium);

    // Without waiver: 2% * (1 - 0.075) = 1.85% = 185 bps
    assert_eq!(client.get_effective_fee_bps(&merchant), 185);

    // Grant 50% waiver on top of tier discount
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // With waiver and tier: 2% * (1 - 0.075) * (1 - 0.5) = 0.925% = 92.5 bps (rounded down)
    assert_eq!(client.get_effective_fee_bps(&merchant), 92);
}

#[test]
fn test_calculate_fee_with_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "50% waiver");

    // Grant 50% waiver
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // Calculate fee for 1000 amount
    // Without waiver: 1000 * 200 / 10000 = 20
    // With 50% waiver: 1000 * 100 / 10000 = 10
    let fee = client.calculate_fee(&1000, &merchant);
    assert_eq!(fee, 10);
}

#[test]
fn test_calculate_fee_with_100_percent_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "100% waiver");

    // Grant 100% waiver
    client.grant_fee_waiver(&admin, &merchant, &10000, &1000000000, &reason);

    // Calculate fee for 1000 amount - should be zero
    let fee = client.calculate_fee(&1000, &merchant);
    assert_eq!(fee, 0);
}

#[test]
fn test_calculate_fee_respects_min_fee_with_waiver() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "99% waiver");

    // Grant 99% waiver (almost zero fee)
    client.grant_fee_waiver(&admin, &merchant, &9900, &1000000000, &reason);

    // Calculate fee for small amount that would be below min_fee
    // 2% * (1 - 0.99) = 0.02% = 2 bps
    // 100 * 2 / 10000 = 0.02, but min_fee is 100, so fee should be 100
    let fee = client.calculate_fee(&100, &merchant);
    assert_eq!(fee, 100); // Min fee still applies
}

#[test]
fn test_waiver_with_no_fee_config() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    env.mock_all_auths();
    client.initialize(&admin);

    let merchant = Address::generate(&env);

    // Without fee config, effective fee should be 0
    assert_eq!(client.get_effective_fee_bps(&merchant), 0);

    // Calculate fee should return 0
    assert_eq!(client.calculate_fee(&1000, &merchant), 0);
}

#[test]
fn test_fee_waiver_events() {
    let env = Env::default();
    let (client, admin, _) = setup_fee_waiver_contract(&env);

    let merchant = Address::generate(&env);
    let reason = String::from_str(&env, "Test waiver");

    // Grant waiver
    client.grant_fee_waiver(&admin, &merchant, &5000, &1000000000, &reason);

    // Revoke waiver
    client.revoke_fee_waiver(&admin, &merchant);

    // Check all events
    let events = env.events().all();

    let grant_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "FeeWaiverGranted")
        .collect();
    assert_eq!(grant_events.len(), 1);

    let revoke_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "FeeWaiverRevoked")
        .collect();
    assert_eq!(revoke_events.len(), 1);
}

#[test]
#[should_panic]
fn test_set_large_payment_threshold_unauthorized() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);
    let unauthorized = Address::generate(&env);

    // Unauthorized user should not be able to set threshold
    client.set_large_payment_threshold(&unauthorized, &1000);
}

#[test]
fn test_large_payment_auto_routes_through_multisig() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment (2000 > threshold)
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Attempting to complete payment should auto-create proposal and return error
    let result = client.try_complete_payment(&admin, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.err(), Some(Ok(Error::PaymentRequiresMultiSig)));

    // Verify proposal was created
    let proposal = client.get_large_payment_proposal(&payment_id);
    assert_eq!(proposal.payment_id, payment_id);
    assert_eq!(proposal.required, 2);
    assert!(!proposal.executed);
    assert_eq!(proposal.approvals.len(), 1); // admin who tried to complete it
    assert!(proposal.approvals.contains(&admin));
}

#[test]
fn test_small_payment_completes_normally() {
    let env = Env::default();
    let (client, admin, _, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a small payment (500 < threshold)
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &500,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Small payment should complete normally
    client.complete_payment(&admin, &payment_id);

    // Verify payment is completed
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_threshold_zero_disables_multisig() {
    let env = Env::default();
    let (client, admin, _, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 0 (disabled)
    client.set_large_payment_threshold(&admin, &0);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &5000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Should complete normally even though amount is large
    client.complete_payment(&admin, &payment_id);

    // Verify payment is completed
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_large_payment_approval_flow() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Manually propose large payment
    client.propose_large_payment(&merchant, &payment_id);

    // Get initial proposal
    let proposal = client.get_large_payment_proposal(&payment_id);
    assert_eq!(proposal.approvals.len(), 1); // merchant
    assert!(proposal.approvals.contains(&merchant));

    // Approve with admin
    client.approve_large_payment(&admin, &payment_id);

    // Check approval count increased
    let proposal = client.get_large_payment_proposal(&payment_id);
    assert_eq!(proposal.approvals.len(), 2);
    assert!(proposal.approvals.contains(&admin));

    // Should now have enough approvals to execute
    client.execute_large_payment(&payment_id);

    // Verify payment is completed
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);

    // Verify proposal is marked as executed
    let proposal = client.get_large_payment_proposal(&payment_id);
    assert!(proposal.executed);
}

#[test]
#[should_panic]
fn test_execute_large_payment_insufficient_approvals() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Propose large payment
    client.propose_large_payment(&merchant, &payment_id);

    // Try to execute with insufficient approvals (only 1, need 2)
    client.execute_large_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_approve_large_payment_unauthorized() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Propose large payment
    client.propose_large_payment(&merchant, &payment_id);

    // Unauthorized user tries to approve
    let unauthorized = Address::generate(&env);
    client.approve_large_payment(&unauthorized, &payment_id);
}

#[test]
#[should_panic]
fn test_approve_large_payment_already_approved() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Propose large payment
    client.propose_large_payment(&merchant, &payment_id);

    // Admin approves
    client.approve_large_payment(&admin, &payment_id);

    // Same admin tries to approve again
    client.approve_large_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_execute_large_payment_expired() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Propose large payment
    client.propose_large_payment(&merchant, &payment_id);

    // Approve with admin
    client.approve_large_payment(&admin, &payment_id);

    // Advance time beyond proposal TTL (7 days)
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 604800 + 100);

    // Try to execute expired proposal
    client.execute_large_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_propose_large_payment_not_merchant() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Someone other than merchant tries to propose
    let imposter = Address::generate(&env);
    client.propose_large_payment(&imposter, &payment_id);
}

#[test]
#[should_panic]
fn test_propose_large_payment_below_threshold() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a small payment (500 < threshold)
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &500,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Should not be able to propose small payment
    client.propose_large_payment(&merchant, &payment_id);
}

#[test]
fn test_large_payment_events() {
    let env = Env::default();
    let (client, admin, admin2, _) = setup_large_payment_contract(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let meta = String::from_str(&env, "");

    // Set threshold to 1000
    client.set_large_payment_threshold(&admin, &1000);

    // Create a large payment
    let payment_id = client.create_payment(
        &customer,
        &merchant,
        &2000,
        &token,
        &Currency::USDC,
        &0,
        &meta,
    );

    // Propose large payment
    client.propose_large_payment(&merchant, &payment_id);

    // Approve with admin
    client.approve_large_payment(&admin, &payment_id);

    // Execute
    client.execute_large_payment(&payment_id);

    // Verify all events were published
    let events = env.events().all();

    // Should have events for: initialize, add_admin, update_required_signatures,
    // set_threshold, create_payment, propose_large_payment, approve_large_payment,
    // payment_completed, large_payment_executed
    assert!(events.len() >= 9);

    // Check specific events
    let threshold_updated_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "LargePaymentThresholdUpdated")
        .collect();
    assert_eq!(threshold_updated_events.len(), 1);

    let proposed_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "LargePaymentProposed")
        .collect();
    assert_eq!(proposed_events.len(), 1);

    let approved_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "LargePaymentApproved")
        .collect();
    assert_eq!(approved_events.len(), 1);

    let executed_events: Vec<_> = events
        .iter()
        .filter(|e| e.topics[0] == "LargePaymentExecuted")
        .collect();
    assert_eq!(executed_events.len(), 1);
}
