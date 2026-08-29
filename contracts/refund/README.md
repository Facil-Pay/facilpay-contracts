# Refund Contract

A Soroban smart contract on Stellar for processing and managing refunds. Supports the full refund lifecycle, arbitration, policy templates, fraud detection, notification hooks, batch operations, and more.

## 📋 Prerequisites

Same as the root project — see the [main README](../../README.md) for setup instructions.

## 🚀 Quick Start

```bash
# Build the contract
make

# Run refund-specific tests
cargo test -p refund
```

## ⚠️ Error Codes

The refund contract defines a set of numeric error codes in [src/lib.rs](src/lib.rs). For a complete reference of every error variant, see [ERRORS.md](ERRORS.md).

## 🏷️ Refund Reason Codes

The `request_refund()` function requires a type-safe `RefundReasonCode` enum variant to categorize refund requests for structured querying and analytics (`get_reason_code_analytics()`).

| Variant           | Description                                                                                          | Intended Scenario                                                                         |
| ----------------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `ProductDefect`   | Item or service was defective, damaged, non-functional, or significantly different from description. | Received a broken item, malfunctioning software, or flawed goods.                         |
| `NonDelivery`     | Goods or services were not received within the expected fulfillment window or were lost in transit.  | Package lost in shipping, or service provider failed to show up.                          |
| `DuplicateCharge` | Customer was billed multiple times by accident for a single order or transaction.                    | System or network error causing duplicate payment execution.                              |
| `Unauthorized`    | Transaction was initiated without the account owner's knowledge or consent (fraudulent activity).    | Compromised account, stolen credentials, or unapproved charge.                            |
| `CustomerRequest` | Buyer requested cancellation, return, or exchange under standard buyer's remorse or return policy.   | Customer changed their mind, ordered wrong size, or no longer needed the item.            |
| `Other`           | Fallback or unclassified refund reason that does not fit standard variants, or legacy backfills.     | Custom agreements, edge cases, or initial code migrations for unknown historical reasons. |

## 📂 Public Functions

### Initialization & Schema

- `initialize()` — Initializes the contract with an admin address and default refund/appeal windows.
- `get_schema_version()` — Returns the current schema version number.
- `migrate_schema()` — Admin-only migration of the contract schema to a new version.

### Core Refund Lifecycle

- `request_refund()` — Merchant initiates a refund request with reason and reason code.
- `get_refund()` — Retrieves a refund record by its ID.
- `approve_refund()` — Admin approves a refund (moves from Requested to Approved).
- `reject_refund()` — Admin rejects a refund (moves from Requested to PendingAppeal).
- `finalize_denial()` — Finalizes a denied refund after the appeal window expires.
- `process_refund()` — Processes an approved refund, deducting platform fees.

### Appeals

- `file_appeal()` — Customer files an appeal against a rejected refund.
- `resolve_appeal()` — Admin resolves an appeal (uphold or deny).
- `get_appeal()` — Retrieves an appeal by its ID.
- `get_appeals_by_customer()` — Returns all appeals filed by a specific customer.

### Auto-Refund Triggers

- `register_auto_refund_trigger()` — Merchant registers a trigger for automatic refund on a condition.
- `evaluate_auto_refund()` — Evaluates and executes an auto-refund trigger if its condition is met.
- `get_auto_refund_trigger()` — Gets an auto-refund trigger by ID.

### Merchant Quota & Rate Limits

- `set_merchant_refund_quota()` — Admin sets a refund quota (amount limit + period) for a merchant.
- `get_merchant_refund_quota()` — Gets the refund quota configuration for a merchant.
- `reset_merchant_quota()` — Admin resets a merchant's quota usage counter.
- `set_customer_rate_limit()` — Admin sets a custom per-customer rate limit.
- `get_customer_rate_limit_status()` — Gets the rate-limit status for a customer.
- `set_global_refund_rate_limit()` — Admin sets the global refund rate limit.
- `update_rate_limit()` — Admin updates the global rate limit without disrupting in-progress windows.
- `get_global_refund_rate_limit()` — Gets the current global refund rate limit configuration.

### Arbitration

