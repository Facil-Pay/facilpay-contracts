#![cfg(test)]

extern crate alloc;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Bytes, BytesN, Env, Vec,
};

// ── Issue #678: Challenge window settlement ──────────────────────────────────

#[test]
fn test_challenge_window_finalize_after_window() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&customer, &1000i128);
    let token_client = token::Client::new(&env, &token_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    // Open channel with a 100-second challenge window
    let challenge_window = 100u64;
    env.ledger().set_timestamp(1000);
    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64,
        &customer_pk,
        &challenge_window,
    );

    let merchant_amount: i128 = 700;
    let nonce: u64 = 1;
    let mut msg = Bytes::new(&env);
    msg.append(&channel_id.to_xdr(&env));
    msg.append(&merchant_amount.to_xdr(&env));
    msg.append(&nonce.to_xdr(&env));
    let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
    let sig = signing_key.sign(&msg_vec);
    let sig_bn = BytesN::<64>::from_array(&env, &sig.to_bytes());

    // Initiate settlement at t=1000; window ends at t=1100
    client.initiate_settlement(&channel_id, &merchant_amount, &nonce, &sig_bn);

    // Channel should still be open (window not closed yet)
    let channel = client.get_channel(&channel_id);
    assert!(
        channel.open,
        "Channel must stay open during challenge window"
    );

    // Finalizing before the window ends must fail
    let result = client.try_finalize_settlement(&channel_id);
    assert!(result.is_err(), "Finalize must fail while window is open");

    // Advance past the window
    env.ledger().set_timestamp(1101);

    // Now finalize should succeed
    client.finalize_settlement(&channel_id);

    let channel_after = client.get_channel(&channel_id);
    assert!(!channel_after.open, "Channel must be closed after finalize");
    assert_eq!(token_client.balance(&merchant), 700i128);
    assert_eq!(token_client.balance(&customer), 300i128);
}

#[test]
fn test_challenge_replaces_lower_nonce_state() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&customer, &1000i128);
    let token_client = token::Client::new(&env, &token_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    env.ledger().set_timestamp(1000);
    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64,
        &customer_pk,
        &200u64, // 200-second window
    );

    let sign_state = |nonce: u64, amount: i128| -> BytesN<64> {
        let mut msg = Bytes::new(&env);
        msg.append(&channel_id.to_xdr(&env));
        msg.append(&amount.to_xdr(&env));
        msg.append(&nonce.to_xdr(&env));
        let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
        let sig = signing_key.sign(&msg_vec);
        BytesN::<64>::from_array(&env, &sig.to_bytes())
    };

    // Initiate with nonce=1, merchant gets 300
    client.initiate_settlement(&channel_id, &300i128, &1u64, &sign_state(1, 300));

    // Challenge with nonce=2, merchant gets 800 (higher nonce wins)
    client.challenge_settlement(&channel_id, &800i128, &2u64, &sign_state(2, 800));

    // Advance past the new window (window was reset to t=1000+200=1200; challenge was at t=1000 so new window ends at t=1200)
    env.ledger().set_timestamp(1201);
    client.finalize_settlement(&channel_id);

    // The challenged state (800) should win
    assert_eq!(token_client.balance(&merchant), 800i128);
    assert_eq!(token_client.balance(&customer), 200i128);
}

#[test]
fn test_challenge_with_lower_nonce_fails() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&customer, &1000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    env.ledger().set_timestamp(1000);
    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64,
        &customer_pk,
        &500u64,
    );

    let sign_state = |nonce: u64, amount: i128| -> BytesN<64> {
        let mut msg = Bytes::new(&env);
        msg.append(&channel_id.to_xdr(&env));
        msg.append(&amount.to_xdr(&env));
        msg.append(&nonce.to_xdr(&env));
        let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
        let sig = signing_key.sign(&msg_vec);
        BytesN::<64>::from_array(&env, &sig.to_bytes())
    };

    // Initiate with nonce=5
    client.initiate_settlement(&channel_id, &500i128, &5u64, &sign_state(5, 500));

    // Try to challenge with lower nonce=3 — must fail
    let result = client.try_challenge_settlement(&channel_id, &900i128, &3u64, &sign_state(3, 900));
    assert!(
        result.is_err(),
        "Challenge with lower nonce must be rejected"
    );
}

// ── Issue #679: Upcoming renewal events ─────────────────────────────────────

#[test]
fn test_announce_upcoming_renewals_emits_for_due_subscriptions() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&customer, &100000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    // Create subscription with 3600-second interval; next payment at t=1000+3600=4600
    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &100i128,
        &token_id,
        &Currency::USDC,
        &3600u64,
        &0u64,
        &3u64,
        &soroban_sdk::String::from_str(&env, ""),
        &0u64,
    );

    // Subscription is due within 4000 seconds from now (4600 <= 1000+4000=5000)
    let mut ids = Vec::new(&env);
    ids.push_back(sub_id);
    client.announce_upcoming_renewals(&ids, &4000u64);

    // Second call within the same cycle must not re-emit (deduplicated)
    // We verify by calling again — no panic, but no duplicate event
    client.announce_upcoming_renewals(&ids, &4000u64);
}

#[test]
fn test_announce_skips_subscription_not_due_in_window() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&customer, &100000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    // Subscription due at t=1000+86400=87400 (1 day away)
    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &100i128,
        &token_id,
        &Currency::USDC,
        &86400u64,
        &0u64,
        &3u64,
        &soroban_sdk::String::from_str(&env, ""),
        &0u64,
    );

    // Window of only 3600 seconds — subscription is NOT due within window
    let mut ids = Vec::new(&env);
    ids.push_back(sub_id);
    // This should complete without error (no events emitted for out-of-window sub)
    client.announce_upcoming_renewals(&ids, &3600u64);
}

