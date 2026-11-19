#![cfg(test)]
//! Issue #838: insurance pool and watchdog auto-release.

use crate::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{token, Address, Env, Symbol, TryFromVal};

const AMOUNT: i128 = 10_000;
const START: u64 = 1_000;
const RELEASE_AT: u64 = START + 500;
const INACTIVITY: u64 = 86_400;

struct Setup<'a> {
    env: Env,
    client: EscrowContractClient<'a>,
    contract_id: Address,
    admin: Address,
    customer: Address,
    merchant: Address,
    token: Address,
}

fn setup<'a>() -> Setup<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = START);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&customer, &(AMOUNT * 10));

    Setup {
        env,
        client,
        contract_id,
        admin,
        customer,
        merchant,
        token,
    }
}

fn create_escrow(s: &Setup, amount: i128) -> u64 {
    s.client.create_escrow(
        &s.customer,
        &s.merchant,
        &amount,
        &s.token,
        &RELEASE_AT,
        &0_u64,
        &0_u64,
        &false,
    )
}

fn balance(s: &Setup, who: &Address) -> i128 {
    token::Client::new(&s.env, &s.token).balance(who)
}

fn set_time(s: &Setup, timestamp: u64) {
    s.env.ledger().with_mut(|l| l.timestamp = timestamp);
}

fn has_event(env: &Env, name: &str) -> bool {
    let expected = Symbol::new(env, name);
    env.events().all().iter().any(|(_, topics, _)| {
        topics
            .get(0)
            .and_then(|t| Symbol::try_from_val(env, &t).ok())
            .is_some_and(|s| s == expected)
    })
}

fn insurance_config(premium_bps: i128, max_coverage_bps: i128, enabled: bool) -> InsuranceConfig {
    InsuranceConfig {
        premium_bps,
        max_coverage_bps,
        enabled,
    }
}

fn watchdog_config(enabled: bool, favor_customer_on_release: bool) -> WatchdogConfig {
    WatchdogConfig {
        inactivity_release_seconds: INACTIVITY,
        enabled,
        favor_customer_on_release,
        keeper_reward_bps: 0,
    }
}

/// Disputes `escrow_id` and resolves it for the customer, leaving it `Resolved`.
fn resolve_for_customer(s: &Setup, escrow_id: u64) {
    s.client.dispute_escrow(&s.customer, &escrow_id);
    s.client.resolve_dispute(&s.admin, &escrow_id, &false);
}

// ── Insurance pool ───────────────────────────────────────────────────────────

#[test]
fn test_premium_funds_pool_and_approved_claim_pays_customer() {
    let s = setup();
    // 5% premium, claims capped at 50% of the (post-premium) escrow amount.
    s.client
        .set_insurance_config(&s.admin, &insurance_config(500, 5_000, true));

    let escrow_id = create_escrow(&s, AMOUNT);
    let customer_start = balance(&s, &s.customer);
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);

    s.client.opt_into_insurance(&escrow_id);

    // The premium is carved out of the escrow into the token's pool; the tokens
    // stay in the contract's custody.
    let premium = AMOUNT * 500 / 10_000;
    assert_eq!(s.client.get_escrow(&escrow_id).amount, AMOUNT - premium);
    let pool = s.client.get_insurance_pool(&s.token);
    assert_eq!(pool.token, s.token);
    assert_eq!(pool.balance, premium);
    assert_eq!(pool.total_premiums_collected, premium);
    assert_eq!(pool.total_claims_paid, 0);
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);

    // Resolving pays out only the insured (post-premium) amount; the premium
    // stays behind as the pool.
    resolve_for_customer(&s, escrow_id);
    assert_eq!(balance(&s, &s.customer), customer_start + AMOUNT - premium);
    assert_eq!(balance(&s, &s.contract_id), premium);

    let claim_amount = 400;
    let claim_id = s
        .client
        .file_insurance_claim(&s.admin, &escrow_id, &claim_amount);
    assert_eq!(claim_id, 1);
    // Filing alone moves nothing.
    assert_eq!(s.client.get_insurance_pool(&s.token).balance, premium);
    assert_eq!(balance(&s, &s.contract_id), premium);

    s.client.approve_claim(&s.admin, &claim_id);

    assert_eq!(
        balance(&s, &s.customer),
        customer_start + AMOUNT - premium + claim_amount
    );
    assert_eq!(balance(&s, &s.contract_id), premium - claim_amount);
    let pool = s.client.get_insurance_pool(&s.token);
    assert_eq!(pool.balance, premium - claim_amount);
    assert_eq!(pool.total_premiums_collected, premium);
    assert_eq!(pool.total_claims_paid, claim_amount);

    // A claim is paid at most once.
    assert_eq!(
        s.client.try_approve_claim(&s.admin, &claim_id),
        Err(Ok(Error::Escrow(EscrowError::AlreadyProcessed)))
    );
    assert_eq!(balance(&s, &s.contract_id), premium - claim_amount);
}