- `register_arbitrator()` — Admin registers a new arbitrator.
- `assign_arbitrator()` — Admin manually assigns an arbitrator to an open case.
- `escalate_to_arbitration()` — Customer escalates a rejected refund to arbitration.
- `cast_arbitration_vote()` — Arbitrator casts a vote on an open case.
- `close_arbitration_case()` — Closes a case once quorum is reached.
- `set_arbitration_timeout()` — Admin sets the default timeout for arbitration cases.
- `get_arbitration_timeout_config()` — Gets the arbitration timeout in seconds.
- `trigger_arbitration_timeout()` — Triggers timeout on a case that exceeded its deadline.
- `get_arbitrator_reputation()` — Gets reputation info for a specific arbitrator.
- `get_top_arbitrators()` — Returns top arbitrators sorted by score.
- `deregister_low_performers()` — Admin removes arbitrators below a minimum score.
- `get_arbitration_case()` — Retrieves an arbitration case by ID.
- `set_arbitration_fee_config()` — Admin sets arbitration fee distribution.
- `get_arbitration_fee_config()` — Gets arbitration fee configuration.
- `get_accumulated_arbitration_fees()` — Gets accumulated treasury fees from arbitration.
- `withdraw_treasury_fees()` — Admin withdraws accumulated arbitration treasury fees.
- `set_arbitration_stake_config()` — Admin sets arbitration stake configuration.
- `get_arbitration_stake_config()` — Gets arbitration stake configuration.
- `get_arbitration_stake()` — Gets stake info for a specific case.
- `add_senior_arbitrator()` — Admin adds an arbitrator to the senior list for tiered escalation.
- `set_arbitration_tier_config()` — Admin sets tiered arbitration escalation config.
- `escalate_arbitration_case()` — Escalates a case from junior to senior panel.
- `get_arbitration_tier()` — Returns Senior or Junior tier for a case.

### Policy Templates

- `create_policy_template()` — Admin creates a reusable refund policy template.
- `apply_template_to_merchant()` — Admin applies a template to a merchant.
- `get_policy_template()` — Gets a policy template by ID.
- `list_policy_templates()` — Lists all active policy templates.
- `deactivate_policy_template()` — Admin deactivates a policy template.

### Refund Policies

- `set_refund_policy()` — Merchant sets their tiered refund policy.
- `get_refund_policy()` — Gets the current active refund policy for a merchant.
- `get_refund_policy_version()` — Gets a specific versioned policy.
- `get_refund_policy_at_time()` — Gets the policy version in effect at a given timestamp.
- `get_refund_policy_history()` — Returns the full version history for a merchant's policies.
- `set_default_refund_policy()` — Admin sets the global default refund policy.
- `get_default_refund_policy()` — Gets the global default refund policy.
- `remove_default_refund_policy()` — Admin removes the global default refund policy.
- `deactivate_refund_policy()` — Merchant deactivates their own refund policy.
- `get_effective_refund_policy()` — Traverses inheritance chain to find the effective policy.
- `get_requires_admin_approval()` — Checks if merchant requires admin approval.
- `set_requires_admin_approval()` — Merchant sets whether admin approval is required.
- `get_auto_approve_below()` — Gets the auto-approval threshold amount for a merchant.
- `set_auto_approve_below()` — Merchant sets the auto-approval threshold.
- `get_inherit_from_parent()` — Checks if merchant inherits policy from parent.
- `set_inherit_from_parent()` — Merchant sets policy inheritance from parent.
- `get_applicable_refund_bps()` — Gets the max refund basis points for a merchant and payment.
- `get_policy_inheritance_chain()` — Returns the merchant's inheritance ancestry chain.
- `set_merchant_parent()` — Admin sets the parent merchant for policy inheritance.
- `get_merchant_parent()` — Gets the direct parent merchant of a given merchant.

### Query Functions

- `get_refunds_by_status()` — Paginated refunds filtered by status.
- `get_refund_count_by_status()` — Gets the count of refunds in a given status.
- `get_merchant_refunds()` — Paginated refunds for a specific merchant.
- `get_merchant_refunds_by_status()` — Paginated refunds for a merchant filtered by status.
- `get_merchant_pending_refunds()` — All pending refunds for a merchant.
- `get_merchant_refund_summary()` — Aggregate refund stats for a merchant.
- `get_refunds_by_reason_code()` — Paginated refunds filtered by canonical reason code.
- `get_reason_code_analytics(window_start, window_end)` — Counts refunds by reason code within the given ledger-timestamp window, sorted by frequency. Cached per window and invalidated only when a refund inside that window is processed.
- `get_total_refunded_amount()` — Cumulative refunded amount for a given payment.
- `can_refund_payment()` — Checks if a refund would exceed the original payment amount.

### Batch Operations

- `get_batch_refund_limit()` — Gets the max number of refunds per batch.
- `set_batch_refund_limit()` — Admin sets the batch refund limit.
- `approve_refund_batch()` — Approves multiple refunds in a single batch.
- `process_refund_batch()` — Processes multiple approved refunds in a single batch.
- `batch_reject_refunds()` — Batch rejects multiple refunds.

