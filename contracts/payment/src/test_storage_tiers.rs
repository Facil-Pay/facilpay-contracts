#![cfg(test)]
//! Tests for #648: per-record and per-user data lives in persistent storage,
//! so the instance entry stays small no matter how much the contract is used.

extern crate std;

use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _, Ledger as _},
    xdr::{LedgerEntryData, ScAddress, ScVal},
    Address, Env, String, TryFromVal, Val,
};

use crate::{
    Currency, DataKey, PaymentContract, PaymentContractClient, PaymentKey, PERSISTENT_TTL_EXTEND_TO,
};

struct Fixture {
    env: Env,
    client: PaymentContractClient<'static>,
    contract_id: Address,
    token: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env));

    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();

    Fixture {
        env,
        client,
        contract_id,
        token,
    }
}

fn create_payment(f: &Fixture, customer: &Address, merchant: &Address) -> u64 {
    f.client.create_payment(
        customer,
        merchant,
        &100,
        &f.token,
        &Currency::USDC,
        &0,
        &String::from_str(&f.env, "order"),
    )
}

/// Every key currently held in the payment contract's instance entry.
fn instance_keys(f: &Fixture) -> std::vec::Vec<DataKey> {
    let contract = ScAddress::from(&f.contract_id);
    let snapshot = f.env.to_ledger_snapshot();
    let mut keys = std::vec::Vec::new();
    for (_, (entry, _)) in snapshot.ledger_entries.iter() {
        let LedgerEntryData::ContractData(data) = &entry.data else {
            continue;
        };
        if data.contract != contract || data.key != ScVal::LedgerKeyContractInstance {
            continue;
        }
        let ScVal::ContractInstance(instance) = &data.val else {
            continue;
        };
        if let Some(storage) = &instance.storage {
            for item in storage.iter() {
                let val = Val::try_from_val(&f.env, &item.key).unwrap();
                keys.push(DataKey::try_from_val(&f.env, &val).expect("non-DataKey instance key"));
            }
        }
    }
    keys
}

#[test]
fn test_no_per_record_or_per_user_key_in_instance_storage() {
    let f = setup();
    let customer = Address::generate(&f.env);
    let merchant = Address::generate(&f.env);

    let payment_id = create_payment(&f, &customer, &merchant);
    f.client.cancel_payment(&customer, &payment_id);
    let request_id = f.client.create_payment_request(
        &merchant,
        &250,
        &f.token,
        &Currency::USDC,
        &0,
        &String::from_str(&f.env, "invoice"),
        &None,
    );
    f.client.pay_payment_request(&customer, &request_id);

    let keys = instance_keys(&f);
    assert!(!keys.is_empty());
    for key in keys.iter() {
        assert!(
            key.is_instance(),
            "per-record key found in instance storage"
        );
    }

    // The records themselves are persistent entries.
    f.env.as_contract(&f.contract_id, || {
        let persistent = f.env.storage().persistent();
        assert!(persistent.has(&DataKey::Payment(PaymentKey::Data(payment_id))));
        assert!(persistent.has(&DataKey::Payment(PaymentKey::Request(request_id))));
    });
}

#[test]
fn test_unrelated_call_cost_does_not_grow_with_record_count() {
    let f = setup();
    let customer = Address::generate(&f.env);
    let merchant = Address::generate(&f.env);
    create_payment(&f, &customer, &merchant);

    // Cost of a first payment between a brand-new customer and merchant.
    create_payment(&f, &Address::generate(&f.env), &Address::generate(&f.env));
    let before = f.env.cost_estimate().resources();
    let instance_len_before = instance_keys(&f).len();

    for _ in 0..500 {
        create_payment(&f, &customer, &merchant);
    }
    assert_eq!(f.client.get_payment_count_by_merchant(&merchant), 501);

    // The same shape of call after 500+ records still fits the default budget.
    create_payment(&f, &Address::generate(&f.env), &Address::generate(&f.env));
    let after = f.env.cost_estimate().resources();
    assert!(
        after.instructions < 100_000_000,
        "exceeds default CPU budget"
    );

    // The instance entry is rewritten in full on every call, so if records were
    // still kept there these would grow with the record count. (CPU is not
    // compared: the test host clones its whole in-memory ledger on each call
    // frame, which grows with any stored entry regardless of storage tier.)
    assert_eq!(instance_keys(&f).len(), instance_len_before);
    assert_eq!(after.write_bytes, before.write_bytes);
    assert_eq!(after.write_entries, before.write_entries);
    assert_eq!(after.memory_read_entries, before.memory_read_entries);

    // An unrelated read-only call is unaffected as well.
    assert_eq!(f.client.get_schema_version(), 2);
    assert!(f.env.cost_estimate().resources().instructions < 100_000_000);
}

#[test]
fn test_persistent_entries_ttl_extended_on_write_and_read() {
    let f = setup();
    let customer = Address::generate(&f.env);
    let merchant = Address::generate(&f.env);
    let payment_id = create_payment(&f, &customer, &merchant);
    let key = DataKey::Payment(PaymentKey::Data(payment_id));

    let ttl = f.env.as_contract(&f.contract_id, || {
        f.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl, PERSISTENT_TTL_EXTEND_TO);

    // Let most of the TTL elapse, then read the record: the TTL is bumped back up.
    f.env.ledger().with_mut(|l| {
        l.sequence_number += PERSISTENT_TTL_EXTEND_TO - 10;
    });
    f.client.get_payment(&payment_id);
    let ttl = f.env.as_contract(&f.contract_id, || {
        f.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl, PERSISTENT_TTL_EXTEND_TO);
}

#[test]
fn test_key_classification() {
    let env = Env::default();
    let user = Address::generate(&env);

    assert!(DataKey::Payment(PaymentKey::Counter).is_instance());
    assert!(DataKey::Payment(PaymentKey::RequestCounter).is_instance());
    assert!(!DataKey::Payment(PaymentKey::Data(1)).is_instance());
    assert!(!DataKey::Payment(PaymentKey::Request(1)).is_instance());
    assert!(!DataKey::Customer(crate::CustomerDataKey::PaymentCount(user.clone())).is_instance());
    assert!(!DataKey::Merchant(crate::MerchantDataKey::Analytics(user)).is_instance());
    assert!(DataKey::Config(crate::ConfigKey::FeeConfig).is_instance());

    // The TTL target must stay within the network's maximum entry TTL.
    assert!(PERSISTENT_TTL_EXTEND_TO < 3_110_400);
}
