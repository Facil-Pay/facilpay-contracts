# Payment Contract Error Codes Reference

This document provides a comprehensive reference for all numeric error codes returned by the FacilPay payment contract (`contracts/payment/src/lib.rs`).

The contract errors are organized into five distinct categories by numerical ranges:
- [Basic Errors (100–126)](#basic-errors-100126)
- [Payment Errors (200–243)](#payment-errors-200243)
- [Subscription Errors (300–320)](#subscription-errors-300320)
- [Proposal & Governance Errors (400–406)](#proposal--governance-errors-400406)
- [Feature Errors (500–545)](#feature-errors-500545)

---

## Basic Errors (100–126)

Core contract authorization, parameter validation, rate limiting, and administrative errors.

| Code | Symbolic Name | Condition Triggered |
|:---:|:---|:---|
| 100 | `Unauthorized` | Caller lacks required authorization, is not an admin, or authentication failed. |
| 101 | `MetadataTooLarge` | Provided metadata byte string exceeds the maximum permitted byte length. |
| 102 | `NotesTooLarge` | Payment notes string exceeds the maximum allowed length. |
| 103 | `InvalidCurrency` | Token address is zero address, invalid, or unsupported for the operation. |
| 104 | `InvalidBatchSize` | Batch operation contains 0 elements or exceeds the maximum batch limit. |
| 105 | `BatchPartialFailure` | One or more items in an atomic batch operation encountered a failure. |
| 106 | `RateLimitExceeded` | Global or caller transaction frequency exceeds configured window rate limit. |
| 107 | `DailyVolumeExceeded` | Cumulative transfer volume exceeds daily volume threshold. |
| 108 | `AddressFlagged` | Address is present on compliance or risk blocklist; operation rejected. |
| 109 | `AddressAlreadyFlagged` | Attempted to flag an address that is already active on the blocklist. |
| 110 | `AmountExceedsLimit` | Transaction amount exceeds the absolute maximum single-transaction limit. |
| 111 | `MultiSigNotInitialized` | Multi-signature configuration has not been initialized for the contract. |
| 112 | `InsufficientAdmins` | Provided admin set is below the minimum required admin count. |
| 113 | `NotAnAdmin` | Caller is not a registered administrator in the multi-signature registry. |
| 114 | `AlreadyApproved` | Caller has already submitted an approval for the referenced action or proposal. |
| 115 | `OracleCallFailed` | Price oracle cross-contract invocation failed or returned invalid data. |
| 116 | `ContractPaused` | Entire payment contract is globally paused by administrator. |
| 117 | `FunctionPaused` | Target function or operation category is selectively paused. |
| 118 | `InvalidTierThresholds` | Volume tier or fee tier threshold values are not strictly monotonic or valid. |
| 119 | `OracleFeedStale` | Price feed data timestamp exceeds maximum permissible staleness window. |
| 120 | `OracleNotConfigured` | Price oracle contract address is not configured in contract storage. |
| 121 | `InvalidAmount` | Transfer amount is zero, negative, or invalid for contract operation. |
| 122 | `VerificationLevelNotFound` | Requested merchant or customer KYC verification tier is not found. |
| 123 | `TierLimitsNotConfigured` | Storage tiers or limits have not been configured by administration. |
| 124 | `InvalidInterval` | Time interval duration in seconds is zero or invalid. |
| 125 | `InvalidBps` | Basis points fee or share value exceeds 10,000 (100%). |
| 126 | `SchemaAlreadyAtTarget` | Contract storage schema migration target version equals current version. |

---

## Payment Errors (200–243)

Errors relating to payment lifecycle, escrow, installment schedules, merchant registration, and payment requests.

| Code | Symbolic Name | Condition Triggered |
|:---:|:---|:---|
| 200 | `NotFound` | Referenced payment ID does not exist in contract storage. |
| 201 | `InvalidStatus` | Operation is invalid for current payment status (e.g. refunding non-completed). |
| 202 | `AlreadyProcessed` | Payment has already been processed or completed; cannot re-execute. |
| 203 | `Expired` | Payment timestamp has passed its expiration ledger time. |
| 204 | `NotExpired` | Attempted to reclaim or sweep payment before expiration window elapses. |
| 205 | `NoExpiration` | Payment does not have an expiration timestamp set. |
| 206 | `TransferFailed` | Token contract transfer from caller or escrow failed. |
| 207 | `RefundExceedsPayment` | Requested refund amount exceeds remaining eligible payment amount. |
| 208 | `NotYetDue` | Scheduled payment release is attempted before scheduled due date. |
| 209 | `ScheduledPaymentCancelled` | Target scheduled payment was already cancelled. |
| 210 | `MetadataAlreadySet` | Immutable payment metadata has already been attached to the payment. |
| 211 | `MetadataNotFound` | Query for payment metadata returned no stored record. |
| 212 | `HashMismatch` | Provided verification hash does not match stored payment preimage hash. |
| 213 | `AlreadyFullyPaid` | Installment or partial payment invoice has already been fully settled. |
| 214 | `InstallmentExceedsRemaining` | Installment tranche amount exceeds the remaining unpaid invoice balance. |
| 215 | `PartialPaymentNotFound` | Referenced installment or partial payment tracking record does not exist. |
| 216 | `MerchantRateLimitExceeded` | Merchant has exceeded transaction volume or frequency limit for period. |
| 217 | `AmountRateLimitExceeded` | Transaction exceeds per-account volume velocity or threshold limit. |
| 218 | `PayoutScheduleNotFound` | Automatic payout configuration for merchant does not exist. |
| 219 | `PayoutNotYetDue` | Automated merchant payout cycle has not reached execution timestamp. |
| 220 | `NothingToSettle` | Account or merchant balance to settle is zero. |
| 221 | `BillingOverflow` | Arithmetic overflow occurred calculating total invoice line items. |
| 222 | `InvalidLineItem` | Line item unit price or quantity is zero or invalid. |
| 223 | `InvalidScheduleTime` | Scheduled timestamp is set in the past or invalid ledger window. |
| 224 | `TokenNotAllowed` | Token asset is not on the merchant or platform accepted tokens list. |
| 225 | `InvalidStatusFilter` | Provided status filter mask is invalid or unrecognized. |
| 226 | `InvalidExpiryDuration` | Expiry duration is less than the minimum or exceeds maximum allowed. |
| 227 | `InstallmentScheduleAlreadySet` | Installment schedule has already been configured for this payment. |
| 228 | `InstallmentScheduleNotFound` | Installment schedule record not found for payment ID. |
| 229 | `InstallmentScheduleTotalMismatch` | Sum of installment tranche amounts does not equal payment total. |
| 230 | `MaxHooksPerEventReached` | Maximum event notification webhook registrations reached. |
| 231 | `HookNotFound` | Target webhook subscription ID does not exist. |
| 232 | `HookNotOwnedBySubscriber` | Caller is not the registered owner of the event webhook. |
| 233 | `InvalidHookAddress` | Registered webhook notification destination address is invalid. |
| 234 | `NoEventsSpecified` | Webhook registration specified zero event subscriptions. |
| 235 | `RequestNotFound` | Payment request ID does not exist. |
| 236 | `RequestAlreadyPaid` | Payment request has already been paid and settled. |
| 237 | `RequestCancelled` | Payment request was cancelled by the issuing merchant. |
| 238 | `RequestExpired` | Payment request valid-until timestamp has passed. |
| 239 | `RequestCustomerMismatch` | Caller attempting payment does not match specified customer address. |
| 240 | `MerchantAlreadyRegistered` | Merchant address is already registered in the merchant registry. |
| 241 | `MerchantNotRegistered` | Merchant address is not registered in the merchant registry. |
| 242 | `MerchantInactive` | Merchant account status is inactive or suspended. |
| 243 | `InvalidMerchantName` | Merchant business name string is empty or exceeds character limit. |

---

## Subscription Errors (300–320)

Errors relating to recurring subscription plans, dunning, metered usage, and trials.

| Code | Symbolic Name | Condition Triggered |
|:---:|:---|:---|
| 300 | `NotFound` | Subscription ID does not exist. |
| 301 | `NotActive` | Subscription is paused, cancelled, or inactive. |
| 302 | `PaymentNotDue` | Current ledger timestamp has not reached the next billing date. |
| 303 | `MaxRetriesExceeded` | Failed billing retry attempts have reached maximum configured retry limit. |
| 304 | `Ended` | Subscription lifecycle has ended (term complete or cancelled). |
| 305 | `DunningNotFound` | No active dunning recovery workflow found for subscription. |
| 306 | `NotInDunning` | Subscription is in normal standing, not in dunning state. |
| 307 | `RetryNotDue` | Next retry execution timestamp has not yet been reached. |
| 308 | `GracePeriodExpired` | Dunning grace period elapsed without successful payment. |
| 309 | `RetryTooEarly` | Attempted to retry billing before exponential backoff window elapsed. |
| 310 | `MeteredNotFound` | Metered usage tracking record not found for subscription. |
| 311 | `BillingCapExceeded` | Current cycle metered billing total exceeds user spending cap. |
| 312 | `GroupNotFound` | Subscription group ID not found in storage. |
| 313 | `AlreadyInGroup` | Subscription is already a member of a subscription group. |
| 314 | `GroupSizeLimitExceeded` | Subscription group has reached maximum member limit. |
| 315 | `TrialExpired` | Promotional free trial duration has elapsed. |
| 316 | `MaxTrialDurationExceeded` | Configured trial period exceeds platform maximum allowable trial length. |
| 317 | `MerchantPaused` | Merchant has globally paused all subscriptions for their store. |
| 318 | `UsageCapExceeded` | Unit usage for metered subscription exceeds cycle maximum units. |
| 319 | `SkipCapExceeded` | Subscriber has reached the maximum permitted skip-cycle count for period. |
| 320 | `PauseLimitExceeded` | Subscriber has exceeded annual allowed pause count. |

---

## Proposal & Governance Errors (400–406)

Multi-signature governance and admin proposal lifecycle errors.

| Code | Symbolic Name | Condition Triggered |
|:---:|:---|:---|
| 400 | `NotFound` | Governance proposal ID does not exist in storage. |
| 401 | `Expired` | Governance proposal voting/execution window has expired. |
| 402 | `AlreadyExecuted` | Proposal has already been executed; duplicate execution rejected. |
| 403 | `ThresholdNotMet` | Current approvals count is below required signature threshold. |
| 404 | `RequiresMultiSig` | Action requires multi-signature proposal when threshold is greater than 1. |
| 405 | `InsufficientApprovals` | Proposal cannot be executed because approval count is insufficient. |
| 406 | `ProposalExpired` | Proposal lifetime has lapsed without execution. |

---

## Feature Errors (500–545)

Specialized feature modules including state channels, payment splitting, loyalty, forward routing, fee sweeps, and dispute resolution.

| Code | Symbolic Name | Condition Triggered |
|:---:|:---|:---|
| 500 | `EscrowMappingNotFound` | Cross-contract escrow identifier mapping is not found. |
| 501 | `EscrowBridgeFailed` | Cross-contract escrow call or asset lock failed. |
| 502 | `FeeConfigNotFound` | Protocol or merchant fee configuration not found in storage. |
| 503 | `InsufficientFees` | Provided fee amount is less than calculated required fee. |
| 504 | `ConditionNotMet` | Conditional payment oracle release condition was not met. |
| 505 | `ConditionAlreadyEvaluated` | Condition evaluation has already finalized; cannot re-evaluate. |
| 506 | `AutoEscrowRuleNotFound` | Automated escrow rule mapping not found. |
| 507 | `AutoEscrowBelowMinimum` | Payment amount is below the automated escrow trigger threshold. |
| 508 | `AutoEscrowAlreadyTriggered` | Automated escrow rule has already fired for this transaction. |
| 509 | `ConditionEvaluationFailed` | External oracle condition evaluation reverted or failed. |
| 510 | `ConditionRuntimeNotMet` | Runtime execution state of condition is false. |
| 511 | `InvalidFeeConfig` | Fee configuration basis points or parameters are malformed. |
| 512 | `ChannelNotFound` | State payment channel ID does not exist. |
| 513 | `InvalidSignature` | Cryptographic signature verification failed on payment channel state. |
| 514 | `InvalidNonce` | Channel state nonce is less than or equal to current stored nonce. |
| 515 | `ChannelClosed` | State channel has already been closed and settled. |
| 516 | `ChannelExpired` | State channel expiration deadline has passed. |
| 517 | `ChannelNotExpired` | Attempted emergency unilateral close before channel expiry window. |
| 518 | `InvalidSplitShares` | Sum of basis points shares in split payment does not equal 10,000 (100%). |
| 519 | `TooManyRecipients` | Number of split payment recipients exceeds maximum allowed count. |
| 520 | `SplitConfigNotFound` | Split payment recipient routing configuration not found. |
| 521 | `SplitAlreadyExecuted` | Split payment has already executed; cannot re-split. |
| 522 | `LoyaltyNotConfigured` | Merchant loyalty program configuration is not active. |
| 523 | `InsufficientPoints` | Customer loyalty points balance is insufficient for redemption. |
| 524 | `PointsExpired` | Loyalty points expiration date has passed. |
| 525 | `NothingToSweep` | Accumulated platform fee balance is zero; nothing to sweep. |
| 526 | `SweepRecipientNotSet` | Protocol fee sweep destination address is not configured. |
| 527 | `SpendLimitExceeded` | Transaction exceeds customer configured periodic spend limit. |
| 528 | `SpendLimitNotConfigured` | Query for customer spend limit returned no active rule. |
| 529 | `SettlementNotReady` | Payment settlement is within challenge delay window. |
| 530 | `FinalityConfigNotFound` | Finality delay window configuration not found in storage. |
| 531 | `SettlementAlreadyFinalized` | Payment settlement has already passed finality and paid out. |
| 532 | `RebateThresholdNotMet` | Customer cumulative volume has not reached fee rebate tier. |
| 533 | `RebateAlreadyClaimed` | Customer has already claimed fee rebate for the current period. |
| 534 | `RebateConfigNotFound` | Fee rebate rules are not configured for merchant. |
| 535 | `ForwardConfigNotFound` | Automatic payment forwarding configuration not found for recipient. |
| 536 | `ForwardLoop` | Forwarding destination address creates an infinite forward loop. |
| 537 | `InvalidForwardBps` | Forwarding basis points value is zero or exceeds 10,000. |
| 538 | `SenderIsRecipient` | Split or forwarded payment sender is listed as a recipient. |
| 539 | `BelowMinSplitAmount` | Payment amount is below minimum threshold required for splitting. |
| 540 | `InvalidCounterparty` | Channel counterparty address does not match channel creator or peer. |
| 541 | `BalanceSumMismatch` | Sum of channel participant balances does not match total deposited funds. |
| 542 | `ChallengeWindowOpen` | Settlement cannot finalize while challenge dispute window is open. |
| 543 | `ChallengeWindowClosed` | Challenge dispute submitted after challenge window has closed. |
| 544 | `NoPendingSettlement` | No channel settlement is currently pending challenge. |
| 545 | `TokenNotAllowedForMerchant` | Specified token is not in merchant whitelist of accepted tokens. |
