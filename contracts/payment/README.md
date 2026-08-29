# Payment Contract

Part of the [FacilPay smart contracts](../../README.md) suite on Stellar/Soroban.

## Purpose

The payment contract is the core of the FacilPay platform. It handles the full lifecycle of a payment: creation, completion, refunding, expiry, and cancellation. It also provides a rich set of optional features that can be composed on top of basic payments:

- **Escrowed payments** — funds held in an external escrow contract until released or disputed
- **Scheduled payments** — payments deferred to a future ledger timestamp
- **Installment (partial) payments** — split a single payment into multiple installments
- **Conditional payments** — payments that execute only when an on-chain condition is met
- **Subscriptions** — fixed-interval recurring payments with dunning, trials, pause/resume, and proration
- **Metered billing** — usage-based subscriptions billed per reported unit
- **Payment channels** — off-chain micro-payments settled on-chain in a single transaction
- **Split payments** — distribute a payment to multiple recipients by share percentage
- **Payment forwarding** — automatically forward a portion of a payment to a third address
- **Batch payments** — create or complete many payments in one call
- **Multi-sig governance** — require multiple admin approvals for large or sensitive actions
- **Fee management** — tiered fees, fee waivers, rebate programmes, and platform fee sweeping
- **Analytics** — per-merchant, per-customer, and platform-wide payment analytics
- **Rate limiting & fraud controls** — address flagging, spend limits, and volume caps
- **Loyalty points** — accumulate and redeem customer loyalty balances

---

## Public Functions

### Lifecycle

| Function                                | Description                                                                             |
| --------------------------------------- | --------------------------------------------------------------------------------------- |
| `initialize(admin)`                     | Deploy and configure the contract with an initial admin and default multi-sig settings. |
| `get_schema_version()`                  | Return the current storage schema version number.                                       |
| `migrate_schema(admin, target_version)` | Migrate contract storage to a newer schema version.                                     |

### Core Payments

| Function                                                                                     | Description                                                                                                                                                  |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `create_payment(customer, merchant, amount, token, currency, expiration_duration, metadata)` | Customer initiates a payment; tokens are transferred from the customer to the contract and the payment is stored as `Pending`. Returns the new `payment_id`. |
| `complete_payment(admin, payment_id)`                                                        | Admin releases a `Pending` payment to the merchant. For amounts above the configured large-payment threshold a multi-sig proposal is auto-created instead.   |
| `refund_payment(admin, payment_id)`                                                          | Admin refunds a `Pending` payment in full, returning tokens to the customer.                                                                                 |
| `partial_refund(admin, payment_id, refund_amount)`                                           | Admin issues a partial refund on a `Completed` payment, returning only `refund_amount` to the customer.                                                      |
| `cancel_payment(caller, payment_id)`                                                         | Customer or admin cancels a `Pending` payment and returns funds.                                                                                             |
| `get_payment(payment_id)`                                                                    | Retrieve the full `Payment` record by ID. Panics if not found.                                                                                               |
| `check_payment_customer(payment_id, customer)`                                               | Returns `true` if the payment exists, belongs to `customer`, and is `Completed` (used for cross-contract verification).                                      |
| `expire_payment(payment_id)`                                                                 | Anyone can call this once a payment is past its expiration timestamp; tokens are returned to the customer.                                                   |
| `is_payment_expired(payment_id)`                                                             | Returns `true` if the payment's expiration timestamp has passed.                                                                                             |
| `update_payment_notes(admin, payment_id, notes)`                                             | Admin updates free-text notes on a payment.                                                                                                                  |

### Queries & Pagination

| Function                                   | Description                                                |
| ------------------------------------------ | ---------------------------------------------------------- |
| `get_payments_by_customer(customer, page)` | Paginated list of payment IDs for a customer.              |
| `get_payment_count_by_customer(customer)`  | Total number of payments for a customer.                   |
| `get_payments_by_merchant(merchant, page)` | Paginated list of payment IDs for a merchant.              |
| `get_payment_count_by_merchant(merchant)`  | Total number of payments for a merchant.                   |
| `get_merchant_payments(merchant, page)`    | Alternative paginated index of payment IDs for a merchant. |

### Scheduled Payments

| Function                                                            | Description                                                                            |
| ------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `schedule_payment(customer, merchant, token, amount, scheduled_at)` | Escrow tokens now and execute the payment at `scheduled_at`. Returns the `payment_id`. |
| `execute_scheduled_payment(payment_id)`                             | Anyone can trigger this after the scheduled timestamp to complete the transfer.        |
| `cancel_scheduled_payment(caller, payment_id)`                      | Customer or admin cancels a scheduled payment and reclaims the escrowed tokens.        |
| `get_scheduled_payment(payment_id)`                                 | Retrieve the `ScheduledPayment` record.                                                |

### Installment (Partial) Payments

| Function                                                 | Description                                                                             |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `pay_installment(payer, payment_id, installment_amount)` | Payer submits one installment towards a payment's outstanding balance.                  |
| `finalize_installment_payment(payment_id)`               | Mark a fully-paid installment payment as `Completed` and release funds to the merchant. |
| `get_installment_history(payment_id)`                    | Return all installment records for a payment.                                           |
| `get_outstanding_balance(payment_id)`                    | Return the remaining unpaid balance of an installment payment.                          |

### Escrowed Payments

| Function                                                                                          | Description                                                          |
| ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| `create_escrowed_payment(customer, merchant, amount, token, currency, metadata, escrow_contract)` | Create a payment whose funds are locked in a linked escrow contract. |
| `complete_escrowed_payment(admin, payment_id)`                                                    | Release funds from escrow to the merchant after a successful trade.  |
| `cancel_escrowed_payment(admin, payment_id)`                                                      | Cancel the escrow and return funds to the customer.                  |
| `dispute_escrowed_payment(caller, payment_id)`                                                    | Raise a dispute on an escrowed payment.                              |
| `resolve_escrowed_payment_dispute(admin, payment_id, favor_customer)`                             | Resolve a dispute in favour of the customer or merchant.             |
| `get_escrowed_payment(payment_id)`                                                                | Retrieve the `EscrowedPayment` record.                               |
| `get_escrowed_payment_dispute(payment_id)`                                                        | Retrieve the active dispute record for an escrowed payment.          |

### Cross-Contract Escrow Verification

The payment contract interacts with the escrow contract through Soroban cross-contract invocations. These calls fall into two categories: **state-mutating bridge calls** (create, release, refund, dispute) and **read-only verification calls** that inspect escrow state without modifying it.

#### What the Payment Contract Verifies

When the payment contract calls into the escrow contract, it queries the following state through the escrow's state verification interface:

