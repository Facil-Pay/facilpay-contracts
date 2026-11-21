#![cfg(test)]

//! Tests for #831 — installment schedules, overdue checks and finalization:
//! `set_installment_schedule`, `get_installment_schedule`, `check_installment_overdue`,
//! `get_installment_history`, `get_outstanding_balance`, `finalize_installment_payment`.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _},
    token, vec, Address, Env, Event, String,
};

struct Fixture {
    env: Env,
    client: PaymentContractClient<'static>,
    contract_id: Address,
    customer: Address,
    merchant: Address,
    token: Address,
    token_client: token::Client<'static>,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    let token_client = token::Client::new(&env, &token);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);

    token_admin_client.mint(&customer, &10_000i128);
    token_client.approve(&customer, &contract_id, &10_000i128, &10_000);

    Fixture {
        env,
        client,
        contract_id,
        customer,
        merchant,
        token,
        token_client,
    }
}

fn create_pending_payment(f: &Fixture, amount: i128) -> u64 {
    f.client.create_payment(
        &f.customer,
        &f.merchant,
        &amount,
        &f.token,
        &Currency::USDC,
        &0u64,
        &String::from_str(&f.env, ""),
    )
}

fn assert_last_event(f: &Fixture, event: &impl Event) {
    let all = f.env.events().all();
    assert_eq!(
        all.slice(all.len() - 1..),
        vec![
            &f.env,
            (
                f.contract_id.clone(),
                event.topics(&f.env),
                event.data(&f.env)
            )
        ]
    );
}

/// Pay each of a 3-entry schedule in order, asserting `get_installment_history` and
/// `get_outstanding_balance` after every installment, and that the payment auto-finalizes
/// (via `finalize_installment_payment`'s shared logic) once the balance hits zero.
#[test]
fn installment_schedule_history_and_balance_track_each_payment() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 300i128);

    let entries = vec![
        &f.env,
        InstallmentEntry {
            due_at: 1_000,
            amount: 100i128,
            paid: false,
        },
        InstallmentEntry {
            due_at: 2_000,
            amount: 100i128,
            paid: false,
        },
        InstallmentEntry {
            due_at: 3_000,
            amount: 100i128,
            paid: false,
        },
    ];

    f.client
        .set_installment_schedule(&f.merchant, &payment_id, &entries, &0i128, &0u32);

    let schedule = f.client.get_installment_schedule(&payment_id).unwrap();
    assert_eq!(schedule.payment_id, payment_id);
    assert_eq!(schedule.entries.len(), 3);
    assert_eq!(schedule.late_fee_flat, 0);
    assert_eq!(schedule.late_fee_bps, 0);

    // No payments made yet.
    assert_eq!(f.client.get_outstanding_balance(&payment_id), 300i128);
    assert_eq!(f.client.get_installment_history(&payment_id).len(), 0);

    // First installment.
    f.client.pay_installment(&f.customer, &payment_id, &100i128);
    let history = f.client.get_installment_history(&payment_id);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().installment_number, 1);
    assert_eq!(history.get(0).unwrap().amount_paid, 100i128);
    assert_eq!(history.get(0).unwrap().remaining, 200i128);
    assert_eq!(f.client.get_outstanding_balance(&payment_id), 200i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Pending
    );

    // Second installment.
    f.client.pay_installment(&f.customer, &payment_id, &100i128);
    let history = f.client.get_installment_history(&payment_id);
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(1).unwrap().installment_number, 2);
    assert_eq!(history.get(1).unwrap().amount_paid, 100i128);
    assert_eq!(history.get(1).unwrap().remaining, 100i128);
    assert_eq!(f.client.get_outstanding_balance(&payment_id), 100i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Pending
    );

    // Third and final installment - balance hits zero, payment auto-finalizes.
    f.client.pay_installment(&f.customer, &payment_id, &100i128);
    let history = f.client.get_installment_history(&payment_id);
    assert_eq!(history.len(), 3);
    assert_eq!(history.get(2).unwrap().installment_number, 3);
    assert_eq!(history.get(2).unwrap().amount_paid, 100i128);
    assert_eq!(history.get(2).unwrap().remaining, 0i128);
    assert_eq!(f.client.get_outstanding_balance(&payment_id), 0i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Completed
    );
    assert_eq!(f.token_client.balance(&f.merchant), 300i128);
}

/// `check_installment_overdue` is strictly `now > due_at`: at exactly `due_at` the
/// installment is not yet overdue.
#[test]
fn check_installment_overdue_boundary_at_exact_due_at() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 100i128);

    let due_at = 1_000u64;
    let entries = vec![
        &f.env,
        InstallmentEntry {
            due_at,
            amount: 100i128,
            paid: false,
        },
    ];
    f.client
        .set_installment_schedule(&f.merchant, &payment_id, &entries, &10i128, &0u32);

    f.env.ledger().with_mut(|l| l.timestamp = due_at);
    assert_eq!(
        f.client.check_installment_overdue(&payment_id),
        0,
        "exactly at due_at must not count as overdue"
    );

    f.env.ledger().with_mut(|l| l.timestamp = due_at + 1);
    assert_eq!(
        f.client.check_installment_overdue(&payment_id),
        1,
        "one second past due_at must count as overdue"
    );
}

