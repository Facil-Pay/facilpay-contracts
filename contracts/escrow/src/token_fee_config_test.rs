#![cfg(test)]

use crate::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

const AMOUNT: i128 = 10_000;

struct Setup<'a> {
    env: Env,
    client: EscrowContractClient<'a>,
    contract_id: Address,
    admin: Address,
    customer: Address,
    merchant: Address,
    xlm: Address,
    usdc: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let xlm = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let usdc = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &xlm).mint(&customer, &(AMOUNT * 10));
    token::StellarAssetClient::new(&env, &usdc).mint(&customer, &(AMOUNT * 10));

    Setup {
        env,
        client,
        contract_id,
        admin,
        customer,
        merchant,
        xlm,
        usdc,
    }
}

fn fee_config(fee_bps: i128, fee_recipient: &Address, enabled: bool) -> EscrowFeeConfig {
    EscrowFeeConfig {
        fee_bps,
        fee_recipient: fee_recipient.clone(),
        enabled,
        max_fee: None,
    }
}

fn create_escrow(s: &Setup, token: &Address) -> u64 {
    s.client.create_escrow(
        &s.customer,
        &s.merchant,
        &AMOUNT,
        token,
        &0_u64,
        &0_u64,
        &0_u64,
        &false,
    )
}

#[test]
fn test_token_config_takes_precedence_over_global() {
    let s = setup();
    s.client
        .set_escrow_fee_config(&s.admin, &fee_config(100, &s.contract_id, true));
    s.client
        .set_token_escrow_fee_config(&s.admin, &s.usdc, &fee_config(250, &s.contract_id, true));

    let usdc_escrow = create_escrow(&s, &s.usdc);
    let xlm_escrow = create_escrow(&s, &s.xlm);

    assert_eq!(s.client.get_escrow(&usdc_escrow).fee_bps, 250);
    assert_eq!(s.client.get_escrow(&xlm_escrow).fee_bps, 100);
    assert_eq!(
        s.client.get_effective_escrow_fee_config(&s.usdc).fee_bps,
        250
    );
    assert_eq!(
        s.client.get_effective_escrow_fee_config(&s.xlm).fee_bps,
        100
    );
}

#[test]
fn test_disabled_token_config_overrides_enabled_global() {
    let s = setup();
    s.client
        .set_escrow_fee_config(&s.admin, &fee_config(100, &s.contract_id, true));
    s.client.set_token_escrow_fee_config(
        &s.admin,
        &s.usdc,
        &fee_config(500, &s.contract_id, false),
    );

    let escrow_id = create_escrow(&s, &s.usdc);
    assert_eq!(s.client.get_escrow(&escrow_id).fee_bps, 0);

    s.client.release_escrow(&s.admin, &escrow_id, &false);
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.usdc), 0);
    assert_eq!(
        token::Client::new(&s.env, &s.usdc).balance(&s.merchant),
        AMOUNT
    );
}

#[test]
fn test_token_config_applies_without_global_config() {
    let s = setup();
    s.client
        .set_token_escrow_fee_config(&s.admin, &s.usdc, &fee_config(300, &s.contract_id, true));

    assert_eq!(
        s.client.get_escrow(&create_escrow(&s, &s.usdc)).fee_bps,
        300
    );
    assert_eq!(s.client.get_escrow(&create_escrow(&s, &s.xlm)).fee_bps, 0);
}