| Verification Call                   | What It Returns                                                              | Payment-Side Usage                                                                 |
| ----------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `is_escrow_released(escrow_id)`     | `true` if the escrow exists and its status is `Released`, `false` otherwise  | Confirm escrow funds were released before marking an escrowed payment as completed  |
| `is_escrow_disputed(escrow_id)`     | `true` if the escrow exists and its status is `Disputed`, `false` otherwise  | Check whether a dispute is active on the escrow                                     |
| `get_escrow_status(escrow_id)`      | Current `EscrowStatus` (`Locked`, `Released`, `Disputed`, `Refunded`, etc.)  | Validate escrow state before advancing payment lifecycle                            |
| `get_escrow_parties(escrow_id)`     | `(customer, merchant)` address pair                                           | Confirm payment and escrow parties match                                            |
| `get_escrow_amount(escrow_id)`      | Locked amount (`i128`)                                                       | Verify the escrowed amount matches the payment amount                               |
| `verify_escrow_participant(addr)`   | `true` if `address` is the customer or merchant of the escrow                | Gate caller permissions for dispute/cancel operations                               |

#### State-Mutating Bridge Calls

The payment contract also calls escrow functions that modify state. These are routed through internal helpers (`invoke_escrow_create`, `try_release_escrow`, `try_refund_escrow`, `try_dispute_escrow`, `try_resolve_dispute`) which wrap the `EscrowContractClient`.

For `complete_escrowed_payment`, the payment contract calls `release_escrow` using its **own contract address** as the caller. This works because the escrow contract admin registers the payment contract as a **trusted bridge** (via `add_trusted_bridge`), which bypasses the escrow admin early-release timelock. If the payment contract is not registered as a trusted bridge, `release_escrow` reverts with an auth or timelock error.

#### Failure Modes

| Failure Scenario                                              | Error Returned by Payment Contract                         | Details                                                                 |
| ------------------------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------- |
| Escrow contract address not set / unreachable                 | `FeatureError::EscrowBridgeFailed` (501)                   | Cross-contract `invoke` fails; the payment contract cannot reach escrow |
| Escrow ID does not exist on escrow contract                   | `EscrowError::NotFound` (200) propagated through bridge   | `get_escrow_status`, `get_escrow_parties`, `get_escrow_amount` return `NotFound`; boolean checks return `false` |
| Payment contract not registered as trusted bridge             | `FeatureError::EscrowBridgeFailed` (501)                   | `release_escrow` / `complete_escrowed_payment` reverts — admin must call `add_trusted_bridge` on the escrow contract |
| Escrow in wrong state for requested action                    | `EscrowError::InvalidStatus` (201) propagated through bridge | e.g. attempting to release an already-refunded escrow                 |
| Release timelock has not elapsed and caller is not a trusted bridge | `EscrowError::ReleaseOnHoldPeriod` (205)              | Early release blocked; the caller must be a registered trusted bridge  |
| No `EscrowedPayment` mapping found for a `payment_id`         | `FeatureError::EscrowMappingNotFound` (500)                | Payment-side lookup — the escrow contract was never linked to this payment |
| Active unresolved dispute blocks completion/cancellation       | Internal guard in `require_no_unresolved_escrowed_payment_dispute` | Not an escrow error; the payment contract itself rejects the call   |

All cross-contract bridge failures are normalised to `FeatureError::EscrowBridgeFailed` at the payment contract boundary. Callers should inspect the error code and, if needed, query the escrow contract directly for more granular diagnostics via the read-only verification interface.

#### Test Coverage

The cross-contract verification flow is exercised by `test_cross_contract_escrow_verification.rs`, which registers both the payment and escrow contracts in a shared Soroban test environment and verifies:

- `is_escrow_released` returns `false` before completion and `true` after
- `get_escrow_status` transitions from `Locked` to `Released`
- `get_escrow_parties` matches the original payment parties
- `get_escrow_amount` matches the escrowed amount
- `verify_escrow_participant` correctly identifies customers, merchants, and non-participants

### Conditional Payments

| Function                                                                                       | Description                                                                       |
| ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `create_conditional_payment(customer, merchant, amount, token, currency, condition, metadata)` | Create a payment that only executes when the specified on-chain condition is met. |
| `evaluate_condition(payment_id)`                                                               | Evaluate the condition and store the result without executing the payment.        |
| `complete_conditional_payment(admin, payment_id)`                                              | Complete the payment after its condition has been evaluated as `true`.            |
| `execute_if_condition_met(payment_id)`                                                         | Atomically evaluate the condition and complete the payment in one call.           |
| `get_conditional_payment(payment_id)`                                                          | Retrieve the `ConditionalPayment` record.                                         |

### Subscriptions

| Function                                                                                                                            | Description                                                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `create_subscription(customer, merchant, amount, token, currency, interval, duration, max_retries, metadata, trial_period_seconds)` | Create a recurring subscription; an optional free trial delays the first charge. Returns the `subscription_id`. |
| `execute_recurring_payment(subscription_id)`                                                                                        | Execute the next billing cycle for an active subscription.                                                      |
| `cancel_subscription(caller, subscription_id)`                                                                                      | Cancel a subscription.                                                                                          |
| `pause_subscription(admin, subscription_id)`                                                                                        | Pause a subscription, pausing the billing clock.                                                                |
| `resume_subscription(admin, subscription_id, proration_enabled)`                                                                    | Resume a paused subscription, optionally prorating the next billing date.                                       |
| `extend_trial(admin, subscription_id, extension_seconds)`                                                                           | Extend the trial period of a subscription.                                                                      |
| `get_subscription(subscription_id)`                                                                                                 | Retrieve the `Subscription` record.                                                                             |
| `get_subscriptions_by_customer(customer, page)`                                                                                     | Paginated list of subscription IDs for a customer.                                                              |
| `get_subscriptions_by_merchant(merchant, page)`                                                                                     | Paginated list of subscription IDs for a merchant.                                                              |
| `get_merchant_subscriptions(merchant, page)`                                                                                        | Alternative paginated index of subscription IDs for a merchant.                                                 |

#### How Free Trials Work

A subscription can begin with an optional **free trial** by passing a non-zero
`trial_period_seconds` to `create_subscription`. A trial is a delay on the *first
charge* only — the subscription is `Active` from the moment it is created, but the
customer is not billed until the trial ends. Trial state lives on the subscription
record as `trial_data` (`period_seconds`, `ends_at`, `converted`).

**Starting a trial**

- `trial_period_seconds` is the trial length in seconds. Passing `0` means "no
  trial": `trial_data.period_seconds` and `trial_data.ends_at` are both `0`, and
  `trial_data.converted` stays `false`.
- With a non-zero value, `trial_data.ends_at` is set to
  `created_at + trial_period_seconds` and a `TrialStarted { subscription_id,
  trial_ends_at }` event is emitted alongside the usual `SubscriptionCreated`
  event.
- `interval` must still be non-zero even when a trial is set, otherwise the call
  reverts with `InvalidInterval` (error `124`) — the billing clock and pause
  proration math divide by the interval.
- `next_payment_at` is initialised to `created_at + interval` as normal; the trial
  check is layered on top when a cycle becomes due.