### Cross-Contract

- `set_payment_contract_address()` — Admin sets the payment contract address for cross-contract calls.
- `get_payment_contract_address()` — Gets the payment contract address.
- `verify_payment_ownership()` — Cross-contract call to verify customer owns the payment.

### Analytics

- `get_refund_analytics()` — Overall contract analytics (totals, approval rate, etc.).

### Pause / Circuit Breaker

- `pause_contract()` — Admin pauses the entire contract.
- `unpause_contract()` — Admin unpauses the contract.
- `pause_function()` — Admin pauses a specific function.
- `unpause_function()` — Admin unpauses a specific function.
- `get_pause_state()` — Gets the current global pause state.
- `is_function_paused()` — Checks if a specific function is paused.
- `set_circuit_breaker_config()` — Admin sets circuit breaker thresholds and cooldown.
- `get_circuit_breaker_state()` — Gets the current circuit breaker state.
- `reset_circuit_breaker()` — Admin manually resets the circuit breaker.
- `check_circuit_breaker()` — Returns true if the circuit breaker is active.

#### How the Circuit Breaker Works

The circuit breaker is an automatic kill switch that halts new refund requests
when the ratio of refunded value to paid value spikes over a short window —
protecting the contract (and merchant treasuries) against a runaway refund event,
a compromised approver, or a buggy integration.

**Configuration**

The admin installs it with `set_circuit_breaker_config(admin, config)`:

```rust
pub struct CircuitBreakerConfig {
    pub max_refund_rate_bps: u32,        // trip threshold, in basis points (e.g. 1000 = 10%)
    pub measurement_window_seconds: u64, // rolling window over which volume is summed
    pub cooldown_seconds: u64,           // how long the breaker stays tripped before auto-reset
    pub enabled: bool,                   // master on/off switch
}
```

If no config has ever been set, or `enabled` is `false`, the breaker is inert:
requests are never evaluated or blocked, and `check_circuit_breaker()` returns
`false`.

**What trips it**

The check runs inside `request_refund` (before the customer rate-limit, fraud and
policy checks). For each request the contract maintains, per rolling
`measurement_window_seconds` window, a running sum of requested refund amounts and
a running sum of the corresponding original payment amounts. When a window
elapses (or on first use) both sums reset and the window restarts at the current
ledger time.

For the incoming request it computes
`rate_bps = (window_refund_volume + refund_amount) * 10000 / (window_payment_volume + payment_amount)`.
If that value is **greater than** `max_refund_rate_bps`, the breaker trips:

- `CircuitBreakerState` is updated — `tripped = true`, `tripped_at = now`,
  `trip_count += 1`, `last_refund_rate_bps = rate_bps`,
  `resets_at = now + cooldown_seconds`;
- a `CircuitBreakerTrippedEvent { refund_rate_bps, tripped_at }` is emitted;
- the triggering `request_refund` call itself reverts with
  `CircuitBreakerTripped` (error `29`). Its amounts are **not** added to the
  window totals.

**What it blocks while tripped**

Only `request_refund`. Every new refund request reverts with
`CircuitBreakerTripped` until the breaker resets. Refunds
that were already `Requested`, `Approved`, or `Processed` before the trip are
unaffected — approvals, processing, appeals, arbitration and admin overrides do
not consult the breaker.

**How it resets**

- **Automatically** — the next `request_refund` received at or after `resets_at`
  clears the tripped state (`tripped = false`, `tripped_at`/`resets_at` cleared)
  and proceeds against a fresh measurement window. `trip_count` is retained as a
  historical counter.
- **Manually** — the admin calls `reset_circuit_breaker(admin)`, which clears the
  tripped state immediately and emits `CircuitBreakerResetEvent { reset_by,
  reset_at }`. Use this to restore service before the cooldown elapses (e.g. after
  raising `max_refund_rate_bps` or confirming the spike was legitimate).

`get_circuit_breaker_state()` returns the full state (tripped flag, trip count,
last observed rate, auto-reset timestamp); `check_circuit_breaker()` is a
read-only helper returning `true` only while tripped and still within cooldown.

### Fraud Detection

- `check_fraud_signals()` — Checks an address for fraud signals.
- `get_flagged_addresses()` — Returns all flagged addresses.
- `mark_fraud_reviewed()` — Admin marks a fraud signal as reviewed.
- `set_fraud_config()` — Admin sets fraud detection thresholds.

### Customer History

