#![cfg(test)]
//! Tests for #669: merchant registry with profile and active status.

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _},
    vec, Address, BytesN, Env, Event, String,
};

use crate::{
    BasicError, Currency, Error, MerchantDeactivated, MerchantProfileUpdated, MerchantReactivated,
    MerchantRegistered, PaymentContract, PaymentContractClient, PaymentError,
    RequireRegisteredMerchantsSet, MAX_MERCHANT_NAME_LEN,
};

struct Fixture {
    env: Env,
    client: PaymentContractClient<'static>,
    admin: Address,
    merchant: Address,
    customer: Address,
    token: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();

    Fixture {
        merchant: Address::generate(&env),
        customer: Address::generate(&env),
        env,
        client,
        admin,
        token,
    }
}

fn assert_last_event(f: &Fixture, event: &impl Event) {
    let all = f.env.events().all();
    assert_eq!(
        all.slice(all.len() - 1..),
        vec![
            &f.env,
            (
                f.client.address.clone(),
                event.topics(&f.env),
                event.data(&f.env)
            )
        ]
    );
}

fn name(f: &Fixture, s: &str) -> String {
    String::from_str(&f.env, s)
}

fn hash(f: &Fixture, byte: u8) -> BytesN<32> {
    BytesN::from_array(&f.env, &[byte; 32])
}

fn register(f: &Fixture) {
    f.client
        .register_merchant(&f.merchant, &name(f, "Acme"), &hash(f, 1));
}

fn try_pay(f: &Fixture) -> Result<u64, Error> {
    match f.client.try_create_payment(
        &f.customer,
        &f.merchant,
        &100,
        &f.token,
        &Currency::USDC,
        &0,
        &name(f, ""),
    ) {
        Ok(Ok(id)) => Ok(id),
        Err(Ok(e)) => Err(e),
        other => panic!("unexpected result: {:?}", other),
    }
}

// ── Registration and profile ────────────────────────────────────────────────

#[test]
fn test_register_merchant_stores_active_profile() {
    let f = setup();
    assert_eq!(f.client.get_merchant(&f.merchant), None);

    register(&f);
    assert_last_event(
        &f,
        &MerchantRegistered {
            merchant: f.merchant.clone(),
            name: name(&f, "Acme"),
            metadata_hash: hash(&f, 1),
        },
    );

    let profile = f.client.get_merchant(&f.merchant).unwrap();
    assert_eq!(profile.merchant, f.merchant);
    assert_eq!(profile.name, name(&f, "Acme"));
    assert_eq!(profile.metadata_hash, hash(&f, 1));
    assert!(profile.active);
    assert_eq!(profile.registered_at, 1_000);
    assert_eq!(profile.updated_at, 1_000);
}

#[test]
fn test_register_merchant_twice_fails() {
    let f = setup();
    register(&f);
    assert_eq!(
        f.client
            .try_register_merchant(&f.merchant, &name(&f, "Other"), &hash(&f, 2)),
        Err(Ok(Error::Payment(PaymentError::MerchantAlreadyRegistered)))
    );
}

#[test]
fn test_register_merchant_validates_name() {
    let f = setup();
    assert_eq!(
        f.client
            .try_register_merchant(&f.merchant, &name(&f, ""), &hash(&f, 1)),
        Err(Ok(Error::Payment(PaymentError::InvalidMerchantName)))
    );

    let too_long = [b'a'; (MAX_MERCHANT_NAME_LEN + 1) as usize];
    let too_long = String::from_bytes(&f.env, &too_long);
    assert_eq!(
        f.client
            .try_register_merchant(&f.merchant, &too_long, &hash(&f, 1)),
        Err(Ok(Error::Payment(PaymentError::InvalidMerchantName)))
    );

    let max = [b'a'; MAX_MERCHANT_NAME_LEN as usize];
    let max = String::from_bytes(&f.env, &max);
    f.client.register_merchant(&f.merchant, &max, &hash(&f, 1));
}

#[test]
fn test_update_merchant_profile() {
    let f = setup();
    assert_eq!(
        f.client
            .try_update_merchant_profile(&f.merchant, &name(&f, "New"), &hash(&f, 2)),
        Err(Ok(Error::Payment(PaymentError::MerchantNotRegistered)))
    );

    register(&f);
    f.env.ledger().set_timestamp(2_000);
    f.client
        .update_merchant_profile(&f.merchant, &name(&f, "New"), &hash(&f, 2));
    assert_last_event(
        &f,
        &MerchantProfileUpdated {
            merchant: f.merchant.clone(),
            name: name(&f, "New"),
            metadata_hash: hash(&f, 2),
        },
    );

    let profile = f.client.get_merchant(&f.merchant).unwrap();
    assert_eq!(profile.name, name(&f, "New"));
    assert_eq!(profile.metadata_hash, hash(&f, 2));
    assert_eq!(profile.registered_at, 1_000);
    assert_eq!(profile.updated_at, 2_000);
    assert!(profile.active);

    assert_eq!(
        f.client
            .try_update_merchant_profile(&f.merchant, &name(&f, ""), &hash(&f, 2)),
        Err(Ok(Error::Payment(PaymentError::InvalidMerchantName)))
    );
}

