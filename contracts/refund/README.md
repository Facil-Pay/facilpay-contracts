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
- `get_schema_version()` — Returns the current schema version number. New deployments start at `2` (v2 added the `Withdrawn` refund status); deployments that predate schema tracking report `1`.
- `migrate_schema()` — Admin-only migration of the contract schema to a new version.
- `propose_admin()` / `accept_admin()` / `get_pending_admin()` — Two-step admin rotation. See [Admin Rotation](#admin-rotation).
- `upgrade()` — Admin-only in-place WASM upgrade. See [Contract Upgrades](#contract-upgrades).

### Admin Rotation

The refund admin is rotated in two steps so that a mistyped or unreachable address can never take over (or lock up) admin control. The current admin proposes a successor, and the rotation only completes when that successor accepts.

| Function | Parameters | Returns | Must authorize |
| --- | --- | --- | --- |
| `propose_admin` | `admin: Address`, `new_admin: Address` | `Result<(), Error>` | `admin` (must be the current admin) |
| `accept_admin` | `new_admin: Address` | `Result<(), Error>` | `new_admin` (must be the pending admin) |
| `get_pending_admin` | — | `Option<Address>` | — |

**Flow**

1. The current admin calls `propose_admin(admin, new_admin)`. `new_admin` is stored as the pending admin and `AdminRotationProposed` is emitted. The current admin keeps full control; nothing else changes yet.
2. The proposed address calls `accept_admin(new_admin)`. It becomes the admin, the pending slot is cleared and `AdminRotationAccepted` is emitted. From this point the previous admin can no longer call admin-only functions.

**Overriding a proposal.** There is only one pending slot. Calling `propose_admin` again replaces the pending admin: the earlier proposed address can no longer accept (it gets `NotPendingAdmin`), and only the latest proposal can. To withdraw a proposal, propose the current admin's own address instead; accepting it is then a no-op rotation.

> ⚠️ **Verify the pending admin before accepting.** Before the new admin signs `accept_admin`, check `get_pending_admin()` returns exactly the address you expect. After acceptance the new admin has sole control of every admin-only function, including `upgrade()`, and the previous admin cannot undo the rotation.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::Unauthorized` | `3` | `propose_admin`: caller is not the current admin (or the contract is not initialized). `accept_admin`: no admin is stored |
| `ExtError::NoPendingAdmin` | `59` | `accept_admin`: no rotation has been proposed |
| `ExtError::NotPendingAdmin` | `60` | `accept_admin`: caller is not the pending admin |

Emits `AdminRotationProposed` from `propose_admin` and `AdminRotationAccepted` from `accept_admin`.

**Example (Stellar CLI)**

```bash
# 1. Current admin proposes the new admin
stellar contract invoke \
  --id <REFUND_CONTRACT_ID> \
  --source current-admin \
  --network testnet \
  -- propose_admin \
  --admin <CURRENT_ADMIN_ADDRESS> \
  --new_admin <NEW_ADMIN_ADDRESS>

# 2. Confirm the pending admin is the address you expect
stellar contract invoke \
  --id <REFUND_CONTRACT_ID> \
  --source new-admin \
  --network testnet \
  -- get_pending_admin

# 3. The new admin accepts
stellar contract invoke \
  --id <REFUND_CONTRACT_ID> \
  --source new-admin \
  --network testnet \
  -- accept_admin \
  --new_admin <NEW_ADMIN_ADDRESS>
```

### Contract Upgrades

The refund contract can replace its own code in place, so bug fixes do not require a new contract address or a state migration to a new deployment. Merchants, customers and the admin coordinator keep using the same contract ID.

| Function | Parameters | Returns | Must authorize |
| --- | --- | --- | --- |
| `upgrade` | `admin: Address`, `new_wasm_hash: BytesN<32>` | `Result<(), Error>` | `admin` (must be the current admin — the same check as `propose_admin`) |

`upgrade` calls `update_current_contract_wasm(new_wasm_hash)`. The contract address and all stored data are kept; the new code runs from the next invocation. The WASM must already be uploaded to the network, otherwise the call fails with a host storage error. A proposed-but-not-accepted admin cannot upgrade.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::Unauthorized` | `3` | Caller is not the current admin (or the contract is not initialized) |

Emits `ContractUpgraded` (`old_schema_version`, `new_wasm_hash`, `upgraded_by`), where `old_schema_version` is `get_schema_version()` at the time of the upgrade.

**Upgrade + migration sequence**

1. Build the new version and upload it: `stellar contract upload --wasm refund.wasm --source admin --network testnet` (prints the WASM hash).
2. Call `upgrade(admin, <wasm hash>)`. Keep the storage layout compatible: add new keys or fields rather than re-interpreting existing ones.
3. If the new code changes stored data shapes, call `migrate_schema(admin, <new version>)` — now running the **new** code — to perform the migration and bump the schema version (see [Storage Schema Versioning](../../docs/STORAGE_VERSIONING.md)).
4. Check `get_schema_version()` returns the new version.

The refund contract has no multi-step storage migration window, so there is no "migration in progress" state that would block an upgrade; `migrate_schema` completes in a single call.

```bash
stellar contract invoke \
  --id <REFUND_CONTRACT_ID> \
  --source admin \
  --network testnet \
  -- upgrade \
  --admin <ADMIN_ADDRESS> \
  --new_wasm_hash <WASM_HASH>
```

### Core Refund Lifecycle

- `request_refund()` — Merchant initiates a refund request with reason and reason code.
- `amend_refund_request()` — Merchant corrects the amount, reason or reason code of a request still in `Requested` status (see [Refund Request Amendments](#refund-request-amendments)).
- `get_refund_amendment_count()` — Number of times a refund request has been amended.
- `get_refund()` — Retrieves a refund record by its ID.
- `approve_refund()` — Admin approves a refund (moves from Requested to Approved).
- `reject_refund()` — Admin rejects a refund (moves from Requested to PendingAppeal).
- `finalize_denial()` — Finalizes a denied refund after the appeal window expires.
- `process_refund()` — Processes an approved refund, deducting platform fees.
- `withdraw_refund_request()` — Customer withdraws a refund still in `Requested` status. See [Withdrawing a Refund Request](#withdrawing-a-refund-request).
- `counter_offer()` — Merchant offers a smaller refund than requested. See [Counter-Offers](#counter-offers).
- `accept_counter_offer()` — Customer accepts the merchant's counter-offer, approving the refund at the offered amount.
- `reject_counter_offer()` — Customer rejects the merchant's counter-offer; the refund goes back to review.
- `reject_counter_offer_escalate()` — Customer rejects the counter-offer and escalates the refund to arbitration in one call.
- `get_counter_offer()` — Gets the pending counter-offer for a refund, if any.

### Withdrawing a Refund Request

A customer who filed a refund by mistake can withdraw it while it is still `Requested`.

| Function | Parameters | Returns |
| --- | --- | --- |
| `withdraw_refund_request` | `customer: Address` (must authorize), `refund_id: u64` | `Result<(), Error>` |

On success the refund moves to the terminal `Withdrawn` status, and several side effects follow:

- The request's per-payment refund-cap usage (count and amount) is released.
- Any pending counter-offer is discarded, and so is any response-SLA deadline.
- Withdrawn refunds are not counted as pending in `get_merchant_refund_summary`.

Customer rate-limit and cooldown counters are **not** released. Otherwise a customer could file and withdraw requests to get around those limits.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::RefundNotFound` | `2` | No refund with this ID |
| `CoreError::Unauthorized` | `3` | Caller is not the refund's customer |
| `CoreError::InvalidStatus` | `7` | Refund is not `Requested` (e.g. already approved, processed, rejected or withdrawn) |
| `CoreError::FunctionPaused` / `ContractPaused` | `18` / `17` | The contract or `withdraw_refund_request` is paused |

Emits `RefundWithdrawn`.

### Counter-Offers

Merchants can counter a refund request with a partial amount instead of only approving or rejecting it.

| Function | Parameters | Returns |
| --- | --- | --- |
| `counter_offer` | `merchant: Address` (must authorize), `refund_id: u64`, `amount: i128` | `Result<(), Error>` |
| `accept_counter_offer` | `customer: Address` (must authorize), `refund_id: u64` | `Result<(), Error>` |
| `reject_counter_offer` | `customer: Address` (must authorize), `refund_id: u64` | `Result<(), Error>` |
| `reject_counter_offer_escalate` | `customer: Address` (must authorize), `refund_id: u64`, `fee_token: Address`, `fee_amount: i128` | `Result<u64, Error>` (the new arbitration case ID) |
| `get_counter_offer` | `refund_id: u64` | `Option<CounterOffer>` (`refund_id`, `merchant`, `amount`, `offered_at`, `expires_at`) |

- **Making an offer.** The refund must be `Requested`, and `amount` must be greater than `0` and less than the requested amount. The offer stays open for 7 days (`expires_at = offered_at + 604800`). A new offer replaces the previous one.
- **Accepting an offer.** The customer can accept up to and including `expires_at`. The refund's `amount` is set to the offered amount, and the per-payment refund-cap usage shrinks to match. The refund is then approved (`approved_by` is the merchant), so `process_refund()` pays out the accepted amount.
- **Rejecting an offer.** `reject_counter_offer` discards the offer and returns the refund to review: it stays `Requested` at its original amount, so the admin can still approve or reject it and the merchant can make a new offer.
- **Rejecting and escalating.** `reject_counter_offer_escalate` treats the offer as the merchant declining the full amount. It discards the offer, moves the refund to `PendingAppeal` (`rejected_by` is the offering merchant, reason `"counter-offer rejected"`) and opens an arbitration case exactly like `escalate_to_arbitration`, charging the arbitration fee (and stake, if enabled) to the customer. If any step fails, the whole call reverts and the offer stays pending.
- **Expiry.** An expired offer can no longer be accepted, and the original request stays `Requested` at its original amount. The admin can still approve or reject it, and the merchant can make a new offer. An expired offer is treated as rejected: the customer can still call either reject function on it, and `CounterOfferRejected` reports `expired: true`.
- **Other decisions.** Approving, rejecting, TTL-expiring or withdrawing the refund discards any pending offer.
- **Response SLA.** A counter-offer does not stop the merchant's response-SLA clock.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::RefundNotFound` | `2` | No refund with this ID |
| `CoreError::Unauthorized` | `3` | `counter_offer`: caller is not the refund's merchant. `accept_counter_offer` / `reject_counter_offer` / `reject_counter_offer_escalate`: caller is not the refund's customer |
| `CoreError::InvalidStatus` | `7` | Refund is not `Requested` |
| `CoreError::RefundWindowExpired` | `11` | `accept_counter_offer`: the refund's TTL has expired |
| `ExtError::InvalidCounterOffer` | `72` | `counter_offer`: `amount <= 0` or `amount >= requested amount` |
| `ExtError::CounterOfferNotFound` | `73` | `accept_counter_offer` / `reject_counter_offer` / `reject_counter_offer_escalate`: no offer is pending |
| `CoreError::InvalidAmount` / `QuorumNotReached` / `AlreadyProcessed` | `1` / `15` / `8` | `reject_counter_offer_escalate`: escalation failed (same rules as `escalate_to_arbitration`) |
| `CoreError::FunctionPaused` / `ContractPaused` | `18` / `17` | The contract or the called function is paused |
| `ExtError::CounterOfferExpired` | `74` | `accept_counter_offer`: the offer's `expires_at` has passed |

Emits `CounterOfferMade` from `counter_offer`. Emits `CounterOfferAccepted` and `RefundApproved` from `accept_counter_offer`. Emits `CounterOfferRejected` from `reject_counter_offer`. Emits `CounterOfferRejected`, `RefundRejected` and `RefundEscalatedToArbitration` (plus `StakeDeposited` when staking is enabled) from `reject_counter_offer_escalate`.

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
- `set_auto_approve_below()` — Merchant sets the auto-approval threshold (capped by the admin ceiling).
- `get_auto_approve_below_ceiling()` / `set_auto_approve_below_ceiling()` — Platform-wide cap on merchant auto-approval thresholds. See [Auto-Approval Ceiling](#auto-approval-ceiling).
- `schedule_refund_policy_change()` / `cancel_scheduled_refund_policy()` / `get_scheduled_refund_policy()` / `activate_scheduled_refund_policy()` — Future-dated policy changes. See [Scheduled Refund Policy Changes](#scheduled-refund-policy-changes).
- `set_refund_cooldown_config()` / `get_refund_cooldown_config()` — Minimum gap between a customer's refund requests. See [Refund Cooldown](#refund-cooldown).
- `get_inherit_from_parent()` — Checks if merchant inherits policy from parent.
- `set_inherit_from_parent()` — Merchant sets policy inheritance from parent.
- `get_applicable_refund_bps()` — Gets the max refund basis points for a merchant and payment.
- `get_policy_inheritance_chain()` — Returns the merchant's inheritance ancestry chain.
- `set_merchant_parent()` — Admin sets the parent merchant for policy inheritance.
- `get_merchant_parent()` — Gets the direct parent merchant of a given merchant.

#### Scheduled Refund Policy Changes

A merchant can schedule a policy change that takes effect at a future ledger timestamp, so customers can be told about new terms before they apply.

| Function | Parameters | Auth | Returns |
| --- | --- | --- | --- |
| `schedule_refund_policy_change` | `merchant: Address`, `tiers: Vec<RefundTier>`, `effective_at: u64` | `merchant` | `Result<(), Error>` |
| `cancel_scheduled_refund_policy` | `merchant: Address` | `merchant` | `Result<(), Error>` |
| `get_scheduled_refund_policy` | `merchant: Address` | none | `Option<ScheduledRefundPolicy>` (`merchant`, `tiers`, `effective_at`, `scheduled_at`) |
| `activate_scheduled_refund_policy` | `merchant: Address` | none (permissionless) | `Result<u32, Error>` (the new policy version) |

How it works:

- **Scheduling.** `effective_at` must be strictly after the current ledger timestamp. Each tier's `max_refund_bps` must be in `1..=10000`. Tiers are sorted by `days_from_purchase`. A merchant has at most one pending change: scheduling again replaces it. If the pending change has already come due, it is applied first so it is not silently discarded.
- **Taking effect.** From `effective_at` on, `get_refund_policy` (and so `get_effective_refund_policy` and refund requests) returns the new tiers, even before anything is written to storage.
- **Writing it to storage.** The change is written as a new policy version, dated `effective_at`, the next time the merchant calls `set_refund_policy`, `deactivate_refund_policy` or `schedule_refund_policy_change`, the next time `request_refund` uses the merchant's policy, or when anyone calls `activate_scheduled_refund_policy`. Because the version is dated `effective_at`, `get_refund_policy_at_time` is correct even if the write happens later. The new policy keeps the existing policy's `created_at` and `default_window_seconds` (30 days if there was no policy) and is `active`.
- **Cancelling.** `cancel_scheduled_refund_policy` only works before `effective_at`. A change that is already due cannot be cancelled: the call applies it and returns `ScheduledPolicyNotFound`.

Timeline example (the merchant's current policy allows 100% within 30 days):

```text
t = 1_000      schedule_refund_policy_change(merchant, [{days_from_purchase: 14, max_refund_bps: 5000}], 2_000)
               -> RefundPolicyChangeScheduled { merchant, tiers_count: 1, effective_at: 2_000 }
               get_refund_policy(merchant)              -> old policy (100% within 30 days)
               get_scheduled_refund_policy(merchant)    -> Some({ effective_at: 2_000, scheduled_at: 1_000, .. })
t = 1_500      activate_scheduled_refund_policy(merchant) -> Err(ScheduledPolicyNotDue)
               (cancel_scheduled_refund_policy(merchant) would still succeed here)
t = 2_000      get_refund_policy(merchant)              -> new policy (50% within 14 days), not yet stored
t = 2_300      activate_scheduled_refund_policy(merchant) -> Ok(version)
               -> RefundPolicySet { merchant, tiers_count: 1 }
               -> RefundPolicyChangeActivated { merchant, version, effective_at: 2_000 }
               get_scheduled_refund_policy(merchant)    -> None
               get_refund_policy_at_time(merchant, 2_100) -> the new version (dated 2_000)
```

```bash
stellar contract invoke --id <REFUND_CONTRACT_ID> --source <MERCHANT_SECRET> --network testnet \
  -- schedule_refund_policy_change \
  --merchant <MERCHANT_ADDRESS> \
  --tiers '[{"days_from_purchase": 14, "max_refund_bps": 5000}]' \
  --effective_at 2000
```

| Error | Code | When |
| --- | --- | --- |
| `CoreError::RefundExceedsPolicy` | `12` | `schedule_refund_policy_change`: a tier's `max_refund_bps` is outside `1..=10000` |
| `ExtError::InvalidEffectiveTime` | `69` | `schedule_refund_policy_change`: `effective_at` is not after the current ledger timestamp |
| `ExtError::ScheduledPolicyNotFound` | `70` | `cancel_scheduled_refund_policy`: nothing pending, or the pending change is already due. `activate_scheduled_refund_policy`: nothing pending |
| `ExtError::ScheduledPolicyNotDue` | `71` | `activate_scheduled_refund_policy`: `effective_at` has not been reached |

Emits `RefundPolicyChangeScheduled` from `schedule_refund_policy_change`, `RefundPolicyChangeCancelled` from `cancel_scheduled_refund_policy`, and `RefundPolicySet` followed by `RefundPolicyChangeActivated` whenever a due change is written (by `activate_scheduled_refund_policy` or lazily by the functions listed above). `get_scheduled_refund_policy` emits nothing.

#### Auto-Approval Ceiling

A merchant's `set_auto_approve_below` threshold lets refunds at or below that amount skip review. The admin sets a platform-wide ceiling on that threshold.

| Function | Parameters | Auth | Returns |
| --- | --- | --- | --- |
| `set_auto_approve_below_ceiling` | `admin: Address`, `value: i128` | `admin` (must be the contract admin) | `Result<(), Error>` |
| `get_auto_approve_below_ceiling` | none | none | `i128` (`0` if never set) |
| `set_auto_approve_below` | `merchant: Address`, `value: i128` | `merchant` | `Result<(), Error>` |
| `get_auto_approve_below` | `merchant: Address` | none | `i128` (`0` if never set) |

How the ceiling limits a merchant:

- **When the merchant sets a threshold.** `set_auto_approve_below` fails with `AutoApproveThresholdExceedsCeiling` if `value` is greater than the current ceiling. The default ceiling is `0`, so merchants cannot enable auto-approval until the admin raises it.
- **When a refund is requested.** The threshold actually used is `min(merchant threshold, current ceiling)`. If the admin lowers the ceiling later, merchants whose stored threshold is now higher are capped at the new ceiling straight away; their stored value is not changed.
- A refund is auto-approved only if the effective merchant does not require admin approval (`get_requires_admin_approval` is `false`) and `amount <= min(threshold, ceiling)`.

Example: the admin calls `set_auto_approve_below_ceiling(admin, 500)`. A merchant's `set_auto_approve_below(merchant, 1_000)` fails with code `32`, while `set_auto_approve_below(merchant, 300)` succeeds. A 250 refund is then auto-approved and a 400 refund stays `Requested`. If the admin later lowers the ceiling to `200`, the same 250 refund would stay `Requested`.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::Unauthorized` | `3` | `set_auto_approve_below_ceiling`: caller is not the contract admin, or no admin is set |
| `CoreError::AutoApproveThresholdExceedsCeiling` | `32` | `set_auto_approve_below`: `value` is greater than the ceiling |

None of these functions emit events; read the current values with the getters.

#### Refund Cooldown

The admin can require a minimum gap between refund requests from the same customer.

| Function | Parameters | Auth | Returns |
| --- | --- | --- | --- |
| `set_refund_cooldown_config` | `admin: Address`, `config: RefundCooldownConfig` | `admin` (must be the contract admin) | `Result<(), Error>` |
| `get_refund_cooldown_config` | none | none | `Option<RefundCooldownConfig>` (`None` if never set) |

`RefundCooldownConfig` fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `cooldown_seconds` | `u64` | Minimum time between two refund requests for the same customer |
| `enabled` | `bool` | Whether the cooldown is enforced. When `false`, requests are not checked and no new cooldown is recorded |

How it works:

- Each successful refund request (while the cooldown is enabled) records the customer's request time together with the `cooldown_seconds` in force at that moment.
- A later request from that customer fails with `RefundCooldownActive` while `now - last_request_at < cooldown_seconds` (using the recorded value, so changing `cooldown_seconds` does not move an existing cooldown). The request is allowed again from `last_request_at + cooldown_seconds`.
- Disabling the config (`enabled: false`) stops all checks immediately. Withdrawing a request does not reset the cooldown.

```bash
stellar contract invoke --id <REFUND_CONTRACT_ID> --source <ADMIN_SECRET> --network testnet \
  -- set_refund_cooldown_config \
  --admin <ADMIN_ADDRESS> \
  --config '{"cooldown_seconds": 86400, "enabled": true}'
```

| Error | Code | When |
| --- | --- | --- |
| `CoreError::Unauthorized` | `3` | `set_refund_cooldown_config`: caller is not the contract admin, or no admin is set |
| `CoreError::RefundCooldownActive` | `33` | `request_refund` (and the other refund-request entry points): the customer is still inside their cooldown |

`set_refund_cooldown_config` and `get_refund_cooldown_config` emit no events. When a request is blocked, `RefundCooldownEnforced` (`customer`, `last_refund_at`, `cooldown_seconds`, `available_at`) is published, but because the call then fails the event is rolled back with the transaction; it is only visible in simulation/diagnostics.

### Query Functions

- `get_refunds_by_status()` — Paginated refunds filtered by status.
- `get_refund_count_by_status()` — Gets the count of refunds in a given status.
- `get_merchant_refunds()` — Paginated refunds for a specific merchant.
- `get_merchant_refunds_by_status()` — Paginated refunds for a merchant filtered by status.
- `get_merchant_refunds_in_range(merchant, from, to, limit, offset)` — Paginated refunds for a merchant whose `requested_at` falls in `[from, to]` (inclusive ledger timestamps), in request order. Returns `InvalidDateRange` (66) if `from > to`.
- `get_merchant_refund_statement(merchant, from, to)` — Accounting statement for a period. See [Merchant Refund Statement](#merchant-refund-statement).
- `get_merchant_pending_refunds()` — All pending refunds for a merchant.
- `get_merchant_refund_summary()` — Aggregate refund stats for a merchant.
- `get_refunds_by_reason_code()` — Paginated refunds filtered by canonical reason code.
- `get_reason_code_analytics(window_start, window_end)` — Counts refunds by reason code within the given ledger-timestamp window, sorted by frequency. Cached per window and invalidated only when a refund inside that window is processed.
- `get_refund_reserve()` — Returns the merchant's current refund reserve balance for a token.
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

#### Syncing processed refunds to the payment contract (#809)

After `process_refund()` pays out a refund, the contract calls `record_refund(refund_contract, payment_id, refund_id, amount)` on the payment contract. This sets the payment's `refunded_amount` and moves its status to `PartialRefunded` or `Refunded`. The same applies to every path that processes a refund: `process_refund_batch()`, `evaluate_auto_refund()` and `resolve_appeal()` when it approves an appeal. On the payment contract, the admin must first register this contract with `set_refund_contract`. `amount` is the gross refund amount, before the platform fee.

The sync never reverts a payout:

- If no payment contract address is set, the sync is skipped and `RefundPaymentSyncFailed` is emitted with `payment_contract_set: false`.
- If the payment contract rejects the call, `RefundPaymentSyncFailed` is emitted with `payment_contract_set: true`. This happens, for example, when this contract is not registered there or the payment contract is paused.

In both cases the refund stays `Processed` and the customer keeps the funds. Once the problem is fixed, call `retry_refund_sync()`.

| Function | Parameters | Returns |
| --- | --- | --- |
| `retry_refund_sync` | `refund_id: u64` | `Result<bool, Error>`: `true` if the payment contract accepted the update, `false` if it rejected it again (emits `RefundPaymentSyncFailed`) |
| `is_refund_synced` | `refund_id: u64` | `bool`: whether the refund has been recorded on the payment contract |

`retry_refund_sync` needs no authorization. It can only push a refund that is already processed, and the payment contract applies each `refund_id` at most once.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::RefundNotFound` | `2` | No refund with this ID |
| `CoreError::InvalidStatus` | `7` | The refund is not `Processed` |
| `CoreError::PaymentContractNotSet` | `27` | No payment contract address is configured |
| `ExtError::RefundAlreadySynced` | `82` | The refund was already recorded on the payment contract |

| Event | Fields | Emitted |
| --- | --- | --- |
| `RefundPaymentSynced` | `refund_id`, `payment_id`, `amount` | The payment contract accepted the update |
| `RefundPaymentSyncFailed` | `refund_id`, `payment_id`, `payment_contract_set` | The sync was skipped (no payment contract) or rejected |

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

### Category Refund Caps

Different product categories can have different refund percentage limits. A category cap applies to payments tagged with that category via `tag_payment_category()`.

| Function | Parameters | Returns | Auth |
|---|---|---|---|
| `set_category_refund_cap` | `merchant: Address`, `category: PaymentCategory`, `max_bps: u32` | `Result<(), Error>` | `merchant` |
| `get_category_refund_cap` | `merchant: Address`, `category: PaymentCategory` | `Option<u32>` (`None` if not set) | — |

- `max_bps` is in basis points (0–10000). Setting `10000` removes the effect of the cap.
- **Combining with policy tiers:** the effective limit is the **lower** of the category cap and the policy tier's `max_refund_bps` (after any customer tier override). This applies both to `get_applicable_refund_bps()` and to the validation done in `request_refund()`, which fails with `RefundExceedsPolicy` (12) above the limit.
- Untagged payments, and categories without a cap, use the policy tier alone.
- Caps are per merchant.

**Errors:** `InvalidCategoryRefundCap` (78) if `max_bps > 10000`.

**Event:** `CategoryRefundCapSet { merchant, category, max_bps }`.

### Merchant Refund Statement

`get_merchant_refund_statement(merchant: Address, from: u64, to: u64) -> Result<MerchantRefundStatement, Error>`

Aggregates the merchant's refunds whose `requested_at` falls in `[from, to]` (inclusive ledger timestamps). It returns the same refunds as `get_merchant_refunds_in_range`, so its totals match summing those refunds individually.

```rust
pub struct MerchantRefundStatement {
    pub merchant: Address,
    pub from: u64,
    pub to: u64,
    pub refund_count: u64,
    pub total_amount: i128,                 // sum of `amount` over all refunds in range
    pub by_status: Vec<StatusRefundTotals>, // one entry per RefundStatus, zeros included
    pub by_token: Vec<TokenRefundTotals>,   // one entry per token seen, first-seen order
}
pub struct StatusRefundTotals { pub status: RefundStatus, pub count: u64, pub total_amount: i128 }
pub struct TokenRefundTotals  { pub token: Address,       pub count: u64, pub total_amount: i128 }
```

**Cost bound:** the start of the range is found by binary search over the merchant's request-ordered index, and a statement may cover at most `MAX_STATEMENT_REFUNDS` (200) refunds. For busier periods, request several shorter statements.

**Errors:** `InvalidDateRange` (66) if `from > to`, `StatementRangeTooLarge` (79) if more than 200 refunds fall in the range.

**Events:** none (read-only).

### Arbitrator Auto-Assignment

- `configure_auto_assignment()` — Admin configures round-robin auto-assignment of arbitrators.
- `auto_assign_arbitrators()` — Automatically assigns a panel of arbitrators.
- `get_next_arbitrators()` — Previews the next arbitrators without advancing the rotation.
- `reset_rotation_index()` — Admin resets the round-robin rotation index.

### Refund Request Amendments

A merchant can correct a refund request before it is reviewed, instead of cancelling and re-filing it.

- Only the merchant that filed the refund can amend it, and only while it is in `Requested` status and its TTL has not expired. Otherwise it fails with `Unauthorized` (3), `InvalidStatus` (7) or `RefundWindowExpired` (11).
- The amount can be lowered or kept, never raised (`AmendmentIncreasesAmount`, 64). A lower amount always stays within the policy, payment and cap checks the original request passed, so they are not re-run. To ask for more, file a new request.
- Lowering the amount gives the difference back to the payment's refund-cap usage.
- Each refund can be amended at most 3 times (`AmendmentLimitReached`, 65).
- Changing the reason code invalidates cached `get_reason_code_analytics` windows that cover the refund.
- Emits `RefundAmended`.

### Arbitrator Availability

Unavailable arbitrators are skipped by `escalate_to_arbitration`, `auto_assign_arbitrators` and `get_next_arbitrators`, and `assign_arbitrator` rejects them with `ArbitratorUnavailable` (61). They keep their seats and votes on cases they are already assigned to.

- `set_arbitrator_availability()` — Arbitrator opts in or out of new case assignments.
- `admin_set_arbiter_availability()` — Admin overrides an arbitrator's availability.
- `is_arbitrator_available()` — Returns whether a registered arbitrator accepts new cases.
- `get_available_arbitrators()` — Lists registered arbitrators currently accepting cases.

### Refund TTL

- `set_refund_ttl_config()` — Admin sets the default TTL for refund requests.
- `expire_stale_refund()` — Expires a refund that exceeded its TTL.
- `get_expired_refunds()` — Gets refund IDs that have expired past TTL.

### Merchant Response SLA

When an SLA is active, each refund created in `Requested` status records a response deadline (`requested_at + response_sla_seconds`). Later config changes do not move that deadline. Once it passes and the refund is still `Requested`, anyone can call `auto_approve_on_sla_breach`. The refund then moves to `Approved` with the contract address as the approver. A TTL that expired first takes precedence and the call returns `RefundWindowExpired`.

- `set_response_sla_config()` — Admin sets the platform-wide response SLA.
- `get_response_sla_config()` — Gets the platform-wide response SLA.
- `set_merchant_response_sla()` — Admin sets a per-merchant SLA override (can also disable it for that merchant).
- `remove_merchant_response_sla()` — Admin removes a merchant override.
- `get_merchant_response_sla()` — Gets the effective SLA seconds for a merchant.
- `get_refund_sla_deadline()` — Gets the response deadline recorded for a refund.
- `auto_approve_on_sla_breach()` — Permissionless: auto-approves a refund past its SLA deadline.
- `get_sla_breached_refunds()` — Lists `Requested` refunds whose SLA has passed.

Errors: `SlaNotConfigured` (62) if the refund has no recorded deadline, `SlaNotBreached` (63) if the deadline hasn't passed.

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
- `redeem_refund_voucher()` — Customer redeems a refund voucher's remaining balance against a future payment.
- `redeem_voucher_amount()` — Customer redeems part of a voucher, keeping the rest for later.
- `get_voucher_balance()` — Gets a voucher's remaining redeemable balance.
- `get_voucher()` — Gets a refund voucher by ID.
- `get_customer_vouchers()` — Gets all refund vouchers currently owned by a customer.
- `transfer_voucher()` — Owner transfers an unredeemed, unexpired voucher to another address.
- `set_vouchers_transferable()` — Merchant chooses whether vouchers for its refunds are issued transferable.
- `get_vouchers_transferable()` — Whether a merchant's new vouchers are issued transferable (default `true`).
- `is_voucher_transferable()` — Whether a specific voucher can be transferred.
- `issue_promotional_vouchers()` — Merchant issues goodwill credit vouchers to many customers at once, funded from its refund reserve.

#### Voucher transfers

| Function | Parameters | Returns |
| --- | --- | --- |
| `transfer_voucher` | `owner: Address` (must authorize), `voucher_id: u64`, `new_owner: Address` | `Result<(), Error>` |
| `set_vouchers_transferable` | `merchant: Address` (must authorize), `transferable: bool` | `Result<(), Error>` |
| `get_vouchers_transferable` | `merchant: Address` | `bool` |
| `is_voucher_transferable` | `voucher_id: u64` | `bool` |

A transfer sets the voucher's `customer` field to `new_owner`. It also moves the voucher from the old owner's `get_customer_vouchers()` list to the new owner's. After a transfer, only the new owner can redeem the voucher or transfer it again.

The merchant's `set_vouchers_transferable` setting is snapshotted onto each voucher when `issue_refund_voucher()` runs. Changing the setting later does not affect vouchers that already exist. The owner of a non-transferable voucher can still redeem it.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::Unauthorized` | `3` | Caller does not own the voucher |
| `ExtError::VoucherNotFound` | `52` | No voucher with this ID |
| `ExtError::VoucherExpired` | `53` | Ledger time is past the voucher's `expires_at` |
| `ExtError::VoucherAlreadyRedeemed` | `54` | The voucher has been redeemed |
| `ExtError::VoucherNotTransferable` | `75` | The voucher was issued while its merchant had transfers disabled |
| `ExtError::InvalidVoucherRecipient` | `76` | `new_owner` is the current owner |

Emits `VoucherTransferred` from `transfer_voucher`, and `VoucherTransferabilitySet` from `set_vouchers_transferable`.

#### Partial voucher redemption

| Function | Parameters | Returns |
| --- | --- | --- |
| `redeem_voucher_amount` | `customer: Address` (must authorize), `voucher_id: u64`, `amount: i128` | `Result<i128, Error>` (the remaining balance) |
| `get_voucher_balance` | `voucher_id: u64` | `Result<i128, Error>` |

`redeem_voucher_amount` transfers `amount` of the voucher's token to the customer and lowers the voucher's remaining balance. When the balance reaches `0`, the voucher is marked `redeemed`, and further redemptions fail with `VoucherAlreadyRedeemed`. The sum of all partial redemptions can never exceed the voucher's `amount`.

`redeem_refund_voucher()` pays out the remaining balance only. A voucher of 1000 that already had 400 redeemed pays out 600.

Expiry covers the remaining balance. After `expires_at`, neither function can redeem anything that is left. The remaining balance stays with the voucher when it is transferred.

`get_voucher_balance` returns the full `amount` for a voucher with no redemptions and `0` for a fully redeemed voucher. It does not take expiry into account.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::Unauthorized` | `3` | Caller does not own the voucher |
| `CoreError::InvalidAmount` | `1` | `amount` is zero or negative |
| `ExtError::VoucherNotFound` | `52` | No voucher with this ID |
| `ExtError::VoucherExpired` | `53` | Ledger time is past the voucher's `expires_at` |
| `ExtError::VoucherAlreadyRedeemed` | `54` | The voucher's balance is already `0` |
| `ExtError::VoucherInsufficientBalance` | `77` | `amount` is greater than the remaining balance |

Emits `VoucherAmountRedeemed` from `redeem_voucher_amount`. `redeem_voucher_amount` can be paused on its own with `pause_function("redeem_voucher_amount")`.

#### Promotional vouchers

| Function | Parameters | Returns |
| --- | --- | --- |
| `issue_promotional_vouchers` | `merchant: Address` (must authorize), `recipients: Vec<(Address, i128)>` (`(customer, amount)` pairs), `token: Address`, `expiry_seconds: u64` | `Result<Vec<u64>, Error>` (new voucher IDs, in `recipients` order) |

`issue_promotional_vouchers` lets a merchant issue goodwill credit without a refund behind it. Every voucher is paid for from the merchant's refund reserve for `token` (see `deposit_refund_reserve()`). A batch may contain at most `MAX_PROMOTIONAL_BATCH` (50) recipients.

The whole batch is validated first. Then the reserve is checked against the batch total. Only after both checks pass are any vouchers issued, so a batch either issues every voucher or none. The reserve is debited by the batch total. The tokens are already held by the contract, so no transfer happens at issuance.

Each voucher expires `expiry_seconds` after the ledger time of the call. Promotional vouchers have `refund_id == 0`, which tells them apart from refund vouchers (refund IDs start at `1`). They work like any other voucher: they can be redeemed with `redeem_refund_voucher()` or `redeem_voucher_amount()`, and transferred subject to the merchant's `set_vouchers_transferable` setting at issue time. A customer may appear more than once in one batch and receives one voucher per entry. The function can be paused on its own with `pause_function("issue_promotional_vouchers")`.

| Error | Code | When |
| --- | --- | --- |
| `CoreError::InvalidAmount` | `1` | An amount is zero or negative, or the batch total overflows `i128` |
| `ExtError::InsufficientRefundReserve` | `67` | The merchant's reserve for `token` is less than the batch total, or the merchant has no reserve in `token` |
| `ExtError::InvalidVoucherRecipient` | `76` | A recipient is the merchant itself |
| `ExtError::EmptyPromotionalBatch` | `80` | `recipients` is empty |
| `ExtError::PromotionalBatchTooLarge` | `81` | `recipients` has more than `MAX_PROMOTIONAL_BATCH` entries |

| Event | Fields | Emitted |
| --- | --- | --- |
| `PromotionalVoucherIssued` | `voucher_id`, `merchant`, `customer`, `token`, `amount`, `expires_at` | Once per voucher issued |
| `PromotionalVouchersIssued` | `merchant`, `token`, `count`, `total_amount`, `new_reserve_balance` | Once per batch, after the vouchers |

#### Voucher expiry and value handling

A voucher is created with a concrete TTL at issue time:

```rust
let now = env.ledger().timestamp();
let voucher = RefundVoucher {
    issued_at: now,
    expires_at: now.saturating_add(expiry_seconds),
    redeemed: false,
};
```

The contract treats the expiry check as `now > expires_at`:

- redemption succeeds while `now <= expires_at`
- redemption fails once the timestamp moves past the stored expiry
- equality is still valid, so the voucher remains usable at the exact `expires_at` boundary

This is enforced in `redeem_refund_voucher()` by rejecting with `VoucherExpired` when the ledger time is past the voucher's expiry. The value itself is not automatically reclaimed or reissued once a voucher expires; the voucher simply stops being redeemable. There is no on-chain “refund the expired amount back to merchant/treasury” flow in the contract; the unredeemed value is effectively dead credit unless a new voucher is created or a different manual process handles it.

#### Policy inheritance and merchant ancestry

Refund policy inheritance is a separate but related mechanism for merchants organized in a parent/child hierarchy. A merchant can be assigned a parent via `set_merchant_parent(admin, merchant, parent)`, and the contract checks for:

- self-parent loops (`merchant == parent`)
- circular ancestry chains
- maximum inheritance depth (`MAX_INHERITANCE_DEPTH = 5`)

The effective policy is resolved via `get_effective_refund_policy(merchant)`, which walks the ancestry chain and returns the first active policy it finds. In practical terms:

- a merchant with an active policy sees their own policy first
- if they do not have a policy, or their policy is inactive and inheritance is enabled, the parent chain is consulted
- `set_inherit_from_parent(merchant, false)` disables fall-through to a parent when the merchant's local policy is inactive
- `get_policy_inheritance_chain(merchant)` returns the explicit ancestry path for auditing

This is distinct from voucher redemption: inheritance is about the merchant refund policy that governs future refund decisions, while vouchers are per-refund credit instruments with their own expiry and redemption logic.

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
| `RefundAmended`   | `RefundAmended`   | `refund_id`, `merchant`, `old_amount`, `new_amount`, `reason_code`, `amendment_count` | `amend_refund_request()` corrects a refund still in `Requested` status |
| `RefundApproved`  | `RefundApproved`  | `refund_id`, `payment_id`, `amount`, `approved_by`, `approved_at`          | `approve_refund()` moves refund to `Approved` status                 |
| `RefundRejected`  | `RefundRejected`  | `refund_id`, `rejected_by`, `rejected_at`, `rejection_reason`              | `reject_refund()` moves refund to `PendingAppeal` status             |
| `RefundProcessed` | `RefundProcessed` | `refund_id`, `processed_by`, `customer`, `amount`, `token`, `processed_at` | `process_refund()` executes approved refund and moves to `Processed` |
| `RefundPaymentSynced` | `refund_payment_synced` | `refund_id`, `payment_id`, `amount` | A processed refund is recorded on the payment contract (#809) |
| `RefundPaymentSyncFailed` | `refund_payment_sync_failed` | `refund_id`, `payment_id`, `payment_contract_set` | The payment-contract sync is skipped or rejected; the payout stands (#809) |
| `RefundWithdrawn` | `refund_withdrawn` | `refund_id`, `payment_id`, `customer`, `amount`, `withdrawn_at` | `withdraw_refund_request()` moves refund to `Withdrawn` status |
| `CounterOfferMade` | `counter_offer_made` | `refund_id`, `merchant`, `requested_amount`, `offered_amount`, `expires_at` | `counter_offer()` records a merchant counter-offer |
| `CounterOfferAccepted` | `counter_offer_accepted` | `refund_id`, `customer`, `requested_amount`, `accepted_amount` | `accept_counter_offer()` approves the refund at the offered amount |
| `RefundReserveLow` | `refund_reserve_low` | `merchant`, `token`, `balance`, `warning_level` | A withdrawal or refund drops the merchant's reserve below its warning level (fires once per crossing) |

### Voucher Events

| Event | Topic Name | Payload Fields | Fires When |
| --- | --- | --- | --- |
| `VoucherTransferred` | `voucher_transferred` | `voucher_id`, `from`, `to` | `transfer_voucher()` moves a voucher to a new owner |
| `VoucherTransferabilitySet` | `voucher_transferability_set` | `merchant`, `transferable` | `set_vouchers_transferable()` changes a merchant's voucher setting |
| `VoucherAmountRedeemed` | `voucher_amount_redeemed` | `voucher_id`, `customer`, `amount`, `remaining_balance` | `redeem_voucher_amount()` redeems part (or the rest) of a voucher |

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
| `RefundPolicySet` | `refund_policy_set` | `merchant`, `tiers_count` | A policy is written, including when a scheduled change is activated |
| `RefundPolicyChangeScheduled` | `refund_policy_change_scheduled` | `merchant`, `tiers_count`, `effective_at` | `schedule_refund_policy_change()` records a pending change |
| `RefundPolicyChangeCancelled` | `refund_policy_change_cancelled` | `merchant`, `effective_at` | `cancel_scheduled_refund_policy()` discards a pending change |
| `RefundPolicyChangeActivated` | `refund_policy_change_activated` | `merchant`, `version`, `effective_at` | A due scheduled change is written as a new policy version |

| `CategoryRefundCapSet` | `category_refund_cap_set` | `merchant`, `category`, `max_bps` | `set_category_refund_cap()` |

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
| `AdminRotationProposed` | `admin_rotation_proposed` | `current_admin`, `pending_admin` | `propose_admin()` records a pending admin |
| `AdminRotationAccepted` | `admin_rotation_accepted` | `previous_admin`, `new_admin` | `accept_admin()` completes the rotation |
| `ContractUpgraded` | `contract_upgraded` | `old_schema_version`, `new_wasm_hash`, `upgraded_by` | `upgrade()` replaces the contract WASM |

### Circuit Breaker Events

| Event                        | Topic Name                   | Payload Fields                           | Fires When                                                                      |
| ---------------------------- | ---------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------- |
| `CircuitBreakerTrippedEvent` | `CircuitBreakerTrippedEvent` | `triggered_by`, `reason`, `triggered_at` | Circuit breaker activates due to refund volume threshold or error rate exceeded |

---

## 💰 Refund Reserve & Low-Reserve Alerts

Merchants keep a per-token refund reserve on the contract. Refunds and withdrawals draw down this reserve, and a merchant can set a warning level so they are alerted before the reserve is empty and refunds start failing.

| Function | Parameters | Returns | Auth |
| --- | --- | --- | --- |
| `set_refund_reserve_warning_level` | `merchant: Address`, `token: Address`, `warning_level: i128` | `Result<(), Error>` | `merchant` |
| `get_refund_reserve_warning_level` | `merchant: Address`, `token: Address` | `Option<i128>` (`None` if not set) | — |
| `get_refund_reserve` | `merchant: Address`, `token: Address` | `i128` | — |

`set_refund_reserve_warning_level` stores the level at which the merchant wants to be warned. `warning_level` must be `>= 0`; a negative value fails with `CoreError::InvalidAmount` (1). Setting the level does not emit an event by itself.

**When `RefundReserveLow` fires.** The event is emitted from any path that reduces the reserve (a refund payout or a reserve withdrawal) when the resulting balance is **below** the merchant's warning level. It fires only on the transition: once the reserve is already below the level, further reductions do not re-emit it. The alert re-arms after the reserve is topped back up to or above the warning level. If no warning level is set for the merchant/token pair, no alert is emitted.

`RefundReserveLow` payload: `merchant`, `token`, `balance` (the new reserve balance), `warning_level` (the configured level at the time of the crossing).

`get_refund_reserve` returns the merchant's current reserve balance for `token` (`0` if no reserve has been deposited).

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