#[test]
fn test_premiums_accumulate_and_claim_ids_increment() {
    let s = setup();
    s.client
        .set_insurance_config(&s.admin, &insurance_config(1_000, 10_000, true));

    let first = create_escrow(&s, AMOUNT);
    let second = create_escrow(&s, AMOUNT * 2);
    s.client.opt_into_insurance(&first);
    s.client.opt_into_insurance(&second);

    let pool = s.client.get_insurance_pool(&s.token);
    assert_eq!(pool.balance, 1_000 + 2_000);
    assert_eq!(pool.total_premiums_collected, 3_000);

    resolve_for_customer(&s, first);
    resolve_for_customer(&s, second);
    assert_eq!(s.client.file_insurance_claim(&s.admin, &first, &100), 1);
    assert_eq!(s.client.file_insurance_claim(&s.admin, &second, &200), 2);

    s.client.approve_claim(&s.admin, &2);
    let pool = s.client.get_insurance_pool(&s.token);
    assert_eq!(pool.balance, 3_000 - 200);
    assert_eq!(pool.total_claims_paid, 200);
}

#[test]
fn test_premium_rounding_to_zero_leaves_escrow_untouched() {
    let s = setup();
    s.client
        .set_insurance_config(&s.admin, &insurance_config(500, 10_000, true));

    // 10 * 500 / 10_000 rounds down to 0.
    let escrow_id = create_escrow(&s, 10);
    s.client.opt_into_insurance(&escrow_id);

    assert_eq!(s.client.get_escrow(&escrow_id).amount, 10);
    let pool = s.client.get_insurance_pool(&s.token);
    assert_eq!(pool.balance, 0);
    assert_eq!(pool.total_premiums_collected, 0);
}

#[test]
fn test_unknown_pool_defaults_to_empty() {
    let s = setup();
    let other_token = Address::generate(&s.env);
    let pool = s.client.get_insurance_pool(&other_token);
    assert_eq!(pool.token, other_token);
    assert_eq!(pool.balance, 0);
    assert_eq!(pool.total_premiums_collected, 0);
    assert_eq!(pool.total_claims_paid, 0);
}

