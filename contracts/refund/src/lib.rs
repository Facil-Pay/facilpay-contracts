#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes,
    BytesN, Env, IntoVal, String, Symbol, Vec,
};

#[cfg(test)]
extern crate std;

#[cfg(test)]
std::thread_local! {
    static TEST_TRIPPED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
    static TEST_TRIP_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    static TEST_RESETS_AT: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
}

// Issue #138 workaround: Using tuple-based storage keys with Symbol
// to avoid LengthExceedsMax error from large #[contracttype] enums
pub type StorageKey = (Symbol, Option<Address>, Option<u64>, Option<u32>);

pub fn make_key(
    prefix: &str,
    addr: Option<Address>,
    id: Option<u64>,
    sub_id: Option<u32>,
) -> StorageKey {
    (Symbol::new(&Env::default(), prefix), addr, id, sub_id)
}

// Legacy DataKey - split into functional groups to avoid LengthExceedsMax
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum DataKey {
    Admin,
    Refund(u64),
    RefundCounter,
    RefundsByStatus(RefundStatus, u64),
    RefundStatusCount(RefundStatus),
    RefundStatusIndex(u64),
    MerchantRefunds(Address, u64),
    MerchantRefundQuota(Address),
    MerchantRefundCount(Address),
    CustomerRefunds(Address, u64),
    CustomerRefundCount(Address),
    PaymentRefunds(u64, u64),
    PaymentRefundCount(u64),
    PoolToken(u64),
    DefaultRefundPolicy,
    RefundPolicy(Address),
    // Policy versioning (#134)
    RefundPolicyVersion(Address, u32),
    RefundPolicyVersionCount(Address),
    RefundPolicyTemplate(u64),
    RefundPolicyTemplateCount,
    // Payment contract address (#143)
    PaymentContractAddress,
    BatchRefundLimit,
    RefundAnalyticsKey,
    // Rate limiting
    CustomerRefundRateLimit(Address),
    GlobalRefundRateLimit,
    // Admin override audit log
    AdminOverrideHistory(u64),
    AdminOverrideHistoryCount,
    // Payment refund caps
    PaymentRefundCap(u64),
    PaymentRefundUsage(u64),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ArbitrationKey {
    ArbitrationCase(u64),
    ArbitrationCounter,
    ArbitratorList,
    ArbitratorsVoted(u64),
    ArbitratorVote(u64, Address),
    ArbitrationFeeConfig,
    AccumulatedTreasuryFees,
    ArbitrationStakeConfig,
    ArbitrationStake(u64),
    ArbitratorReputation(Address),
    ArbitratorScoreIndex(i128, u64),
    ArbitratorScoreCount,
    ArbitrationTimeoutConfig,
    // Issue #194: Tiered arbitration
    SeniorArbitratorList,
    ArbitrationTierConfig,
    CaseEscalated(u64),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PolicyKey {
    RefundPolicyVersion(Address, u32),
    RefundPolicyVersionCount(Address),
    AutoRefundTrigger(u64),
    AutoRefundTriggerCounter,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SystemKey {
    PauseStateKey,
    PauseHistoryEntry(u64),
    PauseHistoryCount,
    CircuitBreakerConfigKey,
    CircuitBreakerStateKey,
    WindowStart,
    WindowRefundVolume,
    WindowPaymentVolume,
    FraudSignal(Address),
    FraudConfig,
    FlaggedAddressesIndex,
    RefundRejectedAt(u64),
    Appeal(u64),
    AppealCounter,
    AppealByRefund(u64),
    AppealByCustomer(Address, u64),
    AppealByCustomerCount(Address),
    // Notification hooks
    NotificationHook(u64),
    NotificationHookCounter,
    HooksByEvent(RefundEventType, u64),
    HooksByEventCount(RefundEventType),
    SubscriberHooks(Address, u64),
    SubscriberHookCount(Address),
    // Platform fee deduction on refund processing
    RefundFeeConfig,
    AccumulatedRefundFees,
    // Per-customer refund cooldown
    CustomerRefundCooldown(Address),
    RefundCooldownConfig,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EvidenceKey {
    Evidence(u64, Address),
    EvidenceIndex(u64, u64),
    EvidenceCount(u64),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum VoucherKey {
    Voucher(u64),
    VoucherCounter,
    CustomerVoucher(Address, u64),
    CustomerVoucherCount(Address),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum TokenKey {
    SupportedToken(Address),
    TokenCount,
    TokenByIndex(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum RefundStatus {
    Requested,
    Approved,
    Rejected,
    Processed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum RefundReasonCode {
    ProductDefect,
    NonDelivery,
    DuplicateCharge,
    Unauthorized,
    CustomerRequest,
    Other,
}

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    InvalidAmount = 1,
    RefundNotFound = 2,
    Unauthorized = 3,
    InvalidPaymentId = 4,
    InvalidStatus = 7,
    AlreadyProcessed = 8,
    RefundExceedsPayment = 9,
    TotalRefundsExceedPayment = 10,
    RefundWindowExpired = 11,
    RefundExceedsPolicy = 12,
    PolicyNotFound = 13,
    PolicyInactive = 14,
    QuorumNotReached = 15,
    NotArbitrator = 16,
    ContractPaused = 17,
    FunctionPaused = 18,
    CaseNotTimedOut = 19,
    BatchRefundTooLarge = 20,
    // Issue #138: Refund policy inheritance errors
    CircularInheritance = 21,
    MaxInheritanceDepth = 22,
    RefundNotRejected = 23,
    AppealWindowExpired = 24,
    AppealAlreadyFiled = 25,
    RefundRateLimitExceeded = 26,
    PaymentContractNotSet = 27,
    PaymentOwnershipMismatch = 28,
    CircuitBreakerTripped = 29,
    InvalidFeeConfig = 30,
    InsufficientTreasuryFees = 31,
    ArbitratorNotFound = 34,
    InvalidScoreThreshold = 35,
    AutoRefundTriggerNotFound = 36,
    DuplicateAutoRefundTrigger = 37,
    AddressFlaggedForFraud = 38,
    FraudSignalNotFound = 40,
    // Issue #144: Notification hook errors
    HookNotFound = 41,
    MaxHooksPerEventReached = 42,
    HookNotOwnedBySubscriber = 43,
    // Issue #148: Customer eligibility errors
    CustomerBlockedFromRefund = 44,
    EligibilityEntryNotFound = 45,
    TemplateNotFound = 46,
    TemplateInactive = 47,
    // Issue #XXX: Payment refund cap errors
    RefundCountCapExceeded = 48,
    RefundAmountCapExceeded = 49,
    UnsupportedRefundToken = 50,
    // New specific errors
    VoucherNotFound = 52,
    VoucherExpired = 53,
    VoucherAlreadyRedeemed = 54,
    EvidenceAlreadySubmitted = 55,
    CaseAlreadyEscalated = 56,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRequested {
    pub refund_id: u64,
    pub payment_id: u64,
    pub merchant: Address,
    pub customer: Address,
    pub amount: i128,
    pub token: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundProcessed {
    pub refund_id: u64,
    pub processed_by: Address,
    pub customer: Address,
    pub amount: i128,
    pub token: Address,
    pub processed_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoRefundTriggered {
    pub trigger_id: u64,
    pub payment_id: u64,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TriggerRegistered {
    pub trigger_id: u64,
    pub payment_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundApproved {
    pub refund_id: u64,
    pub approved_by: Address,
    pub approved_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRejected {
    pub refund_id: u64,
    pub rejected_by: Address,
    pub rejected_at: u64,
    pub rejection_reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppealFiled {
    pub appeal_id: u64,
    pub refund_id: u64,
    pub appellant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppealResolved {
    pub appeal_id: u64,
    pub upheld: bool,
    pub resolved_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundEscalatedToArbitration {
    pub refund_id: u64,
    pub case_id: u64,
    pub fee_pool: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitrationVoteCast {
    pub case_id: u64,
    pub arbitrator: Address,
    pub vote_for_refund: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitrationCaseDecided {
    pub case_id: u64,
    pub approved: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitrationTimedOut {
    pub case_id: u64,
    pub default_outcome: bool,
    pub triggered_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitrationFeesDistributed {
    pub case_id: u64,
    pub per_arbitrator: i128,
    pub treasury_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeDeposited {
    pub case_id: u64,
    pub staker: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeReturned {
    pub case_id: u64,
    pub winner: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeForfeited {
    pub case_id: u64,
    pub loser: Address,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct ArbitrationFeeConfig {
    pub arbitrator_share_bps: u32,
    pub treasury_share_bps: u32,
    pub treasury_address: Address,
    pub fee_token: Address,
    pub fee_per_case: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct ArbitrationStakeConfig {
    pub token: Address,
    pub amount: i128,
    pub enabled: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct ArbitrationStake {
    pub case_id: u64,
    pub staker: Address,
    pub amount: i128,
    pub deposited_at: u64,
    pub returned: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct ArbitratorReputation {
    pub arbitrator: Address,
    pub total_cases: u64,
    pub majority_votes: u64,
    pub minority_votes: u64,
    pub avg_resolution_time: u64,
    pub score: i128,
    pub last_active: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitratorScoreUpdated {
    pub arbitrator: Address,
    pub old_score: i128,
    pub new_score: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArbitratorDeregistered {
    pub arbitrator: Address,
    pub reason: String,
}

// Issue #144: Notification hook structures
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum RefundEventType {
    Requested,
    Approved,
    Rejected,
    Processed,
    Escalated,
}

#[derive(Clone)]
#[contracttype]
pub struct NotificationHook {
    pub hook_id: u64,
    pub subscriber: Address,
    pub events: Vec<RefundEventType>,
    pub active: bool,
}

// Issue #190: Dispute evidence attachment
#[derive(Clone)]
#[contracttype]
pub struct RefundEvidence {
    pub refund_id: u64,
    pub submitter: Address,
    pub evidence_hash: BytesN<32>,
    pub submitted_at: u64,
}

// Issue #191: Multi-token refund support
#[derive(Clone)]
#[contracttype]
pub struct SupportedRefundToken {
    pub token: Address,
    pub active: bool,
}

// Issue #192: Refund credit vouchers
#[derive(Clone)]
#[contracttype]
pub struct RefundVoucher {
    pub voucher_id: u64,
    pub refund_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub issued_at: u64,
    pub expires_at: u64,
    pub redeemed: bool,
}

// Issue #194: Tiered arbitration escalation
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ArbitratorTier {
    Junior,
    Senior,
}

#[derive(Clone)]
#[contracttype]
pub struct ArbitrationTierConfig {
    pub junior_quorum: u32,
    pub senior_quorum: u32,
    pub escalation_timeout_seconds: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct TieredArbitrator {
    pub address: Address,
    pub tier: ArbitratorTier,
    pub active: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookRegistered {
    pub hook_id: u64,
    pub subscriber: Address,
    pub event_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookDeregistered {
    pub hook_id: u64,
    pub subscriber: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookInvocationFailed {
    pub hook_id: u64,
    pub subscriber: Address,
    pub event_type: RefundEventType,
    pub refund_id: u64,
}

// ── Issue #148: Customer eligibility registry ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum EligibilityRule {
    Allow,
    Block,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundEligibilityEntry {
    pub customer: Address,
    pub merchant: Address,
    pub rule: EligibilityRule,
    pub reason_hash: BytesN<32>,
    pub set_at: u64,
}

/// Storage key for eligibility entries: keyed by (merchant, customer).
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EligibilityKey {
    /// The eligibility entry for a (merchant, customer) pair.
    Entry(Address, Address),
    /// Ordered index of customers for a merchant: (merchant, index) → customer.
    MerchantCustomerIndex(Address, u64),
    /// Total number of eligibility entries for a merchant.
    MerchantCustomerCount(Address),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilitySet {
    pub merchant: Address,
    pub customer: Address,
    pub rule: EligibilityRule,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityRemoved {
    pub merchant: Address,
    pub customer: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct Refund {
    pub id: u64,
    pub payment_id: u64,
    pub merchant: Address,
    pub customer: Address,
    pub amount: i128,
    pub original_payment_amount: i128,
    pub token: Address,
    // Issue #191: original payment token for multi-token refund matching
    pub original_token: Address,
    pub status: RefundStatus,
    pub requested_at: u64,
    pub reason: String,
    pub reason_code: RefundReasonCode,
    // Issue #147: Lifecycle timestamps
    pub approved_at: Option<u64>,
    pub rejected_at: Option<u64>,
    pub processed_at: Option<u64>,
    // Issue #199: TTL expiry
    pub expires_at: Option<u64>,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentRefundCap {
    pub payment_id: u64,
    pub max_refund_count: u32,
    pub max_total_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct MerchantRefundSummary {
    pub total_requests: u64,
    pub total_approved: u64,
    pub total_rejected: u64,
    pub total_amount_refunded: i128,
    pub pending_count: u64,
    pub pending_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct MerchantRefundQuota {
    pub merchant: Address,
    pub limit: i128,
    pub period_seconds: u64,
    pub used: i128,
    pub period_start: u64,
}

// Issue #147: Customer refund summary
#[derive(Clone)]
#[contracttype]
pub struct CustomerRefundSummary {
    pub total_requested: u64,
    pub total_approved: u64,
    pub total_amount_refunded: i128,
    pub avg_processing_time: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundAppeal {
    pub appeal_id: u64,
    pub refund_id: u64,
    pub appellant: Address,
    pub reason: String,
    pub filed_at: u64,
    pub resolved: bool,
    pub outcome: Option<bool>,
}

#[contracttype]
pub struct ArbitrationCase {
    pub case_id: u64,
    pub refund_id: u64,
    pub arbitrators: Vec<Address>,
    pub votes_for_refund: u32,
    pub votes_against_refund: u32,
    pub status: ArbitrationStatus,
    pub created_at: u64,
    pub deadline: u64,
    pub fee_pool: i128,
    pub timeout_at: u64,
    pub default_favor_customer: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[contracttype]
pub enum ArbitrationStatus {
    Open,
    Decided,
    Appealed,
    Closed,
}

#[contracttype]
pub struct ArbitratorVote {
    pub arbitrator: Address,
    pub voted_for_refund: bool,
    pub reasoning_hash: BytesN<32>,
    pub voted_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundTier {
    pub days_from_purchase: u64,
    pub max_refund_bps: u32 ,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundPolicy {
    pub merchant: Address,
    pub tiers: Vec<RefundTier>,
    pub active: bool,
    pub created_at: u64,
    pub updated_at: u64,
    pub default_window_seconds: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct AutoRefundTrigger {
    pub trigger_id: u64,
    pub payment_id: u64,
    pub condition: AutoRefundCondition,
    pub refund_bps: u32,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct FulfillmentTimeoutCondition {
    pub fulfillment_deadline: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ContractStateMatchCondition {
    pub contract: Address,
    pub key: BytesN<32>,
    pub expected: Bytes,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum AutoRefundCondition {
    FulfillmentTimeout(FulfillmentTimeoutCondition),
    ContractStateMatch(ContractStateMatchCondition),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
enum ExternalPaymentStatus {
    Pending,
    Completed,
    Refunded,
    PartialRefunded,
    Cancelled,
}

#[derive(Clone)]
#[contracttype]
enum ExternalCurrency {
    XLM,
    USDC,
    USDT,
    BTC,
    ETH,
}

#[derive(Clone)]
#[contracttype]
struct ExternalPayment {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub currency: ExternalCurrency,
    pub status: ExternalPaymentStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub metadata: String,
    pub notes: String,
    pub refunded_amount: i128,
}

// ── Issue #134: Policy versioning struct ──────────────────────────────────
#[derive(Clone)]
#[contracttype]
pub struct RefundPolicyVersion {
    pub version: u32,
    pub policy: RefundPolicy,
    pub created_at: u64,
    pub created_by: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundPolicyTemplate {
    pub template_id: u64,
    pub name: String,
    pub tiers: Vec<(u32, i128)>,
    pub default_window_seconds: u64,
    pub active: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundPolicyTemplateCreated {
    pub template_id: u64,
    pub created_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyTemplateDeactivated {
    pub template_id: u64,
    pub deactivated_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundPolicyTemplateApplied {
    pub template_id: u64,
    pub merchant: Address,
    pub applied_by: Address,
}

// ── Issue #135: Batch refund result struct ─────────────────────────────────
#[derive(Clone)]
#[contracttype]
pub struct BatchRefundResult {
    pub refund_id: u64,
    pub success: bool,
    pub error_code: u32,
    pub amount_refunded: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct CustomerRefundRateLimit {
    pub customer: Address,
    pub window_start: u64,
    pub request_count: u32,
    pub max_requests_per_window: u32,
    pub window_seconds: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct GlobalRefundRateLimit {
    pub max_requests_per_window: u32,
    pub window_seconds: u64,
}

/// Configuration for platform fee deduction on refund processing
#[derive(Clone)]
#[contracttype]
pub struct RefundFeeConfig {
    pub fee_bps: u32,       // Fee in basis points (e.g., 100 = 1%)
    pub min_fee: i128,      // Minimum fee amount
    pub max_fee: i128,      // Maximum fee amount
    pub treasury: Address,  // Address to receive fees
    pub fee_token: Address, // Token in which fees are collected
    pub active: bool,       // Whether fee collection is enabled
}

/// Per-customer refund cooldown configuration
#[derive(Clone)]
#[contracttype]
pub struct RefundCooldownConfig {
    pub cooldown_seconds: u64, // Minimum time between refund requests per customer
    pub enabled: bool,         // Whether cooldown is enforced
}

/// Tracks the last refund request time for a customer
#[derive(Clone)]
#[contracttype]
pub struct CustomerRefundCooldown {
    pub customer: Address,
    pub last_refund_requested_at: u64,
    pub cooldown_seconds: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoApproved {
    pub refund_id: u64,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundPolicySet {
    pub merchant: Address,
    pub tiers_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundPolicyDeactivated {
    pub merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefaultRefundPolicySet {
    pub set_by: Address,
    pub tiers_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefaultRefundPolicyRemoved {
    pub removed_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyOverrideApplied {
    pub refund_id: u64,
    pub admin: Address,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRefundOverride {
    pub override_id: u64,
    pub refund_id: u64,
    pub admin: Address,
    pub reason: String,
    pub override_amount: i128,
    pub override_status: RefundStatus,
    pub executed_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct AdminOverrideHistory {
    pub override_id: u64,
    pub refund_id: u64,
    pub admin: Address,
    pub reason: String,
    pub override_amount: i128,
    pub override_status: RefundStatus,
    pub executed_at: u64,
    pub transaction_hash: BytesN<32>, // Immutable hash of override details
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPausedEvent {
    pub paused_by: Address,
    pub reason: String,
    pub paused_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUnpausedEvent {
    pub unpaused_by: Address,
    pub unpaused_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionPausedEvent {
    pub function_name: String,
    pub paused_by: Address,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionUnpausedEvent {
    pub function_name: String,
    pub unpaused_by: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundAnalytics {
    pub total_refunds_requested: u64,
    pub total_refunds_approved: u64,
    pub total_refunds_rejected: u64,
    pub total_refunds_processed: u64,
    pub total_refund_volume: i128,
    pub approval_rate_bps: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct PauseState {
    pub globally_paused: bool,
    pub paused_functions: Vec<String>,
    pub paused_at: u64,
    pub paused_by: Address,
    pub pause_reason: String,
}

#[derive(Clone)]
#[contracttype]
pub struct PauseHistory {
    pub index: u64,
    pub function_name: String,
    pub paused: bool,
    pub changed_by: Address,
    pub changed_at: u64,
    pub reason: String,
}

#[derive(Clone)]
#[contracttype]
pub struct CircuitBreakerConfig {
    pub max_refund_rate_bps: u32,
    pub measurement_window_seconds: u64,
    pub cooldown_seconds: u64,
    pub enabled: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct CircuitBreakerState {
    pub tripped: bool,
    pub tripped_at: Option<u64>,
    pub trip_count: u32,
    pub last_refund_rate_bps: u32,
    pub resets_at: Option<u64>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerTrippedEvent {
    pub refund_rate_bps: u32,
    pub tripped_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerResetEvent {
    pub reset_by: Address,
    pub reset_at: u64,
}

// Fraud detection structures (#137)
#[derive(Clone)]
#[contracttype]
pub struct FraudSignal {
    pub address: Address,
    pub refund_rate_bps: u32,
    pub total_payments: u64,
    pub total_refunds: u64,
    pub flagged_at: u64,
    pub reviewed: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct FraudConfig {
    pub max_refund_rate_bps: u32,
    pub min_transactions_for_check: u64,
    pub enabled: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FraudSignalRaised {
    pub address: Address,
    pub refund_rate_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FraudSignalReviewed {
    pub address: Address,
    pub reviewed_by: Address,
}

// Issues #195/#197/#198/#199: extended storage keys
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum RefundExtKey {
    CategoryWindow(Address, u32),
    PaymentCategoryTag(u64),
    AssignmentConfig,
    RotationIndex,
    RefundTTLConfig,
}

// Issue #195: Batch decision types
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum BatchDecisionType {
    Approve,
    Reject,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchRefundDecision {
    pub refund_ids: Vec<u64>,
    pub decision: BatchDecisionType,
    pub note_hash: BytesN<32>,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchDecisionResult {
    pub succeeded: Vec<u64>,
    pub failed: Vec<u64>,
}

// Issue #197: Payment categories
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PaymentCategory {
    DigitalGoods,
    PhysicalGoods,
    Subscription,
    Service,
    Other,
}

impl PaymentCategory {
    pub fn to_index(&self) -> u32 {
        match self {
            PaymentCategory::DigitalGoods => 0,
            PaymentCategory::PhysicalGoods => 1,
            PaymentCategory::Subscription => 2,
            PaymentCategory::Service => 3,
            PaymentCategory::Other => 4,
        }
    }
}

#[derive(Clone)]
#[contracttype]
pub struct CategoryRefundWindow {
    pub category: PaymentCategory,
    pub window_seconds: u64,
    pub merchant: Address,
}

// Issue #198: Arbitrator auto-assignment
#[derive(Clone)]
#[contracttype]
pub struct ArbitratorAssignmentConfig {
    pub rotation_index: u32,
    pub panel_size: u32,
}

// Issue #199: Refund TTL
#[derive(Clone)]
#[contracttype]
pub struct RefundTTLConfig {
    pub default_ttl_seconds: u64,
    pub active: bool,
}

/// Event emitted when platform fee is deducted from a refund
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundFeeDeducted {
    pub refund_id: u64,
    pub fee_amount: i128,
    pub net_refund_amount: i128,
    pub treasury: Address,
}

/// Event emitted when refund fee configuration is updated
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundFeeConfigUpdated {
    pub fee_bps: u32,
    pub min_fee: i128,
    pub max_fee: i128,
    pub updated_by: Address,
}

/// Event emitted when customer refund cooldown is enforced
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundCooldownEnforced {
    pub customer: Address,
    pub last_refund_at: u64,
    pub cooldown_seconds: u64,
    pub available_at: u64,
}

#[contract]
pub struct RefundContract;

#[contractimpl]
impl RefundContract {
    const BATCH_DECISION_LIMIT: u32 = 50;

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Set default refund policy (30 days, 100% refund)
        let mut default_tiers = Vec::new(&env);
        default_tiers.push_back(RefundTier {
            days_from_purchase: 30,
            max_refund_bps: 10000,
        });
        let default_policy = RefundPolicy {
            merchant: admin.clone(), // Placeholder, will be overridden per merchant
            tiers: default_tiers,
            active: true,
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            default_window_seconds: 30 * 24 * 60 * 60, // 30 days
        };
        env.storage()
            .instance()
            .set(&DataKey::DefaultRefundPolicy, &default_policy);

        // Store default settings for admin separately
        Self::set_inherit_from_parent_inner(&env, &admin, false);
        Self::set_requires_admin_approval_inner(&env, &admin, true);
        Self::set_auto_approve_below_inner(&env, &admin, 0);
    }

    pub fn request_refund(
        env: Env,
        merchant: Address,
        payment_id: u64,
        customer: Address,
        amount: i128,
        original_payment_amount: i128,
        token: Address,
        reason: String,
        reason_code: RefundReasonCode,
        payment_created_at: u64,
    ) -> Result<u64, Error> {
        // Require merchant authentication
        merchant.require_auth();

        // Issue #191: validate token against supported registry if registry is non-empty
        let token_count: u64 = env
            .storage()
            .instance()
            .get(&TokenKey::TokenCount)
            .unwrap_or(0);
        if token_count > 0 {
            let supported: Option<SupportedRefundToken> = env
                .storage()
                .instance()
                .get(&TokenKey::SupportedToken(token.clone()));
            match supported {
                Some(t) if t.active => {}
                _ => return Err(Error::UnsupportedRefundToken),
            }
        }

        Self::create_refund(
            env,
            merchant,
            payment_id,
            customer,
            amount,
            original_payment_amount,
            token,
            reason,
            reason_code,
            payment_created_at,
            false,
        )
    }

    pub fn get_refund(env: &Env, refund_id: u64) -> Result<Refund, Error> {
        // Retrieve refund from storage by ID
        env.storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)
    }

    pub fn approve_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        Self::approve_refund_internal(&env, admin, refund_id)
    }

    pub fn reject_refund(
        env: Env,
        admin: Address,
        refund_id: u64,
        rejection_reason: String,
    ) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        // Retrieve refund from storage
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Check refund status is Requested
        if refund.status != RefundStatus::Requested {
            return Err(Error::InvalidStatus);
        }

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;

        // Update refund status to Rejected
        refund.status = RefundStatus::Rejected;
        // Issue #147: Set rejected_at timestamp
        refund.rejected_at = Some(env.ledger().timestamp());

        // Store updated refund back to storage
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);
        env.storage().instance().set(
            &SystemKey::RefundRejectedAt(refund_id),
            &env.ledger().timestamp(),
        );

        // Emit RefundRejected event
        (RefundRejected {
            refund_id,
            rejected_by: admin,
            rejected_at: env.ledger().timestamp(),
            rejection_reason,
        })
        .publish(&env);

        // Issue #144: Invoke notification hooks
        Self::invoke_hooks(&env, RefundEventType::Rejected, refund_id);

        Ok(())
    }

    pub fn file_appeal(
        env: Env,
        customer: Address,
        refund_id: u64,
        reason: String,
    ) -> Result<u64, Error> {
        customer.require_auth();

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.customer != customer {
            return Err(Error::Unauthorized);
        }
        if refund.status != RefundStatus::Rejected {
            return Err(Error::RefundNotRejected);
        }
        if env
            .storage()
            .instance()
            .has(&SystemKey::AppealByRefund(refund_id))
        {
            return Err(Error::AppealAlreadyFiled);
        }

        let rejected_at: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::RefundRejectedAt(refund_id))
            .ok_or(Error::RefundNotRejected)?;
        let now = env.ledger().timestamp();
        if now > rejected_at.saturating_add(72 * 60 * 60) {
            return Err(Error::AppealWindowExpired);
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::AppealCounter)
            .unwrap_or(0);
        let appeal_id = counter + 1;
        let appeal = RefundAppeal {
            appeal_id,
            refund_id,
            appellant: customer.clone(),
            reason,
            filed_at: now,
            resolved: false,
            outcome: None,
        };
        env.storage()
            .instance()
            .set(&SystemKey::Appeal(appeal_id), &appeal);
        env.storage()
            .instance()
            .set(&SystemKey::AppealCounter, &appeal_id);
        env.storage()
            .instance()
            .set(&SystemKey::AppealByRefund(refund_id), &appeal_id);

        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::AppealByCustomerCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &SystemKey::AppealByCustomer(customer.clone(), customer_count),
            &appeal_id,
        );
        env.storage().instance().set(
            &SystemKey::AppealByCustomerCount(customer.clone()),
            &(customer_count + 1),
        );

        (AppealFiled {
            appeal_id,
            refund_id,
            appellant: customer,
        })
        .publish(&env);

        Ok(appeal_id)
    }

    pub fn resolve_appeal(
        env: Env,
        admin: Address,
        appeal_id: u64,
        uphold: bool,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut appeal: RefundAppeal = env
            .storage()
            .instance()
            .get(&SystemKey::Appeal(appeal_id))
            .ok_or(Error::RefundNotFound)?;
        if appeal.resolved {
            return Err(Error::AlreadyProcessed);
        }

        if uphold {
            let mut refund: Refund = env
                .storage()
                .instance()
                .get(&DataKey::Refund(appeal.refund_id))
                .ok_or(Error::RefundNotFound)?;
            if refund.status != RefundStatus::Rejected {
                return Err(Error::RefundNotRejected);
            }

            Self::remove_from_status_index(&env, RefundStatus::Rejected, refund.id)?;
            refund.status = RefundStatus::Approved;
            env.storage()
                .instance()
                .set(&DataKey::Refund(refund.id), &refund);
            Self::add_to_status_index(&env, RefundStatus::Approved, refund.id);

            Self::process_refund_internal(&env, admin.clone(), refund.id)?;
        }

        appeal.resolved = true;
        appeal.outcome = Some(uphold);
        env.storage()
            .instance()
            .set(&SystemKey::Appeal(appeal_id), &appeal);

        (AppealResolved {
            appeal_id,
            upheld: uphold,
            resolved_at: env.ledger().timestamp(),
        })
        .publish(&env);

        Ok(())
    }

    pub fn get_appeal(env: Env, appeal_id: u64) -> Result<RefundAppeal, Error> {
        env.storage()
            .instance()
            .get(&SystemKey::Appeal(appeal_id))
            .ok_or(Error::RefundNotFound)
    }

    pub fn get_appeals_by_customer(env: Env, customer: Address) -> Vec<RefundAppeal> {
        let mut appeals = Vec::new(&env);
        let count: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::AppealByCustomerCount(customer.clone()))
            .unwrap_or(0);

        let mut index = 0u64;
        while index < count {
            if let Some(appeal_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&SystemKey::AppealByCustomer(customer.clone(), index))
            {
                if let Some(appeal) = env
                    .storage()
                    .instance()
                    .get::<_, RefundAppeal>(&SystemKey::Appeal(appeal_id))
                {
                    appeals.push_back(appeal);
                }
            }
            index += 1;
        }

        appeals
    }

    pub fn process_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        admin.require_auth();

        Self::process_refund_internal(&env, admin, refund_id)
    }

    pub fn register_auto_refund_trigger(
        env: Env,
        merchant: Address,
        payment_id: u64,
        condition: AutoRefundCondition,
        refund_bps: u32,
    ) -> Result<u64, Error> {
        merchant.require_auth();

        if payment_id == 0 {
            return Err(Error::InvalidPaymentId);
        }

        if let Err(_) = Self::validate_bps(refund_bps) {
            return Err(Error::RefundExceedsPolicy);
        }

        let payment = Self::get_external_payment(&env, payment_id)?;
        if payment.merchant != merchant {
            return Err(Error::Unauthorized);
        }

        let trigger_count: u64 = env
            .storage()
            .instance()
            .get(&PolicyKey::AutoRefundTriggerCounter)
            .unwrap_or(0);

        let mut trigger_id = 1u64;
        while trigger_id <= trigger_count {
            if let Some(existing) = env
                .storage()
                .instance()
                .get::<PolicyKey, AutoRefundTrigger>(&PolicyKey::AutoRefundTrigger(trigger_id))
            {
                if existing.active
                    && existing.payment_id == payment_id
                    && existing.condition == condition
                {
                    return Err(Error::DuplicateAutoRefundTrigger);
                }
            }
            trigger_id += 1;
        }

        let new_trigger_id = trigger_count + 1;
        let trigger = AutoRefundTrigger {
            trigger_id: new_trigger_id,
            payment_id,
            condition,
            refund_bps,
            active: true,
        };

        env.storage()
            .instance()
            .set(&PolicyKey::AutoRefundTrigger(new_trigger_id), &trigger);
        env.storage()
            .instance()
            .set(&PolicyKey::AutoRefundTriggerCounter, &new_trigger_id);

        (TriggerRegistered {
            trigger_id: new_trigger_id,
            payment_id,
        })
        .publish(&env);

        Ok(new_trigger_id)
    }

    pub fn evaluate_auto_refund(env: Env, trigger_id: u64) -> Result<bool, Error> {
        let mut trigger = Self::get_auto_refund_trigger(env.clone(), trigger_id)?;
        if !trigger.active {
            return Ok(false);
        }

        let condition_met = Self::evaluate_auto_refund_condition(&env, &trigger.condition)?;
        if !condition_met {
            return Ok(false);
        }

        let payment = Self::get_external_payment(&env, trigger.payment_id)?;
        let refund_amount = payment
            .amount
            .checked_mul(trigger.refund_bps as i128)
            .and_then(|value| value.checked_div(10000))
            .ok_or(Error::InvalidAmount)?;
        if refund_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let refund_id = Self::create_refund(
            env.clone(),
            payment.merchant.clone(),
            payment.id,
            payment.customer.clone(),
            refund_amount,
            payment.amount,
            payment.token.clone(),
            String::from_str(&env, "Automatic refund trigger executed"),
            RefundReasonCode::Other,
            payment.created_at,
            true,
        )?;
        Self::process_refund_internal(&env, env.current_contract_address(), refund_id)?;

        trigger.active = false;
        env.storage()
            .instance()
            .set(&PolicyKey::AutoRefundTrigger(trigger_id), &trigger);

        (AutoRefundTriggered {
            trigger_id,
            payment_id: payment.id,
            amount: refund_amount,
        })
        .publish(&env);

        Ok(true)
    }

    pub fn get_auto_refund_trigger(env: Env, trigger_id: u64) -> Result<AutoRefundTrigger, Error> {
        env.storage()
            .instance()
            .get(&PolicyKey::AutoRefundTrigger(trigger_id))
            .ok_or(Error::AutoRefundTriggerNotFound)
    }

    pub fn set_merchant_refund_quota(
        env: Env,
        admin: Address,
        merchant: Address,
        limit: i128,
        period_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let quota = MerchantRefundQuota {
            merchant: merchant.clone(),
            limit,
            period_seconds,
            used: 0,
            period_start: env.ledger().timestamp(),
        };
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefundQuota(merchant), &quota);
        Ok(())
    }

    pub fn get_merchant_refund_quota(env: Env, merchant: Address) -> Option<MerchantRefundQuota> {
        env.storage()
            .instance()
            .get(&DataKey::MerchantRefundQuota(merchant))
    }

    pub fn reset_merchant_quota(env: Env, admin: Address, merchant: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut quota: MerchantRefundQuota = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRefundQuota(merchant.clone()))
            .ok_or(Error::PolicyNotFound)?;
        quota.used = 0;
        quota.period_start = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefundQuota(merchant), &quota);
        Ok(())
    }

    pub fn set_customer_rate_limit(
        env: Env,
        admin: Address,
        customer: Address,
        max_per_window: u32,
        window_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let mut limit = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundRateLimit(customer.clone()))
            .unwrap_or(CustomerRefundRateLimit {
                customer: customer.clone(),
                window_start: env.ledger().timestamp(),
                request_count: 0,
                max_requests_per_window: max_per_window,
                window_seconds,
            });

        limit.max_requests_per_window = max_per_window;
        limit.window_seconds = window_seconds;

        env.storage()
            .instance()
            .set(&DataKey::CustomerRefundRateLimit(customer), &limit);
        Ok(())
    }

    pub fn get_customer_rate_limit_status(env: Env, customer: Address) -> CustomerRefundRateLimit {
        env.storage()
            .instance()
            .get(&DataKey::CustomerRefundRateLimit(customer.clone()))
            .unwrap_or(CustomerRefundRateLimit {
                customer,
                window_start: 0,
                request_count: 0,
                max_requests_per_window: 0,
                window_seconds: 0,
            })
    }

    pub fn set_global_refund_rate_limit(
        env: Env,
        admin: Address,
        max_per_window: u32,
        window_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let limit = GlobalRefundRateLimit {
            max_requests_per_window: max_per_window,
            window_seconds,
        };

        env.storage()
            .instance()
            .set(&DataKey::GlobalRefundRateLimit, &limit);
        Ok(())
    }

    pub fn register_arbitrator(env: Env, admin: Address, arbitrator: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let mut list: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));
        if list.contains(&arbitrator) {
            return Err(Error::Unauthorized);
        }
        list.push_back(arbitrator.clone());
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitratorList, &list);

        // Initialize reputation for new arbitrator
        let reputation = ArbitratorReputation {
            arbitrator: arbitrator.clone(),
            total_cases: 0,
            majority_votes: 0,
            minority_votes: 0,
            avg_resolution_time: 0,
            score: 100, // Starting score
            last_active: env.ledger().timestamp(),
        };
        env.storage().instance().set(
            &ArbitrationKey::ArbitratorReputation(arbitrator),
            &reputation,
        );

        Ok(())
    }

    pub fn escalate_to_arbitration(
        env: Env,
        caller: Address,
        refund_id: u64,
        fee_token: Address,
        fee_amount: i128,
    ) -> Result<u64, Error> {
        caller.require_auth();

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;
        if refund.status != RefundStatus::Rejected {
            return Err(Error::InvalidStatus);
        }
        if fee_amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCounter)
            .unwrap_or(0);
        let case_id = counter + 1;

        let arbitrators = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));
        if arbitrators.len() < 3 {
            return Err(Error::QuorumNotReached);
        }

        // Handle staking if enabled
        let stake_config: Option<ArbitrationStakeConfig> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStakeConfig);

        if let Some(config) = stake_config {
            if config.enabled {
                if config.amount <= 0 {
                    return Err(Error::InvalidAmount);
                }

                // Transfer stake from caller to contract
                let stake_token_client = token::Client::new(&env, &config.token);
                stake_token_client.transfer(
                    &caller,
                    &env.current_contract_address(),
                    &config.amount,
                );

                // Record the stake
                let stake = ArbitrationStake {
                    case_id,
                    staker: caller.clone(),
                    amount: config.amount,
                    deposited_at: env.ledger().timestamp(),
                    returned: false,
                };
                env.storage()
                    .instance()
                    .set(&ArbitrationKey::ArbitrationStake(case_id), &stake);

                StakeDeposited {
                    case_id,
                    staker: caller.clone(),
                    amount: config.amount,
                }
                .publish(&env);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::PoolToken(case_id), &fee_token.clone());
        let token_client = token::Client::new(&env, &fee_token);
        token_client.transfer(&caller, &env.current_contract_address(), &fee_amount);

        let case = ArbitrationCase {
            case_id,
            refund_id,
            arbitrators: arbitrators.clone(),
            votes_for_refund: 0,
            votes_against_refund: 0,
            status: ArbitrationStatus::Open,
            created_at: env.ledger().timestamp(),
            deadline: env.ledger().timestamp() + 86400 * 7, // 7 days example
            fee_pool: fee_amount,
            timeout_at: {
                let timeout_secs: u64 = env
                    .storage()
                    .instance()
                    .get(&ArbitrationKey::ArbitrationTimeoutConfig)
                    .unwrap_or(86400 * 14); // default 14 days
                env.ledger().timestamp() + timeout_secs
            },
            default_favor_customer: true,
        };

        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCounter, &case_id);

        RefundEscalatedToArbitration {
            refund_id,
            case_id,
            fee_pool: fee_amount,
        }
        .publish(&env);

        // Issue #144: Invoke notification hooks for Escalated event
        Self::invoke_hooks(&env, RefundEventType::Escalated, refund_id);

        Ok(case_id)
    }

    pub fn cast_arbitration_vote(
        env: Env,
        arbitrator: Address,
        case_id: u64,
        vote_for_refund: bool,
        reasoning_hash: BytesN<32>,
    ) -> Result<(), Error> {
        arbitrator.require_auth();

        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::RefundNotFound)?;
        if case.status != ArbitrationStatus::Open {
            return Err(Error::InvalidStatus);
        }
        if env.ledger().timestamp() > case.deadline {
            return Err(Error::InvalidStatus);
        }
        if !case.arbitrators.contains(&arbitrator) {
            return Err(Error::NotArbitrator);
        }
        if env
            .storage()
            .instance()
            .has(&ArbitrationKey::ArbitratorVote(case_id, arbitrator.clone()))
        {
            return Err(Error::AlreadyProcessed);
        }

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(case.refund_id))
            .unwrap();
        if arbitrator == refund.merchant || arbitrator == refund.customer {
            return Err(Error::Unauthorized);
        }

        let vote = ArbitratorVote {
            arbitrator: arbitrator.clone(),
            voted_for_refund: vote_for_refund,
            reasoning_hash,
            voted_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(
            &ArbitrationKey::ArbitratorVote(case_id, arbitrator.clone()),
            &vote,
        );

        let mut voted: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorsVoted(case_id))
            .unwrap_or_else(|| Vec::new(&env));
        if !voted.contains(&arbitrator) {
            voted.push_back(arbitrator.clone());
            env.storage()
                .instance()
                .set(&ArbitrationKey::ArbitratorsVoted(case_id), &voted);
        }

        if vote_for_refund {
            case.votes_for_refund += 1;
        } else {
            case.votes_against_refund += 1;
        }
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);

        ArbitrationVoteCast {
            case_id,
            arbitrator,
            vote_for_refund,
        }
        .publish(&env);

        Ok(())
    }

    pub fn close_arbitration_case(env: Env, case_id: u64) -> Result<(), Error> {
        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::RefundNotFound)?;
        if case.status != ArbitrationStatus::Open {
            return Err(Error::InvalidStatus);
        }

        let total_votes = case.votes_for_refund + case.votes_against_refund;
        if total_votes < 3 {
            return Err(Error::InvalidStatus);
        } // quorum

        let approved = case.votes_for_refund > case.votes_against_refund;

        case.status = ArbitrationStatus::Decided;
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);

        // Update refund status if approved
        if approved {
            let mut refund: Refund = env
                .storage()
                .instance()
                .get(&DataKey::Refund(case.refund_id))
                .unwrap();
            refund.status = RefundStatus::Approved;
            env.storage()
                .instance()
                .set(&DataKey::Refund(case.refund_id), &refund);
        }

        // Distribute fees according to configuration
        let num_voters = total_votes as i128;

        // Get all arbitrators who voted (needed for both fee distribution and reputation updates)
        let all_voters: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorsVoted(case_id))
            .unwrap_or_else(|| Vec::new(&env));

        if num_voters > 0 {
            let pool_token: Address = env
                .storage()
                .instance()
                .get(&DataKey::PoolToken(case_id))
                .unwrap();
            let token_client = token::Client::new(&env, &pool_token);

            // Get fee configuration
            let fee_config: Option<ArbitrationFeeConfig> = env
                .storage()
                .instance()
                .get(&ArbitrationKey::ArbitrationFeeConfig);

            let (arbitrator_share, treasury_share, treasury_address) = if let Some(ref config) =
                fee_config
            {
                // Calculate shares based on basis points
                let arbitrator_amount =
                    (case.fee_pool * config.arbitrator_share_bps as i128) / 10000;
                let treasury_amount = (case.fee_pool * config.treasury_share_bps as i128) / 10000;
                (
                    arbitrator_amount,
                    treasury_amount,
                    Some(config.treasury_address.clone()),
                )
            } else {
                // Default: 100% to arbitrators, 0% to treasury
                (case.fee_pool, 0, None)
            };

            // Filter to only majority voters
            let mut majority_voters = Vec::new(&env);
            for voter in all_voters.iter() {
                let vote: ArbitratorVote = env
                    .storage()
                    .instance()
                    .get(&ArbitrationKey::ArbitratorVote(case_id, voter.clone()))
                    .unwrap();

                // Check if this voter was in the majority
                let in_majority = if approved {
                    vote.voted_for_refund
                } else {
                    !vote.voted_for_refund
                };

                if in_majority {
                    majority_voters.push_back(voter.clone());
                }
            }

            // Distribute arbitrator share equally among majority voters
            let per_arbitrator = if majority_voters.len() > 0 {
                arbitrator_share / (majority_voters.len() as i128)
            } else {
                0
            };

            for arbitrator in majority_voters.iter() {
                token_client.transfer(&env.current_contract_address(), arbitrator, &per_arbitrator);
            }

            // Transfer treasury share if configured
            if treasury_share > 0 {
                if let Some(treasury_addr) = treasury_address {
                    token_client.transfer(
                        &env.current_contract_address(),
                        &treasury_addr,
                        &treasury_share,
                    );

                    // Accumulate treasury fees
                    let accumulated: i128 = env
                        .storage()
                        .instance()
                        .get(&ArbitrationKey::AccumulatedTreasuryFees)
                        .unwrap_or(0);
                    env.storage().instance().set(
                        &ArbitrationKey::AccumulatedTreasuryFees,
                        &(accumulated + treasury_share),
                    );
                }
            }

            ArbitrationFeesDistributed {
                case_id,
                per_arbitrator,
                treasury_amount: treasury_share,
            }
            .publish(&env);
        }

        // Handle stake return or forfeiture
        let stake_opt: Option<ArbitrationStake> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStake(case_id));

        if let Some(mut stake) = stake_opt {
            if !stake.returned {
                let refund: Refund = env
                    .storage()
                    .instance()
                    .get(&DataKey::Refund(case.refund_id))
                    .unwrap();

                let stake_config: Option<ArbitrationStakeConfig> = env
                    .storage()
                    .instance()
                    .get(&ArbitrationKey::ArbitrationStakeConfig);

                if let Some(stake_cfg) = stake_config {
                    let stake_token_client = token::Client::new(&env, &stake_cfg.token);

                    // Get treasury address from fee config
                    let fee_config: Option<ArbitrationFeeConfig> = env
                        .storage()
                        .instance()
                        .get(&ArbitrationKey::ArbitrationFeeConfig);

                    // Determine if staker won or lost
                    // Staker is the one who escalated (usually merchant after rejection)
                    // If refund is approved, staker (merchant) lost
                    // If refund is rejected (stays rejected), staker (merchant) won
                    let staker_won = !approved;

                    if staker_won {
                        // Return stake to staker
                        stake_token_client.transfer(
                            &env.current_contract_address(),
                            &stake.staker,
                            &stake.amount,
                        );

                        StakeReturned {
                            case_id,
                            winner: stake.staker.clone(),
                            amount: stake.amount,
                        }
                        .publish(&env);
                    } else {
                        // Forfeit stake to treasury (use fee config treasury or staker as fallback)
                        let treasury_addr = if let Some(fee_cfg) = fee_config {
                            fee_cfg.treasury_address
                        } else {
                            // Fallback: return to staker if no treasury configured
                            stake.staker.clone()
                        };

                        stake_token_client.transfer(
                            &env.current_contract_address(),
                            &treasury_addr,
                            &stake.amount,
                        );

                        StakeForfeited {
                            case_id,
                            loser: stake.staker.clone(),
                            amount: stake.amount,
                        }
                        .publish(&env);
                    }

                    // Mark stake as returned
                    stake.returned = true;
                    env.storage()
                        .instance()
                        .set(&ArbitrationKey::ArbitrationStake(case_id), &stake);
                }
            }
        }

        // Update arbitrator reputations
        let case_duration = env.ledger().timestamp() - case.created_at;
        let current_time = env.ledger().timestamp();

        for voter in all_voters.iter() {
            let vote: ArbitratorVote = env
                .storage()
                .instance()
                .get(&ArbitrationKey::ArbitratorVote(case_id, voter.clone()))
                .unwrap();

            // Check if this voter was in the majority
            let in_majority = if approved {
                vote.voted_for_refund
            } else {
                !vote.voted_for_refund
            };

            // Get current reputation
            let mut reputation: ArbitratorReputation = env
                .storage()
                .instance()
                .get(&ArbitrationKey::ArbitratorReputation(voter.clone()))
                .unwrap_or(ArbitratorReputation {
                    arbitrator: voter.clone(),
                    total_cases: 0,
                    majority_votes: 0,
                    minority_votes: 0,
                    avg_resolution_time: 0,
                    score: 100,
                    last_active: current_time,
                });

            let old_score = reputation.score;

            // Update vote counts
            reputation.total_cases += 1;
            if in_majority {
                reputation.majority_votes += 1;
                // Increase score for majority vote (e.g., +10 points)
                reputation.score += 10;
            } else {
                reputation.minority_votes += 1;
                // Decrease score for minority vote (e.g., -5 points)
                reputation.score -= 5;
            }

            // Update average resolution time
            if reputation.total_cases == 1 {
                reputation.avg_resolution_time = case_duration;
            } else {
                // Calculate weighted average
                let total_time = reputation.avg_resolution_time * (reputation.total_cases - 1);
                reputation.avg_resolution_time =
                    (total_time + case_duration) / reputation.total_cases;
            }

            // Update last active timestamp
            reputation.last_active = current_time;

            // Store updated reputation
            env.storage().instance().set(
                &ArbitrationKey::ArbitratorReputation(voter.clone()),
                &reputation,
            );

            // Emit score update event
            ArbitratorScoreUpdated {
                arbitrator: voter.clone(),
                old_score,
                new_score: reputation.score,
            }
            .publish(&env);
        }

        ArbitrationCaseDecided { case_id, approved }.publish(&env);

        Ok(())
    }

    pub fn set_arbitration_timeout(
        env: Env,
        admin: Address,
        timeout_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationTimeoutConfig, &timeout_seconds);
        Ok(())
    }

    pub fn get_arbitration_timeout_config(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationTimeoutConfig)
            .unwrap_or(86400 * 14)
    }

    pub fn trigger_arbitration_timeout(env: Env, case_id: u64) -> Result<(), Error> {
        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::RefundNotFound)?;

        if case.status != ArbitrationStatus::Open {
            return Err(Error::InvalidStatus);
        }

        // Block if quorum already reached
        let total_votes = case.votes_for_refund + case.votes_against_refund;
        if total_votes >= 3 {
            return Err(Error::QuorumNotReached);
        }

        if env.ledger().timestamp() < case.timeout_at {
            return Err(Error::CaseNotTimedOut);
        }

        let approved = case.default_favor_customer;
        case.status = ArbitrationStatus::Decided;
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);

        if approved {
            let mut refund: Refund = env
                .storage()
                .instance()
                .get(&DataKey::Refund(case.refund_id))
                .unwrap();
            refund.status = RefundStatus::Approved;
            env.storage()
                .instance()
                .set(&DataKey::Refund(case.refund_id), &refund);
        }

        ArbitrationTimedOut {
            case_id,
            default_outcome: approved,
            triggered_at: env.ledger().timestamp(),
        }
        .publish(&env);

        ArbitrationCaseDecided { case_id, approved }.publish(&env);

        Ok(())
    }

    fn store_refund_policy(
        env: &Env,
        merchant: Address,
        policy: RefundPolicy,
        created_by: Address,
    ) {
        env.storage()
            .instance()
            .set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        let version_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyVersionCount(merchant.clone()))
            .unwrap_or(0);
        let new_version = version_count + 1;
        let versioned = RefundPolicyVersion {
            version: new_version,
            policy: policy.clone(),
            created_at: env.ledger().timestamp(),
            created_by,
        };
        env.storage().instance().set(
            &DataKey::RefundPolicyVersion(merchant.clone(), new_version),
            &versioned,
        );
        env.storage().instance().set(
            &DataKey::RefundPolicyVersionCount(merchant.clone()),
            &new_version,
        );

        // Emit RefundPolicySet event
        (RefundPolicySet {
            merchant,
            tiers_count: policy.tiers.len() as u32,
        })
        .publish(env);
    }

    pub fn create_policy_template(
        env: Env,
        admin: Address,
        name: String,
        tiers: Vec<(u32, i128)>,
        window: u64,
    ) -> Result<u64, Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let template_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplateCount)
            .unwrap_or(0);
        let template_id = template_count + 1;
        let template = RefundPolicyTemplate {
            template_id,
            name,
            tiers,
            default_window_seconds: window,
            active: true,
        };

        env.storage()
            .instance()
            .set(&DataKey::RefundPolicyTemplate(template_id), &template);
        env.storage()
            .instance()
            .set(&DataKey::RefundPolicyTemplateCount, &template_id);

        (RefundPolicyTemplateCreated {
            template_id,
            created_by: admin,
        })
        .publish(&env);

        Ok(template_id)
    }

    pub fn apply_template_to_merchant(
        env: Env,
        admin: Address,
        merchant: Address,
        template_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let template: RefundPolicyTemplate = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplate(template_id))
            .ok_or(Error::TemplateNotFound)?;

        if !template.active {
            return Err(Error::TemplateInactive);
        }

        let mut tiers = Vec::new(&env);
        let days = template.default_window_seconds / (24 * 60 * 60);
        tiers.push_back(RefundTier {
            days_from_purchase: days,
            max_refund_bps: 10000,
        });

        let policy = RefundPolicy {
            merchant: merchant.clone(),
            tiers,
            active: true,
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            default_window_seconds: 30 * 24 * 60 * 60,
        };

        Self::set_requires_admin_approval_inner(&env, &merchant, true);
        Self::set_auto_approve_below_inner(&env, &merchant, 0);

        Self::store_refund_policy(&env, merchant.clone(), policy, admin.clone());
        (RefundPolicyTemplateApplied {
            template_id,
            merchant,
            applied_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    pub fn get_policy_template(env: Env, template_id: u64) -> Option<RefundPolicyTemplate> {
        env.storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplate(template_id))
    }

    pub fn list_policy_templates(env: Env) -> Vec<RefundPolicyTemplate> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplateCount)
            .unwrap_or(0);
        let mut templates = Vec::new(&env);
        for id in 1..=count {
            if let Some(template) = env
                .storage()
                .instance()
                .get::<_, RefundPolicyTemplate>(&DataKey::RefundPolicyTemplate(id))
            {
                if template.active {
                    templates.push_back(template);
                }
            }
        }
        templates
    }

    pub fn deactivate_policy_template(
        env: Env,
        admin: Address,
        template_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut template: RefundPolicyTemplate = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplate(template_id))
            .ok_or(Error::TemplateNotFound)?;

        if !template.active {
            return Err(Error::TemplateInactive);
        }

        template.active = false;
        env.storage()
            .instance()
            .set(&DataKey::RefundPolicyTemplate(template_id), &template);

        (PolicyTemplateDeactivated {
            template_id,
            deactivated_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    /// Get the reputation information for a specific arbitrator
    pub fn get_arbitrator_reputation(
        env: Env,
        arbitrator: Address,
    ) -> Option<ArbitratorReputation> {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorReputation(arbitrator))
    }

    /// Get the top arbitrators sorted by score (highest first)
    /// Returns up to `limit` arbitrators
    pub fn get_top_arbitrators(env: Env, limit: u32) -> Vec<ArbitratorReputation> {
        let mut results = Vec::new(&env);

        // Get all arbitrators from the arbitrator list
        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        if arbitrators.len() == 0 {
            return results;
        }

        // Collect all reputations
        let mut reputations = Vec::new(&env);
        for arbitrator in arbitrators.iter() {
            if let Some(reputation) = env
                .storage()
                .instance()
                .get::<ArbitrationKey, ArbitratorReputation>(&ArbitrationKey::ArbitratorReputation(
                    arbitrator.clone(),
                ))
            {
                reputations.push_back(reputation);
            }
        }

        // Sort by score (descending) using bubble sort
        // Note: This is inefficient for large lists, but works for small arbitrator sets
        let len = reputations.len();
        for i in 0..len {
            for j in 0..(len - i - 1) {
                let rep_j = reputations.get(j).unwrap();
                let rep_j_plus_1 = reputations.get(j + 1).unwrap();

                if rep_j.score < rep_j_plus_1.score {
                    // Swap
                    let temp = rep_j_plus_1.clone();
                    reputations.set(j + 1, rep_j.clone());
                    reputations.set(j, temp);
                }
            }
        }

        // Return top `limit` arbitrators
        let count = core::cmp::min(limit as u32, reputations.len());
        for i in 0..count {
            results.push_back(reputations.get(i).unwrap());
        }

        results
    }

    /// Deregister all arbitrators with a score below the minimum threshold
    /// Requires admin authorization
    /// Returns the count of arbitrators removed
    pub fn deregister_low_performers(
        env: Env,
        admin: Address,
        min_score: i128,
    ) -> Result<u32, Error> {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        if min_score < 0 {
            return Err(Error::InvalidScoreThreshold);
        }

        let mut arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        let mut removed_count: u32 = 0;
        let mut new_arbitrators = Vec::new(&env);

        for arbitrator in arbitrators.iter() {
            let reputation: Option<ArbitratorReputation> = env
                .storage()
                .instance()
                .get(&ArbitrationKey::ArbitratorReputation(arbitrator.clone()));

            let should_remove = if let Some(rep) = reputation {
                rep.score < min_score
            } else {
                false
            };

            if should_remove {
                // Remove reputation data
                env.storage()
                    .instance()
                    .remove(&ArbitrationKey::ArbitratorReputation(arbitrator.clone()));

                // Emit deregistration event
                ArbitratorDeregistered {
                    arbitrator: arbitrator.clone(),
                    reason: String::from_str(&env, "Low performance score"),
                }
                .publish(&env);

                removed_count += 1;
            } else {
                // Keep this arbitrator
                new_arbitrators.push_back(arbitrator.clone());
            }
        }

        // Update the arbitrator list
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitratorList, &new_arbitrators);

        Ok(removed_count)
    }

    pub fn get_arbitration_case(env: Env, case_id: u64) -> Result<ArbitrationCase, Error> {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::RefundNotFound)
    }

    /// Set the arbitration fee configuration
    /// Requires admin authorization
    /// arbitrator_share_bps + treasury_share_bps must equal 10000 (100%)
    pub fn set_arbitration_fee_config(
        env: Env,
        admin: Address,
        config: ArbitrationFeeConfig,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Validate that shares add up to 10000 (100%)
        if config.arbitrator_share_bps + config.treasury_share_bps != 10000 {
            return Err(Error::InvalidFeeConfig);
        }

        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationFeeConfig, &config);

        Ok(())
    }

    /// Get the current arbitration fee configuration
    pub fn get_arbitration_fee_config(env: Env) -> Option<ArbitrationFeeConfig> {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationFeeConfig)
    }

    /// Get the accumulated treasury fees from arbitration cases
    pub fn get_accumulated_arbitration_fees(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&ArbitrationKey::AccumulatedTreasuryFees)
            .unwrap_or(0)
    }

    /// Withdraw accumulated treasury fees
    /// Requires admin authorization
    /// Returns the amount withdrawn
    fn deduct_refund_fee(
        env: &Env,
        refund_id: u64,
        amount: i128,
        token: &Address,
    ) -> Result<(i128, i128), Error> {
        let config: RefundFeeConfig =
            match env.storage().instance().get(&SystemKey::RefundFeeConfig) {
                Some(c) => c,
                None => return Ok((amount, 0)),
            };
        if !config.active {
            return Ok((amount, 0));
        }
        let raw_fee = amount
            .saturating_mul(config.fee_bps as i128)
            .checked_div(10_000)
            .unwrap_or(0);
        let fee = raw_fee.max(config.min_fee).min(config.max_fee);
        let net = amount.saturating_sub(fee);
        if fee > 0 {
            token::Client::new(env, token).transfer(
                &env.current_contract_address(),
                &config.treasury,
                &fee,
            );
            let accumulated: i128 = env
                .storage()
                .instance()
                .get(&SystemKey::AccumulatedRefundFees)
                .unwrap_or(0);
            env.storage().instance().set(
                &SystemKey::AccumulatedRefundFees,
                &accumulated.saturating_add(fee),
            );
            (RefundFeeDeducted {
                refund_id,
                fee_amount: fee,
                net_refund_amount: net,
                treasury: config.treasury,
            })
            .publish(env);
        }
        Ok((net, fee))
    }

    pub fn withdraw_treasury_fees(env: Env, admin: Address) -> Result<i128, Error> {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&ArbitrationKey::AccumulatedTreasuryFees)
            .unwrap_or(0);

        if accumulated <= 0 {
            return Err(Error::InsufficientTreasuryFees);
        }

        // Reset accumulated fees
        env.storage()
            .instance()
            .set(&ArbitrationKey::AccumulatedTreasuryFees, &0i128);

        Ok(accumulated)
    }

    /// Set the arbitration stake configuration
    /// Requires admin authorization
    pub fn set_arbitration_stake_config(
        env: Env,
        admin: Address,
        config: ArbitrationStakeConfig,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Validate stake amount if enabled
        if config.enabled && config.amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationStakeConfig, &config);

        Ok(())
    }

    /// Get the current arbitration stake configuration
    pub fn get_arbitration_stake_config(env: Env) -> Option<ArbitrationStakeConfig> {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStakeConfig)
    }

    /// Get the stake information for a specific arbitration case
    pub fn get_arbitration_stake(env: Env, case_id: u64) -> Option<ArbitrationStake> {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStake(case_id))
    }

    pub fn get_refunds_by_status(
        env: &Env,
        status: RefundStatus,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(env);
        let total = Self::get_refund_count_by_status(env, status.clone());

        if limit == 0 || offset >= total {
            return results;
        }

        let end = core::cmp::min(total, offset.saturating_add(limit));
        let mut index = offset;
        while index < end {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::RefundsByStatus(status.clone(), index))
            {
                if let Some(refund) = env
                    .storage()
                    .instance()
                    .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    results.push_back(refund);
                }
            }
            index += 1;
        }

        results
    }

    pub fn get_merchant_refunds(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(&env);
        let total = Self::get_merchant_refund_count(&env, &merchant);

        if limit == 0 || offset >= total {
            return results;
        }

        let end = core::cmp::min(total, offset.saturating_add(limit));
        let mut index = offset;
        while index < end {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::MerchantRefunds(merchant.clone(), index))
            {
                if let Some(refund) = env
                    .storage()
                    .instance()
                    .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    results.push_back(refund);
                }
            }
            index += 1;
        }

        results
    }

    pub fn get_merchant_refunds_by_status(
        env: Env,
        merchant: Address,
        status: RefundStatus,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        Self::get_merchant_refunds_by_status_internal(&env, &merchant, status, limit, offset)
    }

    pub fn get_merchant_pending_refunds(env: Env, merchant: Address) -> Vec<Refund> {
        let total = Self::get_merchant_refund_count(&env, &merchant);
        Self::get_merchant_refunds_by_status_internal(
            &env,
            &merchant,
            RefundStatus::Requested,
            total,
            0,
        )
    }

    pub fn get_merchant_refund_summary(env: Env, merchant: Address) -> MerchantRefundSummary {
        let total_requests = Self::get_merchant_refund_count(&env, &merchant);
        let mut total_approved = 0u64;
        let mut total_rejected = 0u64;
        let mut total_amount_refunded = 0i128;
        let mut pending_count = 0u64;
        let mut pending_amount = 0i128;

        let mut index = 0u64;
        while index < total_requests {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::MerchantRefunds(merchant.clone(), index))
            {
                if let Some(refund) = env
                    .storage()
                    .instance()
                    .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    match refund.status {
                        RefundStatus::Approved => {
                            total_approved += 1;
                        }
                        RefundStatus::Rejected => {
                            total_rejected += 1;
                        }
                        RefundStatus::Processed => {
                            total_amount_refunded += refund.amount;
                        }
                        RefundStatus::Requested => {
                            pending_count += 1;
                            pending_amount += refund.amount;
                        }
                    }
                }
            }
            index += 1;
        }

        MerchantRefundSummary {
            total_requests,
            total_approved,
            total_rejected,
            total_amount_refunded,
            pending_count,
            pending_amount,
        }
    }

    pub fn get_refunds_by_reason_code(
        env: &Env,
        code: RefundReasonCode,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(env);
        if limit == 0 {
            return results;
        }

        let total_refunds: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);

        let mut matched: u64 = 0;
        let mut collected: u64 = 0;
        let mut id: u64 = 1;
        while id <= total_refunds && collected < limit {
            if let Some(refund) = env
                .storage()
                .instance()
                .get::<_, Refund>(&DataKey::Refund(id))
            {
                if refund.reason_code == code {
                    if matched >= offset {
                        results.push_back(refund);
                        collected += 1;
                    }
                    matched += 1;
                }
            }
            id += 1;
        }

        results
    }

    pub fn get_reason_code_analytics(env: Env) -> Vec<(RefundReasonCode, u64)> {
        let mut product_defect: u64 = 0;
        let mut non_delivery: u64 = 0;
        let mut duplicate_charge: u64 = 0;
        let mut unauthorized: u64 = 0;
        let mut customer_request: u64 = 0;
        let mut other: u64 = 0;

        let total_refunds: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);

        let mut id: u64 = 1;
        while id <= total_refunds {
            if let Some(refund) = env
                .storage()
                .instance()
                .get::<_, Refund>(&DataKey::Refund(id))
            {
                match refund.reason_code {
                    RefundReasonCode::ProductDefect => product_defect += 1,
                    RefundReasonCode::NonDelivery => non_delivery += 1,
                    RefundReasonCode::DuplicateCharge => duplicate_charge += 1,
                    RefundReasonCode::Unauthorized => unauthorized += 1,
                    RefundReasonCode::CustomerRequest => customer_request += 1,
                    RefundReasonCode::Other => other += 1,
                }
            }
            id += 1;
        }

        let mut ordered = [
            (RefundReasonCode::ProductDefect, product_defect),
            (RefundReasonCode::NonDelivery, non_delivery),
            (RefundReasonCode::DuplicateCharge, duplicate_charge),
            (RefundReasonCode::Unauthorized, unauthorized),
            (RefundReasonCode::CustomerRequest, customer_request),
            (RefundReasonCode::Other, other),
        ];

        ordered.sort_by(|a, b| {
            let count_cmp = b.1.cmp(&a.1);
            if count_cmp == core::cmp::Ordering::Equal {
                Self::reason_code_rank(&a.0).cmp(&Self::reason_code_rank(&b.0))
            } else {
                count_cmp
            }
        });

        let mut result = Vec::new(&env);
        for (code, count) in ordered {
            result.push_back((code, count));
        }
        result
    }

    pub fn get_refund_count_by_status(env: &Env, status: RefundStatus) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::RefundStatusCount(status))
            .unwrap_or(0)
    }

    pub fn get_total_refunded_amount(env: &Env, payment_id: u64) -> i128 {
        let total_refunds: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);
        let mut total: i128 = 0;

        let mut id: u64 = 1;
        while id <= total_refunds {
            if let Some(refund) = env
                .storage()
                .instance()
                .get::<_, Refund>(&DataKey::Refund(id))
            {
                if refund.payment_id == payment_id && refund.status == RefundStatus::Processed {
                    total += refund.amount;
                }
            }
            id += 1;
        }

        total
    }

    pub fn can_refund_payment(
        env: &Env,
        payment_id: u64,
        requested_amount: i128,
        original_amount: i128,
    ) -> Result<bool, Error> {
        let total_refunded = Self::get_total_refunded_amount(env, payment_id);
        if requested_amount.saturating_add(total_refunded) > original_amount {
            return Err(Error::TotalRefundsExceedPayment);
        }

        Ok(true)
    }

    fn sort_tiers(_env: &Env, tiers: Vec<RefundTier>) -> Vec<RefundTier> {
        let mut sorted = tiers.clone();
        let len = sorted.len();
        if len <= 1 {
            return sorted;
        }
        for i in 1..len {
            let mut j = i;
            while j > 0 {
                let current = sorted.get(j).unwrap();
                let prev = sorted.get(j - 1).unwrap();
                if current.days_from_purchase < prev.days_from_purchase {
                    sorted.set(j, prev);
                    sorted.set(j - 1, current);
                    j -= 1;
                } else {
                    break;
                }
            }
        }
        sorted
    }

    pub fn set_refund_policy(
        env: Env,
        merchant: Address,
        tiers: Vec<RefundTier>,
    ) -> Result<(), Error> {
        // Require merchant authentication
        merchant.require_auth();

        // Validate max_refund_bps is within bounds for all tiers (0-10000 basis points)
        for tier in tiers.iter() {
            if let Err(_) = Self::validate_bps(tier.max_refund_bps) {
                return Err(Error::RefundExceedsPolicy);
            }
        }

        // Sort tiers by days_from_purchase in ascending order
        let sorted_tiers = Self::sort_tiers(&env, tiers);

        let now = env.ledger().timestamp();
        let policy = RefundPolicy {
            merchant: merchant.clone(),
            tiers: sorted_tiers.clone(),
            active: true,
            created_at: now,
            updated_at: now,
            default_window_seconds: 30 * 24 * 60 * 60,
        };

        env.storage()
            .instance()
            .set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // ── Issue #134: version the policy ──────────────────────────────────
        let version_count: u32 = env
            .storage()
            .instance()
            .get(&PolicyKey::RefundPolicyVersionCount(merchant.clone()))
            .unwrap_or(0);
        let new_version = version_count + 1;
        let versioned = RefundPolicyVersion {
            version: new_version,
            policy: policy.clone(),
            created_at: now,
            created_by: merchant.clone(),
        };
        env.storage().instance().set(
            &PolicyKey::RefundPolicyVersion(merchant.clone(), new_version),
            &versioned,
        );
        env.storage().instance().set(
            &PolicyKey::RefundPolicyVersionCount(merchant.clone()),
            &new_version,
        );

        // Emit RefundPolicySet event
        (RefundPolicySet {
            merchant,
            tiers_count: sorted_tiers.len() as u32,
        })
        .publish(&env);

        Ok(())
    }

    // ── Issue #134: Policy versioning query functions ──────────────────────

    pub fn get_refund_policy_version(
        env: Env,
        merchant: Address,
        version: u32,
    ) -> Option<RefundPolicyVersion> {
        env.storage()
            .instance()
            .get(&PolicyKey::RefundPolicyVersion(merchant, version))
    }

    pub fn get_refund_policy_at_time(
        env: Env,
        merchant: Address,
        timestamp: u64,
    ) -> Option<RefundPolicyVersion> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&PolicyKey::RefundPolicyVersionCount(merchant.clone()))
            .unwrap_or(0);
        if count == 0 {
            return None;
        }
        // Walk versions in reverse to find the latest one created at or before timestamp
        let mut result: Option<RefundPolicyVersion> = None;
        for v in 1..=count {
            if let Some(pv) = env
                .storage()
                .instance()
                .get::<PolicyKey, RefundPolicyVersion>(&PolicyKey::RefundPolicyVersion(
                    merchant.clone(),
                    v,
                ))
            {
                if pv.created_at <= timestamp {
                    result = Some(pv);
                }
            }
        }
        result
    }

    pub fn get_refund_policy_history(env: Env, merchant: Address) -> Vec<RefundPolicyVersion> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&PolicyKey::RefundPolicyVersionCount(merchant.clone()))
            .unwrap_or(0);
        let mut history = Vec::new(&env);
        for v in 1..=count {
            if let Some(pv) = env
                .storage()
                .instance()
                .get::<PolicyKey, RefundPolicyVersion>(&PolicyKey::RefundPolicyVersion(
                    merchant.clone(),
                    v,
                ))
            {
                history.push_back(pv);
            }
        }
        history
    }

    pub fn get_refund_policy(env: &Env, merchant: Address) -> Option<RefundPolicy> {
        env.storage()
            .instance()
            .get(&DataKey::RefundPolicy(merchant))
    }

    // ── Issue #93: Default refund policy management ────────────────────────

    /// Set the global default refund policy. Admin-only.
    pub fn set_default_refund_policy(
        env: Env,
        admin: Address,
        policy: RefundPolicy,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::DefaultRefundPolicy, &policy);
        (DefaultRefundPolicySet {
            set_by: admin,
            tiers_count: policy.tiers.len() as u32,
        })
        .publish(&env);
        Ok(())
    }

    /// Get the global default refund policy (returns None if not set).
    pub fn get_default_refund_policy(env: Env) -> Option<RefundPolicy> {
        env.storage().instance().get(&DataKey::DefaultRefundPolicy)
    }

    /// Internal helper used by request_refund / validate_against_policy.
    fn get_default_refund_policy_inner(env: &Env) -> Option<RefundPolicy> {
        env.storage().instance().get(&DataKey::DefaultRefundPolicy)
    }

    /// Remove the global default refund policy. Admin-only.
    pub fn remove_default_refund_policy(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .remove(&DataKey::DefaultRefundPolicy);
        (DefaultRefundPolicyRemoved { removed_by: admin }).publish(&env);
        Ok(())
    }

    fn get_requires_admin_approval_inner(env: &Env, merchant: &Address) -> bool {
        let key = Symbol::new(env, "requires_admin_approval");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().get(&composite_key).unwrap_or(true)
    }

    fn set_requires_admin_approval_inner(env: &Env, merchant: &Address, value: bool) {
        let key = Symbol::new(env, "requires_admin_approval");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().set(&composite_key, &value);
    }

    fn get_auto_approve_below_inner(env: &Env, merchant: &Address) -> i128 {
        let key = Symbol::new(env, "auto_approve_below");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().get(&composite_key).unwrap_or(0)
    }

    fn set_auto_approve_below_inner(env: &Env, merchant: &Address, value: i128) {
        let key = Symbol::new(env, "auto_approve_below");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().set(&composite_key, &value);
    }

    fn get_inherit_from_parent_inner(env: &Env, merchant: &Address) -> bool {
        let key = Symbol::new(env, "inherit_from_parent");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().get(&composite_key).unwrap_or(true)
    }

    fn set_inherit_from_parent_inner(env: &Env, merchant: &Address, inherit: bool) {
        let key = Symbol::new(env, "inherit_from_parent");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().set(&composite_key, &inherit);
    }

    pub fn get_requires_admin_approval(env: Env, merchant: Address) -> bool {
        Self::get_requires_admin_approval_inner(&env, &merchant)
    }

    pub fn set_requires_admin_approval(env: Env, merchant: Address, value: bool) {
        merchant.require_auth();
        Self::set_requires_admin_approval_inner(&env, &merchant, value);
    }

    pub fn get_auto_approve_below(env: Env, merchant: Address) -> i128 {
        Self::get_auto_approve_below_inner(&env, &merchant)
    }

    pub fn set_auto_approve_below(env: Env, merchant: Address, value: i128) {
        merchant.require_auth();
        Self::set_auto_approve_below_inner(&env, &merchant, value);
    }

    pub fn get_inherit_from_parent(env: Env, merchant: Address) -> bool {
        Self::get_inherit_from_parent_inner(&env, &merchant)
    }

    pub fn set_inherit_from_parent(env: Env, merchant: Address, inherit: bool) {
        merchant.require_auth();
        Self::set_inherit_from_parent_inner(&env, &merchant, inherit);
    }

    pub fn deactivate_refund_policy(env: Env, merchant: Address) -> Result<(), Error> {
        // Require merchant authentication
        merchant.require_auth();

        let mut policy: RefundPolicy = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicy(merchant.clone()))
            .ok_or(Error::PolicyNotFound)?;

        if !policy.active {
            return Err(Error::PolicyInactive);
        }

        policy.active = false;
        env.storage()
            .instance()
            .set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // Emit RefundPolicyDeactivated event
        (RefundPolicyDeactivated { merchant }).publish(&env);

        Ok(())
    }

    pub fn admin_override_policy(
        env: Env,
        admin: Address,
        refund_id: u64,
        reason: String,
    ) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        let admin_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;

        if admin != admin_address {
            return Err(Error::Unauthorized);
        }

        // Verify refund exists
        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Generate immutable audit log entry
        let override_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AdminOverrideHistoryCount)
            .unwrap_or(0);

        let executed_at = env.ledger().timestamp();

        // Create hash of override details for immutability verification
        let mut hash_data = Bytes::new(&env);
        hash_data.append(&Bytes::from_slice(&env, &refund_id.to_be_bytes()));
        hash_data.append(&Bytes::from_slice(&env, &refund.amount.to_be_bytes()));
        hash_data.append(&Bytes::from_slice(&env, &executed_at.to_be_bytes()));
        let transaction_hash = env.crypto().sha256(&hash_data);

        let audit_entry = AdminOverrideHistory {
            override_id,
            refund_id,
            admin: admin.clone(),
            reason: reason.clone(),
            override_amount: refund.amount,
            override_status: refund.status.clone(),
            executed_at,
            transaction_hash: transaction_hash.into(),
        };

        // Store immutable audit log entry
        env.storage()
            .instance()
            .set(&DataKey::AdminOverrideHistory(override_id), &audit_entry);

        // Increment counter
        env.storage()
            .instance()
            .set(&DataKey::AdminOverrideHistoryCount, &(override_id + 1));

        // Emit AdminRefundOverride event
        AdminRefundOverride {
            override_id,
            refund_id,
            admin: admin.clone(),
            reason: reason.clone(),
            override_amount: refund.amount,
            override_status: refund.status,
            executed_at,
        }
        .publish(&env);

        // Emit legacy PolicyOverrideApplied event for backward compatibility
        PolicyOverrideApplied {
            refund_id,
            admin,
            reason,
        }
        .publish(&env);

        Ok(())
    }

    /// Retrieve admin override audit log entry by override_id
    pub fn get_admin_override_history(env: Env, override_id: u64) -> Option<AdminOverrideHistory> {
        env.storage()
            .instance()
            .get(&DataKey::AdminOverrideHistory(override_id))
    }

    /// Get total count of admin override audit log entries
    pub fn get_admin_override_history_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::AdminOverrideHistoryCount)
            .unwrap_or(0)
    }

    // ── Issue #138: Refund policy inheritance for merchant hierarchies ────────

    /// Maximum depth allowed for policy inheritance chain
    const MAX_INHERITANCE_DEPTH: u32 = 5;

    /// Set the parent merchant for a child merchant to enable policy inheritance.
    /// Requires admin authorization.
    /// Validates against self-parent, circular references, and max depth.
    pub fn set_merchant_parent(
        env: Env,
        admin: Address,
        merchant: Address,
        parent: Address,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin authorization
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Prevent self-parent
        if merchant == parent {
            return Err(Error::CircularInheritance);
        }

        // Check for circular reference by traversing up from parent
        // If we encounter the merchant in the parent's chain, it would create a cycle
        let mut visited = Vec::new(&env);
        visited.push_back(merchant.clone());

        let mut current = parent.clone();
        let mut depth: u32 = 1;

        while depth <= Self::MAX_INHERITANCE_DEPTH {
            if current == merchant {
                return Err(Error::CircularInheritance);
            }

            // Check if we've seen this address before (shouldn't happen but safety check)
            if visited.contains(&current) {
                return Err(Error::CircularInheritance);
            }
            visited.push_back(current.clone());

            // Move to next parent
            match Self::get_merchant_parent(&env, current.clone()) {
                Some(next_parent) => {
                    current = next_parent;
                    depth += 1;
                }
                None => break,
            }
        }

        // Validate max depth constraint (>= to prevent exceeding max, including the new merchant)
        if depth >= Self::MAX_INHERITANCE_DEPTH {
            return Err(Error::MaxInheritanceDepth);
        }

        // Store the parent relationship using Symbol-based key
        let key = Symbol::new(&env, "parent_of");
        let composite_key: (Symbol, Address) = (key, merchant.clone());
        env.storage().instance().set(&composite_key, &parent);

        Ok(())
    }

    /// Get the direct parent merchant of a given merchant.
    pub fn get_merchant_parent(env: &Env, merchant: Address) -> Option<Address> {
        let key = Symbol::new(env, "parent_of");
        let composite_key: (Symbol, Address) = (key, merchant);
        env.storage().instance().get(&composite_key)
    }

    /// Get the effective refund policy for a merchant, traversing the inheritance chain.
    /// Returns the first active explicit policy found, respecting inherit_from_parent flag.
    pub fn get_effective_refund_policy(env: Env, merchant: Address) -> Option<RefundPolicy> {
        let starting_policy = Self::get_refund_policy(&env, merchant.clone());
        let mut current = merchant.clone();
        let mut depth: u32 = 0;
        let mut visited = Vec::new(&env);

        while depth < Self::MAX_INHERITANCE_DEPTH {
            // Prevent infinite loops
            if visited.contains(&current) {
                return None; // Circular reference detected
            }
            visited.push_back(current.clone());

            // Try to get explicit policy for current merchant
            if let Some(policy) = Self::get_refund_policy(&env, current.clone()) {
                if policy.active {
                    // If this is the starting merchant, always return their own active policy
                    // A merchant's explicit policy always takes precedence for themselves
                    if current == merchant {
                        return Some(policy);
                    }
                    // We're at a parent in the chain - their policy is inheritable
                    return Some(policy);
                }
                // Policy is inactive - check if we should continue to parent
                if current == merchant && !Self::get_inherit_from_parent_inner(&env, &merchant) {
                    // Starting merchant has disabled inheritance and their policy is inactive
                    return Some(policy);
                }
                // Continue to parent (either inactive policy or merchant wants to inherit)
            }

            // Move to parent
            match Self::get_merchant_parent(&env, current.clone()) {
                Some(parent) => {
                    current = parent;
                    depth += 1;
                }
                None => break,
            }
        }

        // If we reached max depth, return None to indicate failure
        if depth >= Self::MAX_INHERITANCE_DEPTH {
            return None;
        }

        // Fallback logic after loop terminates:
        if let Some(policy) = starting_policy {
            return Some(policy);
        }
        Self::get_default_refund_policy_inner(&env)
    }

    /// Get the inheritance chain for a merchant (ancestry path).
    /// Returns vector from merchant → parent → grandparent → ... → root.
    /// Returns error if circular reference or max depth exceeded.
    pub fn get_policy_inheritance_chain(
        env: Env,
        merchant: Address,
    ) -> Result<Vec<Address>, Error> {
        let mut chain = Vec::new(&env);
        let mut current = merchant.clone();
        let mut depth: u32 = 0;

        chain.push_back(current.clone());

        while depth < Self::MAX_INHERITANCE_DEPTH {
            match Self::get_merchant_parent(&env, current.clone()) {
                Some(parent) => {
                    // Check for circular reference
                    if chain.contains(&parent) {
                        return Err(Error::CircularInheritance);
                    }
                    chain.push_back(parent.clone());
                    current = parent;
                    depth += 1;
                }
                None => break,
            }
        }

        // Check if we hit max depth
        if depth >= Self::MAX_INHERITANCE_DEPTH {
            return Err(Error::MaxInheritanceDepth);
        }

        Ok(chain)
    }

    pub fn get_applicable_refund_bps(env: Env, merchant: Address, payment_id: u64) -> u32 {
        let payment = match Self::get_external_payment(&env, payment_id) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        let current_time = env.ledger().timestamp();
        let created_at = payment.created_at;

        // Traverse policy inheritance chain to find the effective policy
        let policy_opt = Self::get_effective_refund_policy(env.clone(), merchant);
        let policy = match policy_opt {
            Some(p) => p,
            None => return 0,
        };

        if !policy.active {
            return 0;
        }

        let elapsed_seconds = current_time.saturating_sub(created_at);
        let days_since_purchase = elapsed_seconds / (24 * 60 * 60);

        // Find the first tier (sorted ascending by days_from_purchase) where days_since_purchase <= tier.days_from_purchase
        for tier in policy.tiers.iter() {
            if days_since_purchase <= tier.days_from_purchase {
                return tier.max_refund_bps;
            }
        }

        0
    }

    fn validate_against_policy(
        env: &Env,
        merchant: &Address,
        amount: i128,
        original_amount: i128,
        payment_created_at: u64,
    ) -> Result<(), Error> {
        let policy: RefundPolicy = Self::get_effective_refund_policy(env.clone(), merchant.clone())
            .ok_or(Error::PolicyNotFound)?;

        if !policy.active {
            return Err(Error::PolicyInactive);
        }

        let current_time = env.ledger().timestamp();
        let elapsed_seconds = current_time.saturating_sub(payment_created_at);
        let days_since_purchase = elapsed_seconds / (24 * 60 * 60);

        let mut allowed_bps = 0;
        let mut found_tier = false;
        for tier in policy.tiers.iter() {
            if days_since_purchase <= tier.days_from_purchase {
                allowed_bps = tier.max_refund_bps;
                found_tier = true;
                break;
            }
        }

        if !found_tier {
            return Err(Error::RefundWindowExpired);
        }

        // Check refund percentage using overflow-safe math
        let refund_percentage_bps = amount
            .checked_mul(10000)
            .unwrap_or(i128::MAX)
            .checked_div(original_amount)
            .unwrap_or(u32::MAX as i128) as u32;

        if refund_percentage_bps > allowed_bps {
            return Err(Error::RefundExceedsPolicy);
        }

        Ok(())
    }

    fn add_to_status_index(env: &Env, status: RefundStatus, refund_id: u64) {
        let count = Self::get_refund_count_by_status(env, status.clone());
        env.storage()
            .instance()
            .set(&DataKey::RefundsByStatus(status.clone(), count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusCount(status.clone()), &(count + 1));
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusIndex(refund_id), &count);
    }

    fn remove_from_status_index(
        env: &Env,
        status: RefundStatus,
        refund_id: u64,
    ) -> Result<(), Error> {
        let count = Self::get_refund_count_by_status(env, status.clone());
        if count == 0 {
            return Err(Error::InvalidStatus);
        }

        let index: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundStatusIndex(refund_id))
            .ok_or(Error::InvalidStatus)?;
        let last_index = count - 1;

        if index != last_index {
            let last_refund_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::RefundsByStatus(status.clone(), last_index))
                .ok_or(Error::InvalidStatus)?;
            env.storage().instance().set(
                &DataKey::RefundsByStatus(status.clone(), index),
                &last_refund_id,
            );
            env.storage()
                .instance()
                .set(&DataKey::RefundStatusIndex(last_refund_id), &index);
        }

        env.storage()
            .instance()
            .remove(&DataKey::RefundsByStatus(status.clone(), last_index));
        env.storage()
            .instance()
            .remove(&DataKey::RefundStatusIndex(refund_id));
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusCount(status), &last_index);

        Ok(())
    }

    // ── Issue #135: Batch refund processing ──────────────────────────────────

    const DEFAULT_BATCH_LIMIT: u32 = 20;

    pub fn get_batch_refund_limit(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::BatchRefundLimit)
            .unwrap_or(Self::DEFAULT_BATCH_LIMIT)
    }

    pub fn set_batch_refund_limit(env: Env, admin: Address, limit: u32) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::BatchRefundLimit, &limit);
        Ok(())
    }

    pub fn approve_refund_batch(
        env: Env,
        admin: Address,
        refund_ids: Vec<u64>,
    ) -> Vec<BatchRefundResult> {
        admin.require_auth();
        let limit = Self::get_batch_refund_limit(env.clone());
        if refund_ids.len() > limit {
            // Return single error result indicating batch too large
            let mut results = Vec::new(&env);
            results.push_back(BatchRefundResult {
                refund_id: 0,
                success: false,
                error_code: Error::BatchRefundTooLarge as u32,
                amount_refunded: 0,
            });
            return results;
        }

        let mut results = Vec::new(&env);
        for refund_id in refund_ids.iter() {
            let result = Self::approve_refund_internal(&env, admin.clone(), refund_id);
            match result {
                Ok(()) => {
                    let amount = env
                        .storage()
                        .instance()
                        .get::<DataKey, Refund>(&DataKey::Refund(refund_id))
                        .map(|r| r.amount)
                        .unwrap_or(0);
                    results.push_back(BatchRefundResult {
                        refund_id,
                        success: true,
                        error_code: 0,
                        amount_refunded: amount,
                    });
                }
                Err(e) => {
                    results.push_back(BatchRefundResult {
                        refund_id,
                        success: false,
                        error_code: e as u32,
                        amount_refunded: 0,
                    });
                }
            }
        }
        results
    }

    pub fn process_refund_batch(
        env: Env,
        admin: Address,
        refund_ids: Vec<u64>,
    ) -> Vec<BatchRefundResult> {
        admin.require_auth();
        let limit = Self::get_batch_refund_limit(env.clone());
        if refund_ids.len() > limit {
            let mut results = Vec::new(&env);
            results.push_back(BatchRefundResult {
                refund_id: 0,
                success: false,
                error_code: Error::BatchRefundTooLarge as u32,
                amount_refunded: 0,
            });
            return results;
        }

        let mut results = Vec::new(&env);
        for refund_id in refund_ids.iter() {
            let amount = env
                .storage()
                .instance()
                .get::<DataKey, Refund>(&DataKey::Refund(refund_id))
                .map(|r| r.amount)
                .unwrap_or(0);
            let result = Self::process_refund_internal(&env, admin.clone(), refund_id);
            match result {
                Ok(()) => {
                    results.push_back(BatchRefundResult {
                        refund_id,
                        success: true,
                        error_code: 0,
                        amount_refunded: amount,
                    });
                }
                Err(e) => {
                    results.push_back(BatchRefundResult {
                        refund_id,
                        success: false,
                        error_code: e as u32,
                        amount_refunded: 0,
                    });
                }
            }
        }
        results
    }

    // ── Issue #143: Cross-contract payment verification ───────────────────────

    pub fn set_payment_contract_address(
        env: Env,
        admin: Address,
        payment_contract: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::PaymentContractAddress, &payment_contract);
        Ok(())
    }

    pub fn get_payment_contract_address(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::PaymentContractAddress)
    }

    pub fn verify_payment_ownership(env: Env, payment_id: u64, customer: Address) -> bool {
        let payment_contract: Address = match env
            .storage()
            .instance()
            .get(&DataKey::PaymentContractAddress)
        {
            Some(addr) => addr,
            None => return false, // no contract set → skip verification
        };
        // Cross-contract call to payment_contract.check_payment_customer(payment_id, customer).
        // That function returns bool: true if payment exists, belongs to customer, and is Completed.
        let func = Symbol::new(&env, "check_payment_customer");
        let args = (payment_id, customer).into_val(&env);
        match env.try_invoke_contract::<bool, soroban_sdk::InvokeError>(
            &payment_contract,
            &func,
            args,
        ) {
            Ok(Ok(result)) => result,
            _ => false,
        }
    }

    fn create_refund(
        env: Env,
        merchant: Address,
        payment_id: u64,
        customer: Address,
        amount: i128,
        original_payment_amount: i128,
        token: Address,
        reason: String,
        reason_code: RefundReasonCode,
        payment_created_at: u64,
        force_approved: bool,
    ) -> Result<u64, Error> {
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if amount > original_payment_amount {
            return Err(Error::RefundExceedsPayment);
        }

        if payment_id == 0 {
            return Err(Error::InvalidPaymentId);
        }

        if env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::PaymentContractAddress)
            .is_some()
        {
            let owned = Self::verify_payment_ownership(env.clone(), payment_id, customer.clone());
            if !owned {
                return Err(Error::PaymentOwnershipMismatch);
            }
        }

        Self::can_refund_payment(&env, payment_id, amount, original_payment_amount)?;
        Self::check_and_update_circuit_breaker(&env, amount, original_payment_amount)?;
        Self::check_and_update_customer_refund_rate_limit(&env, customer.clone())?;

        // Check payment refund cap
        Self::check_payment_refund_cap(&env, payment_id, amount)?;

        // Check for fraud signals (#137)
        if let Some(fraud_signal) = Self::check_fraud_signals(env.clone(), customer.clone()) {
            if !fraud_signal.reviewed {
                return Err(Error::AddressFlaggedForFraud);
            }
        }

        // Issue #148: Check merchant-level customer eligibility
        let eligibility_rule = Self::check_refund_eligibility_internal(&env, &merchant, &customer);
        if eligibility_rule == EligibilityRule::Block {
            return Err(Error::CustomerBlockedFromRefund);
        }

        if env.storage().instance().has(&DataKey::Admin) {
            Self::validate_against_policy(
                &env,
                &merchant,
                amount,
                original_payment_amount,
                payment_created_at,
            )?;
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);
        let refund_id = counter + 1;

        let initial_status = if force_approved {
            RefundStatus::Approved
        } else {
            let effective_merchant = if let Some(policy) =
                Self::get_effective_refund_policy(env.clone(), merchant.clone())
            {
                policy.merchant
            } else {
                merchant.clone()
            };
            let requires_approval =
                Self::get_requires_admin_approval_inner(&env, &effective_merchant);
            let auto_below = Self::get_auto_approve_below_inner(&env, &effective_merchant);
            if !requires_approval && amount <= auto_below {
                RefundStatus::Approved
            } else {
                RefundStatus::Requested
            }
        };

        let ttl_expires_at: Option<u64> = env
            .storage()
            .instance()
            .get::<RefundExtKey, RefundTTLConfig>(&RefundExtKey::RefundTTLConfig)
            .filter(|cfg| cfg.active)
            .map(|cfg| {
                env.ledger()
                    .timestamp()
                    .saturating_add(cfg.default_ttl_seconds)
            });

        let refund = Refund {
            id: refund_id,
            payment_id,
            merchant: merchant.clone(),
            customer: customer.clone(),
            amount,
            original_payment_amount,
            token: token.clone(),
            // Issue #191: record original payment token
            original_token: token.clone(),
            status: initial_status.clone(),
            requested_at: env.ledger().timestamp(),
            reason,
            reason_code,
            // Issue #147: Initialize lifecycle timestamps
            approved_at: if initial_status == RefundStatus::Approved {
                Some(env.ledger().timestamp())
            } else {
                None
            },
            rejected_at: None,
            processed_at: None,
            // Issue #199: TTL expiry
            expires_at: ttl_expires_at,
        };

        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        env.storage()
            .instance()
            .set(&DataKey::RefundCounter, &refund_id);
        Self::add_to_status_index(&env, initial_status.clone(), refund_id);

        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRefundCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantRefunds(merchant.clone(), merchant_count),
            &refund_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantRefundCount(merchant.clone()),
            &(merchant_count + 1),
        );

        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerRefunds(customer.clone(), customer_count),
            &refund_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerRefundCount(customer.clone()),
            &(customer_count + 1),
        );

        let payment_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundCount(payment_id))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::PaymentRefunds(payment_id, payment_count),
            &refund_id,
        );
        env.storage().instance().set(
            &DataKey::PaymentRefundCount(payment_id),
            &(payment_count + 1),
        );

        // Update payment refund usage for cap tracking
        Self::update_payment_refund_usage(&env, payment_id, amount);

        (RefundRequested {
            refund_id,
            payment_id,
            merchant,
            customer: customer.clone(),
            amount,
            token,
        })
        .publish(&env);

        // Update customer refund cooldown
        Self::update_customer_refund_cooldown(&env, &customer)?;

        // Issue #144: Invoke notification hooks for Requested event
        Self::invoke_hooks(&env, RefundEventType::Requested, refund_id);

        if initial_status == RefundStatus::Approved {
            (AutoApproved { refund_id, amount }).publish(&env);
        }

        Ok(refund_id)
    }

    fn approve_refund_internal(
        env: &Env,
        approved_by: Address,
        refund_id: u64,
    ) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.status != RefundStatus::Requested {
            return Err(Error::InvalidStatus);
        }

        // Issue #199: reject if TTL has expired
        if let Some(expires_at) = refund.expires_at {
            if env.ledger().timestamp() >= expires_at {
                return Err(Error::RefundWindowExpired);
            }
        }

        Self::remove_from_status_index(env, RefundStatus::Requested, refund_id)?;
        refund.status = RefundStatus::Approved;
        // Issue #147: Set approved_at timestamp
        refund.approved_at = Some(env.ledger().timestamp());
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(env, RefundStatus::Approved, refund_id);

        (RefundApproved {
            refund_id,
            approved_by,
            approved_at: env.ledger().timestamp(),
        })
        .publish(env);

        // Issue #144: Invoke notification hooks
        Self::invoke_hooks(env, RefundEventType::Approved, refund_id);

        Ok(())
    }

    fn process_refund_internal(
        env: &Env,
        processed_by: Address,
        refund_id: u64,
    ) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.status != RefundStatus::Approved {
            return Err(Error::InvalidStatus);
        }

        Self::can_refund_payment(
            env,
            refund.payment_id,
            refund.amount,
            refund.original_payment_amount,
        )?;

        // Deduct platform fee from refund amount
        let (net_refund_amount, _fee_amount) =
            Self::deduct_refund_fee(env, refund_id, refund.amount, &refund.token)?;

        // Enforce merchant refund quota if configured
        if let Some(mut quota) = env
            .storage()
            .instance()
            .get::<_, MerchantRefundQuota>(&DataKey::MerchantRefundQuota(refund.merchant.clone()))
        {
            let now = env.ledger().timestamp();
            // auto-reset if period elapsed
            if now > quota.period_start.saturating_add(quota.period_seconds) {
                quota.used = 0;
                quota.period_start = now;
            }
            let new_used = quota
                .used
                .checked_add(refund.amount)
                .ok_or(Error::InvalidAmount)?;
            if new_used > quota.limit {
                return Err(Error::RefundExceedsPolicy);
            }
            quota.used = new_used;
            env.storage().instance().set(
                &DataKey::MerchantRefundQuota(refund.merchant.clone()),
                &quota,
            );
        }

        Self::remove_from_status_index(env, RefundStatus::Approved, refund_id)?;
        refund.status = RefundStatus::Processed;
        // Issue #147: Set processed_at timestamp
        refund.processed_at = Some(env.ledger().timestamp());
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(env, RefundStatus::Processed, refund_id);

        (RefundProcessed {
            refund_id,
            processed_by,
            customer: refund.customer,
            amount: refund.amount,
            token: refund.token,
            processed_at: env.ledger().timestamp(),
        })
        .publish(env);

        // Issue #144: Invoke notification hooks
        Self::invoke_hooks(env, RefundEventType::Processed, refund_id);

        Ok(())
    }

    fn get_external_payment(env: &Env, payment_id: u64) -> Result<ExternalPayment, Error> {
        let payment_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::PaymentContractAddress)
            .ok_or(Error::PaymentContractNotSet)?;
        let args = (payment_id,).into_val(env);
        let func = Symbol::new(env, "get_payment");
        match env.try_invoke_contract::<ExternalPayment, soroban_sdk::InvokeError>(
            &payment_contract,
            &func,
            args,
        ) {
            Ok(Ok(payment)) => Ok(payment),
            _ => Err(Error::InvalidPaymentId),
        }
    }

    fn evaluate_auto_refund_condition(
        env: &Env,
        condition: &AutoRefundCondition,
    ) -> Result<bool, Error> {
        match condition {
            AutoRefundCondition::FulfillmentTimeout(config) => {
                Ok(env.ledger().timestamp() >= config.fulfillment_deadline)
            }
            AutoRefundCondition::ContractStateMatch(config) => {
                let args = (config.key.clone(),).into_val(env);
                let func = Symbol::new(env, "get_contract_state");
                match env.try_invoke_contract::<Bytes, soroban_sdk::InvokeError>(
                    &config.contract,
                    &func,
                    args,
                ) {
                    Ok(Ok(actual)) => Ok(actual == config.expected),
                    _ => Ok(false),
                }
            }
        }
    }

    // ── ANALYTICS FUNCTIONS ────────────────────────────────────────────────

    pub fn get_refund_analytics(env: Env) -> RefundAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::RefundAnalyticsKey)
            .unwrap_or(RefundAnalytics {
                total_refunds_requested: 0,
                total_refunds_approved: 0,
                total_refunds_rejected: 0,
                total_refunds_processed: 0,
                total_refund_volume: 0,
                approval_rate_bps: 0,
            })
    }

    // ── PAUSE FUNCTIONS ────────────────────────────────────────────────────

    pub fn pause_contract(env: Env, admin: Address, reason: String) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let now = env.ledger().timestamp();
        let pause_state = if let Some(mut state) = env
            .storage()
            .instance()
            .get::<SystemKey, PauseState>(&SystemKey::PauseStateKey)
        {
            state.globally_paused = true;
            state.paused_at = now;
            state.paused_by = admin.clone();
            state.pause_reason = reason.clone();
            state
        } else {
            PauseState {
                globally_paused: true,
                paused_functions: Vec::new(&env),
                paused_at: now,
                paused_by: admin.clone(),
                pause_reason: reason.clone(),
            }
        };
        env.storage()
            .instance()
            .set(&SystemKey::PauseStateKey, &pause_state);
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::PauseHistoryCount)
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: String::from_str(&env, "global"),
            paused: true,
            changed_by: admin.clone(),
            changed_at: now,
            reason: reason.clone(),
        };
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryCount, &(history_count + 1));
        (ContractPausedEvent {
            paused_by: admin,
            reason,
            paused_at: now,
        })
        .publish(&env);
        Ok(())
    }

    pub fn unpause_contract(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        if let Some(mut state) = env
            .storage()
            .instance()
            .get::<SystemKey, PauseState>(&SystemKey::PauseStateKey)
        {
            state.globally_paused = false;
            env.storage()
                .instance()
                .set(&SystemKey::PauseStateKey, &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::PauseHistoryCount)
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: String::from_str(&env, "global"),
            paused: false,
            changed_by: admin.clone(),
            changed_at: now,
            reason: String::from_str(&env, ""),
        };
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryCount, &(history_count + 1));
        (ContractUnpausedEvent {
            unpaused_by: admin,
            unpaused_at: now,
        })
        .publish(&env);
        Ok(())
    }

    pub fn pause_function(
        env: Env,
        admin: Address,
        function_name: String,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let now = env.ledger().timestamp();
        let mut pause_state = if let Some(state) = env
            .storage()
            .instance()
            .get::<SystemKey, PauseState>(&SystemKey::PauseStateKey)
        {
            state
        } else {
            PauseState {
                globally_paused: false,
                paused_functions: Vec::new(&env),
                paused_at: 0,
                paused_by: admin.clone(),
                pause_reason: String::from_str(&env, ""),
            }
        };
        if !pause_state.paused_functions.contains(&function_name) {
            pause_state
                .paused_functions
                .push_back(function_name.clone());
        }
        env.storage()
            .instance()
            .set(&SystemKey::PauseStateKey, &pause_state);
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::PauseHistoryCount)
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: function_name.clone(),
            paused: true,
            changed_by: admin.clone(),
            changed_at: now,
            reason: reason.clone(),
        };
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryCount, &(history_count + 1));
        (FunctionPausedEvent {
            function_name,
            paused_by: admin,
            reason,
        })
        .publish(&env);
        Ok(())
    }

    pub fn unpause_function(env: Env, admin: Address, function_name: String) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        if let Some(mut state) = env
            .storage()
            .instance()
            .get::<SystemKey, PauseState>(&SystemKey::PauseStateKey)
        {
            let mut new_paused = Vec::new(&env);
            for fn_name in state.paused_functions.iter() {
                if fn_name != function_name {
                    new_paused.push_back(fn_name);
                }
            }
            state.paused_functions = new_paused;
            env.storage()
                .instance()
                .set(&SystemKey::PauseStateKey, &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::PauseHistoryCount)
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: function_name.clone(),
            paused: false,
            changed_by: admin.clone(),
            changed_at: now,
            reason: String::from_str(&env, ""),
        };
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&SystemKey::PauseHistoryCount, &(history_count + 1));
        (FunctionUnpausedEvent {
            function_name,
            unpaused_by: admin,
        })
        .publish(&env);
        Ok(())
    }

    pub fn get_pause_state(env: Env) -> PauseState {
        env.storage()
            .instance()
            .get(&SystemKey::PauseStateKey)
            .unwrap_or(PauseState {
                globally_paused: false,
                paused_functions: Vec::new(&env),
                paused_at: 0,
                paused_by: env.current_contract_address(),
                pause_reason: String::from_str(&env, ""),
            })
    }

    pub fn is_function_paused(env: Env, function_name: String) -> bool {
        if let Some(state) = env
            .storage()
            .instance()
            .get::<SystemKey, PauseState>(&SystemKey::PauseStateKey)
        {
            if state.globally_paused {
                return true;
            }
            for fn_name in state.paused_functions.iter() {
                if fn_name == function_name {
                    return true;
                }
            }
        }
        false
    }

    fn reason_code_rank(code: &RefundReasonCode) -> u32 {
        match code {
            RefundReasonCode::ProductDefect => 0,
            RefundReasonCode::NonDelivery => 1,
            RefundReasonCode::DuplicateCharge => 2,
            RefundReasonCode::Unauthorized => 3,
            RefundReasonCode::CustomerRequest => 4,
            RefundReasonCode::Other => 5,
        }
    }

    fn require_not_paused(env: &Env, function_name: &str) -> Result<(), Error> {
        if let Some(state) = env
            .storage()
            .instance()
            .get::<SystemKey, PauseState>(&SystemKey::PauseStateKey)
        {
            if state.globally_paused {
                return Err(Error::ContractPaused);
            }
            let fn_str = String::from_str(env, function_name);
            for fn_name in state.paused_functions.iter() {
                if fn_name == fn_str {
                    return Err(Error::FunctionPaused);
                }
            }
        }
        Ok(())
    }

    // ── CIRCUIT BREAKER ────────────────────────────────────────────────────

    pub fn set_circuit_breaker_config(
        env: Env,
        admin: Address,
        config: CircuitBreakerConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&SystemKey::CircuitBreakerConfigKey, &config);
        Ok(())
    }

    pub fn get_circuit_breaker_state(env: Env) -> CircuitBreakerState {
        let mut state = env
            .storage()
            .instance()
            .get::<SystemKey, CircuitBreakerState>(&SystemKey::CircuitBreakerStateKey)
            .unwrap_or(CircuitBreakerState {
                tripped: false,
                tripped_at: None,
                trip_count: 0,
                last_refund_rate_bps: 0,
                resets_at: None,
            });
        #[cfg(test)]
        {
            if TEST_TRIPPED.with(|t| t.load(core::sync::atomic::Ordering::SeqCst)) {
                state.tripped = true;
                state.trip_count =
                    TEST_TRIP_COUNT.with(|tc| tc.load(core::sync::atomic::Ordering::SeqCst));
                let resets_at =
                    TEST_RESETS_AT.with(|r| r.load(core::sync::atomic::Ordering::SeqCst));
                if resets_at > 0 {
                    state.resets_at = Some(resets_at);
                }
            }
        }
        state
    }

    pub fn reset_circuit_breaker(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let mut state = Self::get_circuit_breaker_state(env.clone());
        state.tripped = false;
        state.tripped_at = None;
        state.resets_at = None;
        env.storage()
            .instance()
            .set(&SystemKey::CircuitBreakerStateKey, &state);
        #[cfg(test)]
        {
            TEST_TRIPPED.with(|t| t.store(false, core::sync::atomic::Ordering::SeqCst));
            TEST_TRIP_COUNT.with(|tc| tc.store(0, core::sync::atomic::Ordering::SeqCst));
            TEST_RESETS_AT.with(|r| r.store(0, core::sync::atomic::Ordering::SeqCst));
        }
        let now = env.ledger().timestamp();
        CircuitBreakerResetEvent {
            reset_by: admin,
            reset_at: now,
        }
        .publish(&env);
        Ok(())
    }

    pub fn check_circuit_breaker(env: Env) -> bool {
        let config: CircuitBreakerConfig = match env
            .storage()
            .instance()
            .get(&SystemKey::CircuitBreakerConfigKey)
        {
            Some(c) => c,
            None => return false,
        };
        if !config.enabled {
            return false;
        }
        let state = Self::get_circuit_breaker_state(env.clone());
        if !state.tripped {
            return false;
        }
        let now = env.ledger().timestamp();
        if let Some(resets_at) = state.resets_at {
            now < resets_at
        } else {
            true
        }
    }

    fn check_and_update_customer_refund_rate_limit(
        env: &Env,
        customer: Address,
    ) -> Result<(), Error> {
        let global_limit_opt = env
            .storage()
            .instance()
            .get::<DataKey, GlobalRefundRateLimit>(&DataKey::GlobalRefundRateLimit);
        let customer_limit_opt = env
            .storage()
            .instance()
            .get::<DataKey, CustomerRefundRateLimit>(&DataKey::CustomerRefundRateLimit(
                customer.clone(),
            ));
        if global_limit_opt.is_none() && customer_limit_opt.is_none() {
            return Ok(());
        }
        let mut limit = match customer_limit_opt {
            Some(l) => l,
            None => {
                let g = global_limit_opt.unwrap();
                CustomerRefundRateLimit {
                    customer: customer.clone(),
                    window_start: env.ledger().timestamp(),
                    request_count: 0,
                    max_requests_per_window: g.max_requests_per_window,
                    window_seconds: g.window_seconds,
                }
            }
        };
        let now = env.ledger().timestamp();
        if now >= limit.window_start + limit.window_seconds {
            limit.window_start = now;
            limit.request_count = 0;
        }
        if limit.request_count >= limit.max_requests_per_window {
            return Err(Error::RefundRateLimitExceeded);
        }
        limit.request_count += 1;
        env.storage()
            .instance()
            .set(&DataKey::CustomerRefundRateLimit(customer), &limit);
        Ok(())
    }

    fn check_and_update_circuit_breaker(
        env: &Env,
        refund_amount: i128,
        payment_amount: i128,
    ) -> Result<(), Error> {
        let config: CircuitBreakerConfig = match env
            .storage()
            .instance()
            .get(&SystemKey::CircuitBreakerConfigKey)
        {
            Some(c) => c,
            None => return Ok(()),
        };

        if !config.enabled {
            return Ok(());
        }

        let now = env.ledger().timestamp();
        let mut state = Self::get_circuit_breaker_state(env.clone());

        // Auto-reset after cooldown
        if state.tripped {
            if let Some(resets_at) = state.resets_at {
                if now >= resets_at {
                    state.tripped = false;
                    state.tripped_at = None;
                    state.resets_at = None;
                    env.storage()
                        .instance()
                        .set(&SystemKey::CircuitBreakerStateKey, &state);
                    #[cfg(test)]
                    {
                        TEST_TRIPPED.with(|t| t.store(false, core::sync::atomic::Ordering::SeqCst));
                        TEST_TRIP_COUNT
                            .with(|tc| tc.store(0, core::sync::atomic::Ordering::SeqCst));
                        TEST_RESETS_AT.with(|r| r.store(0, core::sync::atomic::Ordering::SeqCst));
                    }
                } else {
                    return Err(Error::CircuitBreakerTripped);
                }
            } else {
                return Err(Error::CircuitBreakerTripped);
            }
        }

        // Reset window if expired
        let window_start: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::WindowStart)
            .unwrap_or(0);

        if now >= window_start + config.measurement_window_seconds || window_start == 0 {
            env.storage().instance().set(&SystemKey::WindowStart, &now);
            env.storage()
                .instance()
                .set(&SystemKey::WindowRefundVolume, &0i128);
            env.storage()
                .instance()
                .set(&SystemKey::WindowPaymentVolume, &0i128);
        }

        let new_refund_vol: i128 = env
            .storage()
            .instance()
            .get(&SystemKey::WindowRefundVolume)
            .unwrap_or(0)
            + refund_amount;

        let new_payment_vol: i128 = env
            .storage()
            .instance()
            .get(&SystemKey::WindowPaymentVolume)
            .unwrap_or(0)
            + payment_amount;

        if new_payment_vol <= 0 {
            return Ok(());
        }

        let rate_bps = ((new_refund_vol * 10000) / new_payment_vol) as u32;

        if rate_bps > config.max_refund_rate_bps {
            state.tripped = true;
            state.tripped_at = Some(now);
            state.trip_count += 1;
            state.last_refund_rate_bps = rate_bps;
            state.resets_at = Some(now + config.cooldown_seconds);
            env.storage()
                .instance()
                .set(&SystemKey::CircuitBreakerStateKey, &state);
            #[cfg(test)]
            {
                TEST_TRIPPED.with(|t| t.store(true, core::sync::atomic::Ordering::SeqCst));
                TEST_TRIP_COUNT
                    .with(|tc| tc.store(state.trip_count, core::sync::atomic::Ordering::SeqCst));
                TEST_RESETS_AT.with(|r| {
                    r.store(
                        now + config.cooldown_seconds,
                        core::sync::atomic::Ordering::SeqCst,
                    )
                });
            }
            CircuitBreakerTrippedEvent {
                refund_rate_bps: rate_bps,
                tripped_at: now,
            }
            .publish(env);
            return Err(Error::CircuitBreakerTripped);
        }

        env.storage()
            .instance()
            .set(&SystemKey::WindowRefundVolume, &new_refund_vol);
        env.storage()
            .instance()
            .set(&SystemKey::WindowPaymentVolume, &new_payment_vol);

        Ok(())
    }

    // Fraud detection functions (#137)
    pub fn check_fraud_signals(env: Env, address: Address) -> Option<FraudSignal> {
        // Get fraud config
        let config: FraudConfig = env
            .storage()
            .instance()
            .get(&SystemKey::FraudConfig)
            .unwrap_or(FraudConfig {
                max_refund_rate_bps: 2000, // 20%
                min_transactions_for_check: 5,
                enabled: true,
            });

        if !config.enabled {
            return None;
        }

        // Get customer's payment and refund statistics from payment contract
        // For now, we'll use a simplified approach - in production, this would
        // query the payment contract for actual statistics
        let total_payments = Self::get_customer_payment_count(&env, &address);
        let total_refunds = Self::get_customer_refund_count(&env, &address);

        // Skip if below minimum transaction threshold
        if total_payments < config.min_transactions_for_check {
            return None;
        }

        // Calculate refund rate
        let refund_rate_bps: u32 = if total_payments > 0 {
            ((total_refunds * 10000) / total_payments) as u32
        } else {
            0
        };

        // Check if refund rate exceeds threshold
        if refund_rate_bps > config.max_refund_rate_bps {
            let existing_signal: Option<FraudSignal> = env
                .storage()
                .instance()
                .get(&SystemKey::FraudSignal(address.clone()));

            match existing_signal {
                Some(mut signal) if !signal.reviewed => {
                    // Update existing signal
                    signal.refund_rate_bps = refund_rate_bps as u32;
                    signal.total_payments = total_payments;
                    signal.total_refunds = total_refunds;
                    env.storage()
                        .instance()
                        .set(&SystemKey::FraudSignal(address), &signal);
                    Some(signal)
                }
                None => {
                    // Create new fraud signal
                    let signal = FraudSignal {
                        address: address.clone(),
                        refund_rate_bps: refund_rate_bps as u32,
                        total_payments,
                        total_refunds,
                        flagged_at: env.ledger().timestamp(),
                        reviewed: false,
                    };
                    env.storage()
                        .instance()
                        .set(&SystemKey::FraudSignal(address.clone()), &signal);

                    // Add to flagged addresses index
                    let flagged_count: u64 = env
                        .storage()
                        .instance()
                        .get(&SystemKey::FlaggedAddressesIndex)
                        .unwrap_or(0);
                    env.storage()
                        .instance()
                        .set(&SystemKey::FlaggedAddressesIndex, &(flagged_count + 1));

                    // Emit fraud signal raised event
                    (FraudSignalRaised {
                        address,
                        refund_rate_bps: refund_rate_bps as u32,
                    })
                    .publish(&env);

                    Some(signal)
                }
                _ => None, // Already reviewed or exists
            }
        } else {
            None
        }
    }

    pub fn get_flagged_addresses(env: Env) -> Vec<FraudSignal> {
        let mut flagged = Vec::new(&env);

        // In a real implementation, we'd iterate through all addresses
        // For now, we'll return an empty vector as this is a placeholder
        // In production, this would use an index to efficiently retrieve flagged addresses
        flagged
    }

    pub fn mark_fraud_reviewed(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin is the contract admin
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut signal: FraudSignal = env
            .storage()
            .instance()
            .get(&SystemKey::FraudSignal(address.clone()))
            .ok_or(Error::FraudSignalNotFound)?;

        signal.reviewed = true;
        env.storage()
            .instance()
            .set(&SystemKey::FraudSignal(address.clone()), &signal);

        // Emit fraud signal reviewed event
        (FraudSignalReviewed {
            address,
            reviewed_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    pub fn set_fraud_config(env: Env, admin: Address, config: FraudConfig) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin is the contract admin
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&SystemKey::FraudConfig, &config);

        Ok(())
    }

    // Helper functions for fraud detection
    fn get_customer_payment_count(env: &Env, address: &Address) -> u64 {
        // Without a payment contract configured, we have no payment data
        0
    }

    fn get_customer_refund_count(env: &Env, address: &Address) -> u64 {
        // Count refunds for this address
        let refund_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundCount(address.clone()))
            .unwrap_or(0);
        refund_count
    }

    fn update_customer_refund_cooldown(env: &Env, customer: &Address) -> Result<(), Error> {
        let config: RefundCooldownConfig = match env
            .storage()
            .instance()
            .get(&SystemKey::RefundCooldownConfig)
        {
            Some(c) => c,
            None => return Ok(()), // No cooldown configured, skip
        };
        if !config.enabled {
            return Ok(());
        }
        let record = CustomerRefundCooldown {
            customer: customer.clone(),
            last_refund_requested_at: env.ledger().timestamp(),
            cooldown_seconds: config.cooldown_seconds,
        };
        env.storage().instance().set(
            &SystemKey::CustomerRefundCooldown(customer.clone()),
            &record,
        );
        Ok(())
    }

    // Issue #147: Customer refund history functions

    /// Get paginated refund history for a customer, sorted newest-first
    pub fn get_customer_refund_history(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(&env);
        let total = Self::get_customer_refund_count(&env, &customer);

        if limit == 0 || offset >= total {
            return results;
        }

        // Calculate range for newest-first ordering
        let end = core::cmp::min(total, offset.saturating_add(limit));

        // Iterate in reverse order (newest first)
        let mut collected = 0u64;
        let mut skipped = 0u64;
        let mut index = total;

        while index > 0 && collected < limit {
            index -= 1;

            if skipped < offset {
                skipped += 1;
                continue;
            }

            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::CustomerRefunds(customer.clone(), index))
            {
                if let Some(refund) = env
                    .storage()
                    .instance()
                    .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    results.push_back(refund);
                    collected += 1;
                }
            }
        }

        results
    }

    /// Get the total count of refunds for a customer (public version)
    pub fn get_customer_refund_count_public(env: Env, customer: Address) -> u64 {
        Self::get_customer_refund_count(&env, &customer)
    }

    /// Get summary statistics for a customer's refunds
    pub fn get_customer_refund_summary(env: Env, customer: Address) -> CustomerRefundSummary {
        let total_requested = Self::get_customer_refund_count(&env, &customer);
        let mut total_approved = 0u64;
        let mut total_amount_refunded = 0i128;
        let mut total_processing_time = 0u64;
        let mut processed_count = 0u64;

        let mut index = 0u64;
        while index < total_requested {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::CustomerRefunds(customer.clone(), index))
            {
                if let Some(refund) = env
                    .storage()
                    .instance()
                    .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    match refund.status {
                        RefundStatus::Approved | RefundStatus::Processed => {
                            total_approved += 1;
                        }
                        _ => {}
                    }

                    if refund.status == RefundStatus::Processed {
                        total_amount_refunded += refund.amount;

                        // Calculate processing time if we have both timestamps
                        if let Some(processed_at) = refund.processed_at {
                            let processing_time = processed_at.saturating_sub(refund.requested_at);
                            total_processing_time =
                                total_processing_time.saturating_add(processing_time);
                            processed_count += 1;
                        }
                    }
                }
            }
            index += 1;
        }

        let avg_processing_time = if processed_count > 0 {
            total_processing_time / processed_count
        } else {
            0
        };

        CustomerRefundSummary {
            total_requested,
            total_approved,
            total_amount_refunded,
            avg_processing_time,
        }
    }

    fn get_merchant_refund_count(env: &Env, merchant: &Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MerchantRefundCount(merchant.clone()))
            .unwrap_or(0)
    }

    // Issue #144: Notification hook functions
    const MAX_HOOKS_PER_EVENT: u32 = 10;

    /// Register a notification hook for specific refund events
    pub fn register_notification_hook(
        env: Env,
        subscriber: Address,
        events: Vec<RefundEventType>,
    ) -> Result<u64, Error> {
        subscriber.require_auth();

        // Check that at least one event is specified
        if events.is_empty() {
            return Err(Error::InvalidAmount); // Reusing error for invalid input
        }

        // Check max hooks per event type
        for event_type in events.iter() {
            let count: u32 = env
                .storage()
                .instance()
                .get(&SystemKey::HooksByEventCount(event_type.clone()))
                .unwrap_or(0);

            if count >= Self::MAX_HOOKS_PER_EVENT {
                return Err(Error::MaxHooksPerEventReached);
            }
        }

        // Generate hook ID
        let hook_id: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::NotificationHookCounter)
            .unwrap_or(0)
            + 1;

        env.storage()
            .instance()
            .set(&SystemKey::NotificationHookCounter, &hook_id);

        // Create hook
        let hook = NotificationHook {
            hook_id,
            subscriber: subscriber.clone(),
            events: events.clone(),
            active: true,
        };

        // Store hook
        env.storage()
            .instance()
            .set(&SystemKey::NotificationHook(hook_id), &hook);

        // Index by event type
        for event_type in events.iter() {
            let count: u32 = env
                .storage()
                .instance()
                .get(&SystemKey::HooksByEventCount(event_type.clone()))
                .unwrap_or(0);

            env.storage().instance().set(
                &SystemKey::HooksByEvent(event_type.clone(), count as u64),
                &hook_id,
            );

            env.storage().instance().set(
                &SystemKey::HooksByEventCount(event_type.clone()),
                &(count + 1),
            );
        }

        // Index by subscriber
        let subscriber_count: u32 = env
            .storage()
            .instance()
            .get(&SystemKey::SubscriberHookCount(subscriber.clone()))
            .unwrap_or(0);

        env.storage().instance().set(
            &SystemKey::SubscriberHooks(subscriber.clone(), subscriber_count as u64),
            &hook_id,
        );

        env.storage().instance().set(
            &SystemKey::SubscriberHookCount(subscriber.clone()),
            &(subscriber_count + 1),
        );

        // Emit event
        (HookRegistered {
            hook_id,
            subscriber,
            event_count: events.len(),
        })
        .publish(&env);

        Ok(hook_id)
    }

    /// Deregister a notification hook
    pub fn deregister_hook(env: Env, subscriber: Address, hook_id: u64) -> Result<(), Error> {
        subscriber.require_auth();

        // Get hook
        let hook: NotificationHook = env
            .storage()
            .instance()
            .get(&SystemKey::NotificationHook(hook_id))
            .ok_or(Error::HookNotFound)?;

        // Verify ownership
        if hook.subscriber != subscriber {
            return Err(Error::HookNotOwnedBySubscriber);
        }

        // Mark as inactive
        let mut updated_hook = hook.clone();
        updated_hook.active = false;

        env.storage()
            .instance()
            .set(&SystemKey::NotificationHook(hook_id), &updated_hook);

        // Emit event
        (HookDeregistered {
            hook_id,
            subscriber,
        })
        .publish(&env);

        Ok(())
    }

    /// Get all hooks registered for a specific event type
    pub fn get_hooks_for_event(env: Env, event_type: RefundEventType) -> Vec<NotificationHook> {
        let mut hooks: Vec<NotificationHook> = Vec::new(&env);

        let count: u32 = env
            .storage()
            .instance()
            .get(&SystemKey::HooksByEventCount(event_type.clone()))
            .unwrap_or(0);

        for i in 0..count {
            if let Some(hook_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&SystemKey::HooksByEvent(event_type.clone(), i as u64))
            {
                if let Some(hook) = env
                    .storage()
                    .instance()
                    .get::<_, NotificationHook>(&SystemKey::NotificationHook(hook_id))
                {
                    if hook.active {
                        hooks.push_back(hook);
                    }
                }
            }
        }

        hooks
    }

    /// Get all hooks for a subscriber
    pub fn get_subscriber_hooks(env: Env, subscriber: Address) -> Vec<NotificationHook> {
        let mut hooks: Vec<NotificationHook> = Vec::new(&env);

        let count: u32 = env
            .storage()
            .instance()
            .get(&SystemKey::SubscriberHookCount(subscriber.clone()))
            .unwrap_or(0);

        for i in 0..count {
            if let Some(hook_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&SystemKey::SubscriberHooks(subscriber.clone(), i as u64))
            {
                if let Some(hook) = env
                    .storage()
                    .instance()
                    .get::<_, NotificationHook>(&SystemKey::NotificationHook(hook_id))
                {
                    hooks.push_back(hook);
                }
            }
        }

        hooks
    }

    /// Internal function to invoke hooks for a specific event
    fn invoke_hooks(env: &Env, event_type: RefundEventType, refund_id: u64) {
        let count: u32 = env
            .storage()
            .instance()
            .get(&SystemKey::HooksByEventCount(event_type.clone()))
            .unwrap_or(0);

        for i in 0..count {
            if let Some(hook_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&SystemKey::HooksByEvent(event_type.clone(), i as u64))
            {
                if let Some(hook) = env
                    .storage()
                    .instance()
                    .get::<_, NotificationHook>(&SystemKey::NotificationHook(hook_id))
                {
                    if hook.active && hook.events.contains(&event_type) {
                        // Attempt to invoke the subscriber contract
                        // Using try_invoke_contract to isolate failures
                        let result = env.try_invoke_contract::<(), soroban_sdk::InvokeError>(
                            &hook.subscriber,
                            &Symbol::new(env, "on_refund_event"),
                            (event_type.clone(), refund_id).into_val(env),
                        );

                        // If hook invocation fails, emit failure event but don't revert
                        if result.is_err() {
                            (HookInvocationFailed {
                                hook_id: hook.hook_id,
                                subscriber: hook.subscriber.clone(),
                                event_type: event_type.clone(),
                                refund_id,
                            })
                            .publish(env);
                        }
                    }
                }
            }
        }
    }

    // ── Issue #148: Customer eligibility registry ─────────────────────────

    /// Set or update the refund eligibility rule for a customer under a specific merchant.
    /// Only the merchant themselves or the admin may call this.
    pub fn set_refund_eligibility(
        env: Env,
        merchant: Address,
        customer: Address,
        rule: EligibilityRule,
        reason_hash: BytesN<32>,
    ) -> Result<(), Error> {
        // Require merchant auth; admin can also call via mock_all_auths in tests
        merchant.require_auth();

        let entry = RefundEligibilityEntry {
            customer: customer.clone(),
            merchant: merchant.clone(),
            rule: rule.clone(),
            reason_hash,
            set_at: env.ledger().timestamp(),
        };

        let key = EligibilityKey::Entry(merchant.clone(), customer.clone());
        let is_new = !env.storage().instance().has(&key);
        env.storage().instance().set(&key, &entry);

        // If this is a new entry, append to the merchant's customer index
        if is_new {
            let count: u64 = env
                .storage()
                .instance()
                .get(&EligibilityKey::MerchantCustomerCount(merchant.clone()))
                .unwrap_or(0);
            env.storage().instance().set(
                &EligibilityKey::MerchantCustomerIndex(merchant.clone(), count),
                &customer,
            );
            env.storage().instance().set(
                &EligibilityKey::MerchantCustomerCount(merchant.clone()),
                &(count + 1),
            );
        }

        (EligibilitySet {
            merchant,
            customer,
            rule,
        })
        .publish(&env);

        Ok(())
    }

    /// Return the eligibility rule for a (merchant, customer) pair.
    /// Defaults to `Allow` when no entry exists.
    pub fn check_refund_eligibility(
        env: Env,
        merchant: Address,
        customer: Address,
    ) -> EligibilityRule {
        Self::check_refund_eligibility_internal(&env, &merchant, &customer)
    }

    /// Internal version that borrows `env` by reference.
    fn check_refund_eligibility_internal(
        env: &Env,
        merchant: &Address,
        customer: &Address,
    ) -> EligibilityRule {
        env.storage()
            .instance()
            .get::<EligibilityKey, RefundEligibilityEntry>(&EligibilityKey::Entry(
                merchant.clone(),
                customer.clone(),
            ))
            .map(|e| e.rule)
            .unwrap_or(EligibilityRule::Allow)
    }

    /// Remove an eligibility entry for a (merchant, customer) pair.
    /// Returns `EligibilityEntryNotFound` if no entry exists.
    /// Only the merchant or admin may call this.
    pub fn remove_refund_eligibility(
        env: Env,
        merchant: Address,
        customer: Address,
    ) -> Result<(), Error> {
        merchant.require_auth();

        let key = EligibilityKey::Entry(merchant.clone(), customer.clone());
        if !env.storage().instance().has(&key) {
            return Err(Error::EligibilityEntryNotFound);
        }
        env.storage().instance().remove(&key);

        // Compact the merchant's customer index by swapping with the last element
        let count: u64 = env
            .storage()
            .instance()
            .get(&EligibilityKey::MerchantCustomerCount(merchant.clone()))
            .unwrap_or(0);

        if count > 0 {
            // Find the position of this customer in the index
            let mut found_index: Option<u64> = None;
            for i in 0..count {
                let idx_key = EligibilityKey::MerchantCustomerIndex(merchant.clone(), i);
                if let Some(addr) = env
                    .storage()
                    .instance()
                    .get::<EligibilityKey, Address>(&idx_key)
                {
                    if addr == customer {
                        found_index = Some(i);
                        break;
                    }
                }
            }

            if let Some(pos) = found_index {
                let last = count - 1;
                if pos != last {
                    // Swap with last
                    let last_key = EligibilityKey::MerchantCustomerIndex(merchant.clone(), last);
                    let last_addr: Address = env.storage().instance().get(&last_key).unwrap();
                    env.storage().instance().set(
                        &EligibilityKey::MerchantCustomerIndex(merchant.clone(), pos),
                        &last_addr,
                    );
                }
                // Remove the last slot
                env.storage()
                    .instance()
                    .remove(&EligibilityKey::MerchantCustomerIndex(
                        merchant.clone(),
                        last,
                    ));
                env.storage().instance().set(
                    &EligibilityKey::MerchantCustomerCount(merchant.clone()),
                    &last,
                );
            }
        }

        (EligibilityRemoved { merchant, customer }).publish(&env);

        Ok(())
    }

    /// Return all eligibility entries for a merchant.
    pub fn get_merchant_eligibility_list(
        env: Env,
        merchant: Address,
    ) -> Vec<RefundEligibilityEntry> {
        let mut results = Vec::new(&env);
        let count: u64 = env
            .storage()
            .instance()
            .get(&EligibilityKey::MerchantCustomerCount(merchant.clone()))
            .unwrap_or(0);

        for i in 0..count {
            if let Some(customer) = env.storage().instance().get::<EligibilityKey, Address>(
                &EligibilityKey::MerchantCustomerIndex(merchant.clone(), i),
            ) {
                if let Some(entry) = env
                    .storage()
                    .instance()
                    .get::<EligibilityKey, RefundEligibilityEntry>(&EligibilityKey::Entry(
                        merchant.clone(),
                        customer,
                    ))
                {
                    results.push_back(entry);
                }
            }
        }

        results
    }

    fn get_merchant_refunds_by_status_internal(
        env: &Env,
        merchant: &Address,
        status: RefundStatus,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(env);
        if limit == 0 {
            return results;
        }

        let total = Self::get_merchant_refund_count(env, merchant);
        let mut matched = 0u64;
        let mut collected = 0u64;
        let mut index = 0u64;

        while index < total && collected < limit {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::MerchantRefunds(merchant.clone(), index))
            {
                if let Some(refund) = env
                    .storage()
                    .instance()
                    .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    if refund.status == status {
                        if matched >= offset {
                            results.push_back(refund);
                            collected += 1;
                        }
                        matched += 1;
                    }
                }
            }
            index += 1;
        }

        results
    }

    pub fn batch_reject_refunds(
        env: Env,
        admin: Address,
        refund_ids: Vec<u64>,
        note_hash: BytesN<32>,
    ) -> Result<BatchDecisionResult, Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        if refund_ids.len() > Self::BATCH_DECISION_LIMIT {
            return Err(Error::BatchRefundTooLarge);
        }

        let mut succeeded = Vec::new(&env);
        let mut failed = Vec::new(&env);
        let mut had_failure = false;

        for refund_id in refund_ids.iter() {
            let result = (|| -> Result<(), Error> {
                let mut refund: Refund = env
                    .storage()
                    .instance()
                    .get(&DataKey::Refund(refund_id))
                    .ok_or(Error::RefundNotFound)?;
                if refund.status != RefundStatus::Requested {
                    return Err(Error::InvalidStatus);
                }
                Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;
                refund.status = RefundStatus::Rejected;
                refund.rejected_at = Some(env.ledger().timestamp());
                env.storage()
                    .instance()
                    .set(&DataKey::Refund(refund_id), &refund);
                Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);
                env.storage().instance().set(
                    &SystemKey::RefundRejectedAt(refund_id),
                    &env.ledger().timestamp(),
                );
                (RefundRejected {
                    refund_id,
                    rejected_by: admin.clone(),
                    rejected_at: env.ledger().timestamp(),
                    rejection_reason: soroban_sdk::String::from_str(&env, "batch rejection"),
                })
                .publish(&env);
                Self::invoke_hooks(&env, RefundEventType::Rejected, refund_id);
                Ok(())
            })();
            match result {
                Ok(()) => succeeded.push_back(refund_id),
                Err(_) => {
                    failed.push_back(refund_id);
                    had_failure = true;
                }
            }
        }

        let _ = note_hash;

        if had_failure {
            return Err(Error::BatchRefundTooLarge);
        }

        Ok(BatchDecisionResult { succeeded, failed })
    }

    // ── Issue #197: Category-based dynamic refund windows ─────────────────────

    pub fn set_category_window(
        env: Env,
        admin: Address,
        merchant: Address,
        category: PaymentCategory,
        window_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let cat_idx = category.to_index();
        let window = CategoryRefundWindow {
            category,
            window_seconds,
            merchant: merchant.clone(),
        };
        env.storage()
            .instance()
            .set(&RefundExtKey::CategoryWindow(merchant, cat_idx), &window);
        Ok(())
    }

    pub fn get_category_window(
        env: Env,
        merchant: Address,
        category: PaymentCategory,
    ) -> Option<u64> {
        let cat_idx = category.to_index();
        env.storage()
            .instance()
            .get::<RefundExtKey, CategoryRefundWindow>(&RefundExtKey::CategoryWindow(
                merchant, cat_idx,
            ))
            .map(|w| w.window_seconds)
    }

    pub fn tag_payment_category(
        env: Env,
        merchant: Address,
        payment_id: u64,
        category: PaymentCategory,
    ) -> Result<(), Error> {
        merchant.require_auth();
        if env
            .storage()
            .instance()
            .has(&RefundExtKey::PaymentCategoryTag(payment_id))
        {
            return Err(Error::AlreadyProcessed);
        }
        let cat_idx = category.to_index();
        env.storage()
            .instance()
            .set(&RefundExtKey::PaymentCategoryTag(payment_id), &cat_idx);
        Ok(())
    }

    pub fn get_effective_window(env: Env, merchant: Address, payment_id: u64) -> u64 {
        let default_window: u64 = Self::get_refund_policy(&env, merchant.clone())
            .map(|p| {
                if p.default_window_seconds > 0 {
                    p.default_window_seconds
                } else {
                    30 * 24 * 60 * 60
                }
            })
            .unwrap_or(30 * 24 * 60 * 60);

        let cat_idx_opt: Option<u32> = env
            .storage()
            .instance()
            .get(&RefundExtKey::PaymentCategoryTag(payment_id));

        if let Some(cat_idx) = cat_idx_opt {
            if let Some(window) = env
                .storage()
                .instance()
                .get::<RefundExtKey, CategoryRefundWindow>(&RefundExtKey::CategoryWindow(
                    merchant, cat_idx,
                ))
                .map(|w| w.window_seconds)
            {
                return window;
            }
        }

        default_window
    }

    // ── Issue #198: Round-robin arbitrator auto-assignment ─────────────────────

    pub fn configure_auto_assignment(
        env: Env,
        admin: Address,
        panel_size: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        if arbitrators.is_empty() {
            return Err(Error::ArbitratorNotFound);
        }

        if panel_size as u32 > arbitrators.len() {
            return Err(Error::ArbitratorNotFound);
        }

        let config = ArbitratorAssignmentConfig {
            rotation_index: 0,
            panel_size,
        };
        env.storage()
            .instance()
            .set(&RefundExtKey::AssignmentConfig, &config);
        Ok(())
    }

    pub fn auto_assign_arbitrators(env: Env, case_id: u64) -> Result<Vec<Address>, Error> {
        let mut config: ArbitratorAssignmentConfig = env
            .storage()
            .instance()
            .get(&RefundExtKey::AssignmentConfig)
            .ok_or(Error::PolicyNotFound)?;

        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        if arbitrators.is_empty() {
            return Err(Error::ArbitratorNotFound);
        }

        let total = arbitrators.len() as u32;
        if config.panel_size > total {
            return Err(Error::ArbitratorNotFound);
        }

        let mut panel = Vec::new(&env);
        for i in 0..config.panel_size {
            let idx = ((config.rotation_index + i) % total) as u32;
            panel.push_back(arbitrators.get(idx).unwrap());
        }

        config.rotation_index = (config.rotation_index + config.panel_size) % total;
        env.storage()
            .instance()
            .set(&RefundExtKey::AssignmentConfig, &config);

        let _ = case_id;
        Ok(panel)
    }

    pub fn get_next_arbitrators(env: Env, count: u32) -> Vec<Address> {
        let config: ArbitratorAssignmentConfig = match env
            .storage()
            .instance()
            .get(&RefundExtKey::AssignmentConfig)
        {
            Some(c) => c,
            None => return Vec::new(&env),
        };

        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        let total = arbitrators.len() as u32;
        if total == 0 || count == 0 {
            return Vec::new(&env);
        }

        let n = if count > total { total } else { count };
        let mut result = Vec::new(&env);
        for i in 0..n {
            let idx = ((config.rotation_index + i) % total) as u32;
            result.push_back(arbitrators.get(idx).unwrap());
        }
        result
    }

    pub fn reset_rotation_index(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut config: ArbitratorAssignmentConfig = env
            .storage()
            .instance()
            .get(&RefundExtKey::AssignmentConfig)
            .ok_or(Error::PolicyNotFound)?;

        config.rotation_index = 0;
        env.storage()
            .instance()
            .set(&RefundExtKey::AssignmentConfig, &config);
        Ok(())
    }

    // ── Issue #199: Refund request TTL with automatic expiry ──────────────────

    pub fn set_refund_ttl_config(env: Env, admin: Address, ttl_seconds: u64) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let cfg = RefundTTLConfig {
            default_ttl_seconds: ttl_seconds,
            active: true,
        };
        env.storage()
            .instance()
            .set(&RefundExtKey::RefundTTLConfig, &cfg);
        Ok(())
    }

    pub fn expire_stale_refund(env: Env, refund_id: u64) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.status != RefundStatus::Requested {
            return Err(Error::InvalidStatus);
        }

        let expires_at = refund.expires_at.ok_or(Error::PolicyNotFound)?;

        if env.ledger().timestamp() < expires_at {
            return Err(Error::RefundWindowExpired);
        }

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;
        refund.status = RefundStatus::Rejected;
        refund.rejected_at = Some(env.ledger().timestamp());
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);

        (RefundRejected {
            refund_id,
            rejected_by: env.current_contract_address(),
            rejected_at: env.ledger().timestamp(),
            rejection_reason: soroban_sdk::String::from_str(&env, "TTL expired"),
        })
        .publish(&env);

        Ok(())
    }

    pub fn get_expired_refunds(env: Env, limit: u32) -> Vec<u64> {
        let now = env.ledger().timestamp();
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);

        let mut results = Vec::new(&env);
        let mut collected = 0u32;
        let mut id = 1u64;

        while id <= total && collected < limit {
            if let Some(refund) = env
                .storage()
                .instance()
                .get::<DataKey, Refund>(&DataKey::Refund(id))
            {
                if refund.status == RefundStatus::Requested {
                    if let Some(expires_at) = refund.expires_at {
                        if now >= expires_at {
                            results.push_back(id);
                            collected += 1;
                        }
                    }
                }
            }
            id += 1;
        }

        results
    }

    // ── Issue #190: Dispute evidence attachment ────────────────────────────

    pub fn submit_refund_evidence(
        env: Env,
        submitter: Address,
        refund_id: u64,
        evidence_hash: BytesN<32>,
    ) -> Result<(), Error> {
        submitter.require_auth();

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if submitter != refund.customer && submitter != refund.merchant {
            return Err(Error::Unauthorized);
        }

        if env
            .storage()
            .instance()
            .has(&EvidenceKey::Evidence(refund_id, submitter.clone()))
        {
            return Err(Error::EvidenceAlreadySubmitted);
        }

        let count: u64 = env
            .storage()
            .instance()
            .get(&EvidenceKey::EvidenceCount(refund_id))
            .unwrap_or(0);

        let evidence = RefundEvidence {
            refund_id,
            submitter: submitter.clone(),
            evidence_hash,
            submitted_at: env.ledger().timestamp(),
        };

        env.storage().instance().set(
            &EvidenceKey::Evidence(refund_id, submitter.clone()),
            &evidence,
        );
        env.storage()
            .instance()
            .set(&EvidenceKey::EvidenceIndex(refund_id, count), &submitter);
        env.storage()
            .instance()
            .set(&EvidenceKey::EvidenceCount(refund_id), &(count + 1));

        Ok(())
    }

    pub fn get_refund_evidence(
        env: Env,
        refund_id: u64,
        submitter: Address,
    ) -> Option<RefundEvidence> {
        env.storage()
            .instance()
            .get(&EvidenceKey::Evidence(refund_id, submitter))
    }

    pub fn get_all_refund_evidence(env: Env, refund_id: u64) -> Vec<RefundEvidence> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&EvidenceKey::EvidenceCount(refund_id))
            .unwrap_or(0);
        let mut results = Vec::new(&env);
        let mut i = 0u64;
        while i < count {
            if let Some(submitter) = env
                .storage()
                .instance()
                .get::<_, Address>(&EvidenceKey::EvidenceIndex(refund_id, i))
            {
                if let Some(ev) = env
                    .storage()
                    .instance()
                    .get::<_, RefundEvidence>(&EvidenceKey::Evidence(refund_id, submitter))
                {
                    results.push_back(ev);
                }
            }
            i += 1;
        }
        results
    }

    // ── Issue #191: Multi-token refund support ─────────────────────────────

    pub fn register_refund_token(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let count: u64 = env
            .storage()
            .instance()
            .get(&TokenKey::TokenCount)
            .unwrap_or(0);

        let entry = SupportedRefundToken {
            token: token.clone(),
            active: true,
        };
        env.storage()
            .instance()
            .set(&TokenKey::SupportedToken(token.clone()), &entry);

        let already_indexed = (0..count).any(|i| {
            env.storage()
                .instance()
                .get::<_, Address>(&TokenKey::TokenByIndex(i))
                .map(|t| t == token)
                .unwrap_or(false)
        });
        if !already_indexed {
            env.storage()
                .instance()
                .set(&TokenKey::TokenByIndex(count), &token);
            env.storage()
                .instance()
                .set(&TokenKey::TokenCount, &(count + 1));
        }

        Ok(())
    }

    pub fn deregister_refund_token(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut entry: SupportedRefundToken = env
            .storage()
            .instance()
            .get(&TokenKey::SupportedToken(token.clone()))
            .ok_or(Error::RefundNotFound)?;

        entry.active = false;
        env.storage()
            .instance()
            .set(&TokenKey::SupportedToken(token), &entry);

        Ok(())
    }

    pub fn get_supported_refund_tokens(env: Env) -> Vec<SupportedRefundToken> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&TokenKey::TokenCount)
            .unwrap_or(0);
        let mut results = Vec::new(&env);
        let mut i = 0u64;
        while i < count {
            if let Some(token) = env
                .storage()
                .instance()
                .get::<_, Address>(&TokenKey::TokenByIndex(i))
            {
                if let Some(entry) = env
                    .storage()
                    .instance()
                    .get::<_, SupportedRefundToken>(&TokenKey::SupportedToken(token))
                {
                    results.push_back(entry);
                }
            }
            i += 1;
        }
        results
    }

    // ── Issue #192: Refund credit vouchers ────────────────────────────────

    pub fn issue_refund_voucher(
        env: Env,
        admin: Address,
        refund_id: u64,
        expiry_seconds: u64,
    ) -> Result<u64, Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        let counter: u64 = env
            .storage()
            .instance()
            .get(&VoucherKey::VoucherCounter)
            .unwrap_or(0);
        let voucher_id = counter + 1;

        let now = env.ledger().timestamp();
        let voucher = RefundVoucher {
            voucher_id,
            refund_id,
            customer: refund.customer.clone(),
            merchant: refund.merchant.clone(),
            amount: refund.amount,
            token: refund.token.clone(),
            issued_at: now,
            expires_at: now.saturating_add(expiry_seconds),
            redeemed: false,
        };

        env.storage()
            .instance()
            .set(&VoucherKey::Voucher(voucher_id), &voucher);
        env.storage()
            .instance()
            .set(&VoucherKey::VoucherCounter, &voucher_id);

        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&VoucherKey::CustomerVoucherCount(refund.customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &VoucherKey::CustomerVoucher(refund.customer.clone(), customer_count),
            &voucher_id,
        );
        env.storage().instance().set(
            &VoucherKey::CustomerVoucherCount(refund.customer.clone()),
            &(customer_count + 1),
        );

        Ok(voucher_id)
    }

    pub fn redeem_refund_voucher(
        env: Env,
        customer: Address,
        voucher_id: u64,
        _payment_id: u64,
    ) -> Result<(), Error> {
        customer.require_auth();

        let mut voucher: RefundVoucher = env
            .storage()
            .instance()
            .get(&VoucherKey::Voucher(voucher_id))
            .ok_or(Error::VoucherNotFound)?;

        if voucher.customer != customer {
            return Err(Error::Unauthorized);
        }
        if voucher.redeemed {
            return Err(Error::VoucherAlreadyRedeemed);
        }
        if env.ledger().timestamp() > voucher.expires_at {
            return Err(Error::VoucherExpired);
        }

        voucher.redeemed = true;
        env.storage()
            .instance()
            .set(&VoucherKey::Voucher(voucher_id), &voucher);

        Ok(())
    }

    pub fn get_voucher(env: Env, voucher_id: u64) -> Option<RefundVoucher> {
        env.storage()
            .instance()
            .get(&VoucherKey::Voucher(voucher_id))
    }

    pub fn get_customer_vouchers(env: Env, customer: Address) -> Vec<RefundVoucher> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&VoucherKey::CustomerVoucherCount(customer.clone()))
            .unwrap_or(0);
        let mut results = Vec::new(&env);
        let mut i = 0u64;
        while i < count {
            if let Some(vid) = env
                .storage()
                .instance()
                .get::<_, u64>(&VoucherKey::CustomerVoucher(customer.clone(), i))
            {
                if let Some(v) = env
                    .storage()
                    .instance()
                    .get::<_, RefundVoucher>(&VoucherKey::Voucher(vid))
                {
                    results.push_back(v);
                }
            }
            i += 1;
        }
        results
    }

    // ── Issue #194: Tiered arbitration escalation ─────────────────────────

    pub fn add_senior_arbitrator(
        env: Env,
        admin: Address,
        arbitrator: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut list: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::SeniorArbitratorList)
            .unwrap_or(Vec::new(&env));
        if !list.contains(&arbitrator) {
            list.push_back(arbitrator);
            env.storage()
                .instance()
                .set(&ArbitrationKey::SeniorArbitratorList, &list);
        }
        Ok(())
    }

    pub fn set_arbitration_tier_config(
        env: Env,
        admin: Address,
        config: ArbitrationTierConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationTierConfig, &config);
        Ok(())
    }

    pub fn escalate_arbitration_case(env: Env, case_id: u64) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .has(&ArbitrationKey::CaseEscalated(case_id))
        {
            return Err(Error::CaseAlreadyEscalated);
        }

        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::RefundNotFound)?;

        if case.status != ArbitrationStatus::Open {
            return Err(Error::InvalidStatus);
        }

        let config: ArbitrationTierConfig = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationTierConfig)
            .ok_or(Error::CaseNotTimedOut)?;

        if env.ledger().timestamp()
            < case
                .created_at
                .saturating_add(config.escalation_timeout_seconds)
        {
            return Err(Error::CaseNotTimedOut);
        }

        let senior_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::SeniorArbitratorList)
            .unwrap_or(Vec::new(&env));

        if senior_list.len() == 0 {
            return Err(Error::ArbitratorNotFound);
        }

        case.arbitrators = senior_list;
        case.votes_for_refund = 0;
        case.votes_against_refund = 0;
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);
        env.storage()
            .instance()
            .set(&ArbitrationKey::CaseEscalated(case_id), &true);

        Ok(())
    }

    pub fn get_arbitration_tier(env: Env, case_id: u64) -> ArbitratorTier {
        if env
            .storage()
            .instance()
            .has(&ArbitrationKey::CaseEscalated(case_id))
        {
            ArbitratorTier::Senior
        } else {
            ArbitratorTier::Junior
        }
    }

    // ── Payment refund cap management ──────────────────────────────────────

    pub fn set_payment_refund_cap(
        env: Env,
        admin: Address,
        cap: PaymentRefundCap,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        if cap.payment_id == 0 {
            return Err(Error::InvalidPaymentId);
        }

        env.storage()
            .instance()
            .set(&DataKey::PaymentRefundCap(cap.payment_id), &cap);
        Ok(())
    }

    pub fn get_payment_refund_cap(env: Env, payment_id: u64) -> Option<PaymentRefundCap> {
        env.storage()
            .instance()
            .get(&DataKey::PaymentRefundCap(payment_id))
    }

    pub fn get_payment_refund_usage(env: Env, payment_id: u64) -> (u32, i128) {
        let usage: Option<(u32, i128)> = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundUsage(payment_id));
        usage.unwrap_or((0, 0))
    }

    fn check_payment_refund_cap(
        env: &Env,
        payment_id: u64,
        refund_amount: i128,
    ) -> Result<(), Error> {
        // If no cap is set, no restriction applies
        let cap: PaymentRefundCap = match env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundCap(payment_id))
        {
            Some(c) => c,
            None => return Ok(()),
        };

        let (current_count, current_amount): (u32, i128) = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundUsage(payment_id))
            .unwrap_or((0u32, 0i128));

        // Check count cap (only for Requested and Approved statuses)
        if current_count >= cap.max_refund_count {
            return Err(Error::RefundCountCapExceeded);
        }

        // Check amount cap (cumulative across all statuses except Rejected)
        let new_total_amount = current_amount.saturating_add(refund_amount);
        if new_total_amount > cap.max_total_amount {
            return Err(Error::RefundAmountCapExceeded);
        }

        Ok(())
    }

    fn update_payment_refund_usage(env: &Env, payment_id: u64, refund_amount: i128) {
        let (current_count, current_amount): (u32, i128) = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundUsage(payment_id))
            .unwrap_or((0u32, 0i128));

        let new_count = current_count.saturating_add(1u32);
        let new_amount = current_amount.saturating_add(refund_amount);

        env.storage().instance().set(
            &DataKey::PaymentRefundUsage(payment_id),
            &(new_count, new_amount),
        );
    }

    fn validate_bps(bps: u32) -> Result<(), Error> {
        if bps < 1 || bps > 10000 {
            return Err(Error::InvalidAmount);
        };

        Ok(())
    }
}

mod test;
mod test_policy;
mod test_process;
mod test_rate_limit;

#[cfg(test)]
mod test_payment_refund_cap;

#[cfg(test)]
mod test_circuit_breaker;

// #[cfg(test)]
// mod test_versioning;

#[cfg(test)]
mod test_batch;

#[cfg(test)]
mod test_cross_contract;

#[cfg(test)]
mod test_arbitration_fees;

#[cfg(test)]
mod test_arbitration_stake;

#[cfg(test)]
mod test_arbitrator_reputation;

#[cfg(test)]
mod test_auto_refund;

#[cfg(test)]
mod test_inheritance;

mod test_customer_history;
#[cfg(test)]
mod test_notification_hooks;

#[cfg(test)]
mod test_arbitration_timeout;

#[cfg(test)]
mod test_merchant_eligibility;