**Duration and extension**

- The trial runs until `trial_data.ends_at`. There is no minimum length; the
  maximum *total* trial length is `MAX_TRIAL_DURATION` = **90 days**.
- Only the subscription's merchant can lengthen an active trial, via
  `extend_trial(merchant, subscription_id, additional_seconds)`. This adds
  `additional_seconds` to both `trial_data.ends_at` and
  `trial_data.period_seconds`.
- Extension is rejected when:
  - the caller is not the subscription's merchant — `Unauthorized` (error `100`);
  - the trial has already expired (`now >= trial_data.ends_at`) — `TrialExpired`
    (error `315`);
  - the new total trial length would exceed 90 days —
    `MaxTrialDurationExceeded` (error `316`).
- There is no "shorten trial" call. To end a trial early, the customer cancels
  (see below); otherwise it simply expires.

**The trial-to-paid transition**

- The transition is driven by `execute_recurring_payment`, which an off-chain
  keeper / cron calls once a cycle is due. There is **no separate "convert" call**,
  and the customer does **not** need to take any action to convert to a paid
  subscription.
- While `now < trial_data.ends_at`, a due `execute_recurring_payment` performs
  **no token transfer**, does **not** increment `payment_count`, advances
  `next_payment_at` by one `interval`, and returns `Ok`. Repeated due cycles
  during the trial are each skipped this way.
- On the first `execute_recurring_payment` where `now >= trial_data.ends_at` and a
  payment is due, the customer is charged normally (running through the usual
  merchant-not-paused, not-expired, spend-limit and discount checks). If that
  transfer succeeds, `trial_data.converted` is set to `true` and a
  `TrialConverted { subscription_id, converted_at }` event is emitted. Later
  charges behave like any recurring payment.
- If the first post-trial charge fails, the subscription follows the standard
  retry / dunning path and `converted` stays `false` until a charge succeeds.

**Cancelling during a trial**

- If `cancel_subscription` is called while `now < trial_data.ends_at`, the
  subscription moves to `Cancelled`, a `TrialCancelled { subscription_id,
  cancelled_at }` event is emitted (in addition to `SubscriptionCancelled`), and
  the customer is never charged. A customer who does not want to convert must
  cancel before the trial ends.

### Metered Billing

| Function                                                                                                               | Description                                                                                |
| ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `create_metered_subscription(merchant, customer, token, price_per_unit, unit_name, billing_cap, max_units_per_period)` | Create a usage-based subscription billed per reported unit. Returns the `subscription_id`. |
| `report_usage(merchant, subscription_id, units)`                                                                       | Merchant reports consumed units to be billed in the next cycle.                            |
| `execute_metered_billing(subscription_id)`                                                                             | Bill the customer for all accumulated usage since the last billing cycle.                  |
| `set_billing_cap(admin, subscription_id, cap)`                                                                         | Set an upper billing limit per cycle to protect the customer.                              |
| `get_current_usage(subscription_id)`                                                                                   | Return the current accumulated usage for a metered subscription.                           |

### Dunning (Failed Payment Recovery)

| Function                                           | Description                                                                              |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `set_dunning_config(admin, config)`                | Configure retry cadence, grace period, and max retries for failed subscription payments. |
| `get_dunning_config()`                             | Retrieve the current dunning configuration.                                              |
| `get_dunning_state(subscription_id)`               | Return the dunning state for a subscription in a failed-payment cycle.                   |
| `retry_failed_payment(subscription_id)`            | Retry a subscription that is in dunning.                                                 |
| `resolve_dunning(admin, subscription_id)`          | Admin manually resolves a subscription stuck in dunning.                                 |
| `update_dunning_config(admin, config)`             | Update the dunning configuration (alias for `set_dunning_config`).                       |
| `manually_resolve_dunning(admin, subscription_id)` | Alias for `resolve_dunning`.                                                             |

### Subscription Groups

| Function                                              | Description                                                                            |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `create_subscription_group(admin, merchant, name)`    | Create a named group of subscriptions for coordinated billing. Returns the `group_id`. |
| `add_to_group(admin, group_id, subscription_id)`      | Add a subscription to a group.                                                         |
| `remove_from_group(admin, group_id, subscription_id)` | Remove a subscription from a group.                                                    |
| `get_subscription_group(group_id)`                    | Retrieve the `SubscriptionGroup` record.                                               |
| `get_group_next_billing(group_id)`                    | Return the earliest next billing timestamp across all subscriptions in the group.      |

### Payment Channels

| Function                                                                   | Description                                                                                |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `open_channel(customer, merchant, token, amount, expires_at, customer_pk)` | Open an off-chain payment channel by depositing tokens on-chain. Returns the `channel_id`. |
| `settle_channel(caller, channel_id, merchant_amount, nonce, signature)`    | Merchant submits the final signed balance proof to settle the channel on-chain.            |
| `close_channel_expired(caller, channel_id)`                                | Anyone can close an expired channel, refunding the deposited balance to the customer.      |
| `get_channel(channel_id)`                                                  | Retrieve the `PaymentChannel` record.                                                      |

### Split Payments

| Function                                                                              | Description                                                                                         |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `create_split_payment(customer, recipients, token, total_amount, currency, metadata)` | Create a payment that will be distributed among multiple recipients according to their `share_bps`. |
| `execute_split_settlement(admin, payment_id)`                                         | Distribute a completed split payment's funds to all recipients.                                     |
| `get_split_config(payment_id)`                                                        | Retrieve the `PaymentSplitConfig` for a split payment.                                              |
| `set_min_split_amount(admin, min_amount)`                                             | Set the minimum amount required to create a split payment.                                          |
| `get_min_split_amount()`                                                              | Return the current minimum split payment amount.                                                    |

### Batch Payments

| Function                                            | Description                                                                      |
| --------------------------------------------------- | -------------------------------------------------------------------------------- |
| `create_batch_payment(customer, entries)`           | Create multiple payments in a single transaction. Returns a list of payment IDs. |
| `complete_batch_payment(admin, payment_ids)`        | Complete multiple payments in a single transaction.                              |
| `cancel_batch_payment(caller, payment_ids)`         | Cancel multiple payments in a single transaction.                                |
| `create_payment_batch_optimized(customer, entries)` | Optimized variant of batch creation that minimises ledger operations.            |
| `get_batch_gas_estimate(entries)`                   | Estimate the ledger units required for a batch payment operation.                |

### Payment Forwarding

| Function                                              | Description                                                                            |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `set_payment_forward(merchant, forward_to, forward_bps)` | Configure a merchant's payments to forward `forward_bps` basis points to `forward_to`. |
| `remove_payment_forward(merchant)`                    | Remove the forwarding configuration for a merchant.                                    |
| `get_forward_config(merchant)`                        | Retrieve the `PaymentForwardConfig` for a merchant.                                    |

#### How Payment Forwarding Works

