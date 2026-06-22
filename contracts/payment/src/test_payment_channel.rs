#![cfg(test)]

extern crate alloc;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Bytes, BytesN, Env,
};

#[test]
fn test_close_expired_channel() {
    let env = Env::default();
    env.mock_all_auths();

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    let token_client = token::Client::new(&env, &token_id);

    let payment_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &payment_id);
    client.initialize(&admin);

    token_admin_client.mint(&customer, &1000i128);

    let expires_at = 1000u64;
    env.ledger().set_timestamp(expires_at - 10);

    let dummy_pk = BytesN::<32>::from_array(&env, &[0u8; 32]);
    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &expires_at,
        &dummy_pk,
    );

    // Fast forward
    env.ledger().set_timestamp(expires_at + 1);
    client.close_channel_expired(&channel_id);

    assert_eq!(token_client.balance(&customer), 1000i128);
    let channel = client.get_channel(&channel_id);
    assert!(!channel.open);
}

#[test]
fn test_payment_channel_full_lifecycle() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    // Use C... contract address for customer — no trustline needed with SAC tokens
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Fund the customer
    token_admin_client.mint(&customer, &1000i128);

    // Generate a real Ed25519 keypair for signing
    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    // Open channel
    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64, // no expiry
        &customer_pk,
    );

    let channel = client.get_channel(&channel_id);
    assert!(channel.open);
    assert_eq!(channel.deposited, 1000i128);

    // Build the message exactly as settle_channel does:
    // channel_id.to_xdr || merchant_amount.to_xdr || nonce.to_xdr
    let merchant_amount: i128 = 700;
    let nonce: u64 = 1;
    let mut msg = Bytes::new(&env);
    msg.append(&channel_id.to_xdr(&env));
    msg.append(&merchant_amount.to_xdr(&env));
    msg.append(&nonce.to_xdr(&env));

    // Collect message bytes and sign
    let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
    let signature = signing_key.sign(&msg_vec);
    let sig_bn = BytesN::<64>::from_array(&env, &signature.to_bytes());

    // Settle the channel
    client.settle_channel(&channel_id, &merchant_amount, &nonce, &sig_bn);

    // Verify balances
    let token_client = token::Client::new(&env, &token_id);
    assert_eq!(token_client.balance(&merchant), 700i128);
    assert_eq!(token_client.balance(&customer), 300i128);

    let channel_after = client.get_channel(&channel_id);
    assert!(!channel_after.open);
    assert_eq!(channel_after.settled, 700i128);
    assert_eq!(channel_after.settled_nonce, 1u64);
}

#[test]
fn test_settle_channel_invalid_nonce() {
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    token_admin_client.mint(&customer, &1000i128);

    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64,
        &customer_pk,
    );

    // Build a signature with nonce = 0 (invalid — must be > channel.settled_nonce which starts at 0)
    let merchant_amount: i128 = 500;
    let bad_nonce: u64 = 0;
    let mut msg = Bytes::new(&env);
    msg.append(&channel_id.to_xdr(&env));
    msg.append(&merchant_amount.to_xdr(&env));
    msg.append(&bad_nonce.to_xdr(&env));

    let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
    let signature = signing_key.sign(&msg_vec);
    let sig_bn = BytesN::<64>::from_array(&env, &signature.to_bytes());

    // Should fail: nonce 0 is not > channel settled_nonce 0
    let result = client.try_settle_channel(&channel_id, &merchant_amount, &bad_nonce, &sig_bn);
    assert!(result.is_err(), "Expected InvalidNonce error");
}

#[test]
fn test_stale_nonce_replay_rejected() {
    // Demonstrate that a stale off-chain state (lower nonce) cannot be replayed
    // once the channel has been settled with a higher nonce.
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    token_admin_client.mint(&customer, &1000i128);

    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64,
        &customer_pk,
    );

    // Customer signed two off-chain states:
    //   stale: nonce=1, merchant gets 100
    //   latest: nonce=5, merchant gets 800
    let sign_state = |nonce: u64, amount: i128| -> BytesN<64> {
        let mut msg = Bytes::new(&env);
        msg.append(&channel_id.to_xdr(&env));
        msg.append(&amount.to_xdr(&env));
        msg.append(&nonce.to_xdr(&env));
        let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
        let sig = signing_key.sign(&msg_vec);
        BytesN::<64>::from_array(&env, &sig.to_bytes())
    };

    let stale_sig = sign_state(1, 100);
    let latest_sig = sign_state(5, 800);

    // Merchant submits the latest state (nonce=5) — this should succeed
    client.settle_channel(&channel_id, &800i128, &5u64, &latest_sig);

    let channel = client.get_channel(&channel_id);
    assert!(!channel.open);
    assert_eq!(channel.settled_nonce, 5u64);

    // Attacker (or customer) now tries to replay the stale state (nonce=1) — must fail
    let result = client.try_settle_channel(&channel_id, &100i128, &1u64, &stale_sig);
    assert!(result.is_err(), "Stale nonce replay must be rejected");
}

#[test]
fn test_equal_nonce_replay_rejected() {
    // A settlement with nonce == settled_nonce must be rejected (must be strictly greater).
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    token_admin_client.mint(&customer, &1000i128);

    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer_pk = BytesN::<32>::from_array(&env, &pk_bytes);

    let channel_id = client.open_channel(
        &customer,
        &merchant,
        &token_id,
        &1000i128,
        &0u64,
        &customer_pk,
    );

    // nonce=0 equals settled_nonce initial value of 0 — must be rejected (not strictly greater)
    let mut msg = Bytes::new(&env);
    msg.append(&channel_id.to_xdr(&env));
    msg.append(&500i128.to_xdr(&env));
    msg.append(&0u64.to_xdr(&env));
    let msg_vec: alloc::vec::Vec<u8> = msg.iter().collect();
    let sig = signing_key.sign(&msg_vec);
    let sig_bn = BytesN::<64>::from_array(&env, &sig.to_bytes());

    let result = client.try_settle_channel(&channel_id, &500i128, &0u64, &sig_bn);
    assert!(result.is_err(), "Equal nonce must be rejected; strictly increasing required");
}