- `get_customer_refund_history()` — Paginated refund history for a customer.
- `get_customer_refund_count_public()` — Total count of refunds for a customer.
- `get_customer_refund_summary()` — Summary stats for a customer's refunds.

### Notification Hooks

- `register_notification_hook()` — Registers a notification hook for specific refund events.
- `deregister_hook()` — Deregisters a notification hook.
- `get_hooks_for_event()` — Gets all active hooks for a specific event type.
- `get_subscriber_hooks()` — Gets all hooks for a subscriber.

### Customer Eligibility

- `set_refund_eligibility()` — Merchant sets the eligibility rule for a customer.
- `check_refund_eligibility()` — Returns the eligibility rule for a merchant-customer pair.
- `remove_refund_eligibility()` — Merchant removes an eligibility entry.
- `get_merchant_eligibility_list()` — Returns all eligibility entries for a merchant.

#### Merchant Eligibility Rules

Beyond the time/percentage caps of a refund policy, a merchant can maintain an
explicit **allow / block list** that decides *which customers may open a refund
request at all* against that merchant. This is the customer-eligibility registry.

**How a rule is determined**

- Entries are keyed by the **(merchant, customer)** pair, so a rule only applies to
  that one customer under that one merchant. The same customer can be blocked by
  merchant A and unaffected under merchant B.
- `set_refund_eligibility(merchant, customer, rule, reason_hash)` creates or
  overwrites the entry. `rule` is `EligibilityRule::Allow` or
  `EligibilityRule::Block`; `reason_hash` is an opaque `BytesN<32>` for off-chain
  reason text (pass the zero hash if unused). The call requires the merchant's
  authorization. Re-calling it for an existing pair updates the rule in place
  without creating a duplicate list entry, and emits
  `EligibilitySet { merchant, customer, rule }`.
- `check_refund_eligibility(merchant, customer)` returns the stored rule and
  **defaults to `Allow` when no entry exists** — customers are eligible unless a
  merchant explicitly blocks them. There is no global block list; blocking is
  always per merchant.
- `get_merchant_eligibility_list(merchant)` returns every `RefundEligibilityEntry`
  for the merchant (`customer`, `merchant`, `rule`, `reason_hash`, `set_at`).

**What makes a customer ineligible**

Only an explicit `Block` entry. During `request_refund`, after the fraud check and
before policy validation, the contract reads the (merchant, customer) rule; if it
is `Block`, the call reverts with `CustomerBlockedFromRefund` (error `44`). An
`Allow` entry or no entry lets the request proceed to the normal policy checks.

**Removing a rule**

`remove_refund_eligibility(merchant, customer)` deletes the entry (the merchant's
list is compacted) and emits `EligibilityRemoved { merchant, customer }`. After
removal the pair falls back to the `Allow` default. Removing an entry that does
not exist reverts with `EligibilityEntryNotFound` (error `45`). To un-block a
customer you can either remove the entry or overwrite it with `Allow`.

**Effect on refunds already in flight**

The eligibility rule is consulted **only at `request_refund` time**. Blocking a
customer afterwards does not claw back or freeze a refund they already submitted —
an existing `Requested`/`Approved` refund can still be approved and processed. The
block only prevents *new* requests. Likewise, switching a customer from `Block`
back to `Allow` (or removing the entry) immediately lets them request again.

### Admin Override

- `admin_override_policy()` — Admin overrides a refund decision with audit logging.
- `get_admin_override_history()` — Gets an admin override audit log entry by ID.
- `get_admin_override_history_count()` — Gets total count of admin override audit log entries.

### Payment Category Windows

- `set_category_window()` — Admin sets a category-specific refund window for a merchant.
- `get_category_window()` — Gets the category-specific refund window.
- `tag_payment_category()` — Merchant tags a payment with a category.
- `get_effective_window()` — Gets the effective refund window for a payment.

### Arbitrator Auto-Assignment

- `configure_auto_assignment()` — Admin configures round-robin auto-assignment of arbitrators.
- `auto_assign_arbitrators()` — Automatically assigns a panel of arbitrators.
- `get_next_arbitrators()` — Previews the next arbitrators without advancing the rotation.
- `reset_rotation_index()` — Admin resets the round-robin rotation index.

### Refund TTL

- `set_refund_ttl_config()` — Admin sets the default TTL for refund requests.
- `expire_stale_refund()` — Expires a refund that exceeded its TTL.
- `get_expired_refunds()` — Gets refund IDs that have expired past TTL.

### Dispute Evidence

