#![cfg(test)]

use crate::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, token, Address, Env, String};

const AMOUNT: i128 = 1_000;

struct Setup<'a> {
    env: Env,
    client: EscrowContractClient<'a>,
    admin: Address,
    customer: Address,
    merchant: Address,
    token: Address,
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
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(&env, &token).mint(&customer, &(AMOUNT * 100));

    Setup {
        env,
        client,
        admin,
        customer,
        merchant,
        token,
    }
}

fn create_template(s: &Setup) -> u64 {
    s.client.create_template(
        &s.merchant,
        &s.token,
        &AMOUNT,
        &3600_u64,
        &String::from_str(&s.env, "retainer"),
    )
}

#[test]
fn recurring_schedule_creates_escrow_on_each_trigger() {
    let s = setup();
    let template_id = create_template(&s);

    let schedule_id =
        s.client
            .create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &3u32);

    let schedule = s
        .client
        .get_recurring_escrow_schedule(&schedule_id)
        .unwrap();
    assert_eq!(schedule.customer, s.customer);
    assert_eq!(schedule.template_id, template_id);
    assert_eq!(schedule.created_count, 0);
    assert!(schedule.active);

    for i in 1..=3u32 {
        s.env.ledger().with_mut(|l| l.timestamp += 1000);
        let escrow_id = s.client.trigger_next_recurring_escrow(&schedule_id);
        assert!(escrow_id > 0);

        let schedule = s
            .client
            .get_recurring_escrow_schedule(&schedule_id)
            .unwrap();
        assert_eq!(schedule.created_count, i);
        assert!(schedule.active);
    }

    // All 3 escrows created; a further trigger is rejected as complete.
    s.env.ledger().with_mut(|l| l.timestamp += 1000);
    let result = s.client.try_trigger_next_recurring_escrow(&schedule_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::RecurringScheduleComplete)))
    );
}

#[test]
fn trigger_before_due_fails() {
    let s = setup();
    let template_id = create_template(&s);
    let schedule_id =
        s.client
            .create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &2u32);

    let result = s.client.try_trigger_next_recurring_escrow(&schedule_id);
    assert_eq!(result, Err(Ok(Error::Action(ActionError::NotReady))));
}

#[test]
fn deactivated_template_stops_schedule_permanently() {
    let s = setup();
    let template_id = create_template(&s);
    let schedule_id =
        s.client
            .create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &3u32);

    s.env.ledger().with_mut(|l| l.timestamp += 1000);
    s.client.deactivate_template(&s.merchant, &template_id);

    let result = s.client.try_trigger_next_recurring_escrow(&schedule_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::TemplateInactive)))
    );

    // There is no way to reactivate a template, so the schedule stays stopped
    // on every later attempt too.
    s.env.ledger().with_mut(|l| l.timestamp += 1000);
    let result = s.client.try_trigger_next_recurring_escrow(&schedule_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::TemplateInactive)))
    );
}

#[test]
fn cancelled_schedule_cannot_be_triggered() {
    let s = setup();
    let template_id = create_template(&s);
    let schedule_id =
        s.client
            .create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &2u32);

    s.client
        .cancel_recurring_escrow_schedule(&s.customer, &schedule_id);

    let schedule = s
        .client
        .get_recurring_escrow_schedule(&schedule_id)
        .unwrap();
    assert!(!schedule.active);

    s.env.ledger().with_mut(|l| l.timestamp += 1000);
    let result = s.client.try_trigger_next_recurring_escrow(&schedule_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::RecurringScheduleInactive)))
    );
}

#[test]
fn reaching_total_count_stops_further_triggers() {
    let s = setup();
    let template_id = create_template(&s);
    let schedule_id =
        s.client
            .create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &1u32);

    s.env.ledger().with_mut(|l| l.timestamp += 1000);
    s.client.trigger_next_recurring_escrow(&schedule_id);

    let schedule = s
        .client
        .get_recurring_escrow_schedule(&schedule_id)
        .unwrap();
    assert!(schedule.active);
    assert_eq!(schedule.created_count, 1);

    s.env.ledger().with_mut(|l| l.timestamp += 1000);
    let result = s.client.try_trigger_next_recurring_escrow(&schedule_id);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::RecurringScheduleComplete)))
    );
}

#[test]
fn create_schedule_rejects_invalid_params() {
    let s = setup();
    let template_id = create_template(&s);

    let result =
        s.client
            .try_create_recurring_escrow_schedule(&s.customer, &template_id, &0_u64, &3u32);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(
            EscrowError::InvalidRecurringScheduleParams
        )))
    );

    let result =
        s.client
            .try_create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &0u32);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(
            EscrowError::InvalidRecurringScheduleParams
        )))
    );
}

#[test]
fn create_schedule_rejects_inactive_template() {
    let s = setup();
    let template_id = create_template(&s);
    s.client.deactivate_template(&s.merchant, &template_id);

    let result =
        s.client
            .try_create_recurring_escrow_schedule(&s.customer, &template_id, &1000_u64, &3u32);
    assert_eq!(
        result,
        Err(Ok(Error::Escrow(EscrowError::TemplateInactive)))
    );
}

#[test]
fn get_recurring_escrow_schedule_unknown_returns_none() {
    let s = setup();
    assert!(s.client.get_recurring_escrow_schedule(&999u64).is_none());
}