Payment forwarding lets a merchant automatically route a fixed percentage of every
completed payment to a third-party address (`forward_to`), in addition to the normal
merchant settlement. It is configured per-merchant and applied automatically at payment
completion — integrators do not need to do anything special when calling `complete_payment`.

**Who can set up a forward**

- `set_payment_forward` and `remove_payment_forward` are authorized by the **merchant
  themselves** via `require_auth()` on the `merchant` argument. The admin does **not**
  configure forwarding on a merchant's behalf.
- The caller must be the merchant whose address will be the source of the forward. The
  `forward_to` address is unconstrained (it may be any address, including another merchant
  or an external wallet).
- The supplied `forward_bps` must be between `1` and `10000` (inclusive); `0` and values
  above `10000` revert with `InvalidForwardBps`. A value of `10000` forwards the entire
  merchant net amount.

**Cycle / depth protection**

- Forwarding chains are validated to prevent infinite transfer loops. When a forward is
  set, the contract walks the existing forward chain beginning at `forward_to` up to a
  maximum depth (default `5`, configurable via `MaxForwardDepth`). If the chain eventually
  returns to the configuring `merchant`, or exceeds the maximum depth, the call reverts
  with `ForwardLoop`. This means a merchant cannot configure a forward that would route
  funds back to itself (directly or transitively).

**Do fees apply to forwarding?**

- Forwarding does **not** add a separate forwarding fee. The platform fee is deducted
  first from the gross charged amount (via `deduct_fee`), producing the merchant's net
  amount. The forwarded portion is calculated only from that **post-fee net amount**:
  `forward_amount = merchant_net_amount * forward_bps / 10000`.
- `forward_to` therefore receives a share of the merchant's already-fee-reduced proceeds;
  the platform fee is never double-charged on the forwarded amount. The merchant keeps the
  remainder (`merchant_net_amount - forward_amount`).
- If `forward_amount` computes to `0` (e.g. a tiny net amount with a small `forward_bps`),
  no transfer and no `PaymentForwarded` event occurs, but the payment still completes.

**Interaction with merchant pause state**

- `set_payment_forward` / `remove_payment_forward` are guarded only by the **contract-wide
  pause** (`require_not_paused`). They are **not** blocked by a per-merchant pause
  (`require_merchant_not_paused` is not enforced for these functions), so a paused merchant
  can still configure or remove its forwarding settings. If the contract is globally paused
  or the individual function is paused, these calls revert.
- Forwarding is applied inside `complete_payment`, which also only checks the contract-wide
  pause — **not** the merchant pause — so a paused merchant's *already-created* pending
  payments still complete and still forward as configured.
- However, a paused merchant **cannot receive new payments**: `create_payment` enforces
  `require_merchant_not_paused`, so no new payment (and therefore no new forwarding event)
  can be created while the merchant is paused. In practice, forwarding only fires on
  payments created before the merchant was paused.
- If a merchant's forwarding config is removed or the `forward_to` address is later paused,
  removal simply stops future forwards; it does not retroactively affect completed payments.

### Routing

| Function                                                                        | Description                                                                  |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `get_optimal_route(customer, merchant)`                                         | Return the recommended token and path for a payment between the two parties. |
| `execute_routed_payment(customer, merchant, amount, token, currency, metadata)` | Create a payment using the optimal route determined by `get_optimal_route`.  |

### Large Payment Multi-sig

| Function                                        | Description                                                              |
| ----------------------------------------------- | ------------------------------------------------------------------------ |
| `set_large_payment_threshold(admin, threshold)` | Set the amount above which a payment requires multi-sig approval.        |
| `get_large_payment_threshold()`                 | Return the current large-payment threshold.                              |
| `propose_large_payment(admin, payment_id)`      | Submit a multi-sig proposal to approve a large payment.                  |
| `approve_large_payment(approver, payment_id)`   | Add an approval signature to a large-payment proposal.                   |
| `execute_large_payment(payment_id)`             | Execute a large payment once the required approvals have been collected. |
| `get_large_payment_proposal(payment_id)`        | Retrieve the `LargePaymentProposal` for a pending large payment.         |

### Multi-sig Admin Governance

| Function                                       | Description                                                                        |
| ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| `get_multisig_config()`                        | Return the current multi-sig configuration (admin list, required signatures, TTL). |
| `propose_action(admin, action, payload)`       | Propose an admin action that requires multi-sig approval.                          |
| `approve_action(approver, proposal_id)`        | Approve a pending admin proposal.                                                  |
| `execute_action(proposal_id)`                  | Execute an approved admin proposal.                                                |
| `reject_action(rejecter, proposal_id)`         | Reject (veto) an admin proposal.                                                   |
| `add_admin(caller, new_admin)`                 | Add a new admin to the multi-sig list.                                             |
| `remove_admin(caller, admin)`                  | Remove an admin from the multi-sig list.                                           |
| `update_required_signatures(caller, required)` | Change the number of signatures required to execute a proposal.                    |

### Merchant Verification

| Function                                                  | Description                                         |
| --------------------------------------------------------- | --------------------------------------------------- |
| `set_merchant_verification_level(admin, merchant, level)` | Assign a verification tier to a merchant.           |
| `get_merchant_verification_level(merchant)`               | Return the verification level of a merchant.        |
| `set_verification_tier_limits(admin, tier, limits)`       | Configure per-tier payment limits.                  |
| `get_tier_limits(tier)`                                   | Return the limits for a specific verification tier. |

### Payout Schedules

| Function                                                 | Description                                                          |
| -------------------------------------------------------- | -------------------------------------------------------------------- |
| `set_payout_schedule(admin, merchant, token, frequency)` | Configure how frequently accumulated merchant balances are paid out. |
| `get_payout_schedule(merchant)`                          | Return the `PayoutSchedule` for a merchant.                          |
| `trigger_scheduled_payout(merchant)`                     | Execute a pending scheduled payout for a merchant.                   |
| `get_accumulated_balance(merchant)`                      | Return a merchant's accumulated but not-yet-paid-out balance.        |

### Finality Delay

| Function                                         | Description                                                           |
| ------------------------------------------------ | --------------------------------------------------------------------- |
| `configure_finality_delay(admin, delay_seconds)` | Set a holding period before merchant settlements are finalised.       |
| `get_finality_config()`                          | Return the current finality delay configuration.                      |
| `finalize_pending_settlement(payment_id)`        | Release a settlement that has passed its finality delay.              |
| `get_pending_settlements(merchant)`              | List all settlements waiting out their finality delay for a merchant. |

### Fee Management