- `submit_refund_evidence()` — Customer or merchant submits evidence for a refund dispute.
- `get_refund_evidence()` — Gets evidence submitted by a specific party.
- `get_all_refund_evidence()` — Gets all evidence entries for a refund dispute.

### Multi-Token Support

- `register_refund_token()` — Admin registers a token as a supported refund method.
- `deregister_refund_token()` — Admin deregisters a refund token.
- `get_supported_refund_tokens()` — Gets all registered refund tokens.

### Refund Vouchers

- `issue_refund_voucher()` — Admin issues a refund credit voucher for an approved refund.
- `redeem_refund_voucher()` — Customer redeems a refund voucher against a future payment.
- `get_voucher()` — Gets a refund voucher by ID.
- `get_customer_vouchers()` — Gets all refund vouchers issued to a customer.

### Payment Refund Caps

- `set_payment_refund_cap()` — Admin sets a refund cap on a specific payment.
- `get_payment_refund_cap()` — Gets the refund cap for a specific payment.
- `get_payment_refund_usage()` — Gets current refund usage for a payment.

### Customer Tier Policies

- `set_customer_tier()` — Admin assigns a tier level to a customer.
- `get_customer_tier()` — Gets the tier level assigned to a customer.
- `set_customer_tier_policy()` — Merchant sets the refund cap for a customer tier.
- `get_customer_tier_policy()` — Gets the refund cap for a specific customer tier.
- `set_strict_tier_policy()` — Merchant enables/disables strict tier policy enforcement.
- `get_strict_tier_policy()` — Checks if strict tier policy is enabled.

#### How the Customer Tier Policy Works

Customer tiers let a merchant apply a **different maximum refund percentage** to
different classes of customer (for example a stricter cap for a "high refund risk"
tier, or a more generous cap for a loyalty tier), on top of the merchant's normal
time-based refund policy.

**How a customer's tier is determined**

- The **contract admin** assigns tiers with
  `set_customer_tier(admin, customer, tier_id)`, where `tier_id` is an arbitrary
  `u32` chosen by the operator. `get_customer_tier(customer)` returns it, or
  `None` if unassigned.
- A tier is a property of the **customer address globally** — it is not
  merchant-scoped and a customer has at most one tier id at a time. Re-calling
  `set_customer_tier` overwrites it.
- Merchants do not set tiers; they only define what each tier id *means* for their
  own refunds.

**Which policy knob varies by tier**

Only the **refund cap** — `max_refund_bps`, the maximum share of the original
payment that may be refunded. A merchant registers a per-tier cap with
`set_customer_tier_policy(merchant, tier_id, max_refund_bps)` (merchant auth;
`max_refund_bps` must be ≤ `10000` or the call reverts with `InvalidAmount`).
Stored as a `RefundCap { max_refund_bps }` keyed by `(merchant, tier_id)` and
readable via `get_customer_tier_policy`.

The **refund window** (the `days_from_purchase` tiers of the merchant's
`set_refund_policy`) is *not* tier-specific. Eligibility by age is always decided
by the merchant's time-based policy first; a tier cap only tightens or loosens the
percentage allowed once the request is inside the window.

Resolution order inside `request_refund` → `validate_against_policy`:

1. The merchant's time-based policy picks `allowed_bps` from the first tier whose
   `days_from_purchase` covers the payment's age. If none does, the refund is
   rejected with `RefundWindowExpired` (error `11`).
2. If the customer has a tier assigned **and** the merchant has a
   `set_customer_tier_policy` entry for that `tier_id`, that tier cap
   **replaces** `allowed_bps` (it can be higher or lower than the time-based
   value).
3. If the customer has a tier but the merchant has **no** matching tier policy
   entry:
   - **default (non-strict):** the tier is ignored and the time-based
     `allowed_bps` stands;
   - **strict mode** (`set_strict_tier_policy(merchant, true)`): the request is
     rejected with `TierPolicyNotFound` (error `57`).
4. Customers with **no** tier assigned are never affected by strict mode — only
   the time-based policy applies.
5. Finally, if the requested `amount / original_amount` exceeds the resolved
   `allowed_bps`, the request is rejected with `RefundExceedsPolicy` (error `12`).

**Effect of tier changes on refunds already in flight**

The customer's tier and the tier cap are read **only when `request_refund` runs**.
Reassigning a customer's tier, changing a tier's `max_refund_bps`, or toggling
strict mode afterwards does **not** re-evaluate refunds that were already
submitted — an existing `Requested`/`Approved` refund keeps the cap that applied
at request time through approval and processing. Only refund requests created
after the change use the new tier or cap.

---

## 📡 Events

