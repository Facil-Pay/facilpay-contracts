#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger},
    Address, Bytes, BytesN, Env,
};
use ed25519_dalek::{Signer, SigningKey};
use rand::thread_rng;

fn address_from_pk(env: &Env, pk: [u8; 32]) -> Address {
    // ScAddress::Account(AccountId::PublicKeyTypeEd25519(Uint256(pk)))
    let mut xdr = [0u8; 40];
    xdr[3] = 0; // ScAddress::Account
    xdr[7] = 0; // AccountId::PublicKeyTypeEd25519
    xdr[8..].copy_from_slice(&pk);
    Address::from_xdr(env, &Bytes::from_array(env, &xdr))
}

#[test]
fn test_payment_channel_full_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    // Generate a real Ed25519 keypair for the customer
    let mut rng = thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();
    
    let customer = address_from_pk(&env, pk_bytes);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    let token_client = token::Client::new(&env, &token_id);

    let payment_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &payment_id);
    client.initialize(&admin);

    let deposit_amount = 1000i128;
    token_admin_client.mint(&customer, &deposit_amount);

    let expires_at = env.ledger().timestamp() + 3600;
    let channel_id = client.open_channel(&customer, &merchant, &token_id, &deposit_amount, &expires_at);

    // Verify channel state
    let channel = client.get_channel(&channel_id);
    assert_eq!(channel.channel_id, channel_id);
    assert_eq!(channel.customer, customer);
    assert_eq!(channel.merchant, merchant);
    assert_eq!(channel.deposited, deposit_amount);
    assert!(channel.open);

    // Prepare settlement
    let merchant_amount = 600i128;
    let nonce = 1u64;

    // Construct the same message as in the contract
    let mut msg_bytes = Bytes::new(&env);
    msg_bytes.append(&channel_id.to_xdr(&env));
    msg_bytes.append(&merchant_amount.to_xdr(&env));
    msg_bytes.append(&nonce.to_xdr(&env));
    
    let mut msg_raw = vec![0u8; msg_bytes.len() as usize];
    msg_bytes.copy_into_slice(&mut msg_raw);

    // Sign the message
    let signature_bytes = signing_key.sign(&msg_raw).to_bytes();
    let signature = BytesN::from_array(&env, &signature_bytes);

    // Settle channel
    client.settle_channel(&channel_id, &merchant_amount, &nonce, &signature);

    // Verify balances after settlement
    assert_eq!(token_client.balance(&customer), 400i128);
    assert_eq!(token_client.balance(&merchant), 600i128);

    // Verify channel closed
    let channel_after = client.get_channel(&channel_id);
    assert!(!channel_after.open);
    assert_eq!(channel_after.settled, merchant_amount);
    assert_eq!(channel_after.nonce, nonce);
}

#[test]
fn test_settle_channel_invalid_nonce() {
    let env = Env::default();
    env.mock_all_auths();

    let mut rng = thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    let pk_bytes = signing_key.verifying_key().to_bytes();
    let customer = address_from_pk(&env, pk_bytes);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let payment_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &payment_id);
    client.initialize(&admin);

    token_admin_client.mint(&customer, &1000i128);
    let channel_id = client.open_channel(&customer, &merchant, &token_id, &1000i128, &0);

    // Message with nonce 0
    let mut msg = Bytes::new(&env);
    msg.append(&channel_id.to_xdr(&env));
    msg.append(&500i128.to_xdr(&env));
    msg.append(&0u64.to_xdr(&env));
    let mut msg_raw = vec![0u8; msg.len() as usize];
    msg.copy_into_slice(&mut msg_raw);
    let sig = BytesN::from_array(&env, &signing_key.sign(&msg_raw).to_bytes());
    
    let result = client.try_settle_channel(&channel_id, &500i128, &0, &sig);
    assert!(result.is_err());
    // Since it's a Result<Result<(), Error>, ...>
    match result {
        Err(Ok(Error::InvalidNonce)) => {},
        _ => panic!("Expected InvalidNonce error, got {:?}", result),
    }
}

#[test]
fn test_close_expired_channel() {
    let env = Env::default();
    env.mock_all_auths();

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    let token_client = token::Client::new(&env, &token_id);

    let payment_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &payment_id);
    client.initialize(&admin);

    token_admin_client.mint(&customer, &1000i128);
    
    let expires_at = 1000u64;
    env.ledger().set_timestamp(expires_at - 10);
    
    let channel_id = client.open_channel(&customer, &merchant, &token_id, &1000i128, &expires_at);

    // Fast forward
    env.ledger().set_timestamp(expires_at + 1);
    client.close_channel_expired(&channel_id);

    assert_eq!(token_client.balance(&customer), 1000i128);
    let channel = client.get_channel(&channel_id);
    assert!(!channel.open);
}
