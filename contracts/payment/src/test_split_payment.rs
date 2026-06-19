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
    assert_eq!(result, Err(Ok(Error::InvalidSplitShares)));
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
    assert_eq!(result, Err(Ok(Error::TooManyRecipients)));
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
    assert_eq!(result, Err(Ok(Error::SplitAlreadyExecuted)));
}