The contract emits Soroban events for all state-changing operations. Off-chain integrators (Horizon subscribers, indexers) can subscribe to these events to monitor refund lifecycle, arbitration, and policy changes.

### Core Refund Lifecycle Events

| Event             | Topic Name        | Payload Fields                                                             | Fires When                                                           |
| ----------------- | ----------------- | -------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `RefundRequested` | `RefundRequested` | `refund_id`, `payment_id`, `merchant`, `customer`, `amount`, `token`       | `request_refund()` creates refund in `Requested` status              |
| `RefundApproved`  | `RefundApproved`  | `refund_id`, `payment_id`, `amount`, `approved_by`, `approved_at`          | `approve_refund()` moves refund to `Approved` status                 |
| `RefundRejected`  | `RefundRejected`  | `refund_id`, `rejected_by`, `rejected_at`, `rejection_reason`              | `reject_refund()` moves refund to `PendingAppeal` status             |
| `RefundProcessed` | `RefundProcessed` | `refund_id`, `processed_by`, `customer`, `amount`, `token`, `processed_at` | `process_refund()` executes approved refund and moves to `Processed` |

### Auto-Refund Trigger Events

| Event                 | Topic Name            | Payload Fields                       | Fires When                                                       |
| --------------------- | --------------------- | ------------------------------------ | ---------------------------------------------------------------- |
| `TriggerRegistered`   | `TriggerRegistered`   | `trigger_id`, `payment_id`           | `register_auto_refund_trigger()` creates trigger record          |
| `AutoRefundTriggered` | `AutoRefundTriggered` | `trigger_id`, `payment_id`, `amount` | `evaluate_auto_refund()` executes auto-refund when condition met |

### Appeal Events

| Event            | Topic Name       | Payload Fields                        | Fires When                                                  |
| ---------------- | ---------------- | ------------------------------------- | ----------------------------------------------------------- |
| `AppealFiled`    | `AppealFiled`    | `appeal_id`, `refund_id`, `appellant` | `file_appeal()` customer files appeal against rejection     |
| `AppealResolved` | `AppealResolved` | `appeal_id`, `upheld`, `resolved_at`  | `resolve_appeal()` admin resolves appeal (upheld or denied) |

### Arbitration Case Events

| Event                          | Topic Name                     | Payload Fields                               | Fires When                                                                        |
| ------------------------------ | ------------------------------ | -------------------------------------------- | --------------------------------------------------------------------------------- |
| `RefundEscalatedToArbitration` | `RefundEscalatedToArbitration` | `refund_id`, `case_id`, `fee_pool`           | `escalate_to_arbitration()` creates arbitration case with initial fee pool        |
| `ArbitrationVoteCast`          | `ArbitrationVoteCast`          | `case_id`, `arbitrator`, `vote_for_refund`   | `cast_arbitration_vote()` arbitrator votes on case                                |
| `ArbitrationCaseDecided`       | `ArbitrationCaseDecided`       | `case_id`, `approved`                        | `close_arbitration_case()` case closes after quorum reached with majority outcome |
| `ArbitrationTimedOut`          | `ArbitrationTimedOut`          | `case_id`, `default_outcome`, `triggered_at` | `trigger_arbitration_timeout()` case timeout expires and default outcome applied  |

### Arbitration Fee & Stake Events

| Event                        | Topic Name                   | Payload Fields                                 | Fires When                                                                 |
| ---------------------------- | ---------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------- |
| `ArbitrationFeesDistributed` | `ArbitrationFeesDistributed` | `case_id`, `per_arbitrator`, `treasury_amount` | Case closes and fee pool distributed to arbitrators and treasury           |
| `StakeDeposited`             | `StakeDeposited`             | `case_id`, `staker`, `amount`                  | `escalate_to_arbitration()` with staking enabled; escalator deposits stake |
| `StakeReturned`              | `StakeReturned`              | `case_id`, `winner`, `amount`                  | Case closes with stake returned to winning party                           |
| `StakeForfeited`             | `StakeForfeited`             | `case_id`, `loser`, `amount`                   | Case closes with stake forfeited to treasury from losing party             |

### Arbitrator Events

| Event                    | Topic Name               | Payload Fields                         | Fires When                                                                     |
| ------------------------ | ------------------------ | -------------------------------------- | ------------------------------------------------------------------------------ |
| `ArbitratorScoreUpdated` | `ArbitratorScoreUpdated` | `arbitrator`, `old_score`, `new_score` | Vote outcome recorded; arbitrator reputation score adjusted                    |
| `ArbitratorDeregistered` | `ArbitratorDeregistered` | `arbitrator`, `reason`                 | `deregister_low_performers()` removes arbitrator below minimum score threshold |

