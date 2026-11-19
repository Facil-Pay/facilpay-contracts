#![cfg(test)]

use crate::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{token, Address, Env, String, Symbol, TryFromVal, Vec};

const CLIFF: u64 = 1_000;
const END: u64 = 5_000;
const START: u64 = 500;
const TOTAL: i128 = 10_000;

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
    env.ledger().set_timestamp(START);
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();

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

fn set_time(s: &Setup, timestamp: u64) {
    s.env.ledger().with_mut(|l| l.timestamp = timestamp);
}

fn balance(s: &Setup, who: &Address) -> i128 {
    token::Client::new(&s.env, &s.token).balance(who)
}

// `create_vesting_escrow` does not pull funds from the customer (unlike
// `create_escrow`, which does) — fund the contract directly so
// `release_vested_amount`'s internal transfer has a balance to draw from.
fn fund_contract(s: &Setup, amount: i128) {
    token::StellarAssetClient::new(&s.env, &s.token).mint(&s.contract_id, &amount);
}

fn no_milestones(env: &Env) -> Vec<VestingMilestone> {
    Vec::new(env)
}

fn milestone(
    env: &Env,
    id: u64,
    unlock_timestamp: u64,
    amount: i128,
    description: &str,
) -> VestingMilestone {
    VestingMilestone {
        milestone_id: id,
        unlock_timestamp,
        amount,
        released: false,
        description: String::from_str(env, description),
        approved_by: None,
        approved_at: None,
        deadline: None,
        claimed_missed: false,
    }
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

#[test]
fn cliff_then_linear_then_full_vesting() {
    let s = setup();
    let escrow_id = s.client.create_vesting_escrow(
        &s.customer,
        &s.merchant,
        &TOTAL,
        &s.token,
        &CLIFF,
        &END,
        &no_milestones(&s.env),
    );

    // Before the cliff: nothing vested, cliff not passed.
    set_time(&s, CLIFF - 1);
    assert_eq!(s.client.get_vested_amount(&escrow_id), 0);
    let cliff_status = s.client.get_cliff_status(&escrow_id);
    assert!(!cliff_status.cliff_passed);
    assert_eq!(cliff_status.seconds_remaining, 1);

    // Exactly at the cliff: still 0 vested (linear vesting starts accruing
    // from the cliff, elapsed == 0), but the cliff itself has passed.
    set_time(&s, CLIFF);
    let cliff_status = s.client.get_cliff_status(&escrow_id);
    assert!(cliff_status.cliff_passed);
    assert_eq!(cliff_status.seconds_remaining, 0);
    assert_eq!(s.client.get_vested_amount(&escrow_id), 0);

    // Halfway between cliff and end: proportional linear vesting.
    let halfway = CLIFF + (END - CLIFF) / 2;
    set_time(&s, halfway);
    let vested = s.client.get_vested_amount(&escrow_id);
    assert!(vested > 0 && vested < TOTAL);
    let expected = (halfway - CLIFF) as i128 * TOTAL / (END - CLIFF) as i128;
    assert_eq!(vested, expected);

    // At/after the end: everything is vested.
    set_time(&s, END);
    assert_eq!(s.client.get_vested_amount(&escrow_id), TOTAL);
    set_time(&s, END + 10_000);
    assert_eq!(s.client.get_vested_amount(&escrow_id), TOTAL);
}

#[test]
fn release_vested_amount_transfers_exact_releasable_and_rejects_empty_release() {
    let s = setup();
    fund_contract(&s, TOTAL);
    let escrow_id = s.client.create_vesting_escrow(
        &s.customer,
        &s.merchant,
        &TOTAL,
        &s.token,
        &CLIFF,
        &END,
        &no_milestones(&s.env),
    );

    // Before the cliff, release fails.
    set_time(&s, CLIFF - 1);
    let result = s.client.try_release_vested_amount(&s.admin, &escrow_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::CliffPeriodNotPassed)))
    );

    // Halfway: release exactly the releasable (= vested, since nothing
    // released yet) amount, and the merchant's real token balance moves by
    // exactly that much.
    let halfway = CLIFF + (END - CLIFF) / 2;
    set_time(&s, halfway);
    let releasable_before = s.client.get_releasable_amount(&escrow_id);
    assert!(releasable_before > 0);
    let merchant_before = balance(&s, &s.merchant);

    let released = s.client.release_vested_amount(&s.admin, &escrow_id);
    assert!(has_event(&s.env, "vested_amount_released"));
    assert_eq!(released, releasable_before);
    assert_eq!(
        balance(&s, &s.merchant),
        merchant_before + releasable_before
    );
    assert_eq!(s.client.get_releasable_amount(&escrow_id), 0);

    // Calling again immediately (nothing new vested) fails.
    let result = s.client.try_release_vested_amount(&s.admin, &escrow_id);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::InvalidStatus))));

    // After the end, the remaining amount becomes releasable and transfers.
    set_time(&s, END);
    let remaining = s.client.get_releasable_amount(&escrow_id);
    assert_eq!(remaining, TOTAL - releasable_before);
    let merchant_before = balance(&s, &s.merchant);
    let released = s.client.release_vested_amount(&s.admin, &escrow_id);
    assert_eq!(released, remaining);
    assert_eq!(balance(&s, &s.merchant), merchant_before + remaining);
}