#[test]
fn test_insurance_config_is_admin_only() {
    let s = setup();
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_set_insurance_config(&stranger, &insurance_config(500, 5_000, true)),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );

    // Nothing was stored, so opting in is still refused.
    let escrow_id = create_escrow(&s, AMOUNT);
    assert_eq!(
        s.client.try_opt_into_insurance(&escrow_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
}

#[test]
fn test_opt_in_rejected_when_disabled_or_not_locked() {
    let s = setup();
    let escrow_id = create_escrow(&s, AMOUNT);

    s.client
        .set_insurance_config(&s.admin, &insurance_config(500, 5_000, false));
    assert_eq!(
        s.client.try_opt_into_insurance(&escrow_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );

    s.client
        .set_insurance_config(&s.admin, &insurance_config(500, 5_000, true));
    resolve_for_customer(&s, escrow_id);
    assert_eq!(
        s.client.try_opt_into_insurance(&escrow_id),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
    assert_eq!(s.client.get_insurance_pool(&s.token).balance, 0);
}

#[test]
fn test_file_claim_validation() {
    let s = setup();
    s.client
        .set_insurance_config(&s.admin, &insurance_config(500, 5_000, true));
    let escrow_id = create_escrow(&s, AMOUNT);
    s.client.opt_into_insurance(&escrow_id);

    // Claims can only be filed once the escrow has been resolved or cancelled.
    assert_eq!(
        s.client
            .try_file_insurance_claim(&s.admin, &escrow_id, &100),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );

    resolve_for_customer(&s, escrow_id);

    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_file_insurance_claim(&stranger, &escrow_id, &100),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );

    // Max coverage is 50% of the post-premium amount (9_500) = 4_750.
    assert_eq!(
        s.client
            .try_file_insurance_claim(&s.admin, &escrow_id, &4_751),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert_eq!(
        s.client.file_insurance_claim(&s.admin, &escrow_id, &4_750),
        1
    );
}

#[test]
fn test_approve_claim_validation() {
    let s = setup();
    s.client
        .set_insurance_config(&s.admin, &insurance_config(500, 5_000, true));
    let escrow_id = create_escrow(&s, AMOUNT);
    s.client.opt_into_insurance(&escrow_id);
    resolve_for_customer(&s, escrow_id);

    assert_eq!(
        s.client.try_approve_claim(&s.admin, &99),
        Err(Ok(Error::Escrow(EscrowError::NotFound)))
    );

    // Within coverage (4_750) but larger than the pool (500).
    let claim_id = s.client.file_insurance_claim(&s.admin, &escrow_id, &1_000);
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client.try_approve_claim(&stranger, &claim_id),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    let customer_before = balance(&s, &s.customer);
    assert_eq!(
        s.client.try_approve_claim(&s.admin, &claim_id),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
    assert_eq!(balance(&s, &s.customer), customer_before);
    let pool = s.client.get_insurance_pool(&s.token);
    assert_eq!(pool.balance, 500);
    assert_eq!(pool.total_claims_paid, 0);
}

// ── Watchdog auto-release ────────────────────────────────────────────────────

#[test]
fn test_watchdog_ineligible_before_window_and_releases_to_merchant_after() {
    let s = setup();
    s.client
        .set_watchdog_config(&s.admin, &watchdog_config(true, false));
    assert_eq!(
        s.client.get_watchdog_config().inactivity_release_seconds,
        INACTIVITY
    );
    let escrow_id = create_escrow(&s, AMOUNT);

    for t in [START, RELEASE_AT, RELEASE_AT + INACTIVITY - 1] {
        set_time(&s, t);
        assert!(!s.client.is_watchdog_eligible(&escrow_id));
        assert_eq!(
            s.client
                .try_trigger_watchdog_release(&Address::generate(&s.env), &escrow_id),
            Err(Ok(Error::Action(ActionError::NotReady)))
        );
    }
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);
    assert_eq!(balance(&s, &s.merchant), 0);

    set_time(&s, RELEASE_AT + INACTIVITY);
    assert!(s.client.is_watchdog_eligible(&escrow_id));
    s.client
        .trigger_watchdog_release(&Address::generate(&s.env), &escrow_id);

    assert!(has_event(&s.env, "watchdog_release_triggered"));
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Released
    );
    assert_eq!(balance(&s, &s.merchant), AMOUNT);
    assert_eq!(balance(&s, &s.contract_id), 0);

    // Once released it is no longer eligible, and cannot be released twice.
    assert!(!s.client.is_watchdog_eligible(&escrow_id));
    assert_eq!(
        s.client
            .try_trigger_watchdog_release(&Address::generate(&s.env), &escrow_id),
        Err(Ok(Error::Action(ActionError::NotReady)))
    );
    assert_eq!(balance(&s, &s.merchant), AMOUNT);
}

#[test]
fn test_watchdog_can_favor_customer() {
    let s = setup();
    s.client
        .set_watchdog_config(&s.admin, &watchdog_config(true, true));
    let escrow_id = create_escrow(&s, AMOUNT);
    let customer_before = balance(&s, &s.customer);

    set_time(&s, RELEASE_AT + INACTIVITY);
    s.client
        .trigger_watchdog_release(&Address::generate(&s.env), &escrow_id);

    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Resolved
    );
    assert_eq!(balance(&s, &s.customer), customer_before + AMOUNT);
    assert_eq!(balance(&s, &s.merchant), 0);
    assert_eq!(balance(&s, &s.contract_id), 0);
}

#[test]
fn test_watchdog_disabled_by_default_and_when_turned_off() {
    let s = setup();
    let escrow_id = create_escrow(&s, AMOUNT);
    set_time(&s, RELEASE_AT + INACTIVITY * 10);

    assert!(!s.client.get_watchdog_config().enabled);
    assert!(!s.client.is_watchdog_eligible(&escrow_id));

    s.client
        .set_watchdog_config(&s.admin, &watchdog_config(false, false));
    assert!(!s.client.is_watchdog_eligible(&escrow_id));
    assert_eq!(
        s.client
            .try_trigger_watchdog_release(&Address::generate(&s.env), &escrow_id),
        Err(Ok(Error::Action(ActionError::NotReady)))
    );
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);
}

#[test]
fn test_watchdog_skips_missing_and_disputed_escrows() {
    let s = setup();
    s.client
        .set_watchdog_config(&s.admin, &watchdog_config(true, false));
    let escrow_id = create_escrow(&s, AMOUNT);
    s.client.dispute_escrow(&s.customer, &escrow_id);

    set_time(&s, RELEASE_AT + INACTIVITY);
    assert!(!s.client.is_watchdog_eligible(&escrow_id));
    assert!(!s.client.is_watchdog_eligible(&999));
    assert_eq!(
        s.client
            .try_trigger_watchdog_release(&Address::generate(&s.env), &escrow_id),
        Err(Ok(Error::Action(ActionError::NotReady)))
    );
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);
}

#[test]
fn test_watchdog_config_is_admin_only() {
    let s = setup();
    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_set_watchdog_config(&stranger, &watchdog_config(true, false)),
        Err(Ok(Error::Basic(BasicError::Unauthorized)))
    );
    assert!(!s.client.get_watchdog_config().enabled);
}