### Policy Events

| Event                                       | Topic Name | Payload Fields | Fires When                                                           |
| ------------------------------------------- | ---------- | -------------- | -------------------------------------------------------------------- |
| (Policy changes tracked via function calls) | —          | —              | Use `get_refund_policy_history()` to audit policy versions over time |

### Contract Control Events

| Event                   | Topic Name              | Payload Fields                         | Fires When                                             |
| ----------------------- | ----------------------- | -------------------------------------- | ------------------------------------------------------ |
| `ContractPausedEvent`   | `ContractPausedEvent`   | `paused_by`, `reason`, `paused_at`     | `pause_contract()` halts all state-changing operations |
| `ContractUnpausedEvent` | `ContractUnpausedEvent` | `unpaused_by`, `unpaused_at`           | `unpause_contract()` resumes operations                |
| `FunctionPausedEvent`   | `FunctionPausedEvent`   | `function_name`, `paused_by`, `reason` | `pause_function()` pauses specific function            |
| `FunctionUnpausedEvent` | `FunctionUnpausedEvent` | `function_name`, `unpaused_by`         | `unpause_function()` resumes function                  |

### Circuit Breaker Events

| Event                        | Topic Name                   | Payload Fields                           | Fires When                                                                      |
| ---------------------------- | ---------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------- |
| `CircuitBreakerTrippedEvent` | `CircuitBreakerTrippedEvent` | `triggered_by`, `reason`, `triggered_at` | Circuit breaker activates due to refund volume threshold or error rate exceeded |

---

## ⚖️ Arbitration Workflow

A refund dispute reaches arbitration after a refund has been rejected and the affected party escalates it for review. The contract creates an arbitration case only when there is a sufficient panel of registered arbitrators, and the escalator must provide an arbitration fee in the configured token. If staking is enabled, the escalator also deposits a stake, which acts as a bonding mechanism for the dispute.

Once the case is open, the registered arbitrator panel can vote on whether the refund should be approved or upheld as rejected. A case is only closed after quorum is reached, and the majority vote determines the final outcome. The fee pool collected at escalation is then distributed according to the arbitration fee configuration: a portion goes to the arbitrators who voted with the majority, and the remainder can be routed to the treasury. If a stake was posted, the escrowed amount is returned to the escalator if they ultimately win the case, or forfeited to the treasury if they lose.

If the case is not resolved before its timeout window, it falls back to the configured default outcome rather than remaining indefinitely open. That timeout path still settles the stake so the funds do not remain locked up. Arbitration reputation is tracked alongside each case as well: a vote aligned with the final outcome improves an arbitrator's score, while a minority vote lowers it, and the contract also records total cases and average resolution time.

---

## 🔒 Arbitration Stake Requirement

### Overview & Purpose

To discourage frivolous dispute escalations and ensure escalating parties have financial commitment ("skin in the game"), the refund contract supports a configurable staking/bonding requirement. When enabled by the contract admin, any party escalating a refund dispute to arbitration must deposit a designated token stake in addition to the case fee pool. The contract holds this stake in escrow for the duration of the arbitration proceedings.

### Stake Amount & Configuration

Arbitration staking is configured globally by the contract admin via `set_arbitration_stake_config(admin, config)`:

```rust
pub struct ArbitrationStakeConfig {
    pub token: Address,   // Token contract address used for staking
    pub amount: i128,      // Required stake amount per case (must be > 0 when enabled)
    pub enabled: bool,     // Toggle flag enabling or disabling the stake requirement
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `token` | `Address` | Stellar token address in which the stake must be denominated. |
| `amount` | `i128` | Required stake amount per case. Must be strictly positive (`> 0`) when `enabled` is `true`. |
| `enabled` | `bool` | Enables (`true`) or disables (`false`) the stake deposit requirement on arbitration escalation. |

#### Configuration Rules & Behavior

- **Validation**: When `enabled: true`, setting `amount <= 0` will revert with `CoreError::InvalidAmount`.
- **Admin Authorization**: Only the contract admin can set or update the stake configuration via `set_arbitration_stake_config()`.
- **Disabled/Unconfigured Staking**: If staking is not configured or `enabled: false`, disputes can be escalated without transferring any stake, and no stake record is stored (`get_arbitration_stake(case_id)` returns `None`).
- **Querying Config**: The current configuration can be retrieved via `get_arbitration_stake_config()`.

### Stake Lifecycle

```
                  ┌──────────────────────────────┐
                  │   escalate_to_arbitration    │
                  │   (Staking enabled by admin) │
                  └──────────────┬───────────────┘
                                 │
                     [StakeDeposited Event]
                     [Held in Contract Escrow]
                                 │
                  ┌──────────────┴───────────────┐
                  ▼                              ▼
        [close_arbitration_case]      [trigger_arbitration_timeout]
        (Quorum reached & decided)    (Timeout deadline exceeded)
                  │                              │
         ┌────────┴────────┐            ┌────────┴────────┐
         ▼                 ▼            ▼                 ▼
    Staker Won        Staker Lost  Staker Won        Staker Lost
   (!approved)        (approved)   (!default)        (default)
         │                 │            │                 │
         ▼                 ▼            ▼                 ▼
   [StakeReturned]  [StakeForfeited][StakeReturned]  [StakeForfeited]
   (Transferred to  (Transferred to (Transferred to  (Transferred to
       staker)          treasury)       staker)          treasury)
