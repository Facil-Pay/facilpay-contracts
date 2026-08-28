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

## 🔔 Notification Hooks

Notification hooks let an external contract (an off-chain listener's on-chain relay, another Facil-Pay contract, etc.) subscribe to refund lifecycle events and receive a call when they occur.

### Triggering Events

A hook is registered against one or more `RefundEventType` variants. Each variant is fired from exactly one place in the contract:

| Event       | Fired By                                                                                     |
| ----------- | ---------------------------------------------------------------------------------------------- |
| `Requested` | `request_refund()`, right after the refund is created in `Requested` status                    |
| `Approved`  | `approve_refund()`, when the refund moves to `Approved` status                                  |
| `Rejected`  | `finalize_denial()`, when a `PendingAppeal` refund's appeal window expires and it becomes `Rejected` — **not** `reject_refund()` itself, which only moves the refund from `Requested` to `PendingAppeal` |
| `Processed` | `process_refund()`, when an approved refund is processed and fees are deducted                  |
| `Escalated` | `escalate_to_arbitration()`, when a rejected refund is escalated to an arbitration case          |

### Hook Interface

A subscriber is any deployed contract that implements two entry points:

```
ping() -> ()
on_refund_event(event_type: RefundEventType, refund_id: u64) -> ()
```

- `ping()` takes no arguments and returns nothing. It exists purely so registration can verify the address is a reachable contract; it does not need any real implementation beyond existing.
- `on_refund_event()` is the actual notification callback, invoked with the event type and the affected refund's ID.

### Registering and Deregistering

```
register_notification_hook(subscriber, events) -> hook_id
deregister_hook(subscriber, hook_id)
```

- The `subscriber` address itself must authorize the registration call (`subscriber.require_auth()`) — a third party cannot register a hook on a contract's behalf
- `events` must be non-empty
- Registration calls `ping()` on `subscriber` to confirm it is a reachable contract; if the call fails or `subscriber` doesn't implement `ping()`, registration is rejected with `InvalidHookAddress`
- Each event type accepts at most 10 active hooks (`MaxHooksPerEventReached` once the limit is hit)
- Only the original `subscriber` can deregister its own hook (`HookNotOwnedBySubscriber` otherwise); deregistering marks the hook inactive rather than deleting it

### Failure Handling

Hook invocation is isolated from the triggering operation: the contract calls `on_refund_event()` via `try_invoke_contract`, so a panicking or reverting subscriber cannot block or roll back the refund action that triggered it. If a hook call fails, the contract emits a `HookInvocationFailed` event (`hook_id`, `subscriber`, `event_type`, `refund_id`) and continues — the refund operation itself still succeeds.

### Queries

| Function                          | Returns                                              |
| ---------------------------------- | ----------------------------------------------------- |
| `get_hooks_for_event(event_type)` | Active hooks registered for a given event type        |
| `get_subscriber_hooks(subscriber)` | All hooks (active and inactive) owned by a subscriber |

---

## 🔗 Links

- [Root README](../../README.md)
