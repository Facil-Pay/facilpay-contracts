# Glossary

A comprehensive reference of domain terminology, architectural concepts, contract storage models, and mechanisms across the FacilPay smart contract suite on Stellar/Soroban.

---

## Arbitration

An administrative and decentralized dispute resolution mechanism used when a customer escalates a denied refund request for formal dispute adjudication. Registered third-party arbitrators evaluate evidence and cast votes to determine whether the rejection should be upheld or overturned into an approved refund. The workflow supports configurable case timeouts, minimum quorum thresholds, arbitrator reputation scoring, majority fee pool distribution, and an optional token staking requirement to deter frivolous disputes.

- **README Reference:** [Refund Contract README — Arbitration Workflow](../contracts/refund/README.md#️-arbitration-workflow), [Arbitration Functions](../contracts/refund/README.md#arbitration), and [Arbitration Stake Requirement](../contracts/refund/README.md#-arbitration-stake-requirement)
- **Key Functions:**
  - `escalate_to_arbitration(caller: Address, refund_id: u64, fee_token: Address, fee_amount: i128) -> Result<u64, Error>`
  - `cast_arbitration_vote(arbitrator: Address, case_id: u64, vote_for_refund: bool) -> Result<(), Error>`
  - `close_arbitration_case(case_id: u64) -> Result<(), Error>`
  - `trigger_arbitration_timeout(case_id: u64) -> Result<(), Error>`
  - `set_arbitration_timeout(admin: Address, timeout_seconds: u64) -> Result<(), Error>`
  - `set_arbitration_fee_config(admin: Address, config: ArbitrationFeeConfig) -> Result<(), Error>`
  - `set_arbitration_stake_config(admin: Address, config: ArbitrationStakeConfig) -> Result<(), Error>`
- **Associated Errors:**
  - `CoreError::RefundNotFound` — Refund ID does not exist in storage.
  - `CoreError::InvalidStatus` — Refund status is not `Rejected` or `PendingAppeal`.
  - `CoreError::AlreadyProcessed` — An arbitration case has already been opened for this refund ID.
  - `CoreError::InvalidAmount` — Fee amount or stake amount is non-positive or below configured minimums.
  - `ExtError::QuorumNotReached` — Closing attempted before the minimum number of arbitrator votes was submitted.
  - `ExtError::CaseNotFound` — Arbitration case ID does not exist.
  - `ExtError::CaseAlreadyClosed` — Action attempted on an arbitration case that has already settled.
  - `ExtError::CaseExpired` — Operation attempted after the arbitration window timed out.
  - `ExtError::ArbitratorNotAssigned` — Caller is not in the assigned arbitrator panel for the case.

---

## Basis Points (bps)

A standardized financial unit of measure equal to one-hundredth of a percentage point (1 bp = 0.01% = 1/10,000, such that 10,000 bps equals 100%). Used throughout the contracts to perform exact integer-based percentage arithmetic without rounding inaccuracies or floating-point non-determinism. Basis points define platform fees, volume rebate rates, payment forwarding shares, group subscription bundle discounts, customer tier refund caps, circuit breaker rate limits, and multi-party escrow release weights.

- **README Reference:** [Payment Contract README — Fee Management](../contracts/payment/README.md#fee-management), [Payment Contract README — Subscription Groups](../contracts/payment/README.md#how-subscription-groups-work), [Refund Contract README — Circuit Breaker](../contracts/refund/README.md#how-the-circuit-breaker-works), and [Escrow Contract README — Sub-Accounts](../contracts/escrow/README.md#sub-accounts)
- **Key Functions:**
  - `calculate_fee(amount: i128, merchant: Address) -> Result<i128, Error>`
  - `set_payment_forward(merchant: Address, forward_to: Address, forward_bps: u32) -> Result<(), Error>`
  - `create_subscription_group(owner: Address, discount_bps: u32) -> Result<u64, Error>`
  - `set_customer_tier_policy(merchant: Address, tier_id: u32, max_refund_bps: u32) -> Result<(), Error>`
  - `get_applicable_refund_bps(merchant: Address, payment_id: u64) -> Result<u32, Error>`
  - `configure_fee_rebate(admin: Address, config: FeeRebateConfig) -> Result<(), Error>`
- **Associated Errors:**
  - `BasicError::InvalidAmount` — Basis point value out of allowed range (e.g., exceeding 10,000 bps).
  - `FeatureError::InvalidForwardBps` — Forwarding basis points specified as 0 or greater than 10,000.
  - `CoreError::InvalidAmount` — Refund basis points cap configured outside 0–10,000.

---

## Circuit Breaker

An automated contract protection mechanism that halts new refund requests when the ratio of refunded volume to payment volume spikes abnormally over a rolling measurement window. Designed to protect merchant balances and contract liquidity against compromised credentials, integration bugs, or rapid-fire refund abuse. When tripped, calls to `request_refund()` revert with `CircuitBreakerTripped` until an automatic cooldown timer expires or an authorized admin manually resets the breaker.

- **README Reference:** [Refund Contract README — How the Circuit Breaker Works](../contracts/refund/README.md#how-the-circuit-breaker-works) and [Refund Contract README — Pause / Circuit Breaker](../contracts/refund/README.md#pause--circuit-breaker)
- **Key Functions:**
  - `set_circuit_breaker_config(admin: Address, config: CircuitBreakerConfig) -> Result<(), Error>`
  - `get_circuit_breaker_state() -> CircuitBreakerState`
  - `reset_circuit_breaker(admin: Address) -> Result<(), Error>`
  - `check_circuit_breaker() -> bool`
- **Associated Errors:**
  - `CoreError::CircuitBreakerTripped` (code `29`) — Refund rate exceeded `max_refund_rate_bps` within `measurement_window_seconds`, tripping the breaker.
  - `CoreError::Unauthorized` (code `1`) — Caller attempting to configure or reset the breaker is not the contract admin.

---

## Clawback

An admin-controlled emergency fund-recovery mechanism for the escrow contract. When normal resolution paths (release, dispute, refund) are unavailable — for example, due to fraud, a compliance hold, or an irrecoverable deadlock — a multisig admin can forcibly recover the full escrow balance and transfer it to their own address.

- **README Reference:** [Escrow Contract README — Clawback](../contracts/escrow/README.md#clawback)

### Lifecycle

Clawback follows a strict three-phase sequence:

1. **Initiate** — An admin calls `initiate_clawback(admin, escrow_id, reason_hash, delay_seconds)`. The `reason_hash` is a 32-byte Keccak-256 (or equivalent) hash of an off-chain document that records the justification. The mandatory `delay_seconds` must be at least 86,400 seconds (24 hours), giving all parties a window to contest or seek remediation before funds move. A unique `request_id` is returned and stored on-chain.

2. **Execute** — After the delay elapses, any admin can call `execute_clawback(admin, request_id)`. The full escrow amount is transferred from the contract to the **admin's address** (not to the original customer), and the escrow status is updated to `Resolved`.

3. **Cancel** — Any admin can call `cancel_clawback(admin, request_id)` at any time before execution to abort the request. Cancellation is final — a cancelled request cannot be re-activated. To retry, a new initiation must be filed.

### Key constraints

- Only registered multisig admins may initiate, execute, or cancel a clawback.
- Only one active (non-executed, non-cancelled) clawback request may exist per escrow at a time. A second initiation for the same escrow while a live request exists returns `AlreadyProcessed`.
- Executing before the delay elapses returns `ActionError::NotReady`.
- No fees are deducted — the entire locked amount transfers to the admin.

### Error reference

| Error                                | Cause                                                                                                     |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------- |
| `BasicError::NotAnAdmin`             | Caller is not in the multisig admin set                                                                   |
| `EscrowError::ClawbackDelayTooShort` | `delay_seconds < 86,400`                                                                                  |
| `EscrowError::NotFound`              | Target escrow does not exist                                                                              |
| `EscrowError::AlreadyProcessed`      | A live clawback request already exists for this escrow (on initiate), or the request was already executed |
| `EscrowError::InvalidStatus`         | Request was already cancelled (on execute)                                                                |
| `ActionError::NotReady`              | Execution attempted before `execute_after` timestamp                                                      |
| `BasicError::Unauthorized`           | `request_id` not found in storage                                                                         |

---

## Dunning

The automated retry and recovery workflow executed when a recurring subscription payment fails (for instance, due to insufficient funds or expired allowances). When an execution fails during `execute_recurring_payment`, the subscription status shifts to `InDunning`, initializing a `DunningState` tracking retry counts and exponential backoff deadlines. The subscription returns to `Active` if a subsequent retry succeeds, or is permanently marked `Suspended` if retries exceed `max_retries`.

- **README Reference:** [Payment Contract README — Dunning (Failed Payment Recovery)](../contracts/payment/README.md#dunning-failed-payment-recovery) and [Payment Contract README — Subscriptions](../contracts/payment/README.md#subscriptions)
- **Key Functions:**
  - `set_dunning_config(admin: Address, config: DunningConfig) -> Result<(), Error>`
  - `get_dunning_config() -> Option<DunningConfig>`
  - `get_dunning_state(subscription_id: u64) -> Result<DunningState, Error>`
  - `retry_failed_payment(subscription_id: u64) -> Result<(), Error>`
  - `resolve_dunning(admin: Address, subscription_id: u64) -> Result<(), Error>`
- **Associated Errors:**
  - `SubscriptionError::NotFound` (code `300`) — Specified `subscription_id` does not exist.
  - `SubscriptionError::NotInDunning` (code `307`) — Target subscription is not currently in `InDunning` status.
  - `SubscriptionError::DunningNotFound` (code `308`) — No active `DunningState` record found for the subscription.
  - `SubscriptionError::RetryTooEarly` (code `309`) — Retry called before the `next_retry_at` backoff timestamp.
  - `SubscriptionError::MaxRetriesExceeded` (code `310`) — Maximum retry limit reached; subscription must be suspended or manually resolved.

---

## Escrow

A smart contract that locks funds between a customer and a merchant while a transaction is pending, disputed, or held. Escrows track status through a lifecycle of `Locked`, `Released`, `Disputed`, `Resolved`, and `Cancelled`, and provide independent sub-account allocations, dispute arbitration, observer audit access, and hierarchical release guards.

- **README Reference:** [Escrow Contract README — Public Functions](../contracts/escrow/README.md#public-functions) and [Root README — Contract Overview](../README.md#-contract-overview)
- **Key Functions:**
  - `create_escrow(customer: Address, merchant: Address, token: Address, amount: i128, release_timestamp: u64, min_hold_period: u64, expiry_timestamp: u64, auto_refund_on_expiry: bool) -> Result<u64, Error>`
  - `release_escrow(admin: Address, escrow_id: u64) -> Result<(), Error>`
  - `dispute_escrow(caller: Address, escrow_id: u64) -> Result<(), Error>`
- **Associated Errors:**
  - `EscrowError::NotFound` (code `200`) — Target escrow ID does not exist.
  - `EscrowError::InvalidStatus` (code `201`) — Target escrow is in an incompatible status for the requested transition.
  - `EscrowError::ChildrenNotResolved` (code `208`) — Parent escrow cannot release while child escrows remain unresolved.

---

## Finality Delay

A payment-settlement feature that holds merchant funds in a pending settlement state for a configurable period after payment completion before releasing them. Payments with amounts below a configurable `min_amount_threshold` bypass the delay and settle immediately to optimize cash flow for low-risk micro-transactions.

- **README Reference:** [Payment Contract README — Finality Delay](../contracts/payment/README.md#finality-delay)
- **Key Functions:**
  - `configure_finality_delay(admin: Address, delay_seconds: u64) -> Result<(), Error>`
  - `get_finality_config() -> Option<FinalityConfig>`
  - `finalize_pending_settlement(payment_id: u64) -> Result<(), Error>`
  - `get_pending_settlements(merchant: Address) -> Vec<PendingSettlement>`
- **Associated Errors:**
  - `PaymentError::SettlementNotReady` — Settlement attempted before the holding period has elapsed.
  - `PaymentError::NotFound` — Payment ID has no pending settlement record.

---

## Horizon

The Stellar network's HTTP API used by off-chain services, indexers, and frontend applications to ingest ledger events and contract state changes. Horizon subscriptions ingest events such as `PaymentCreated`, `EscrowCreated`, `RefundRequested`, and `CircuitBreakerTripped` in real time without polling contract storage.

- **README Reference:** [Root README — Architecture](../README.md#️-architecture), [Payment Contract README — Events](../contracts/payment/README.md#-events), and [Refund Contract README — Events](../contracts/refund/README.md#-events)

---

## Instance Storage

A Soroban storage tier (`env.storage().instance()`) where data entries are stored directly alongside the contract instance entry and share its ledger Time-To-Live (TTL). Instance storage is optimized for compact, contract-global state such as administrative addresses, fee structures, multi-sig signer sets, schema versions, and global sequential counters. Because instance storage has a strict size limit and is loaded in full on every contract invocation, it should never be used for unbounded or user-proportional data collections.

- **README Reference:** [Storage Schema Versioning Guide](STORAGE_VERSIONING.md#📐-how-storage-schema-versions-are-tracked) and [Admin Contract README — Role & Permission Model](../contracts/admin/README.md#role--permission-model)
- **Key Functions:**
  - `get_schema_version() -> u32`
  - `migrate_schema(admin: Address, target_version: u32) -> Result<(), Error>`
  - `initialize(admin: Address, ...) -> Result<(), Error>`
- **Associated Errors:**
  - `BasicError::AlreadyInitialized` (code `101`) / `CoreError::AlreadyInitialized` (code `10`) — Contract instance has already been initialized.
  - `ExtError::SchemaAlreadyAtTarget` (code `17`) — Requested schema migration version is less than or equal to current schema version.

---

## Multisig

Multi-signature governance model used for admin operations across contracts. Actions require a configurable number of approvals from an admin set before they execute, following a proposal-based workflow.

- **README Reference:** [Payment Contract README — Multi-sig Admin Governance](../contracts/payment/README.md#multi-sig-admin-governance) and [Admin Contract README — Role & Permission Model](../contracts/admin/README.md#role--permission-model)
- **Key Functions:**
  - `propose_action(admin: Address, action: AdminAction, payload: Bytes) -> Result<u64, Error>`
  - `approve_action(approver: Address, proposal_id: u64) -> Result<(), Error>`
  - `execute_action(proposal_id: u64) -> Result<(), Error>`
- **Associated Errors:**
  - `BasicError::Unauthorized` (code `100`) — Caller is not in the authorized multisig signer set.
  - `ProposalError::AlreadyApproved` (code `402`) — Approver has already signed the given proposal.
  - `ProposalError::QuorumNotMet` (code `404`) — Proposal lacks required approval threshold for execution.

---

## Payment Channel

An off-chain payment structure allowing a customer and a merchant to conduct high-frequency micropayments without committing every transaction to the Stellar ledger. The customer opens a channel by locking token collateral into the payment contract via `open_channel`, exchanging signed balance balance commitments off-chain with the merchant. When settled on-chain via `settle_channel`, the merchant presents the customer's Ed25519 signature over the final merchant payout and nonce, transferring earnings to the merchant and refunding the unused balance to the customer.

- **README Reference:** [Payment Contract README — Payment Channels](../contracts/payment/README.md#payment-channels) and [Payment Contract README — Payment Channel Events](../contracts/payment/README.md#payment-channel-events)
- **Key Functions:**
  - `open_channel(customer: Address, merchant: Address, token: Address, amount: i128, expires_at: u64, customer_pk: BytesN<32>) -> Result<u64, Error>`
  - `top_up_channel(customer: Address, channel_id: u64, amount: i128) -> Result<(), Error>`
  - `settle_channel(channel_id: u64, merchant_amount: i128, nonce: u64, signature: BytesN<64>) -> Result<(), Error>`
  - `close_channel_expired(channel_id: u64) -> Result<(), Error>`
  - `get_channel(channel_id: u64) -> Option<PaymentChannel>`
- **Associated Errors:**
  - `BasicError::InvalidAmount` (code `103`) — Channel deposit or top-up amount is non-positive.
  - `FeatureError::InvalidCounterparty` (code `512`) — Counterparty specified as zero address.
  - `FeatureError::ChannelNotFound` (code `513`) — Channel ID does not exist in storage.
  - `FeatureError::ChannelClosed` (code `514`) — Operation attempted on an already closed channel.
  - `FeatureError::ChannelExpired` (code `515`) — Channel settlement submitted after expiration timestamp.
  - `FeatureError::InvalidNonce` (code `516`) — Settlement nonce is less than or equal to current settled nonce.
  - `BasicError::InvalidSignature` (code `110`) — Off-chain customer balance signature failed Ed25519 cryptographic verification.

---

## Persistent Storage

A Soroban storage tier (`env.storage().persistent()`) where data entries are stored as independent ledger entries, each possessing its own individual Time-To-Live (TTL) and rent footprint. Persistent storage is used for user-generated or scalable data that must not bloat the contract instance entry, such as payment invoices, receipt indexes, and archived customer refund records. Each persistent entry can be inspected, updated, or have its ledger lifetime renewed independently without modifying the contract instance.

- **README Reference:** [Storage Schema Versioning Guide](STORAGE_VERSIONING.md#🛠️-contributor-workflow-changing-stored-data-shapes) and [Payment Contract README — Payment Tags & Invoices](../contracts/payment/README.md#payment-tags--invoices)
- **Key Functions:**
  - `attach_invoice(caller: Address, payment_id: u64, invoice: PaymentInvoice) -> Result<u64, Error>`
  - `get_invoice(invoice_id: u64) -> Option<PaymentInvoice>`
  - `get_payment_invoice(payment_id: u64) -> Option<PaymentInvoice>`
- **Associated Errors:**
  - `BasicError::Unauthorized` (code `100`) — Caller not authorized to attach invoice data to the payment.
  - `PaymentError::NotFound` (code `200`) — Referenced payment ID does not exist.
  - `BasicError::InvalidAmount` (code `103`) — Invoice line items do not sum to total invoice amount.

---

## Proration

The proportional adjustment of recurring subscription billing amounts and cycle dates when a subscription is resumed after being paused. When proration is enabled, the contract calculates the billable charge for the remaining fraction of the cycle using `prorated_amount = (amount * remaining_time) / interval`, immediately transfers that amount to the merchant, shifts subsequent billing dates by the elapsed pause duration, and emits `SubscriptionResumedProrated`. This ensures customers do not pay for paused intervals while compensating merchants fairly for active days.

- **README Reference:** [Payment Contract README — Subscription Proration](../contracts/payment/README.md#subscription-proration) and [Payment Contract README — Subscriptions](../contracts/payment/README.md#subscriptions)
- **Key Functions:**
  - `resume_subscription(caller: Address, subscription_id: u64, proration_enabled: bool) -> Result<(), Error>`
  - `set_subscription_proration(admin: Address, subscription_id: u64, enabled: bool) -> Result<(), Error>`
- **Associated Errors:**
  - `SubscriptionError::NotFound` (code `300`) — Subscription ID does not exist.
  - `SubscriptionError::NotPaused` (code `305`) — Subscription is not in `Paused` status.
  - `SubscriptionError::MerchantPaused` (code `306`) — Cannot resume proration charges while the merchant is paused.
  - `FeatureError::SpendLimitExceeded` (code `527`) — Prorated billing charge exceeds the customer's rolling spend limit.
  - `PaymentError::TransferFailed` (code `203`) — Token transfer of the prorated amount failed.

---

## Reason Code

A type-safe enum (`RefundReasonCode`) that categorises refunds into structured reasons: `ProductDefect`, `NonDelivery`, `DuplicateCharge`, `Unauthorized`, `CustomerRequest`, and `Other`. Required by `request_refund()` to enable structured auditing, filtering, and frequency-sorted analytics via `get_reason_code_analytics()`.

- **README Reference:** [Refund Contract README — Refund Reason Codes](../contracts/refund/README.md#🏷️-refund-reason-codes) and [Root README — Refund Reason Code Migration](../README.md#refund-reason-code-migration-breaking)
- **Key Functions:**
  - `request_refund(caller: Address, payment_id: u64, amount: i128, reason: String, reason_code: RefundReasonCode, payment_created_at: u64) -> Result<u64, Error>`
  - `get_refunds_by_reason_code(reason_code: RefundReasonCode, page: u32, limit: u32) -> Vec<Refund>`
  - `get_reason_code_analytics(window_start: u64, window_end: u64) -> Vec<ReasonCodeCount>`
- **Associated Errors:**
  - `CoreError::RefundNotFound` — Refund record not found during reason code querying.
  - `CoreError::InvalidStatus` — Refund not in a valid state for reason code mutation.

---

## Refund Voucher

An on-chain store credit instrument issued to a customer in lieu of an immediate token payout for an approved refund. Vouchers record the customer address, eligible merchant, token, credit balance, and an expiration timestamp (`expires_at`), allowing customers to redeem store credit against future transactions. If a customer fails to redeem the voucher before the expiration timestamp elapses (`now > expires_at`), the voucher expires and its credit cannot be reclaimed on-chain.

- **README Reference:** [Refund Contract README — Refund Vouchers](../contracts/refund/README.md#refund-vouchers) and [Refund Contract README — Voucher Expiry and Value Handling](../contracts/refund/README.md#voucher-expiry-and-value-handling)
- **Key Functions:**
  - `issue_refund_voucher(admin: Address, refund_id: u64, expiry_seconds: u64) -> Result<u64, Error>`
  - `redeem_refund_voucher(customer: Address, voucher_id: u64, _payment_id: u64) -> Result<(), Error>`
  - `get_voucher(voucher_id: u64) -> Option<RefundVoucher>`
  - `get_customer_vouchers(customer: Address) -> Vec<RefundVoucher>`
- **Associated Errors:**
  - `CoreError::Unauthorized` (code `1`) — Caller issuing the voucher is not the contract admin, or redeemer is not the voucher recipient.
  - `CoreError::RefundNotFound` (code `2`) — Target refund ID does not exist.
  - `CoreError::InvalidAmount` (code `4`) — Target refund is not in `Approved` status or duplicate voucher attempted.
  - `ExtError::VoucherNotFound` (code `25`) — Target voucher ID does not exist in storage.
  - `ExtError::VoucherAlreadyRedeemed` (code `26`) — Voucher has already been redeemed.
  - `ExtError::VoucherExpired` (code `27`) — Ledger timestamp is past `expires_at`.

---

## Spend Limits

Per-customer rolling spending caps enforced during payment creation. Each limit specifies a maximum amount within a configurable time window; the counter resets when the window elapses.

- **README Reference:** [Payment Contract README — How Spend Limits Work](../contracts/payment/README.md#how-spend-limits-work) and [Payment Contract README — Rate Limiting & Fraud Controls](../contracts/payment/README.md#rate-limiting--fraud-controls)
- **Key Functions:**
  - `set_customer_spend_limit(admin: Address, customer: Address, limit: i128, period_seconds: u64) -> Result<(), Error>`
  - `get_spend_limit(customer: Address) -> Option<CustomerSpendLimit>`
  - `remove_customer_spend_limit(admin: Address, customer: Address) -> Result<(), Error>`
  - `check_spend_allowance(customer: Address, amount: i128) -> bool`
- **Associated Errors:**
  - `FeatureError::SpendLimitExceeded` (code `527`) — Cumulative customer expenditure exceeds `limit_amount` within the active `period_seconds` window.
  - `BasicError::InvalidAmount` (code `103`) — Configured limit amount or spending payment amount is non-positive.

---

## Sub-Account

A labelled partition of an escrow that can be funded and released independently. A parent escrow cannot be fully released until all its sub-accounts have been released.

- **README Reference:** [Escrow Contract README — Sub-Accounts](../contracts/escrow/README.md#sub-accounts)
- **Key Functions:**
  - `create_sub_account(merchant: Address, escrow_id: u64, label_hash: BytesN<32>, amount: i128, fee_bps_override: Option<u32>) -> Result<u64, Error>`
  - `fund_sub_account(funder: Address, escrow_id: u64, sub_id: u64, amount: i128) -> Result<(), Error>`
  - `release_sub_account(admin: Address, escrow_id: u64, sub_id: u64) -> Result<(), Error>`
  - `set_sub_account_fee_override(merchant: Address, escrow_id: u64, sub_id: u64, fee_bps_override: Option<u32>) -> Result<(), Error>`
  - `get_sub_account(escrow_id: u64, sub_id: u64) -> Option<EscrowSubAccount>`
  - `list_sub_accounts(escrow_id: u64) -> Vec<EscrowSubAccount>`
- **Associated Errors:**
  - `SubAccountNotFound` (code `214`) — No sub-account exists for the given escrow and sub-account ID.
  - `SubAccountAlreadyReleased` (code `215`) — Target sub-account has already been released.
  - `SubAccountFundingExceedsEscrow` (code `216`) — Sub-account allocations exceed total parent escrow amount.

---

## Threshold

A configurable minimum value used in multiple contexts: the number of multisig approvals required to execute a proposal, the cumulative weight (in basis points) needed to release a multi-party escrow, or an inactivity period before reputation decay begins.

- **README Reference:** [Payment Contract README — Large Payment Multi-sig](../contracts/payment/README.md#large-payment-multi-sig), [Escrow Contract README — Public Functions](../contracts/escrow/README.md#public-functions), and [Escrow Contract README — Health Monitoring](../contracts/escrow/README.md#health-monitoring)
- **Key Functions:**
  - `set_large_payment_threshold(admin: Address, threshold: i128) -> Result<(), Error>`
  - `update_required_signatures(caller: Address, required: u32) -> Result<(), Error>`
  - `set_stale_threshold(admin: Address, config: StaleEscrowConfig) -> Result<(), Error>`
- **Associated Errors:**
  - `ProposalError::QuorumNotMet` (code `404`) — Approvals count has not met the required threshold.
  - `BasicError::InvalidThreshold` (code `121`) — Multisig threshold configured outside valid signer bounds.

---

## WASM

WebAssembly — the compiled bytecode format (`.wasm`) to which Soroban smart contracts are compiled for deployment on the Stellar network, using the `wasm32-unknown-unknown` Rust target.

- **README Reference:** [Root README — Prerequisites](../README.md#-prerequisites) and [Root README — Build All Contracts](../README.md#build-all-contracts)