```

### Deposit Conditions

When `escalate_to_arbitration(caller, refund_id, token, fee_pool)` is executed:
1. The contract checks if `ArbitrationStakeConfig` is present in instance storage and `enabled == true`.
2. If enabled, the contract transfers `config.amount` of `config.token` from the caller (`staker`) into the contract's escrow address.
3. An `ArbitrationStake` record is created and stored under `ArbitrationKey::ArbitrationStake(case_id)`:
   - `case_id`: Unique identifier for the arbitration case.
   - `staker`: Address of the party who deposited the stake.
   - `amount`: Quantity of tokens held in escrow.
   - `deposited_at`: Ledger timestamp at deposit.
   - `returned`: Set to `false`.
4. A `StakeDeposited` event (`case_id`, `staker`, `amount`) is emitted.

### Return Conditions

The full escrowed stake amount is returned to the original `staker` under the following conditions:

1. **Dispute Decided in Staker's Favor**: When the case is closed via `close_arbitration_case()` and the arbitrator panel's majority vote supports the staker's position (e.g. `!approved` when the merchant escalated to uphold a refund rejection):
   - The contract transfers `stake.amount` of `config.token` from escrow back to `stake.staker`.
   - Emits a `StakeReturned { case_id, winner: stake.staker, amount }` event.
   - Updates `stake.returned` to `true`.
2. **Timeout Decided in Staker's Favor**: When an overdue dispute is settled via `trigger_arbitration_timeout()` and the default outcome results in the staker winning (i.e. `default_outcome == false`):
   - The contract transfers `stake.amount` back to `stake.staker`.
   - Emits `StakeReturned { case_id, winner: stake.staker, amount }`.
   - Updates `stake.returned` to `true`.
3. **Missing Treasury Fallback**: If forfeiture conditions are met but no `treasury_address` is configured in `ArbitrationFeeConfig`, the stake falls back to being returned to the `staker` to avoid permanently locked funds.

### Forfeiture Conditions

The deposited stake is forfeited and transferred to the protocol treasury under the following conditions:

1. **Dispute Decided Against Staker**: When the case is closed via `close_arbitration_case()` and the majority vote goes against the staker (e.g. `approved == true`, overturning the merchant's rejection and ordering a refund):
   - The contract transfers `stake.amount` of `config.token` to the configured `treasury_address` (from `ArbitrationFeeConfig`).
   - Emits a `StakeForfeited { case_id, loser: stake.staker, amount }` event.
   - Updates `stake.returned` to `true` (indicating the stake is settled).
2. **Timeout Decided Against Staker**: When `trigger_arbitration_timeout()` executes for an unresolved case and the default outcome goes against the staker (i.e. `default_outcome == true`):
   - The contract transfers `stake.amount` to `treasury_address`.
   - Emits `StakeForfeited { case_id, loser: stake.staker, amount }`.
   - Updates `stake.returned` to `true`.

### Stake Queries & Data Structures

- `get_arbitration_stake(case_id)` — Returns the `Option<ArbitrationStake>` for a given arbitration case:

```rust
pub struct ArbitrationStake {
    pub case_id: u64,         // Arbitration case ID
    pub staker: Address,       // Address of the depositing party
    pub amount: i128,          // Amount deposited
    pub deposited_at: u64,     // Timestamp when stake was locked
    pub returned: bool,        // True if returned to staker or forfeited to treasury
}
```

- `get_arbitration_stake_config()` — Returns the active `Option<ArbitrationStakeConfig>`.

## 🔗 Links

- [Root README](../../README.md)