/// When `late_fee_bps` is zero, the overdue late fee reported is the flat fee.
#[test]
fn check_installment_overdue_applies_flat_late_fee() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 100i128);

    let due_at = 1_000u64;
    let entries = vec![
        &f.env,
        InstallmentEntry {
            due_at,
            amount: 100i128,
            paid: false,
        },
    ];
    f.client
        .set_installment_schedule(&f.merchant, &payment_id, &entries, &25i128, &0u32);

    f.env.ledger().with_mut(|l| l.timestamp = due_at + 50);
    let overdue_count = f.client.check_installment_overdue(&payment_id);
    assert_eq!(overdue_count, 1);

    assert_last_event(
        &f,
        &InstallmentOverdue {
            payment_id,
            installment_index: 0,
            due_at,
            amount: 100i128,
            late_fee: 25i128,
            checked_at: due_at + 50,
        },
    );
}

/// When `late_fee_bps` is non-zero it takes priority over the flat fee, and is computed
/// as `amount * late_fee_bps / 10_000`.
#[test]
fn check_installment_overdue_applies_bps_late_fee() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 200i128);

    let due_at = 1_000u64;
    let entries = vec![
        &f.env,
        InstallmentEntry {
            due_at,
            amount: 200i128,
            paid: false,
        },
    ];
    // late_fee_flat is set but must be ignored since late_fee_bps > 0.
    f.client
        .set_installment_schedule(&f.merchant, &payment_id, &entries, &999i128, &500u32);

    f.env.ledger().with_mut(|l| l.timestamp = due_at + 1);
    let overdue_count = f.client.check_installment_overdue(&payment_id);
    assert_eq!(overdue_count, 1);

    // 200 * 500 / 10_000 = 10
    assert_last_event(
        &f,
        &InstallmentOverdue {
            payment_id,
            installment_index: 0,
            due_at,
            amount: 200i128,
            late_fee: 10i128,
            checked_at: due_at + 1,
        },
    );
}

/// `finalize_installment_payment` fails while a balance remains, and succeeds once the
/// payment has been fully paid through installments.
#[test]
fn finalize_installment_payment_fails_with_balance_then_succeeds_when_paid() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 100i128);

    let entries = vec![
        &f.env,
        InstallmentEntry {
            due_at: 1_000,
            amount: 100i128,
            paid: false,
        },
    ];
    f.client
        .set_installment_schedule(&f.merchant, &payment_id, &entries, &0i128, &0u32);

    // Balance still outstanding - direct finalize call must fail.
    let result = f.client.try_finalize_installment_payment(&payment_id);
    assert_eq!(result, Err(Ok(Error::Payment(PaymentError::InvalidStatus))));

    // A fresh, never-scheduled payment with no installments also has its full amount
    // outstanding, so finalize must fail the same way before any payment is made.
    let untouched_payment_id = create_pending_payment(&f, 50i128);
    let result = f
        .client
        .try_finalize_installment_payment(&untouched_payment_id);
    assert_eq!(result, Err(Ok(Error::Payment(PaymentError::InvalidStatus))));

    // Pay the installment in full; `pay_installment` auto-finalizes once the
    // outstanding balance reaches zero, exercising the same finalization path that
    // `finalize_installment_payment` runs directly.
    f.client.pay_installment(&f.customer, &payment_id, &100i128);
    assert_eq!(f.client.get_outstanding_balance(&payment_id), 0i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Completed
    );

    // Calling finalize again on an already-completed payment fails: it is no longer
    // Pending.
    let result = f.client.try_finalize_installment_payment(&payment_id);
    assert_eq!(result, Err(Ok(Error::Payment(PaymentError::InvalidStatus))));
}

/// `finalize_installment_payment` succeeds when called directly while the payment is
/// still `Pending` and the outstanding balance is exactly zero. The contract is funded
/// directly (bypassing `pay_installment`, which would auto-finalize) so the `Pending` /
/// zero-balance state can be observed and `finalize_installment_payment` itself - not
/// `pay_installment`'s internal call - drives the transition to `Completed`.
#[test]
fn finalize_installment_payment_succeeds_directly_when_balance_is_zero() {
    let f = setup();
    let payment_id = create_pending_payment(&f, 100i128);

    f.token_client
        .transfer(&f.customer, &f.contract_id, &100i128);
    f.env.as_contract(&f.contract_id, || {
        f.env.store().set(
            &DataKey::Payment(PaymentKey::OutstandingBalance(payment_id)),
            &0i128,
        );
    });

    assert_eq!(f.client.get_outstanding_balance(&payment_id), 0i128);
    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Pending
    );

    f.client.finalize_installment_payment(&payment_id);

    assert_eq!(
        f.client.get_payment(&payment_id).status,
        PaymentStatus::Completed
    );
    assert_eq!(f.token_client.balance(&f.merchant), 100i128);

    // Already Completed now - a second direct call fails.
    let result = f.client.try_finalize_installment_payment(&payment_id);
    assert_eq!(result, Err(Ok(Error::Payment(PaymentError::InvalidStatus))));
}
