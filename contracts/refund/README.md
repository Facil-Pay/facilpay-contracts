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

Each merchant maintains its **own** allow/block list of customers for refunds. This is the
first gate a refund request passes through and is completely independent of the tiered
refund policy, the refund window, quotas, and fraud checks — it is a hard yes/no on whether
this merchant will entertain a refund from this customer at all.

**What makes a customer eligible or ineligible**

- Eligibility is stored per `(merchant, customer)` pair as an `EligibilityRule`, which is
  either `Allow` or `Block`.
- `set_refund_eligibility(merchant, customer, rule, reason_hash)` is authorized by the
  **merchant** (`require_auth` on the `merchant` argument). `reason_hash` is a
  `BytesN<32>` for an off-chain audit note (e.g. a hash of the reason for a block); pass
  the zero hash when there is nothing to record. Calling it again for the same pair
  overwrites the previous rule in place — it does not create a second list entry.
- `check_refund_eligibility(merchant, customer)` returns the current rule. When **no entry
  exists** the pair defaults to `Allow`, so customers are eligible unless a merchant has
  explicitly blocked them.
- Eligibility is strictly merchant-scoped: blocking a customer under merchant A has no
  effect on that customer's refunds from merchant B.
- `remove_refund_eligibility(merchant, customer)` deletes the entry (reverting the pair to
  the default `Allow`) and returns `EligibilityEntryNotFound` if there was nothing to
  remove. `get_merchant_eligibility_list(merchant)` returns every stored
  `RefundEligibilityEntry` for the merchant, each carrying `customer`, `merchant`, `rule`,
  `reason_hash`, and `set_at`.
- An admin can effectively override a merchant's block by calling `set_refund_eligibility`
  for the pair with `Allow` (tests exercise this via `mock_all_auths`); there is no
  separate admin-only eligibility entry.

**Enforcement point**

The check runs inside `request_refund`, after fraud screening and before the policy
validation. If the effective rule is `Block`, the request fails immediately with
`CustomerBlockedFromRefund` (`ExtError::CustomerBlockedFromRefund`) and no refund record is
created. `Allow` (or no entry) lets the request continue to policy evaluation.

**What happens to refunds already in flight if a customer is later blocked**

Eligibility is evaluated **only at `request_refund` time**. Blocking a customer afterwards
does **not** claw back or freeze refunds that were already created — `process_refund` does
not re-check eligibility, so any `Pending` or `Approved` refund continues through its normal
lifecycle. A block only prevents that customer from opening **new** refund requests with
that merchant; unblock (via `Allow` or `remove_refund_eligibility`) restores their ability
to request again.

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

Customer tier policies let a merchant cap refunds differently for different classes of
customer (for example: `1` = platinum, `2` = standard, `3` = bronze). The tier only changes
**the maximum refundable percentage** — it never changes the refund *window*, approval
routing, quotas, fees, or any other policy knob. Everything else still comes from the
merchant's base `set_refund_policy` tiers (or the inherited / default policy).

**How a customer's tier is determined**

- A tier is a single `u32` assigned to a customer address by the **contract admin** via
  `set_customer_tier(admin, customer, tier_id)`. Non-admin callers get `Unauthorized`.
- The assignment is **global to the contract**, not per-merchant: `get_customer_tier`
  takes only the customer address. There is no automatic tiering from spend or history —
  the admin sets it explicitly, and it stays until the admin changes it.
- A customer with no assignment has **no tier**; `get_customer_tier` returns `None`.

**Which knob varies by tier**

- `set_customer_tier_policy(merchant, tier_id, max_refund_bps)` is authorized by the
  **merchant** and stores a `RefundCap { max_refund_bps }` (0–10000 bps; values above
  10000 are rejected with `InvalidAmount`) for that `(merchant, tier_id)` pair.
- During `request_refund` the contract first resolves the base allowed percentage from the
  merchant policy's day-based tiers, then — if the customer has a tier assigned — **replaces**
  that allowed percentage with the tier's `max_refund_bps`. The requested
  `amount / original_amount` is then checked against the resulting cap; exceeding it returns
  `RefundExceedsPolicy`.
- The day-based refund window still applies first. If the payment is already outside every
  policy tier's `days_from_purchase`, the request fails with `RefundWindowExpired` before
  the tier cap is even consulted.

**Missing tier policy — fallback vs. strict mode**

- If the customer has a tier but the merchant has **no** `set_customer_tier_policy` entry
  for that tier, the default behaviour is to **fall back** to the base merchant policy cap
  (the refund proceeds as if no tier were assigned).
- `set_strict_tier_policy(merchant, true)` changes this: a missing tier policy entry then
  causes `request_refund` to fail with `TierPolicyNotFound` instead of falling back.
  `get_strict_tier_policy` reports the current setting (default `false`).

**Effect on refunds already in flight**

Tier and tier-policy values are read **only at `request_refund` time** and the resulting
cap is baked into the created `Refund` as its fixed `amount`. Re-tiering a customer,
editing a `set_customer_tier_policy` cap, or toggling strict mode afterwards has **no
effect** on refunds that are already `Pending`, `Approved`, or `Processed` — `process_refund`
does not re-evaluate tier policy. The new values apply only to refund requests created
after the change.

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

### Eligibility & Tier Policy Events

| Event                | Topic Name           | Payload Fields                 | Fires When                                                                              |
| -------------------- | -------------------- | ----------------------------- | -------------------------------------------------------------------------------------- |
| `EligibilitySet`     | `EligibilitySet`     | `merchant`, `customer`, `rule` | `set_refund_eligibility()` stores or overwrites an allow/block rule for a customer      |
| `EligibilityRemoved` | `EligibilityRemoved` | `merchant`, `customer`         | `remove_refund_eligibility()` deletes a customer's eligibility entry                    |

Customer tier changes (`set_customer_tier`, `set_customer_tier_policy`, `set_strict_tier_policy`) do **not** emit events — query the current values with the corresponding `get_*` functions.

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