#[test]
fn milestone_vesting_and_release_marks_milestone_released() {
    let s = setup();
    fund_contract(&s, TOTAL);
    let milestones = Vec::from_array(
        &s.env,
        [
            milestone(&s.env, 1, CLIFF + 500, 4_000, "m1"),
            milestone(&s.env, 2, CLIFF + 1_500, 6_000, "m2"),
        ],
    );
    let escrow_id = s.client.create_vesting_escrow(
        &s.customer,
        &s.merchant,
        &TOTAL,
        &s.token,
        &CLIFF,
        &END,
        &milestones,
    );

    // Before milestone 1 unlocks: nothing vested even though the cliff passed.
    set_time(&s, CLIFF + 100);
    assert_eq!(s.client.get_vested_amount(&escrow_id), 0);

    // Milestone 1 unlocks: only its amount is vested (step function, not linear).
    set_time(&s, CLIFF + 500);
    assert_eq!(s.client.get_vested_amount(&escrow_id), 4_000);

    let merchant_before = balance(&s, &s.merchant);
    let released = s.client.release_vested_amount(&s.admin, &escrow_id);
    assert!(has_event(&s.env, "milestone_released"));
    assert_eq!(released, 4_000);
    assert_eq!(balance(&s, &s.merchant), merchant_before + 4_000);

    let schedule = s.client.get_vesting_schedule(&escrow_id);
    let m1 = schedule.milestones.get(0).unwrap();
    assert!(m1.released);
    let m2 = schedule.milestones.get(1).unwrap();
    assert!(!m2.released);

    // Milestone 2 unlocks: its amount becomes additionally vested/releasable.
    set_time(&s, CLIFF + 1_500);
    assert_eq!(s.client.get_vested_amount(&escrow_id), TOTAL);
    assert_eq!(s.client.get_releasable_amount(&escrow_id), 6_000);
    let merchant_before = balance(&s, &s.merchant);
    let released = s.client.release_vested_amount(&s.admin, &escrow_id);
    assert_eq!(released, 6_000);
    assert_eq!(balance(&s, &s.merchant), merchant_before + 6_000);

    let schedule = s.client.get_vesting_schedule(&escrow_id);
    assert!(schedule.milestones.get(1).unwrap().released);
}

#[test]
fn acceleration_increases_vested_amount_as_configured() {
    let s = setup();
    let escrow_id = s.client.create_vesting_escrow(
        &s.customer,
        &s.merchant,
        &TOTAL,
        &s.token,
        &CLIFF,
        &END,
        &no_milestones(&s.env),
    );

    // Before the cliff: base vesting is 0, so acceleration is the only
    // component, making its effect clearly attributable.
    set_time(&s, CLIFF - 1);
    assert_eq!(s.client.get_vested_amount(&escrow_id), 0);
    assert_eq!(s.client.calculate_accelerated_amount(&escrow_id), 0);

    // Non-admin cannot configure acceleration.
    let non_admin = Address::generate(&s.env);
    let result = s
        .client
        .try_set_vesting_acceleration_config(&non_admin, &escrow_id, &1_000u32, &3_000u32);
    assert_eq!(result, Err(Ok(Error::Basic(BasicError::NotAnAdmin))));

    // No vesting schedule for this id -> NotFound.
    let missing_id = escrow_id + 999;
    let result =
        s.client
            .try_set_vesting_acceleration_config(&s.admin, &missing_id, &1_000u32, &3_000u32);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::NotFound))));

    s.client
        .set_vesting_acceleration_config(&s.admin, &escrow_id, &1_000u32, &3_000u32);
    assert_eq!(s.client.calculate_accelerated_amount(&escrow_id), 0);

    // One "tick": 1000bps of the remaining unvested (= TOTAL, since base is 0).
    s.client.mark_milestone_complete(&s.admin, &escrow_id);
    let expected_accel_1 = TOTAL * 1_000 / 10_000;
    assert_eq!(
        s.client.calculate_accelerated_amount(&escrow_id),
        expected_accel_1
    );
    assert_eq!(s.client.get_vested_amount(&escrow_id), expected_accel_1);

    // Second tick: total_accelerated_bps now 2000bps.
    s.client.mark_milestone_complete(&s.admin, &escrow_id);
    let expected_accel_2 = TOTAL * 2_000 / 10_000;
    assert_eq!(
        s.client.calculate_accelerated_amount(&escrow_id),
        expected_accel_2
    );

    // Third tick reaches the max (3000bps) exactly.
    s.client.mark_milestone_complete(&s.admin, &escrow_id);
    let expected_accel_3 = TOTAL * 3_000 / 10_000;
    assert_eq!(
        s.client.calculate_accelerated_amount(&escrow_id),
        expected_accel_3
    );

    // A further tick would exceed max_acceleration_bps.
    let result = s.client.try_mark_milestone_complete(&s.admin, &escrow_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::MilestoneAlreadyReleased)))
    );
}

#[test]
fn mark_milestone_complete_requires_acceleration_config_first() {
    let s = setup();
    let escrow_id = s.client.create_vesting_escrow(
        &s.customer,
        &s.merchant,
        &TOTAL,
        &s.token,
        &CLIFF,
        &END,
        &no_milestones(&s.env),
    );

    let result = s.client.try_mark_milestone_complete(&s.admin, &escrow_id);
    assert_eq!(result, Err(Ok(Error::Escrow(EscrowError::NotFound))));
}

#[test]
fn getters_handle_unknown_escrow_without_panicking() {
    let s = setup();
    let unknown_id = 999u64;

    assert_eq!(s.client.get_releasable_amount(&unknown_id), 0);
    assert_eq!(s.client.get_vested_amount(&unknown_id), 0);

    let schedule_result = s.client.try_get_vesting_schedule(&unknown_id);
    match schedule_result {
        Err(Ok(Error::Escrow(EscrowError::NotFound))) => {}
        other => panic!("expected NotFound, got {:?}", other.is_err()),
    }

    let cliff_result = s.client.try_get_cliff_status(&unknown_id);
    match cliff_result {
        Err(Ok(Error::Escrow(EscrowError::NotFound))) => {}
        other => panic!("expected NotFound, got {:?}", other.is_err()),
    }
}