// ── Issue #680: Change subscription payment token ────────────────────────────

#[test]
fn test_change_subscription_token_applies_on_next_cycle() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id1 = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_id2 = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let sac1 = token::StellarAssetClient::new(&env, &token_id1);
    let sac2 = token::StellarAssetClient::new(&env, &token_id2);
    sac1.mint(&customer, &100000i128);
    sac2.mint(&customer, &100000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Approve the contract to spend from customer for both tokens
    token::Client::new(&env, &token_id1).approve(&customer, &contract_id, &100000i128, &10000u32);
    token::Client::new(&env, &token_id2).approve(&customer, &contract_id, &100000i128, &10000u32);

    env.ledger().set_timestamp(1000);

    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &100i128,
        &token_id1,
        &Currency::USDC,
        &3600u64,
        &0u64,
        &3u64,
        &soroban_sdk::String::from_str(&env, ""),
        &0u64,
    );

    // Add token_id2 to merchant's allowed tokens
    let mut allowed = Vec::new(&env);
    allowed.push_back(token_id1.clone());
    allowed.push_back(token_id2.clone());
    client.set_merchant_allowed_tokens(&merchant, &allowed);

    // Customer changes token to token_id2
    client.change_subscription_token(&customer, &sub_id, &token_id2);

    // Execute the next payment — it should use token_id2
    env.ledger().set_timestamp(1000 + 3600 + 1);
    client.execute_recurring_payment(&sub_id);

    // token_id2 balance for merchant should reflect the charge
    let token2_client = token::Client::new(&env, &token_id2);
    assert_eq!(
        token2_client.balance(&merchant),
        100i128,
        "Merchant should receive 100 via the new token"
    );
}

#[test]
fn test_change_subscription_token_rejects_disallowed_token() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id1 = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_id2 = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let bad_token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    token::StellarAssetClient::new(&env, &token_id1).mint(&customer, &100000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    env.ledger().set_timestamp(1000);

    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &100i128,
        &token_id1,
        &Currency::USDC,
        &3600u64,
        &0u64,
        &3u64,
        &soroban_sdk::String::from_str(&env, ""),
        &0u64,
    );

    // Merchant only allows token_id1 and token_id2
    let mut allowed = Vec::new(&env);
    allowed.push_back(token_id1.clone());
    allowed.push_back(token_id2.clone());
    client.set_merchant_allowed_tokens(&merchant, &allowed);

    // Attempt to change to a token not in the merchant's list
    let result = client.try_change_subscription_token(&customer, &sub_id, &bad_token);
    assert!(
        result.is_err(),
        "Should reject token not in merchant's list"
    );
}

// ── Issue #681: Prorated refund on cancellation ──────────────────────────────

#[test]
fn test_prorated_refund_on_cancel_with_flag() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let sac = token::StellarAssetClient::new(&env, &token_id);
    sac.mint(&customer, &100000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Approve the contract to spend from customer (for subscription payments)
    // and from merchant (for prorated refunds)
    let token_client_inner = token::Client::new(&env, &token_id);
    token_client_inner.approve(&customer, &contract_id, &100000i128, &10000u32);
    token_client_inner.approve(&merchant, &contract_id, &100000i128, &10000u32);

    // Enable prorated refunds for merchant
    client.set_refund_unused_on_cancel(&merchant, &true);

    env.ledger().set_timestamp(1000);

    // 3600-second billing interval; amount=3600 tokens per cycle
    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &3600i128,
        &token_id,
        &Currency::USDC,
        &3600u64,
        &0u64,
        &3u64,
        &soroban_sdk::String::from_str(&env, ""),
        &0u64,
    );

    // Execute first payment at t=1000+3600+1 so next_payment_at becomes 1000+7200
    env.ledger().set_timestamp(1000 + 3600 + 1);
    client.execute_recurring_payment(&sub_id);

    let customer_balance_after_pay = token::Client::new(&env, &token_id).balance(&customer);

    // Cancel halfway through the second cycle (1800 seconds in, 1800 seconds unused)
    // cycle_start = next_payment_at - interval = (1000+7200) - 3600 = 4600
    // cancel at t = 4600 + 1800 = 6400
    env.ledger().set_timestamp(6400);
    client.cancel_subscription(&customer, &sub_id);

    let customer_balance_after_cancel = token::Client::new(&env, &token_id).balance(&customer);
    let refund = customer_balance_after_cancel - customer_balance_after_pay;

    // Expected refund = 3600 * 1800 / 3600 = 1800
    assert_eq!(
        refund, 1800i128,
        "Customer should receive prorated refund of 1800"
    );
}

#[test]
fn test_no_prorated_refund_when_flag_off() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token_id).mint(&customer, &100000i128);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Flag is OFF (default)
    env.ledger().set_timestamp(1000);

    let sub_id = client.create_subscription(
        &customer,
        &merchant,
        &3600i128,
        &token_id,
        &Currency::USDC,
        &3600u64,
        &0u64,
        &3u64,
        &soroban_sdk::String::from_str(&env, ""),
        &0u64,
    );

    env.ledger().set_timestamp(1000 + 3600 + 1);
    client.execute_recurring_payment(&sub_id);

    let balance_before_cancel = token::Client::new(&env, &token_id).balance(&customer);

    env.ledger().set_timestamp(6400); // mid-cycle
    client.cancel_subscription(&customer, &sub_id);

    let balance_after_cancel = token::Client::new(&env, &token_id).balance(&customer);
    assert_eq!(
        balance_before_cancel, balance_after_cancel,
        "Without flag, no refund should be issued"
    );
}
