#![cfg(test)]
//! Issue #839: oracle conditions and conditional escrows.

use crate::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Bytes, BytesN, Env, Symbol, TryFromVal};

const AMOUNT: i128 = 10_000;
const NOW: u64 = 10_000;
const TARGET_PRICE: i128 = 100;
const STALENESS: u64 = 60;

/// Price oracle stub exposing the `get_price(feed_id)` interface the escrow calls.
#[contract]
pub struct PriceOracleStub;

#[contractimpl]
impl PriceOracleStub {
    pub fn set_price(env: Env, data: OraclePriceData) {
        env.storage().instance().set(&0u32, &data);
    }

    pub fn get_price(env: Env, _feed_id: BytesN<32>) -> OraclePriceData {
        env.storage().instance().get(&0u32).expect("price not set")
    }
}

/// On-chain state stub exposing the `get_state(key)` interface the escrow calls.
#[contract]
pub struct StateSourceStub;

#[contractimpl]
impl StateSourceStub {
    pub fn set_state(env: Env, key: BytesN<32>, value: Bytes) {
        env.storage().instance().set(&key, &value);
    }

    pub fn get_state(env: Env, key: BytesN<32>) -> Bytes {
        env.storage()
            .instance()
            .get(&key)
            .unwrap_or(Bytes::new(&env))
    }
}

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
    env.ledger().with_mut(|l| l.timestamp = NOW);

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

fn balance(s: &Setup, who: &Address) -> i128 {
    token::Client::new(&s.env, &s.token).balance(who)
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

// ── Oracle conditions ────────────────────────────────────────────────────────

fn feed_id(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[7u8; 32])
}

fn create_escrow(s: &Setup) -> u64 {
    s.client.create_escrow(
        &s.customer,
        &s.merchant,
        &AMOUNT,
        &s.token,
        &0_u64,
        &0_u64,
        &0_u64,
        &false,
    )
}

fn register_oracle(s: &Setup, price: i128, timestamp: u64) -> Address {
    let oracle = s.env.register(PriceOracleStub, ());
    PriceOracleStubClient::new(&s.env, &oracle).set_price(&OraclePriceData { price, timestamp });
    oracle
}

/// "Release to the merchant if the price is above TARGET_PRICE."
fn oracle_condition(s: &Setup, escrow_id: u64, oracle: &Address) -> OracleCondition {
    OracleCondition {
        escrow_id,
        oracle: OracleConfig {
            oracle_address: oracle.clone(),
            price_feed_id: feed_id(&s.env),
            staleness_threshold: STALENESS,
        },
        target_price: TARGET_PRICE,
        comparison: PriceComparison::GreaterThan,
        release_to_merchant_if_met: true,
    }
}

/// Creates an escrow, attaches an oracle condition reading `price`, and disputes it.
fn disputed_escrow_with_oracle(s: &Setup, price: i128, price_timestamp: u64) -> u64 {
    let escrow_id = create_escrow(s);
    let oracle = register_oracle(s, price, price_timestamp);
    s.client.attach_oracle_condition(
        &s.admin,
        &escrow_id,
        &oracle_condition(s, escrow_id, &oracle),
    );
    s.client.dispute_escrow(&s.customer, &escrow_id);
    escrow_id
}

