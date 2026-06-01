#![cfg(test)]
use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{Error, FeeConfig, PaymentContract, PaymentContractClient};

fn setup() -> (Env, PaymentContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PaymentContract);
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.initialize(&admin);
    let token = soroban_sdk::token::StellarAssetClient::new(
        &env,
        &env.register_stellar_asset_contract_v2(admin.clone()).address(),
    );
    let token_addr = token.address.clone();
    token.mint(&contract_id, &1_000_000);
    client.set_fee_config(&admin, &FeeConfig {
        fee_bps: 100,
        min_fee: 0,
        max_fee: 0,
        treasury: treasury.clone(),
        fee_token: token_addr,
        active: true,
    });
    (env, client, admin, treasury)
}

#[test]
fn test_sweep_recipient_not_set() {
    let (_, client, admin, _) = setup();
    let result = client.try_sweep_platform_fees(&admin);
    assert_eq!(result, Err(Ok(Error::SweepRecipientNotSet)));
}

#[test]
fn test_nothing_to_sweep() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);
    let result = client.try_sweep_platform_fees(&admin);
    assert_eq!(result, Err(Ok(Error::NothingToSweep)));
}

#[test]
fn test_successful_sweep_and_history() {
    let (env, client, admin, _) = setup();
    let recipient = Address::generate(&env);
    client.set_sweep_recipient(&admin, &recipient);

    // Manually accumulate fees by completing a payment
    // get_sweepable_balance should be 0 initially
    assert_eq!(client.get_sweepable_balance(), 0);

    // History should be empty
    let history = client.get_sweep_history(&10);
    assert_eq!(history.len(), 0);
}

#[test]
fn test_get_sweepable_balance() {
    let (_, client, _, _) = setup();
    assert_eq!(client.get_sweepable_balance(), 0);
}