| Function                                            | Description                                                                        |
| --------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `set_fee_config(admin, fee_config)`                 | Set platform-wide fee tiers and basis-point rates.                                 |
| `get_fee_config()`                                  | Return the current `FeeConfig`.                                                    |
| `calculate_fee(amount, merchant)`                   | Calculate the fee for a given amount and merchant tier.                            |
| `get_merchant_fee_record(merchant)`                 | Return a merchant's cumulative fee payment record.                                 |
| `get_merchant_tier(merchant)`                       | Return the fee tier for a merchant based on volume.                                |
| `manually_set_merchant_tier(admin, merchant, tier)` | Override a merchant's fee tier manually.                                           |
| `get_tier_thresholds()`                             | Return the volume thresholds that trigger each fee tier.                           |
| `set_tier_thresholds(admin, thresholds)`            | Set the volume thresholds for fee tier promotions.                                 |
| `get_accumulated_fees()`                            | Return the total platform fees accumulated in the contract.                        |
| `withdraw_fees(admin, amount)`                      | Withdraw accumulated platform fees.                                                |
| `grant_fee_waiver(admin, merchant, config)`         | Grant a merchant a temporary or permanent fee waiver.                              |
| `revoke_fee_waiver(admin, merchant)`                | Revoke a merchant's fee waiver.                                                    |
| `get_fee_waiver(merchant)`                          | Return a merchant's `FeeWaiver` record if one exists.                              |
| `get_effective_fee_bps(merchant)`                   | Return the effective fee in basis points for a merchant after applying any waiver. |
| `configure_fee_rebate(admin, config)`               | Configure volume-based fee rebate thresholds.                                      |
| `get_rebate_accrual(merchant)`                      | Return a merchant's accrued rebate balance.                                        |
| `claim_fee_rebate(merchant)`                        | Merchant claims their accrued fee rebate.                                          |
| `set_sweep_recipient(admin, recipient)`             | Set the address that receives platform fee sweeps.                                 |
| `sweep_platform_fees(admin)`                        | Transfer all accumulated platform fees to the sweep recipient.                     |
| `get_sweep_history(limit)`                          | Return the most recent fee sweep records.                                          |
| `get_sweepable_balance()`                           | Return the balance available for sweeping.                                         |
| `set_risk_fee_config(admin, config)`                | Configure risk-score-based fee surcharges.                                         |
| `get_effective_fee_for_payment(payment_id)`         | Return the effective fee that would apply to a specific payment.                   |
| `calculate_risk_score(customer, merchant, amount)`  | Compute a fraud-risk score for a potential payment.                                |

### Rate Limiting & Fraud Controls

| Function                                            | Description                                                             |
| --------------------------------------------------- | ----------------------------------------------------------------------- |
| `set_rate_limit_config(admin, config)`              | Configure global per-address transaction rate limits.                   |
| `get_rate_limit_config()`                           | Return the current rate limit configuration.                            |
| `get_address_rate_limit(address)`                   | Return the current rate-limit state for an address.                     |
| `flag_address(admin, address, reason)`              | Flag a customer or merchant address as suspicious.                      |
| `unflag_address(admin, address)`                    | Remove the flag from an address.                                        |
| `is_address_flagged(address)`                       | Return `true` if the address is currently flagged.                      |
| `get_flag_reason(address)`                          | Return the reason an address was flagged, if any.                       |
| `add_to_allowlist(admin, address)`                  | Add an address to the admin allowlist, bypassing rate limits.           |
| `remove_from_allowlist(admin, address)`             | Remove an address from the allowlist.                                   |
| `set_merchant_rate_limit(admin, merchant, config)`  | Set a per-merchant transaction rate or volume cap.                      |
| `get_merchant_rate_limit(merchant)`                 | Return the rate-limit configuration for a specific merchant.            |
| `reset_merchant_rate_limit(admin, merchant)`        | Reset a merchant's rate-limit counters.                                 |
| `check_rate_limit(merchant, amount)`                | Return `true` if a proposed payment would not exceed rate limits.       |
| `set_customer_spend_limit(admin, customer, config)` | Set a daily/monthly spending cap for a customer.                        |
| `get_spend_limit(customer)`                         | Return a customer's spend limit configuration.                          |
| `remove_customer_spend_limit(admin, customer)`      | Remove a customer's spend limit.                                        |
| `check_spend_allowance(customer, amount)`           | Return `true` if the customer has sufficient spend allowance remaining. |

### Token Allowlist

| Function                             | Description                                            |
| ------------------------------------ | ------------------------------------------------------ |
| `add_allowed_token(admin, token)`    | Add a token contract address to the payment allowlist. |
| `remove_allowed_token(admin, token)` | Remove a token from the allowlist.                     |
| `get_allowed_tokens()`               | Return all currently allowed token addresses.          |

### Oracle & Currency Rates

| Function                                          | Description                                                |
| ------------------------------------------------- | ---------------------------------------------------------- |
| `set_conversion_rate(admin, currency, rate)`      | Manually set the conversion rate for a currency.           |
| `get_conversion_rate(currency)`                   | Return the stored conversion rate for a currency.          |
| `set_oracle_rate_config(admin, currency, config)` | Configure an on-chain oracle source for a currency's rate. |
| `get_oracle_rate_config(currency)`                | Return the oracle configuration for a currency.            |
| `refresh_conversion_rate(currency)`               | Pull a fresh rate from the configured oracle contract.     |

### Analytics

| Function                                                 | Description                                                                          |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `get_payment_analytics()`                                | Return platform-wide payment analytics totals.                                       |
| `get_merchant_analytics(merchant)`                       | Return analytics for a specific merchant.                                            |
| `get_merchant_total_volume(merchant)`                    | Return the all-time payment volume for a merchant.                                   |
| `get_customer_analytics(customer)`                       | Return analytics for a specific customer.                                            |
| `get_customer_top_merchants(customer, limit)`            | Return the merchants a customer has paid most.                                       |
| `get_customer_monthly_volume(customer, month_timestamp)` | Return a customer's total volume for the billing month containing `month_timestamp`. |
| `get_merchant_analytics_range(merchant, from, to)`       | Return per-day analytics buckets for a merchant over a date range.                   |
| `get_platform_analytics_daily(day_timestamp)`            | Return the platform-wide analytics bucket for a specific day.                        |
| `get_top_merchants_by_volume(limit)`                     | Return the top-N merchants ranked by total volume.                                   |

### Loyalty

| Function                                                | Description                                                             |
| ------------------------------------------------------- | ----------------------------------------------------------------------- |
| `configure_loyalty(admin, config)`                      | Configure the loyalty points programme (accrual rate, redemption rate). |
| `get_loyalty_balance(customer)`                         | Return a customer's current loyalty point balance.                      |
| `redeem_points(customer, merchant, points, payment_id)` | Redeem loyalty points as a discount on a payment.                       |

### Pause Controls