#[test]
fn test_attach_and_get_oracle_condition() {
    let s = setup();
    let escrow_id = create_escrow(&s);
    let oracle = register_oracle(&s, TARGET_PRICE, NOW);

    assert_eq!(
        s.client.try_get_oracle_condition(&escrow_id).err(),
        Some(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );

    s.client.attach_oracle_condition(
        &s.admin,
        &escrow_id,
        &oracle_condition(&s, escrow_id, &oracle),
    );

    let stored = s.client.get_oracle_condition(&escrow_id);
    assert_eq!(stored.escrow_id, escrow_id);
    assert_eq!(stored.oracle.oracle_address, oracle);
    assert_eq!(stored.oracle.price_feed_id, feed_id(&s.env));
    assert_eq!(stored.oracle.staleness_threshold, STALENESS);
    assert_eq!(stored.target_price, TARGET_PRICE);
    assert_eq!(stored.comparison, PriceComparison::GreaterThan);
    assert!(stored.release_to_merchant_if_met);
}

#[test]
fn test_attach_oracle_condition_validation() {
    let s = setup();
    let escrow_id = create_escrow(&s);
    let oracle = register_oracle(&s, TARGET_PRICE, NOW);
    let condition = oracle_condition(&s, escrow_id, &oracle);

    let stranger = Address::generate(&s.env);
    assert_eq!(
        s.client
            .try_attach_oracle_condition(&stranger, &escrow_id, &condition),
        Err(Ok(Error::Basic(BasicError::NotAnAdmin)))
    );
    assert_eq!(
        s.client
            .try_attach_oracle_condition(&s.admin, &999, &condition),
        Err(Ok(Error::Escrow(EscrowError::NotFound)))
    );

    s.client.dispute_escrow(&s.customer, &escrow_id);
    assert_eq!(
        s.client
            .try_attach_oracle_condition(&s.admin, &escrow_id, &condition),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
    assert!(s.client.try_get_oracle_condition(&escrow_id).is_err());
}

#[test]
fn test_oracle_condition_met_releases_to_merchant() {
    let s = setup();
    let escrow_id = disputed_escrow_with_oracle(&s, TARGET_PRICE + 50, NOW);
    let customer_before = balance(&s, &s.customer);

    s.client.auto_resolve_with_oracle(&escrow_id);

    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Released
    );
    assert_eq!(balance(&s, &s.merchant), AMOUNT);
    assert_eq!(balance(&s, &s.customer), customer_before);
    assert_eq!(balance(&s, &s.contract_id), 0);
}

#[test]
fn test_oracle_condition_not_met_refunds_customer() {
    let s = setup();
    // Exactly at the target is not "GreaterThan".
    let escrow_id = disputed_escrow_with_oracle(&s, TARGET_PRICE, NOW);
    let customer_before = balance(&s, &s.customer);

    s.client.auto_resolve_with_oracle(&escrow_id);

    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Resolved
    );
    assert_eq!(balance(&s, &s.customer), customer_before + AMOUNT);
    assert_eq!(balance(&s, &s.merchant), 0);
    assert_eq!(balance(&s, &s.contract_id), 0);
}

#[test]
fn test_auto_resolve_rejects_stale_price() {
    let s = setup();
    let escrow_id = disputed_escrow_with_oracle(&s, TARGET_PRICE + 50, NOW - STALENESS - 1);

    assert_eq!(
        s.client.try_auto_resolve_with_oracle(&escrow_id),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Disputed
    );
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);
    assert_eq!(balance(&s, &s.merchant), 0);
}

#[test]
fn test_auto_resolve_requires_disputed_escrow_with_condition() {
    let s = setup();
    assert_eq!(
        s.client.try_auto_resolve_with_oracle(&999),
        Err(Ok(Error::Escrow(EscrowError::NotFound)))
    );

    // Locked (not disputed) escrow with a condition attached.
    let escrow_id = create_escrow(&s);
    let oracle = register_oracle(&s, TARGET_PRICE + 50, NOW);
    s.client.attach_oracle_condition(
        &s.admin,
        &escrow_id,
        &oracle_condition(&s, escrow_id, &oracle),
    );
    assert_eq!(
        s.client.try_auto_resolve_with_oracle(&escrow_id),
        Err(Ok(Error::Action(ActionError::NotDisputed)))
    );

    // Disputed escrow with no condition attached.
    let bare_id = create_escrow(&s);
    s.client.dispute_escrow(&s.customer, &bare_id);
    assert_eq!(
        s.client.try_auto_resolve_with_oracle(&bare_id),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
    assert_eq!(balance(&s, &s.contract_id), AMOUNT * 2);
}

// ── Conditional escrows ──────────────────────────────────────────────────────

fn state_key(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[3u8; 32])
}

fn expected_value(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"shipped")
}

/// Registers a state source whose `state_key` currently holds `value`.
fn register_state_source(s: &Setup, value: &Bytes) -> Address {
    let source = s.env.register(StateSourceStub, ());
    StateSourceStubClient::new(&s.env, &source).set_state(&state_key(&s.env), value);
    source
}

fn create_conditional(s: &Setup, source: &Address) -> u64 {
    let escrow_id = s.client.create_conditional_escrow(
        &s.customer,
        &s.merchant,
        &s.token,
        &AMOUNT,
        &OnChainCondition {
            contract_address: source.clone(),
            state_key: state_key(&s.env),
            expected_value: expected_value(&s.env),
        },
    );
    // `create_conditional_escrow` records the escrow but does not move the
    // customer's tokens, so fund the contract to back the payout.
    token::StellarAssetClient::new(&s.env, &s.token).mint(&s.contract_id, &AMOUNT);
    escrow_id
}

