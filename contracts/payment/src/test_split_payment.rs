#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, Vec};

fn setup(env: &Env) -> (PaymentContractClient<'_>, Address, Address) {
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin, contract_id)
}

fn make_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

fn fund(env: &Env, token: &Address, admin: &Address, to: &Address, amount: i128) {
    let token_admin = token::StellarAssetClient::new(env, token);
    token_admin.mint(to, &amount);
    let _ = admin;
}

#[test]
fn test_valid_split_payment() {
    let env = Env::default();
    let (client, admin, contract_id) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    let r1 = SplitRecipient {
        address: merchant.clone(),
        share_bps: 7000,
    };
    let r2 = SplitRecipient {
        address: admin.clone(),
        share_bps: 3000,
    };
    let mut recipients = Vec::new(&env);
    recipients.push_back(r1);
    recipients.push_back(r2);

    let payment_id = client.create_split_payment(&customer, &merchant, &1000, &token, &recipients);
    assert_eq!(payment_id, 1);

    let config = client.get_split_config(&payment_id).unwrap();
    assert_eq!(config.payment_id, 1);
    assert!(!config.executed);

    client.execute_split_settlement(&admin, &payment_id);

    let config = client.get_split_config(&payment_id).unwrap();
    assert!(config.executed);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&merchant), 700);
    assert_eq!(token_client.balance(&admin), 300);
}

#[test]
fn test_invalid_split_shares() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: merchant.clone(),
        share_bps: 5000,
    });
    recipients.push_back(SplitRecipient {
        address: admin.clone(),
        share_bps: 4000,
    }); // total = 9000, not 10000

    let result = client.try_create_split_payment(&customer, &merchant, &1000, &token, &recipients);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::InvalidSplitShares)))
    );
}

#[test]
fn test_too_many_recipients() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    let mut recipients = Vec::new(&env);
    for i in 0..11u32 {
        recipients.push_back(SplitRecipient {
            address: Address::generate(&env),
            share_bps: if i < 10 { 900 } else { 1000 },
        });
    }

    let result = client.try_create_split_payment(&customer, &merchant, &1000, &token, &recipients);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::TooManyRecipients)))
    );
}

#[test]
fn test_double_execution_rejected() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: merchant.clone(),
        share_bps: 10000,
    });

    let payment_id = client.create_split_payment(&customer, &merchant, &1000, &token, &recipients);

    client.execute_split_settlement(&admin, &payment_id);

    let result = client.try_execute_split_settlement(&admin, &payment_id);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::SplitAlreadyExecuted)))
    );
}

// ── Sender-as-recipient tests (#341) ─────────────────────────────────────────

/// A split where the only recipient is the sender must be rejected. Routing the
/// full amount back to the sender while debiting the contract is economically
/// meaningless and indicates a misconfigured call.
#[test]
fn test_split_with_sender_as_recipient_is_rejected() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    // Sender (customer) is the sole recipient
    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: customer.clone(),
        share_bps: 10000,
    });

    let result = client.try_create_split_payment(&customer, &merchant, &1000, &token, &recipients);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::SenderIsRecipient)))
    );
}

/// A split where the sender holds even a small share (partial recipient) must be
/// rejected — the full-amount check is not sufficient on its own.
#[test]
fn test_split_with_sender_as_partial_recipient_is_rejected() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    // Sender (customer) is one of the recipients with a small share
    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: merchant.clone(),
        share_bps: 9900,
    });
    recipients.push_back(SplitRecipient {
        address: customer.clone(), // sender sneaks in as a recipient
        share_bps: 100,
    });
    let _ = other;

    let result = client.try_create_split_payment(&customer, &merchant, &1000, &token, &recipients);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::SenderIsRecipient)))
    );
}

/// A split with all distinct recipients (none being the sender) must succeed.
#[test]
fn test_valid_split_with_distinct_recipients_succeeds() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let recipient_a = Address::generate(&env);
    let recipient_b = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 1000);

    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: recipient_a.clone(),
        share_bps: 6000,
    });
    recipients.push_back(SplitRecipient {
        address: recipient_b.clone(),
        share_bps: 4000,
    });

    let payment_id =
        client.create_split_payment(&customer, &recipient_a, &1000, &token, &recipients);
    assert_eq!(payment_id, 1);

    client.execute_split_settlement(&admin, &payment_id);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient_a), 600);
    assert_eq!(token_client.balance(&recipient_b), 400);
}

// ── Min split amount tests (min_split_amount guard) ──────────────────────────

/// Admin can set a minimum per-recipient split amount. A split where any
/// recipient's computed share falls below that minimum must be rejected.
#[test]
fn test_split_below_min_amount_rejected() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 10000);

    // Set minimum split amount to 500
    client.set_min_split_amount(&admin, &500i128);

    // recipient_b gets 1% of 1000 = 10, which is below 500
    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: merchant.clone(),
        share_bps: 9900,
    });
    recipients.push_back(SplitRecipient {
        address: other.clone(),
        share_bps: 100,
    });

    let result = client.try_create_split_payment(&customer, &merchant, &1000, &token, &recipients);
    assert_eq!(
        result,
        Err(Ok(Error::Feature(FeatureError::BelowMinSplitAmount)))
    );
}

/// When all per-recipient shares meet or exceed the configured minimum, the
/// split must succeed.
#[test]
fn test_split_meeting_min_amount_succeeds() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 10000);

    // Set minimum to 100; split 8000 + 2000 on a 10000 total both exceed 100
    client.set_min_split_amount(&admin, &100i128);

    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: merchant.clone(),
        share_bps: 8000,
    });
    recipients.push_back(SplitRecipient {
        address: other.clone(),
        share_bps: 2000,
    });

    let payment_id = client.create_split_payment(&customer, &merchant, &10000, &token, &recipients);
    assert_eq!(payment_id, 1);

    client.execute_split_settlement(&admin, &payment_id);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&merchant), 8000);
    assert_eq!(token_client.balance(&other), 2000);
}

/// Without a configured minimum, any share amount (including 1 stroop) is allowed.
#[test]
fn test_split_without_min_amount_allows_tiny_shares() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let other = Address::generate(&env);
    let token = make_token(&env, &admin);

    fund(&env, &token, &admin, &customer, 10000);

    // No min split amount set — tiny shares are permitted
    let mut recipients = Vec::new(&env);
    recipients.push_back(SplitRecipient {
        address: merchant.clone(),
        share_bps: 9999,
    });
    recipients.push_back(SplitRecipient {
        address: other.clone(),
        share_bps: 1,
    });

    let payment_id = client.create_split_payment(&customer, &merchant, &10000, &token, &recipients);
    assert_eq!(payment_id, 1);
}