// ── Deactivation ────────────────────────────────────────────────────────────

#[test]
fn test_deactivate_and_reactivate_merchant() {
    let f = setup();
    register(&f);

    f.client.deactivate_merchant(&f.admin, &f.merchant);
    assert_last_event(
        &f,
        &MerchantDeactivated {
            merchant: f.merchant.clone(),
            admin: f.admin.clone(),
        },
    );
    assert!(!f.client.get_merchant(&f.merchant).unwrap().active);
    assert_eq!(
        f.client.try_deactivate_merchant(&f.admin, &f.merchant),
        Err(Ok(Error::Payment(PaymentError::MerchantInactive)))
    );

    // A deactivated merchant cannot re-register itself back to active.
    assert_eq!(
        f.client
            .try_register_merchant(&f.merchant, &name(&f, "Acme"), &hash(&f, 1)),
        Err(Ok(Error::Payment(PaymentError::MerchantAlreadyRegistered)))
    );

    f.client.reactivate_merchant(&f.admin, &f.merchant);
    assert_last_event(
        &f,
        &MerchantReactivated {
            merchant: f.merchant.clone(),
            admin: f.admin.clone(),
        },
    );
    assert!(f.client.get_merchant(&f.merchant).unwrap().active);
    assert_eq!(
        f.client.try_reactivate_merchant(&f.admin, &f.merchant),
        Err(Ok(Error::Payment(PaymentError::InvalidStatus)))
    );
}

#[test]
fn test_deactivate_merchant_requires_admin_and_registration() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    assert_eq!(
        f.client.try_deactivate_merchant(&f.admin, &f.merchant),
        Err(Ok(Error::Payment(PaymentError::MerchantNotRegistered)))
    );

    register(&f);
    assert_eq!(
        f.client.try_deactivate_merchant(&stranger, &f.merchant),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        f.client.try_reactivate_merchant(&stranger, &f.merchant),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        f.client
            .try_set_require_registered_merchants(&stranger, &true),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
}

// ── Enforcement flag ────────────────────────────────────────────────────────

#[test]
fn test_flag_off_by_default_leaves_payments_unchanged() {
    let f = setup();
    assert!(!f.client.get_require_registered_merchants());

    // Unregistered merchant can be paid.
    assert!(try_pay(&f).is_ok());

    // A deactivated merchant can still be paid while the flag is off.
    register(&f);
    f.client.deactivate_merchant(&f.admin, &f.merchant);
    assert!(try_pay(&f).is_ok());
}

#[test]
fn test_flag_on_rejects_unregistered_and_deactivated_merchants() {
    let f = setup();
    f.client.set_require_registered_merchants(&f.admin, &true);
    assert_last_event(
        &f,
        &RequireRegisteredMerchantsSet {
            required: true,
            admin: f.admin.clone(),
        },
    );
    assert!(f.client.get_require_registered_merchants());

    assert_eq!(
        try_pay(&f),
        Err(Error::Payment(PaymentError::MerchantNotRegistered))
    );

    register(&f);
    assert!(try_pay(&f).is_ok());

    f.client.deactivate_merchant(&f.admin, &f.merchant);
    assert_eq!(
        try_pay(&f),
        Err(Error::Payment(PaymentError::MerchantInactive))
    );

    f.client.reactivate_merchant(&f.admin, &f.merchant);
    assert!(try_pay(&f).is_ok());

    // Turning the flag back off restores permissionless payments.
    f.client.deactivate_merchant(&f.admin, &f.merchant);
    f.client.set_require_registered_merchants(&f.admin, &false);
    assert!(try_pay(&f).is_ok());
}

#[test]
fn test_flag_on_blocks_payment_requests_and_subscriptions() {
    let f = setup();
    f.client.set_require_registered_merchants(&f.admin, &true);

    assert_eq!(
        f.client.try_create_payment_request(
            &f.merchant,
            &100,
            &f.token,
            &Currency::USDC,
            &0,
            &name(&f, ""),
            &None,
        ),
        Err(Ok(Error::Payment(PaymentError::MerchantNotRegistered)))
    );
    assert_eq!(
        f.client.try_create_subscription(
            &f.customer,
            &f.merchant,
            &1000,
            &f.token,
            &Currency::USDC,
            &3600,
            &0,
            &3,
            &name(&f, ""),
            &0,
        ),
        Err(Ok(Error::Payment(PaymentError::MerchantNotRegistered)))
    );

    register(&f);
    let request_id = f.client.create_payment_request(
        &f.merchant,
        &100,
        &f.token,
        &Currency::USDC,
        &0,
        &name(&f, ""),
        &None,
    );

    // A request issued while active cannot be paid after deactivation.
    f.client.deactivate_merchant(&f.admin, &f.merchant);
    assert_eq!(
        f.client.try_pay_payment_request(&f.customer, &request_id),
        Err(Ok(Error::Payment(PaymentError::MerchantInactive)))
    );
}