#[test]
fn test_accumulated_fees_are_tracked_per_token() {
    let s = setup();
    s.client
        .set_escrow_fee_config(&s.admin, &fee_config(100, &s.contract_id, true));
    s.client
        .set_token_escrow_fee_config(&s.admin, &s.usdc, &fee_config(250, &s.contract_id, true));

    let usdc_escrow = create_escrow(&s, &s.usdc);
    let xlm_escrow = create_escrow(&s, &s.xlm);
    s.client.release_escrow(&s.admin, &usdc_escrow, &false);
    s.client.release_escrow(&s.admin, &xlm_escrow, &false);

    // 2.5% of 10_000 in USDC, 1% of 10_000 in XLM.
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.usdc), 250);
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.xlm), 100);

    let usdc_client = token::Client::new(&s.env, &s.usdc);
    let xlm_client = token::Client::new(&s.env, &s.xlm);
    assert_eq!(usdc_client.balance(&s.merchant), AMOUNT - 250);
    assert_eq!(xlm_client.balance(&s.merchant), AMOUNT - 100);

    // Withdrawing one token's fees leaves the other untouched.
    let treasury = Address::generate(&s.env);
    assert_eq!(
        s.client.withdraw_escrow_fees(&s.admin, &s.usdc, &treasury),
        250
    );
    assert_eq!(usdc_client.balance(&treasury), 250);
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.usdc), 0);
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.xlm), 100);
}

#[test]
fn test_token_config_recipient_receives_fee() {
    let s = setup();
    let global_recipient = Address::generate(&s.env);
    let usdc_recipient = Address::generate(&s.env);
    s.client
        .set_escrow_fee_config(&s.admin, &fee_config(100, &global_recipient, true));
    s.client.set_token_escrow_fee_config(
        &s.admin,
        &s.usdc,
        &fee_config(200, &usdc_recipient, true),
    );

    let escrow_id = create_escrow(&s, &s.usdc);
    s.client.release_escrow(&s.admin, &escrow_id, &false);

    let usdc_client = token::Client::new(&s.env, &s.usdc);
    assert_eq!(usdc_client.balance(&usdc_recipient), 200);
    assert_eq!(usdc_client.balance(&global_recipient), 0);
    // An external recipient is paid directly, not accrued in the contract.
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.usdc), 0);
}

#[test]
fn test_remove_token_config_falls_back_to_global() {
    let s = setup();
    s.client
        .set_escrow_fee_config(&s.admin, &fee_config(100, &s.contract_id, true));
    s.client
        .set_token_escrow_fee_config(&s.admin, &s.usdc, &fee_config(250, &s.contract_id, true));
    assert!(s.client.get_token_escrow_fee_config(&s.usdc).is_some());

    s.client.remove_token_escrow_fee_config(&s.admin, &s.usdc);

    assert!(s.client.get_token_escrow_fee_config(&s.usdc).is_none());
    assert_eq!(
        s.client.get_effective_escrow_fee_config(&s.usdc).fee_bps,
        100
    );
    assert_eq!(
        s.client.get_escrow(&create_escrow(&s, &s.usdc)).fee_bps,
        100
    );
}

#[test]
fn test_non_admin_cannot_set_or_remove_token_config() {
    let s = setup();
    let outsider = Address::generate(&s.env);

    assert_eq!(
        s.client.try_set_token_escrow_fee_config(
            &outsider,
            &s.usdc,
            &fee_config(250, &s.contract_id, true),
        ),
        Err(Ok(Error::Basic(BasicError::NotAnAdmin)))
    );
    assert_eq!(
        s.client
            .try_remove_token_escrow_fee_config(&outsider, &s.usdc),
        Err(Ok(Error::Basic(BasicError::NotAnAdmin)))
    );
    assert!(s.client.get_token_escrow_fee_config(&s.usdc).is_none());
}

#[test]
fn test_token_config_rejects_out_of_range_bps() {
    let s = setup();

    assert_eq!(
        s.client.try_set_token_escrow_fee_config(
            &s.admin,
            &s.usdc,
            &fee_config(10_001, &s.contract_id, true),
        ),
        Err(Ok(Error::Basic(BasicError::InvalidBps)))
    );
    assert_eq!(
        s.client.try_set_token_escrow_fee_config(
            &s.admin,
            &s.usdc,
            &fee_config(-1, &s.contract_id, true),
        ),
        Err(Ok(Error::Basic(BasicError::InvalidBps)))
    );

    // Boundary values are accepted.
    s.client.set_token_escrow_fee_config(
        &s.admin,
        &s.usdc,
        &fee_config(10_000, &s.contract_id, true),
    );
    s.client
        .set_token_escrow_fee_config(&s.admin, &s.xlm, &fee_config(0, &s.contract_id, true));
}