#[test]
fn test_create_and_get_conditional_escrow() {
    let s = setup();
    let source = register_state_source(&s, &Bytes::new(&s.env));
    let escrow_id = create_conditional(&s, &source);

    let conditional = s.client.get_conditional_escrow(&escrow_id);
    assert_eq!(conditional.escrow_id, escrow_id);
    assert!(!conditional.evaluated);
    assert!(!conditional.result);
    assert_eq!(conditional.condition.contract_address, source);
    assert_eq!(conditional.condition.state_key, state_key(&s.env));
    assert_eq!(conditional.condition.expected_value, expected_value(&s.env));

    let escrow = s.client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.amount, AMOUNT);
    assert_eq!(escrow.customer, s.customer);
    assert_eq!(escrow.merchant, s.merchant);
    assert_eq!(escrow.release_timestamp, u64::MAX);

    assert_eq!(
        s.client.try_get_conditional_escrow(&999).err(),
        Some(Ok(Error::Escrow(EscrowError::ConditionalEscrowNotFound)))
    );
}

#[test]
fn test_evaluate_unmet_condition_does_not_release() {
    let s = setup();
    let source = register_state_source(&s, &Bytes::from_slice(&s.env, b"pending"));
    let escrow_id = create_conditional(&s, &source);

    assert!(!s.client.evaluate_and_release(&escrow_id));

    assert!(has_event(&s.env, "condition_evaluated"));
    assert!(!has_event(&s.env, "conditional_release_executed"));
    assert_eq!(s.client.get_escrow(&escrow_id).status, EscrowStatus::Locked);
    assert_eq!(balance(&s, &s.merchant), 0);
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);

    // The (negative) evaluation is recorded and cannot be re-run.
    let conditional = s.client.get_conditional_escrow(&escrow_id);
    assert!(conditional.evaluated);
    assert!(!conditional.result);
    assert_eq!(
        s.client.try_evaluate_and_release(&escrow_id),
        Err(Ok(Error::Action(ActionError::ConditionAlreadyEvaluated)))
    );
    assert_eq!(balance(&s, &s.merchant), 0);
}

#[test]
fn test_evaluate_met_condition_releases_to_merchant() {
    let s = setup();
    let source = register_state_source(&s, &expected_value(&s.env));
    let escrow_id = create_conditional(&s, &source);

    assert!(s.client.evaluate_and_release(&escrow_id));

    assert!(has_event(&s.env, "conditional_release_executed"));
    assert_eq!(
        s.client.get_escrow(&escrow_id).status,
        EscrowStatus::Released
    );
    assert_eq!(balance(&s, &s.merchant), AMOUNT);
    assert_eq!(balance(&s, &s.contract_id), 0);

    let conditional = s.client.get_conditional_escrow(&escrow_id);
    assert!(conditional.evaluated);
    assert!(conditional.result);
    assert_eq!(
        s.client.try_evaluate_and_release(&escrow_id),
        Err(Ok(Error::Action(ActionError::ConditionAlreadyEvaluated)))
    );
    assert_eq!(balance(&s, &s.merchant), AMOUNT);
}

#[test]
fn test_evaluate_met_condition_deducts_fee() {
    let s = setup();
    s.client.set_escrow_fee_config(
        &s.admin,
        &EscrowFeeConfig {
            fee_bps: 250,
            fee_recipient: s.contract_id.clone(),
            enabled: true,
            max_fee: None,
        },
    );
    let source = register_state_source(&s, &expected_value(&s.env));
    let escrow_id = create_conditional(&s, &source);
    assert_eq!(s.client.get_escrow(&escrow_id).fee_bps, 250);

    assert!(s.client.evaluate_and_release(&escrow_id));

    let fee = AMOUNT * 250 / 10_000;
    assert_eq!(balance(&s, &s.merchant), AMOUNT - fee);
    assert_eq!(balance(&s, &s.contract_id), fee);
    assert_eq!(s.client.get_accumulated_escrow_fees(&s.token), fee);
}

#[test]
fn test_evaluate_rejects_unknown_and_non_locked_escrows() {
    let s = setup();
    assert_eq!(
        s.client.try_evaluate_and_release(&999),
        Err(Ok(Error::Escrow(EscrowError::ConditionalEscrowNotFound)))
    );

    let source = register_state_source(&s, &expected_value(&s.env));
    let escrow_id = create_conditional(&s, &source);
    s.client.dispute_escrow(&s.customer, &escrow_id);

    assert_eq!(
        s.client.try_evaluate_and_release(&escrow_id),
        Err(Ok(Error::Escrow(EscrowError::InvalidStatus)))
    );
    assert!(!s.client.get_conditional_escrow(&escrow_id).evaluated);
    assert_eq!(balance(&s, &s.merchant), 0);
    assert_eq!(balance(&s, &s.contract_id), AMOUNT);
}