| Function                                       | Description                                                        |
| ---------------------------------------------- | ------------------------------------------------------------------ |
| `pause_contract(admin, reason)`                | Pause the entire contract, blocking all state-changing operations. |
| `unpause_contract(admin)`                      | Resume the contract after a pause.                                 |
| `pause_function(admin, function_name, reason)` | Pause a single named function.                                     |
| `unpause_function(admin, function_name)`       | Resume a previously paused function.                               |
| `pause_merchant(admin, merchant)`              | Prevent a specific merchant from receiving new payments.           |
| `unpause_merchant(admin, merchant)`            | Allow a merchant to receive payments again.                        |
| `get_pause_state()`                            | Return the current `PauseState` for the contract.                  |
| `is_function_paused(function_name)`            | Return `true` if the named function is currently paused.           |

### Auto-Escrow

| Function                                      | Description                                                                         |
| --------------------------------------------- | ----------------------------------------------------------------------------------- |
| `set_auto_escrow_rule(admin, merchant, rule)` | Configure a rule to automatically escrow payments above a threshold for a merchant. |
| `get_auto_escrow_rule(merchant)`              | Return the auto-escrow rule for a merchant.                                         |
| `remove_auto_escrow_rule(admin, merchant)`    | Remove the auto-escrow rule for a merchant.                                         |

### Payment Metadata & Memos

| Function                                               | Description                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------------- |
| `set_payment_metadata(admin, payment_id, metadata)`    | Attach structured metadata to a payment.                            |
| `get_payment_metadata(payment_id)`                     | Return the metadata attached to a payment.                          |
| `verify_metadata_integrity(payment_id, expected_hash)` | Return `true` if the stored metadata matches the expected hash.     |
| `set_payment_memo(caller, payment_id, memo)`           | Attach a versioned free-text memo to a payment.                     |
| `get_payment_memo(payment_id)`                         | Return the most recent memo attached to a payment.                  |
| `verify_memo_integrity(payment_id, plaintext_hash)`    | Return `true` if the stored memo matches the expected hash.         |
| `verify_memo_reference(payment_id, reference)`         | Return `true` if the memo contains the expected external reference. |

### Payment Tags & Invoices

| Function                                      | Description                                                       |
| --------------------------------------------- | ----------------------------------------------------------------- |
| `tag_payment(caller, payment_id, tag)`        | Attach a 32-byte tag hash to a payment for custom categorisation. |
| `get_payment_tags(payment_id)`                | Return all tags attached to a payment.                            |
| `remove_payment_tag(caller, payment_id, tag)` | Remove a specific tag from a payment.                             |
| `attach_invoice(caller, payment_id, invoice)` | Attach an invoice record to a payment.                            |
| `get_invoice(invoice_id)`                     | Return an invoice by its ID.                                      |
| `get_payment_invoice(payment_id)`             | Return the invoice attached to a payment.                         |
| `verify_invoice_total(invoice_id)`            | Return `true` if the invoice line items sum to the invoice total. |

### Subscription Proration

| Function                                                      | Description                                                                       |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `set_subscription_proration(admin, subscription_id, enabled)` | Enable or disable billing proration when a subscription is resumed after a pause. |

---

## Data Types

Key types referenced by the functions above:

- **`Payment`** — stores `id`, `customer`, `merchant`, `amount`, `token`, `currency`, `status`, `created_at`, `expires_at`, and `metadata`.
- **`PaymentStatus`** — `Pending | Completed | Refunded | PartialRefunded | Cancelled`
- **`Currency`** — `XLM | USDC | USDT | BTC | ETH`
- **`Subscription`** — full subscription record including trial, pause, and dunning state.
- **`PaymentChannel`** — off-chain channel state including deposited balance and settlement nonce.
- **`MultiSigConfig`** — admin list, required signatures, and proposal TTL.

---

## 📡 Events

The contract emits Soroban events for all state-changing operations. Off-chain integrators (Horizon subscribers, indexers) can subscribe to these events to monitor payment lifecycle, subscriptions, fees, and governance actions.

### Core Payment Events

| Event              | Topic Name         | Payload Fields                                            | Fires When                                                |
| ------------------ | ------------------ | --------------------------------------------------------- | --------------------------------------------------------- |
| `PaymentCreated`   | `PaymentCreated`   | `payment_id`, `customer`, `merchant`, `amount`            | `create_payment()` succeeds, payment stored as `Pending`  |
| `PaymentCompleted` | `PaymentCompleted` | `payment_id`, `merchant`, `amount`                        | `complete_payment()` succeeds, funds released to merchant |
| `PaymentRefunded`  | `PaymentRefunded`  | `payment_id`, `customer`, `amount`                        | `refund_payment()` succeeds, funds returned to customer   |
| `PaymentCancelled` | `PaymentCancelled` | `payment_id`, `cancelled_by`, `timestamp`                 | `cancel_payment()` succeeds                               |
| `PaymentExpired`   | `PaymentExpired`   | `payment_id`, `customer`, `refunded_amount`, `expired_at` | `expire_payment()` called after expiration window passes  |

### Installment Payment Events

| Event              | Topic Name         | Payload Fields                                                                | Fires When                                                                                 |
| ------------------ | ------------------ | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `InstallmentPaid`  | `InstallmentPaid`  | `payment_id`, `installment_number`, `amount`, `remaining`, `payer`, `paid_at` | `pay_installment()` succeeds, records partial payment                                      |
| `PaymentFullyPaid` | `PaymentFullyPaid` | `payment_id`, `total_installments`, `completed_at`                            | `finalize_installment_payment()` marks payment `Completed` after all installments received |

### Escrowed Payment Events

| Event                      | Topic Name                 | Payload Fields                               | Fires When                                                                 |
| -------------------------- | -------------------------- | -------------------------------------------- | -------------------------------------------------------------------------- |
| `EscrowedPaymentCreated`   | `EscrowedPaymentCreated`   | `payment_id`, `escrow_id`, `escrow_contract` | `create_escrowed_payment()` succeeds, escrow contract address recorded     |
| `EscrowedPaymentCompleted` | `EscrowedPaymentCompleted` | `payment_id`, `escrow_id`                    | `complete_escrowed_payment()` releases funds from escrow                   |
| `EscrowedPaymentCancelled` | `EscrowedPaymentCancelled` | `payment_id`, `escrow_id`                    | `cancel_escrowed_payment()` returns funds to customer                      |
| `EscrowedPaymentDisputed`  | `EscrowedPaymentDisputed`  | `payment_id`, `raised_by`                    | `dispute_escrowed_payment()` opens a dispute on escrowed payment           |
| `PaymentDisputeResolved`   | `PaymentDisputeResolved`   | `payment_id`, `favor_customer`               | `resolve_escrowed_payment_dispute()` settles dispute in favor of one party |

### Subscription Events

| Event                      | Topic Name                 | Payload Fields                                                  | Fires When                                              |
| -------------------------- | -------------------------- | --------------------------------------------------------------- | ------------------------------------------------------- |
| `SubscriptionCreated`      | `SubscriptionCreated`      | `subscription_id`, `customer`, `merchant`, `amount`, `interval` | `create_subscription()` succeeds                        |
| `RecurringPaymentExecuted` | `RecurringPaymentExecuted` | `subscription_id`, `payment_count`, `amount`, `next_payment_at` | `execute_recurring_payment()` completes a billing cycle |
| `RecurringPaymentFailed`   | `RecurringPaymentFailed`   | `subscription_id`, `retry_count`                                | `execute_recurring_payment()` fails and enters dunning  |
| `SubscriptionCancelled`    | `SubscriptionCancelled`    | `subscription_id`, `cancelled_by`                               | `cancel_subscription()` terminates subscription         |

