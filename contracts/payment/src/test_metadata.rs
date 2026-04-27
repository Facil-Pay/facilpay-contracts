#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, String};

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

fn setup_test_payment(
    env: &Env,
    client: &PaymentContractClient,
    merchant: &Address,
    customer: &Address,
    token_client: &token::Client,
) -> u64 {
    let payment_id = client.create_payment(
        merchant,
        customer,
        &1000i128,
        &token_client.address,
        &Currency::USDC,
        &3600u64,
        &String::from_str(env, "Test payment"),
    );
    payment_id
}

#[test]
fn test_set_payment_metadata_by_merchant() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set metadata
    let content_ref = String::from_str(&env, "QmXYZ123...");
    let content_hash = BytesN::from_array(&env, &[1u8; 32]);

    client.set_payment_metadata(&merchant, &payment_id, &content_ref, &content_hash, &true);

    // Verify metadata was set
    let metadata = client.get_payment_metadata(&payment_id);
    assert!(metadata.is_some());
    let meta = metadata.unwrap();
    assert_eq!(meta.payment_id, payment_id);
    assert_eq!(meta.content_ref, content_ref);
    assert_eq!(meta.content_hash, content_hash);
    assert_eq!(meta.encrypted, true);
    assert_eq!(meta.version, 1);
}

#[test]
fn test_set_payment_metadata_by_customer() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set metadata by customer
    let content_ref = String::from_str(&env, "QmABC456...");
    let content_hash = BytesN::from_array(&env, &[2u8; 32]);

    client.set_payment_metadata(&customer, &payment_id, &content_ref, &content_hash, &false);

    // Verify metadata was set
    let metadata = client.get_payment_metadata(&payment_id);
    assert!(metadata.is_some());
    let meta = metadata.unwrap();
    assert_eq!(meta.encrypted, false);
}

#[test]
fn test_set_payment_metadata_by_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set metadata by admin
    let content_ref = String::from_str(&env, "QmDEF789...");
    let content_hash = BytesN::from_array(&env, &[3u8; 32]);

    client.set_payment_metadata(&admin, &payment_id, &content_ref, &content_hash, &true);

    // Verify metadata was set
    let metadata = client.get_payment_metadata(&payment_id);
    assert!(metadata.is_some());
}

#[test]
#[should_panic]
fn test_set_payment_metadata_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Try to set metadata by unauthorized party
    let content_ref = String::from_str(&env, "QmUNAUTH...");
    let content_hash = BytesN::from_array(&env, &[4u8; 32]);

    client.set_payment_metadata(
        &unauthorized,
        &payment_id,
        &content_ref,
        &content_hash,
        &true,
    );
}

#[test]
#[should_panic]
fn test_set_payment_metadata_nonexistent_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    // Try to set metadata for non-existent payment
    let content_ref = String::from_str(&env, "QmNONE...");
    let content_hash = BytesN::from_array(&env, &[5u8; 32]);

    client.set_payment_metadata(&merchant, &999u64, &content_ref, &content_hash, &true);
}

#[test]
fn test_update_payment_metadata_keeps_original_hash() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set initial metadata
    let content_ref1 = String::from_str(&env, "QmVER1...");
    let original_hash = BytesN::from_array(&env, &[10u8; 32]);

    client.set_payment_metadata(&merchant, &payment_id, &content_ref1, &original_hash, &true);

    // Update metadata with new content ref but different hash
    let content_ref2 = String::from_str(&env, "QmVER2...");
    let new_hash = BytesN::from_array(&env, &[20u8; 32]);

    client.set_payment_metadata(&merchant, &payment_id, &content_ref2, &new_hash, &true);

    // Verify original hash is preserved
    let metadata = client.get_payment_metadata(&payment_id);
    assert!(metadata.is_some());
    let meta = metadata.unwrap();
    assert_eq!(meta.content_ref, content_ref2); // Updated
    assert_eq!(meta.content_hash, original_hash); // Original preserved
    assert_eq!(meta.version, 2); // Version incremented
}

#[test]
fn test_verify_metadata_integrity_success() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set metadata
    let content_ref = String::from_str(&env, "QmHASH...");
    let content_hash = BytesN::from_array(&env, &[42u8; 32]);

    client.set_payment_metadata(&merchant, &payment_id, &content_ref, &content_hash, &true);

    // Verify with correct hash
    let is_valid = client.verify_metadata_integrity(&payment_id, &content_hash);
    assert!(is_valid);
}

#[test]
fn test_verify_metadata_integrity_failure() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set metadata
    let content_ref = String::from_str(&env, "QmHASH...");
    let content_hash = BytesN::from_array(&env, &[42u8; 32]);

    client.set_payment_metadata(&merchant, &payment_id, &content_ref, &content_hash, &true);

    // Verify with incorrect hash
    let wrong_hash = BytesN::from_array(&env, &[99u8; 32]);
    let is_valid = client.verify_metadata_integrity(&payment_id, &wrong_hash);
    assert!(!is_valid);
}

#[test]
fn test_verify_metadata_integrity_no_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Verify without setting metadata
    let some_hash = BytesN::from_array(&env, &[1u8; 32]);
    let is_valid = client.verify_metadata_integrity(&payment_id, &some_hash);
    assert!(!is_valid);
}

#[test]
fn test_get_payment_metadata_none() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Get metadata without setting it
    let metadata = client.get_payment_metadata(&payment_id);
    assert!(metadata.is_none());
}

#[test]
fn test_metadata_version_increments() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    let content_hash = BytesN::from_array(&env, &[1u8; 32]);

    // Set metadata - version 1
    client.set_payment_metadata(
        &merchant,
        &payment_id,
        &String::from_str(&env, "QmV1..."),
        &content_hash,
        &true,
    );

    let meta1 = client.get_payment_metadata(&payment_id).unwrap();
    assert_eq!(meta1.version, 1);

    // Update metadata - version 2
    client.set_payment_metadata(
        &merchant,
        &payment_id,
        &String::from_str(&env, "QmV2..."),
        &content_hash,
        &true,
    );

    let meta2 = client.get_payment_metadata(&payment_id).unwrap();
    assert_eq!(meta2.version, 2);

    // Update again - version 3
    client.set_payment_metadata(
        &merchant,
        &payment_id,
        &String::from_str(&env, "QmV3..."),
        &content_hash,
        &true,
    );

    let meta3 = client.get_payment_metadata(&payment_id).unwrap();
    assert_eq!(meta3.version, 3);
}

#[test]
fn test_encrypted_and_unencrypted_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let customer = Address::generate(&env);

    let (token_client, token_admin) = create_token_contract(&env, &admin);
    token_admin.mint(&customer, &10000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let payment_id = setup_test_payment(&env, &client, &merchant, &customer, &token_client);

    // Set encrypted metadata
    let content_ref = String::from_str(&env, "QmENCRYPTED...");
    let content_hash = BytesN::from_array(&env, &[1u8; 32]);

    client.set_payment_metadata(
        &merchant,
        &payment_id,
        &content_ref,
        &content_hash,
        &true, // encrypted
    );

    let meta = client.get_payment_metadata(&payment_id).unwrap();
    assert_eq!(meta.encrypted, true);

    // Update to unencrypted
    client.set_payment_metadata(
        &merchant,
        &payment_id,
        &String::from_str(&env, "QmPLAIN..."),
        &content_hash,
        &false, // not encrypted
    );

    let meta2 = client.get_payment_metadata(&payment_id).unwrap();
    assert_eq!(meta2.encrypted, false);
}