### Subscription Trial & Pause Events

| Event                         | Topic Name                    | Payload Fields                                                                  | Fires When                                                                      |
| ----------------------------- | ----------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `TrialStarted`                | `TrialStarted`                | `subscription_id`, `trial_ends_at`                                              | `create_subscription()` with `trial_period_seconds > 0`                         |
| `TrialConverted`              | `TrialConverted`              | `subscription_id`, `converted_at`                                               | Trial period ends and subscription begins billing                               |
| `TrialCancelled`              | `TrialCancelled`              | `subscription_id`, `cancelled_at`                                               | `cancel_subscription()` called during trial period                              |
| `SubscriptionPaused`          | `SubscriptionPaused`          | `subscription_id`                                                               | `pause_subscription()` pauses billing                                           |
| `SubscriptionResumed`         | `SubscriptionResumed`         | `subscription_id`, `next_payment_at`                                            | `resume_subscription()` resumes without proration                               |
| `SubscriptionResumedProrated` | `SubscriptionResumedProrated` | `subscription_id`, `pause_duration`, `new_next_billing_date`, `prorated_amount` | `resume_subscription()` with `proration_enabled=true` adjusts next billing date |

### Metered Billing Events

| Event                    | Topic Name               | Payload Fields                              | Fires When                                                  |
| ------------------------ | ------------------------ | ------------------------------------------- | ----------------------------------------------------------- |
| `UsageReported`          | `UsageReported`          | `subscription_id`, `units`, `accumulated`   | `report_usage()` records consumption                        |
| `MeteredBillingExecuted` | `MeteredBillingExecuted` | `subscription_id`, `amount`, `units_billed` | `execute_metered_billing()` charges for accumulated usage   |
| `BillingCapReached`      | `BillingCapReached`      | `subscription_id`, `cap`                    | `execute_metered_billing()` hits the configured billing cap |

### Dunning Events

| Event                        | Topic Name                   | Payload Fields                                | Fires When                                          |
| ---------------------------- | ---------------------------- | --------------------------------------------- | --------------------------------------------------- |
| `SubscriptionEnteredDunning` | `SubscriptionEnteredDunning` | `subscription_id`, `attempt`, `next_retry_at` | Subscription enters dunning after payment failure   |
| `DunningRetryScheduled`      | `DunningRetryScheduled`      | `subscription_id`, `retry_at`                 | Dunning retry scheduled for a failed subscription   |
| `SubscriptionSuspended`      | `SubscriptionSuspended`      | `subscription_id`, `reason`                   | Subscription suspended due to max retries exceeded  |
| `DunningResolved`            | `DunningResolved`            | `subscription_id`, `resolved_at`              | `resolve_dunning()` manually resolves dunning state |

### Payment Channel Events

| Event                  | Topic Name             | Payload Fields                                     | Fires When                                                |
| ---------------------- | ---------------------- | -------------------------------------------------- | --------------------------------------------------------- |
| `ChannelOpened`        | `ChannelOpened`        | `channel_id`, `customer`, `merchant`, `amount`     | `open_channel()` creates a new off-chain channel          |
| `ChannelSettled`       | `ChannelSettled`       | `channel_id`, `merchant_amount`, `customer_refund` | `settle_channel()` finalizes settlement on-chain          |
| `ChannelExpiredClosed` | `ChannelExpiredClosed` | `channel_id`, `refunded_to`                        | `close_channel_expired()` refunds expired channel balance |

### Split Payment Events

| Event                | Topic Name | Payload Fields | Fires When                                                                                |
| -------------------- | ---------- | -------------- | ----------------------------------------------------------------------------------------- |
| (No dedicated event) | —          | —              | Split payment events tracked via `PaymentCreated` + `execute_split_settlement()` via fees |

### Fee Events

| Event                  | Topic Name             | Payload Fields                                                      | Fires When                                                    |
| ---------------------- | ---------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------- |
| `FeeCollected`         | `FeeCollected`         | `payment_id`, `fee_amount`, `merchant`                              | Fees deducted during `complete_payment()`                     |
| `FeesWithdrawn`        | `FeesWithdrawn`        | `amount`, `treasury`                                                | `withdraw_fees()` transfers accumulated fees                  |
| `MerchantTierUpgraded` | `MerchantTierUpgraded` | `merchant`, `old_tier`, `new_tier`                                  | Merchant's fee tier increases after reaching volume threshold |
| `FeeWaiverGranted`     | `FeeWaiverGranted`     | `merchant`, `waiver_bps`, `valid_until`                             | `grant_fee_waiver()` creates waiver record                    |
| `FeeWaiverRevoked`     | `FeeWaiverRevoked`     | `merchant`, `revoked_by`                                            | `revoke_fee_waiver()` removes waiver                          |
| `FeeWaiverExpired`     | `FeeWaiverExpired`     | `merchant`                                                          | Fee waiver validity period expires                            |
| `FeeConfigUpdated`     | `FeeConfigUpdated`     | `fee_bps`, `treasury`                                               | `set_fee_config()` updates fee structure                      |
| `RiskFeeApplied`       | `RiskFeeApplied`       | `payment_id`, `base_fee_bps`, `risk_surcharge_bps`, `total_fee_bps` | `create_payment()` applies dynamic risk surcharge             |

### Fraud & Rate Limit Events

| Event               | Topic Name          | Payload Fields             | Fires When                                   |
| ------------------- | ------------------- | -------------------------- | -------------------------------------------- |
| `AddressFlagged`    | `AddressFlagged`    | `address`, `reason`        | `flag_address()` marks address as suspicious |
| `AddressUnflagged`  | `AddressUnflagged`  | `address`                  | `unflag_address()` removes fraud flag        |
| `RateLimitBreached` | `RateLimitBreached` | `address`, `payment_count` | Payment attempt blocked by rate limit        |

### Conditional Payment Events

| Event                       | Topic Name                  | Payload Fields                      | Fires When                                                                 |
| --------------------------- | --------------------------- | ----------------------------------- | -------------------------------------------------------------------------- |
| `ConditionalPaymentCreated` | `ConditionalPaymentCreated` | `payment_id`, `condition_type`      | `create_conditional_payment()` succeeds                                    |
| `ConditionEvaluated`        | `ConditionEvaluated`        | `payment_id`, `met`, `evaluated_at` | `evaluate_condition()` or `execute_if_condition_met()` evaluates condition |

### Auto-Escrow Events

| Event                 | Topic Name            | Payload Fields                                                   | Fires When                                                                 |
| --------------------- | --------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `AutoEscrowTriggered` | `AutoEscrowTriggered` | `payment_id`, `merchant`, `escrow_id`, `amount`, `escrow_amount` | Payment exceeds merchant's auto-escrow threshold during `create_payment()` |

### Large Payment Multi-sig Events

| Event                          | Topic Name                     | Payload Fields                                               | Fires When                                                      |
| ------------------------------ | ------------------------------ | ------------------------------------------------------------ | --------------------------------------------------------------- |
| `LargePaymentProposed`         | `LargePaymentProposed`         | `payment_id`, `proposer`, `required_approvals`, `expires_at` | Payment exceeds large-payment threshold in `complete_payment()` |
| `LargePaymentApproved`         | `LargePaymentApproved`         | `payment_id`, `approver`, `approval_count`                   | `approve_large_payment()` adds approval                         |
| `LargePaymentExecuted`         | `LargePaymentExecuted`         | `payment_id`                                                 | `execute_large_payment()` releases large payment                |
| `LargePaymentThresholdUpdated` | `LargePaymentThresholdUpdated` | `threshold`, `updated_by`                                    | `set_large_payment_threshold()` changes threshold               |

### Multi-sig Governance Events

| Event            | Topic Name       | Payload Fields                              | Fires When                                     |
| ---------------- | ---------------- | ------------------------------------------- | ---------------------------------------------- |
| `ActionProposed` | `ActionProposed` | `proposal_id`, `proposer`, `action_type`    | `propose_action()` creates governance proposal |
| `ActionApproved` | `ActionApproved` | `proposal_id`, `approver`, `approval_count` | `approve_action()` adds approval to proposal   |
| `ActionExecuted` | `ActionExecuted` | `proposal_id`                               | `execute_action()` executes approved proposal  |
| `ActionRejected` | `ActionRejected` | `proposal_id`, `rejected_by`                | `reject_action()` vetoes proposal              |
| `AdminAdded`     | `AdminAdded`     | `admin`                                     | `add_admin()` adds new admin to multi-sig list |
| `AdminRemoved`   | `AdminRemoved`   | `admin`                                     | `remove_admin()` removes admin from list       |

### Contract Control Events

| Event                   | Topic Name              | Payload Fields                         | Fires When                                             |
| ----------------------- | ----------------------- | -------------------------------------- | ------------------------------------------------------ |
| `ContractPausedEvent`   | `ContractPausedEvent`   | `paused_by`, `reason`, `paused_at`     | `pause_contract()` halts all state-changing operations |
| `ContractUnpausedEvent` | `ContractUnpausedEvent` | `unpaused_by`, `unpaused_at`           | `unpause_contract()` resumes operations                |
| `FunctionPausedEvent`   | `FunctionPausedEvent`   | `function_name`, `paused_by`, `reason` | `pause_function()` pauses specific function            |
| `FunctionUnpausedEvent` | `FunctionUnpausedEvent` | `function_name`, `unpaused_by`         | `unpause_function()` resumes function                  |

### Payment Metadata & Memo Events

| Event                         | Topic Name                    | Payload Fields                                           | Fires When                                                  |
| ----------------------------- | ----------------------------- | -------------------------------------------------------- | ----------------------------------------------------------- |
| `PaymentMetadataSet`          | `PaymentMetadataSet`          | `payment_id`, `content_ref`, `encrypted`, `set_by`       | `set_payment_metadata()` attaches structured metadata       |
| `PaymentMetadataUpdated`      | `PaymentMetadataUpdated`      | `payment_id`, `content_ref`, `updated_by`, `version`     | `set_payment_metadata()` updates existing metadata          |
| `PaymentMemoSet`              | `PaymentMemoSet`              | `payment_id`, `memo_ref`, `set_by`                       | `set_payment_memo()` attaches memo                          |
| `PaymentMemoUpdated`          | `PaymentMemoUpdated`          | `payment_id`, `memo_ref`, `updated_by`, `version`        | `set_payment_memo()` updates existing memo                  |
| `PaymentMemoVerified`         | `PaymentMemoVerified`         | `payment_id`, `memo_hash`, `verified_at`, `verified_by`  | `verify_memo_integrity()` confirms memo authenticity        |
| `PaymentForwardConfigSet`     | `PaymentForwardConfigSet`     | `merchant`, `forward_to`, `forward_bps`                  | `set_payment_forward()` configures forwarding               |
| `PaymentForwardConfigRemoved` | `PaymentForwardConfigRemoved` | `merchant`                                               | `remove_payment_forward()` removes forwarding               |
| `PaymentForwarded`            | `PaymentForwarded`            | `payment_id`, `merchant`, `forward_to`, `forward_amount` | `complete_payment()` forwards portion to configured address |

---

## Error Codes

Errors are grouped into five ranges:

| Range   | Category                                                       |
| ------- | -------------------------------------------------------------- |
| 100–126 | `BasicError` — auth, metadata, rate limits, multi-sig setup    |
| 200–224 | `PaymentError` — payment lifecycle violations                  |
| 300–318 | `SubscriptionError` — subscription and dunning violations      |
| 400–406 | `ProposalError` — multi-sig proposal violations                |
| 500–540 | `FeatureError` — channels, splits, loyalty, escrow, forwarding |

See [`ERRORS.md`](./ERRORS.md) for the full list.

---

## Usage Example

The example below shows the minimum steps to create and complete a payment using the [Stellar CLI](https://developers.stellar.org/docs/tools/cli).

```bash
# 1. Deploy and initialise
stellar contract invoke --id $CONTRACT_ID \
  -- initialize \
  --admin $ADMIN_ADDRESS

# 2. Add an allowed token (e.g. USDC)
stellar contract invoke --id $CONTRACT_ID \
  -- add_allowed_token \
  --admin $ADMIN_ADDRESS \
  --token $USDC_TOKEN_ADDRESS

# 3. Customer creates a payment
stellar contract invoke --id $CONTRACT_ID \
  --source $CUSTOMER_KEY \
  -- create_payment \
  --customer $CUSTOMER_ADDRESS \
  --merchant $MERCHANT_ADDRESS \
  --amount 10000000 \
  --token $USDC_TOKEN_ADDRESS \
  --currency USDC \
  --expiration_duration 86400 \
  --metadata '"order-42"'
# → Returns payment_id, e.g. 1

# 4. Admin completes the payment (releases funds to merchant)
stellar contract invoke --id $CONTRACT_ID \
  --source $ADMIN_KEY \
  -- complete_payment \
  --admin $ADMIN_ADDRESS \
  --payment_id 1

# 5. Query the payment
stellar contract invoke --id $CONTRACT_ID \
  -- get_payment \
  --payment_id 1
```

---

## See Also

- [Root README](../../README.md) — architecture overview and workspace setup
- [Escrow Contract](../escrow/README.md)
- [Refund Contract](../refund/README.md)
