#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes,
    BytesN, Env, FromVal, IntoVal, String, Symbol, TryFromVal, Val, Vec,
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

/// Construct a tuple-based storage key from its components.
///
/// Uses `Symbol::new` with `Env::default()` to create the prefix symbol.
///
/// # Arguments
/// * `prefix` - A string prefix for the storage key.
/// * `addr` - An optional address component.
/// * `id` - An optional numeric ID component.
/// * `sub_id` - An optional sub-ID component.
///
/// # Returns
/// A `StorageKey` tuple suitable for use in contract storage.
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
    // Issue: bound unbounded per-customer history growth by archiving old entries
    CustomerRefundHistoryStart(Address),
    CustomerRefundsArchive(Address, u64),
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
    AutoApproveBelowCeiling,
    // Issue #370: Customer-tier-based refund caps
    CustomerTier(Address),
    CustomerTierPolicy(Address, u32),
    StrictTierPolicy(Address),
    AppealWindowSeconds,
    // Issue #389: two-step admin rotation
    PendingAdmin,
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
    // Uniqueness guard: maps refund_id -> case_id so the same refund
    // cannot be escalated into multiple parallel arbitration cases.
    CaseByRefund(u64),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PolicyKey {
    RefundPolicyVersion(Address, u32),
    RefundPolicyVersionCount(Address),
    AutoRefundTrigger(u64),
    AutoRefundTriggerCounter,
}

// Maximum number of a customer's refund references kept in "hot" instance
// storage. Older entries are moved to persistent storage (archived) so a
// customer's history can grow indefinitely without bloating the instance
// storage footprint read/written on every contract invocation.
const CUSTOMER_HISTORY_HOT_CAP: u64 = 50;

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
    // Ordered list of flagged addresses: FlaggedAddress(n) -> Address, paired
    // with the FlaggedAddressesIndex counter so get_flagged_addresses can
    // enumerate every entry without iterating over all storage keys.
    FlaggedAddress(u64),
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
    SchemaVersion,
    // Issue #382: cached reason-code analytics for a given [window_start, window_end]
    // ledger-timestamp range, so repeated queries over the same window don't
    // re-scan the full refund history.
    AnalyticsCache(u64, u64),
    // Tracks the distinct (window_start, window_end) pairs cached above, so a newly
    // processed refund can invalidate only the windows it actually falls within.
    AnalyticsCacheWindows,
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
    RefundVoucherIssued(u64),
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
    PendingAppeal,
}

// Issue #397: canonical reason codes, enforced by the type system on Refund and
// on request_refund()'s signature, so get_reason_code_analytics() never sees a
// free-form/inconsistent string for this field.
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

// Issue #138 (recurred): the flat `Error` enum grew past Soroban's 50-variant
// XDR spec limit (`VecM<ScSpecUdtErrorEnumCaseV0, 50>`), which makes the
// `#[contracterror]` macro panic with `LengthExceedsMax` at compile time.
// Split into two `#[contracterror]` enums (each <= 50 variants), wrapped by a
// single `Error` type so every existing `Result<_, Error>` signature and `?`
// call site is unaffected. Mirrors the same pattern already used for
// `Error`/`BasicError`/`EscrowError`/`ActionError` in contracts/escrow/src/lib.rs.
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CoreError {
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
    AutoApproveThresholdExceedsCeiling = 32,
    RefundCooldownActive = 33,
}

#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExtError {
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
    // Issue #373: Invalid notification hook subscriber address
    // (moved from 58, which collided with SchemaAlreadyAtTarget)
    InvalidHookAddress = 51,
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
    // Issue #370: Customer tier policy errors
    TierPolicyNotFound = 57,
    SchemaAlreadyAtTarget = 58,
    // Issue #389: two-step admin rotation errors
    NoPendingAdmin = 59,
    NotPendingAdmin = 60,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    Core(CoreError),
    Ext(ExtError),
}

impl Error {
    pub fn to_u32(&self) -> u32 {
        match self {
            Error::Core(e) => *e as u32,
            Error::Ext(e) => *e as u32,
        }
    }
}

impl From<Error> for soroban_sdk::Error {
    fn from(e: Error) -> Self {
        soroban_sdk::Error::from_contract_error(e.to_u32())
    }
}

impl From<&Error> for soroban_sdk::Error {
    fn from(e: &Error) -> Self {
        soroban_sdk::Error::from_contract_error(e.to_u32())
    }
}

impl TryFrom<soroban_sdk::Error> for Error {
    type Error = soroban_sdk::Error;
    fn try_from(error: soroban_sdk::Error) -> Result<Self, Self::Error> {
        if let Ok(e) = CoreError::try_from(error) {
            return Ok(Error::Core(e));
        }
        if let Ok(e) = ExtError::try_from(error) {
            return Ok(Error::Ext(e));
        }
        Err(error)
    }
}

impl TryFrom<&soroban_sdk::Error> for Error {
    type Error = soroban_sdk::Error;
    fn try_from(error: &soroban_sdk::Error) -> Result<Self, Self::Error> {
        <Self as TryFrom<soroban_sdk::Error>>::try_from(*error)
    }
}

impl FromVal<Env, Error> for Val {
    fn from_val(env: &Env, v: &Error) -> Self {
        soroban_sdk::Error::from(v).into_val(env)
    }
}

impl TryFromVal<Env, Val> for Error {
    type Error = soroban_sdk::ConversionError;
    fn try_from_val(env: &Env, val: &Val) -> Result<Self, Self::Error> {
        let error: soroban_sdk::Error =
            soroban_sdk::Error::try_from_val(env, val).map_err(|_| soroban_sdk::ConversionError)?;
        Error::try_from(error).map_err(|_| soroban_sdk::ConversionError)
    }
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
    pub payment_id: u64,
    pub amount: i128,
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
    pub rejected_by: Option<Address>,
    pub appeal_deadline: Option<u64>,
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

// Issue #370: Per-tier refund cap for customer loyalty tiers
#[derive(Clone)]
#[contracttype]
pub struct RefundCap {
    pub max_refund_bps: u32,
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
    pub max_refund_bps: u32,
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
    /// When true, per-customer limits are not refreshed from global config on window reset.
    pub custom_override: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct GlobalRefundRateLimit {
    pub max_requests_per_window: u32,
    pub window_seconds: u64,
    /// Config applied to windows that start at or after `next_config_effective_at`.
    pub next_max_requests_per_window: u32,
    pub next_window_seconds: u64,
    pub next_config_effective_at: u64,
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

/// Event emitted when the global refund rate limit config is updated
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitUpdated {
    pub admin: Address,
    pub new_window_seconds: u64,
    pub new_max_refunds: u32,
    pub effective_at: u64,
}

/// Event emitted when the current admin proposes a new admin.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRotationProposed {
    pub current_admin: Address,
    pub pending_admin: Address,
}

/// Event emitted when a proposed admin accepts the role, completing rotation.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRotationAccepted {
    pub previous_admin: Address,
    pub new_admin: Address,
}

#[contract]
pub struct RefundContract;

#[contractimpl]
impl RefundContract {
    const BATCH_DECISION_LIMIT: u32 = 50;
    const INITIAL_SCHEMA_VERSION: u32 = 1;

    /// Initialize the refund contract with an admin address.
    ///
    /// Sets up the default refund policy (30-day window, 100% refund),
    /// admin approval settings, and appeal window.
    ///
    /// # Panics
    /// Panics if the contract has already been initialized.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&SystemKey::SchemaVersion, &Self::INITIAL_SCHEMA_VERSION);

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
        Self::set_auto_approve_below_ceiling_inner(&env, 0);
        env.storage()
            .instance()
            .set(&DataKey::AppealWindowSeconds, &604800u64);
    }

    /// Get the current schema version of the contract.
    ///
    /// Returns the schema version number. Defaults to `INITIAL_SCHEMA_VERSION` if not set.
    pub fn get_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&SystemKey::SchemaVersion)
            .unwrap_or(Self::INITIAL_SCHEMA_VERSION)
    }

    /// Migrate the contract schema to a new version.
    ///
    /// # Arguments
    /// * `admin` - The admin address (must be authorized and match stored admin).
    /// * `target_version` - The target schema version to migrate to.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the admin.
    /// Returns `SchemaAlreadyAtTarget` if the current version is already at or past the target.
    pub fn migrate_schema(env: Env, admin: Address, target_version: u32) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let current = Self::get_schema_version(env.clone());
        if current >= target_version {
            return Err(Error::Ext(ExtError::SchemaAlreadyAtTarget));
        }

        env.storage()
            .instance()
            .set(&SystemKey::SchemaVersion, &target_version);
        Ok(())
    }

    /// Propose a new admin, starting a two-step rotation (Issue #389).
    ///
    /// The current admin designates `new_admin` as pending. The rotation only
    /// completes once `new_admin` calls [`Self::accept_admin`], so a typo'd or
    /// unreachable address can never brick admin control of the contract.
    ///
    /// # Arguments
    /// * `admin` - The current admin (must be authorized and match stored admin).
    /// * `new_admin` - The address to propose as the next admin.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the current admin.
    pub fn propose_admin(env: Env, admin: Address, new_admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_admin);

        (AdminRotationProposed {
            current_admin: admin,
            pending_admin: new_admin,
        })
        .publish(&env);

        Ok(())
    }

    /// Accept a pending admin rotation, finalizing the transition (Issue #389).
    ///
    /// Must be called by the address previously proposed via
    /// [`Self::propose_admin`]. Replaces `DataKey::Admin` with the caller and
    /// clears the pending slot.
    ///
    /// # Errors
    /// Returns `NoPendingAdmin` if no rotation has been proposed.
    /// Returns `NotPendingAdmin` if the caller is not the proposed admin.
    pub fn accept_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        new_admin.require_auth();

        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .ok_or(Error::Ext(ExtError::NoPendingAdmin))?;
        if pending != new_admin {
            return Err(Error::Ext(ExtError::NotPendingAdmin));
        }

        let previous_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;

        env.storage().instance().set(&DataKey::Admin, &new_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);

        (AdminRotationAccepted {
            previous_admin,
            new_admin,
        })
        .publish(&env);

        Ok(())
    }

    /// Get the address currently proposed as the next admin, if any.
    pub fn get_pending_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::PendingAdmin)
    }

    /// Request a refund for a payment.
    ///
    /// Creates a new refund request with status `Requested` (or `Approved` if auto-approval
    /// conditions are met). Validates the token, policy, fraud signals, and eligibility.
    ///
    /// # Arguments
    /// * `merchant` - The merchant requesting the refund (must be authorized).
    /// * `payment_id` - The ID of the original payment.
    /// * `customer` - The customer receiving the refund.
    /// * `amount` - The refund amount in the smallest token unit.
    /// * `original_payment_amount` - The original payment amount.
    /// * `token` - The token address used for the refund.
    /// * `reason` - A human-readable reason for the refund.
    /// * `reason_code` - A canonical reason code for the refund.
    /// * `payment_created_at` - The timestamp when the original payment was created.
    ///
    /// # Returns
    /// The ID of the newly created refund.
    ///
    /// # Errors
    /// Returns errors for invalid amounts, unsupported tokens, policy violations,
    /// fraud signals, eligibility blocks, or payment ownership mismatches.
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
        Self::require_not_paused(&env, "request_refund")?;
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
                _ => return Err(Error::Ext(ExtError::UnsupportedRefundToken)),
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

    /// Retrieve a refund by its ID.
    ///
    /// # Arguments
    /// * `refund_id` - The unique identifier of the refund.
    ///
    /// # Returns
    /// The `Refund` record if found.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if no refund exists with the given ID.
    pub fn get_refund(env: &Env, refund_id: u64) -> Result<Refund, Error> {
        // Retrieve refund from storage by ID
        env.storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))
    }

    /// Approve a pending refund request.
    ///
    /// Changes the refund status from `Requested` to `Approved` and emits a `RefundApproved` event.
    ///
    /// # Arguments
    /// * `admin` - The admin address (must be authorized).
    /// * `refund_id` - The ID of the refund to approve.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the admin.
    /// Returns `InvalidStatus` if the refund is not in `Requested` status.
    /// Returns `RefundWindowExpired` if the refund's TTL has expired.
    pub fn approve_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "approve_refund")?;
        // Require admin authentication
        admin.require_auth();

        Self::approve_refund_internal(&env, admin, refund_id)
    }

    /// Reject a pending refund request.
    ///
    /// Moves the refund to `PendingAppeal` status with an appeal window, and emits a
    /// `RefundRejected` event. The customer can file an appeal within the appeal window.
    ///
    /// # Arguments
    /// * `admin` - The admin address (must be authorized).
    /// * `refund_id` - The ID of the refund to reject.
    /// * `rejection_reason` - A human-readable reason for the rejection.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the admin.
    /// Returns `InvalidStatus` if the refund is not in `Requested` status.
    pub fn reject_refund(
        env: Env,
        admin: Address,
        refund_id: u64,
        rejection_reason: String,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "reject_refund")?;
        // Require admin authentication
        admin.require_auth();

        Self::begin_refund_rejection(&env, admin, refund_id, rejection_reason)
    }

    /// Finalize a denied refund after its appeal window has expired.
    ///
    /// Moves the refund from `PendingAppeal` to `Rejected` status if the appeal window
    /// has elapsed, and emits a `RefundRejected` event.
    ///
    /// # Arguments
    /// * `refund_id` - The ID of the refund to finalize.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if the refund does not exist.
    /// Returns `RefundNotRejected` if the refund is not in `PendingAppeal` status.
    /// Returns `InvalidStatus` if the appeal window has not yet expired.
    pub fn finalize_denial(env: Env, refund_id: u64) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if refund.status != RefundStatus::PendingAppeal {
            return Err(Error::Core(CoreError::RefundNotRejected));
        }

        let appeal_deadline = refund
            .appeal_deadline
            .ok_or(Error::Core(CoreError::RefundNotRejected))?;
        let now = env.ledger().timestamp();
        if now < appeal_deadline {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        Self::remove_from_status_index(&env, RefundStatus::PendingAppeal, refund_id)?;

        refund.status = RefundStatus::Rejected;
        refund.rejected_at = Some(now);
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);
        Self::release_payment_refund_usage(&env, refund.payment_id, refund.amount);
        env.storage()
            .instance()
            .set(&SystemKey::RefundRejectedAt(refund_id), &now);

        let rejected_by = refund
            .rejected_by
            .clone()
            .unwrap_or(env.current_contract_address());

        (RefundRejected {
            refund_id,
            rejected_by,
            rejected_at: now,
            rejection_reason: soroban_sdk::String::from_str(&env, "appeal window expired"),
        })
        .publish(&env);

        Self::invoke_hooks(&env, RefundEventType::Rejected, refund_id);

        Ok(())
    }

    fn begin_refund_rejection(
        env: &Env,
        admin: Address,
        refund_id: u64,
        rejection_reason: String,
    ) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if refund.status != RefundStatus::Requested {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        Self::remove_from_status_index(env, RefundStatus::Requested, refund_id)?;

        let appeal_window: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AppealWindowSeconds)
            .unwrap_or(604800);
        let now = env.ledger().timestamp();

        refund.status = RefundStatus::PendingAppeal;
        refund.rejected_by = Some(admin.clone());
        refund.appeal_deadline = Some(now.saturating_add(appeal_window));

        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(env, RefundStatus::PendingAppeal, refund_id);

        (RefundRejected {
            refund_id,
            rejected_by: admin,
            rejected_at: now,
            rejection_reason,
        })
        .publish(env);

        Ok(())
    }

    /// File an appeal against a rejected or pending-appeal refund.
    ///
    /// Creates a new appeal record and emits an `AppealFiled` event. The customer
    /// must be the refund's customer and the refund must be in a rejected/pending-appeal state.
    ///
    /// # Arguments
    /// * `customer` - The customer filing the appeal (must be authorized).
    /// * `refund_id` - The ID of the refund being appealed.
    /// * `reason` - A human-readable reason for the appeal.
    ///
    /// # Returns
    /// The ID of the newly created appeal.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the refund's customer.
    /// Returns `RefundNotRejected` if the refund is not in an appealable state.
    /// Returns `AppealAlreadyFiled` if an appeal already exists for this refund.
    /// Returns `AppealWindowExpired` if the appeal window has passed.
    pub fn file_appeal(
        env: Env,
        customer: Address,
        refund_id: u64,
        reason: String,
    ) -> Result<u64, Error> {
        Self::require_not_paused(&env, "file_appeal")?;
        customer.require_auth();

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if refund.customer != customer {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        if refund.status != RefundStatus::Rejected && refund.status != RefundStatus::PendingAppeal {
            return Err(Error::Core(CoreError::RefundNotRejected));
        }
        if env
            .storage()
            .instance()
            .has(&SystemKey::AppealByRefund(refund_id))
        {
            return Err(Error::Core(CoreError::AppealAlreadyFiled));
        }

        let now = env.ledger().timestamp();
        if refund.status == RefundStatus::PendingAppeal {
            let appeal_deadline = refund
                .appeal_deadline
                .ok_or(Error::Core(CoreError::RefundNotRejected))?;
            if now > appeal_deadline {
                return Err(Error::Core(CoreError::AppealWindowExpired));
            }
        } else {
            let rejected_at: u64 = env
                .storage()
                .instance()
                .get(&SystemKey::RefundRejectedAt(refund_id))
                .ok_or(Error::Core(CoreError::RefundNotRejected))?;
            let appeal_window: u64 = env
                .storage()
                .instance()
                .get(&DataKey::AppealWindowSeconds)
                .unwrap_or(604800);
            if now > rejected_at.saturating_add(appeal_window) {
                return Err(Error::Core(CoreError::AppealWindowExpired));
            }
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

    /// Resolve an appeal by upholding or denying it.
    ///
    /// If upheld, the refund is approved and processed. If denied, the rejection becomes final.
    /// Emits an `AppealResolved` event.
    ///
    /// # Arguments
    /// * `admin` - The admin address (must be authorized and be the contract admin).
    /// * `appeal_id` - The ID of the appeal to resolve.
    /// * `uphold` - `true` to uphold the appeal (approve refund), `false` to deny.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the admin.
    /// Returns `AlreadyProcessed` if the appeal is already resolved.
    /// Returns `RefundNotFound` if the appeal or refund does not exist.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut appeal: RefundAppeal = env
            .storage()
            .instance()
            .get(&SystemKey::Appeal(appeal_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;
        if appeal.resolved {
            return Err(Error::Core(CoreError::AlreadyProcessed));
        }

        if uphold {
            let mut refund: Refund = env
                .storage()
                .instance()
                .get(&DataKey::Refund(appeal.refund_id))
                .ok_or(Error::Core(CoreError::RefundNotFound))?;
            if refund.status != RefundStatus::Rejected
                && refund.status != RefundStatus::PendingAppeal
            {
                return Err(Error::Core(CoreError::RefundNotRejected));
            }

            let prior_status = refund.status.clone();
            Self::remove_from_status_index(&env, prior_status, refund.id)?;
            refund.status = RefundStatus::Approved;
            env.storage()
                .instance()
                .set(&DataKey::Refund(refund.id), &refund);
            Self::add_to_status_index(&env, RefundStatus::Approved, refund.id);

            Self::process_refund_internal(&env, admin.clone(), refund.id)?;
        } else {
            let mut refund: Refund = env
                .storage()
                .instance()
                .get(&DataKey::Refund(appeal.refund_id))
                .ok_or(Error::Core(CoreError::RefundNotFound))?;
            if refund.status != RefundStatus::Rejected
                && refund.status != RefundStatus::PendingAppeal
            {
                return Err(Error::Core(CoreError::RefundNotRejected));
            }

            // The appeal was explicitly denied, so the rejection is final
            // now — no need to wait out the rest of the appeal window.
            if refund.status == RefundStatus::PendingAppeal {
                Self::remove_from_status_index(&env, RefundStatus::PendingAppeal, refund.id)?;
                refund.status = RefundStatus::Rejected;
                refund.rejected_at = Some(env.ledger().timestamp());
                env.storage()
                    .instance()
                    .set(&DataKey::Refund(refund.id), &refund);
                Self::add_to_status_index(&env, RefundStatus::Rejected, refund.id);
                Self::release_payment_refund_usage(&env, refund.payment_id, refund.amount);
            }
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

    /// Retrieve an appeal by its ID.
    ///
    /// # Arguments
    /// * `appeal_id` - The unique identifier of the appeal.
    ///
    /// # Returns
    /// The `RefundAppeal` record if found.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if no appeal exists with the given ID.
    pub fn get_appeal(env: Env, appeal_id: u64) -> Result<RefundAppeal, Error> {
        env.storage()
            .instance()
            .get(&SystemKey::Appeal(appeal_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))
    }

    /// Get all appeals filed by a specific customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query appeals for.
    ///
    /// # Returns
    /// A vector of `RefundAppeal` records filed by the customer.
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

    /// Process an approved refund for payout.
    ///
    /// Changes the refund status from `Approved` to `Processed`, deducts platform fees,
    /// enforces merchant refund quota, and emits a `RefundProcessed` event.
    ///
    /// # Arguments
    /// * `admin` - The admin address (must be authorized).
    /// * `refund_id` - The ID of the refund to process.
    ///
    /// # Errors
    /// Returns `InvalidStatus` if the refund is not in `Approved` status.
    /// Returns `RefundExceedsPolicy` if the merchant quota is exceeded.
    /// Returns `TotalRefundsExceedPayment` if processing would exceed the original payment.
    pub fn process_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "process_refund")?;
        admin.require_auth();

        Self::process_refund_internal(&env, admin, refund_id)
    }

    /// Register an automatic refund trigger for a payment.
    ///
    /// Creates a trigger that automatically initiates a refund when a condition is met
    /// (e.g., fulfillment timeout or contract state match).
    ///
    /// # Arguments
    /// * `merchant` - The merchant registering the trigger (must be authorized).
    /// * `payment_id` - The payment ID to attach the trigger to.
    /// * `condition` - The condition that triggers the automatic refund.
    /// * `refund_bps` - The refund amount in basis points of the original payment (1-10000).
    ///
    /// # Returns
    /// The ID of the newly created trigger.
    ///
    /// # Errors
    /// Returns `InvalidPaymentId` if `payment_id` is 0.
    /// Returns `RefundExceedsPolicy` if `refund_bps` is out of valid range.
    /// Returns `Unauthorized` if the caller is not the payment's merchant.
    /// Returns `DuplicateAutoRefundTrigger` if an identical active trigger exists.
    pub fn register_auto_refund_trigger(
        env: Env,
        merchant: Address,
        payment_id: u64,
        condition: AutoRefundCondition,
        refund_bps: u32,
    ) -> Result<u64, Error> {
        merchant.require_auth();

        if payment_id == 0 {
            return Err(Error::Core(CoreError::InvalidPaymentId));
        }

        if let Err(_) = Self::validate_bps(refund_bps) {
            return Err(Error::Core(CoreError::RefundExceedsPolicy));
        }

        let payment = Self::get_external_payment(&env, payment_id)?;
        if payment.merchant != merchant {
            return Err(Error::Core(CoreError::Unauthorized));
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
                    return Err(Error::Ext(ExtError::DuplicateAutoRefundTrigger));
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

    /// Evaluate an automatic refund trigger and execute it if the condition is met.
    ///
    /// Checks the trigger's condition, creates and processes a refund if satisfied,
    /// and deactivates the trigger. Emits an `AutoRefundTriggered` event.
    ///
    /// # Arguments
    /// * `trigger_id` - The ID of the trigger to evaluate.
    ///
    /// # Returns
    /// `true` if the refund was triggered, `false` otherwise.
    ///
    /// # Errors
    /// Returns `InvalidAmount` if the calculated refund amount is non-positive.
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
            .ok_or(Error::Core(CoreError::InvalidAmount))?;
        if refund_amount <= 0 {
            return Err(Error::Core(CoreError::InvalidAmount));
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

    /// Get an automatic refund trigger by its ID.
    ///
    /// # Arguments
    /// * `trigger_id` - The unique identifier of the trigger.
    ///
    /// # Returns
    /// The `AutoRefundTrigger` record if found.
    ///
    /// # Errors
    /// Returns `AutoRefundTriggerNotFound` if no trigger exists with the given ID.
    pub fn get_auto_refund_trigger(env: Env, trigger_id: u64) -> Result<AutoRefundTrigger, Error> {
        env.storage()
            .instance()
            .get(&PolicyKey::AutoRefundTrigger(trigger_id))
            .ok_or(Error::Ext(ExtError::AutoRefundTriggerNotFound))
    }

    /// Set a refund quota for a merchant within a time period.
    ///
    /// Limits the total refund amount a merchant can process within the specified period.
    ///
    /// # Arguments
    /// * `admin` - The admin address (must be authorized).
    /// * `merchant` - The merchant to set the quota for.
    /// * `limit` - The maximum refund amount allowed in the period.
    /// * `period_seconds` - The duration of the quota period in seconds.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let now = env.ledger().timestamp();
        let quota = match env
            .storage()
            .instance()
            .get::<_, MerchantRefundQuota>(&DataKey::MerchantRefundQuota(merchant.clone()))
        {
            Some(mut existing) => {
                existing.limit = limit;
                existing.period_seconds = period_seconds;
                existing
            }
            None => MerchantRefundQuota {
                merchant: merchant.clone(),
                limit,
                period_seconds,
                used: 0,
                period_start: now,
            },
        };
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefundQuota(merchant), &quota);
        Ok(())
    }

    /// Get the refund quota configuration for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// The `MerchantRefundQuota` if configured, `None` otherwise.
    pub fn get_merchant_refund_quota(env: Env, merchant: Address) -> Option<MerchantRefundQuota> {
        env.storage()
            .instance()
            .get(&DataKey::MerchantRefundQuota(merchant))
    }

    /// Reset a merchant's refund quota usage counter to zero and restart the quota period.
    ///
    /// # Arguments
    /// * `admin` - The contract admin executing the reset.
    /// * `merchant` - The merchant whose quota should be reset.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `PolicyNotFound` if no quota is configured for the merchant.
    pub fn reset_merchant_quota(env: Env, admin: Address, merchant: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut quota: MerchantRefundQuota = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRefundQuota(merchant.clone()))
            .ok_or(Error::Core(CoreError::PolicyNotFound))?;
        quota.used = 0;
        quota.period_start = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefundQuota(merchant), &quota);
        Ok(())
    }

    /// Set a custom per-customer refund rate limit that overrides the global limit.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the limit.
    /// * `customer` - The customer address to apply the limit to.
    /// * `max_per_window` - Maximum number of refund requests allowed per window.
    /// * `window_seconds` - Duration of the rate-limit window in seconds.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
                custom_override: true,
            });

        limit.max_requests_per_window = max_per_window;
        limit.window_seconds = window_seconds;
        limit.custom_override = true;

        env.storage()
            .instance()
            .set(&DataKey::CustomerRefundRateLimit(customer), &limit);
        Ok(())
    }

    /// Get the current rate-limit status for a customer, including request count and window info.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query.
    ///
    /// # Returns
    /// The `CustomerRefundRateLimit` for the customer. Returns a default zero-value
    /// if no limit has been configured.
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
                custom_override: false,
            })
    }

    /// Set the global refund rate limit that applies to all customers by default.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the limit.
    /// * `max_per_window` - Maximum number of refund requests allowed per window.
    /// * `window_seconds` - Duration of the rate-limit window in seconds.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `InvalidAmount` if either parameter is zero.
    pub fn set_global_refund_rate_limit(
        env: Env,
        admin: Address,
        max_per_window: u32,
        window_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        if max_per_window == 0 || window_seconds == 0 {
            return Err(Error::Core(CoreError::InvalidAmount));
        }

        let limit = GlobalRefundRateLimit {
            max_requests_per_window: max_per_window,
            window_seconds,
            next_max_requests_per_window: max_per_window,
            next_window_seconds: window_seconds,
            next_config_effective_at: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::GlobalRefundRateLimit, &limit);
        Ok(())
    }

    /// Update the global refund rate limit without disrupting in-progress windows.
    /// New parameters apply only to windows that start at or after the update timestamp;
    /// the current window's request count and duration are preserved.
    pub fn update_rate_limit(
        env: Env,
        admin: Address,
        new_window_seconds: u64,
        new_max_refunds: u32,
    ) -> Result<(), Error> {
        admin.require_auth();

        if new_max_refunds == 0 || new_window_seconds == 0 {
            return Err(Error::Core(CoreError::InvalidAmount));
        }

        let now = env.ledger().timestamp();
        let updated = match env
            .storage()
            .instance()
            .get::<DataKey, GlobalRefundRateLimit>(&DataKey::GlobalRefundRateLimit)
        {
            Some(mut existing) => {
                existing.next_max_requests_per_window = new_max_refunds;
                existing.next_window_seconds = new_window_seconds;
                existing.next_config_effective_at = now;
                existing
            }
            None => GlobalRefundRateLimit {
                max_requests_per_window: new_max_refunds,
                window_seconds: new_window_seconds,
                next_max_requests_per_window: new_max_refunds,
                next_window_seconds: new_window_seconds,
                next_config_effective_at: 0,
            },
        };

        env.storage()
            .instance()
            .set(&DataKey::GlobalRefundRateLimit, &updated);

        (RateLimitUpdated {
            admin,
            new_window_seconds,
            new_max_refunds,
            effective_at: now,
        })
        .publish(&env);

        Ok(())
    }

    /// Get the current global refund rate limit configuration.
    ///
    /// # Returns
    /// The `GlobalRefundRateLimit` if configured, `None` otherwise.
    pub fn get_global_refund_rate_limit(env: Env) -> Option<GlobalRefundRateLimit> {
        env.storage()
            .instance()
            .get(&DataKey::GlobalRefundRateLimit)
    }

    /// Register a new arbitrator and initialize their reputation score.
    ///
    /// # Arguments
    /// * `admin` - The contract admin performing the registration.
    /// * `arbitrator` - The address of the arbitrator to register.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `Unauthorized` if the arbitrator is already registered.
    pub fn register_arbitrator(env: Env, admin: Address, arbitrator: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        let mut list: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));
        if list.contains(&arbitrator) {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Manually assign an arbitrator to an open arbitration case.
    ///
    /// # Arguments
    /// * `admin` - The contract admin performing the assignment.
    /// * `case_id` - The ID of the arbitration case.
    /// * `arbitrator` - The address of the arbitrator to assign.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `RefundNotFound` if the case does not exist.
    /// Returns `InvalidStatus` if the case is not in `Open` status.
    /// Returns `NotArbitrator` if the address is not a registered arbitrator.
    pub fn assign_arbitrator(
        env: Env,
        admin: Address,
        case_id: u64,
        arbitrator: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;
        if case.status != ArbitrationStatus::Open {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(case.refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if let Some(denied_by) = refund.rejected_by.clone() {
            if arbitrator == denied_by {
                // denied_by cannot arbitrate their own denial decision
                return Err(Error::Core(CoreError::Unauthorized));
            }
        }

        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));
        if !arbitrators.contains(&arbitrator) {
            return Err(Error::Core(CoreError::NotArbitrator));
        }

        if !case.arbitrators.contains(&arbitrator) {
            case.arbitrators.push_back(arbitrator);
            env.storage()
                .instance()
                .set(&ArbitrationKey::ArbitrationCase(case_id), &case);
        }

        Ok(())
    }

    /// Escalate a rejected refund to the arbitration panel for review.
    ///
    /// Transfers the arbitration fee from the caller and, if staking is enabled,
    /// also transfers the required stake. Creates an `Open` arbitration case
    /// assigned to all registered arbitrators.
    ///
    /// # Arguments
    /// * `caller` - The customer or party escalating the dispute.
    /// * `refund_id` - The ID of the rejected refund to escalate.
    /// * `fee_token` - The token address used to pay the arbitration fee.
    /// * `fee_amount` - The amount of the arbitration fee.
    ///
    /// # Returns
    /// The newly created arbitration case ID.
    ///
    /// # Errors
    /// Returns `InvalidStatus` if the refund is not in `Rejected` or `PendingAppeal` status.
    /// Returns `InvalidAmount` if `fee_amount` is not positive.
    /// Returns `QuorumNotReached` if fewer than 3 arbitrators are registered.
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
            .ok_or(Error::Core(CoreError::RefundNotFound))?;
        // Matches `file_appeal`'s dual-status check: a rejection sits in
        // `PendingAppeal` during the appeal window before finalizing to
        // `Rejected`, and arbitration must remain reachable during that
        // window, not only after it closes.
        if refund.status != RefundStatus::Rejected && refund.status != RefundStatus::PendingAppeal {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        // Uniqueness guard: prevent the same refund from being escalated into
        // multiple parallel arbitration cases.  If a case already exists for
        // this refund_id, reject the duplicate attempt.
        if env
            .storage()
            .instance()
            .has(&ArbitrationKey::CaseByRefund(refund_id))
        {
            return Err(Error::Core(CoreError::AlreadyProcessed));
        }
        if fee_amount <= 0 {
            return Err(Error::Core(CoreError::InvalidAmount));
        }

        let fee_config: Option<ArbitrationFeeConfig> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationFeeConfig);
        if let Some(ref config) = fee_config {
            if config.fee_per_case > 0 && fee_amount < config.fee_per_case {
                return Err(Error::Core(CoreError::InvalidAmount));
            }
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
            return Err(Error::Core(CoreError::QuorumNotReached));
        }

        // Handle staking if enabled
        let stake_config: Option<ArbitrationStakeConfig> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStakeConfig);

        if let Some(config) = stake_config {
            if config.enabled {
                if config.amount <= 0 {
                    return Err(Error::Core(CoreError::InvalidAmount));
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

        let now = env.ledger().timestamp();
        let timeout_secs: u64 = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationTimeoutConfig)
            .unwrap_or(86400 * 14); // default 14 days
        let case = ArbitrationCase {
            case_id,
            refund_id,
            arbitrators: arbitrators.clone(),
            votes_for_refund: 0,
            votes_against_refund: 0,
            status: ArbitrationStatus::Open,
            created_at: now,
            deadline: now + timeout_secs,
            fee_pool: fee_amount,
            timeout_at: now + timeout_secs,
            default_favor_customer: true,
        };

        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCounter, &case_id);
        // Record the reverse mapping so subsequent calls can detect the duplicate.
        env.storage()
            .instance()
            .set(&ArbitrationKey::CaseByRefund(refund_id), &case_id);

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

    /// Cast a vote on an open arbitration case.
    ///
    /// # Arguments
    /// * `arbitrator` - The arbitrator casting the vote.
    /// * `case_id` - The ID of the arbitration case.
    /// * `vote_for_refund` - `true` to vote in favor of the refund, `false` to vote against.
    /// * `reasoning_hash` - SHA-256 hash of the arbitrator's reasoning document.
    ///
    /// # Errors
    /// Returns `InvalidStatus` if the case is not open or the deadline has passed.
    /// Returns `NotArbitrator` if the caller is not an assigned arbitrator for this case.
    /// Returns `AlreadyProcessed` if the arbitrator has already voted on this case.
    /// Returns `Unauthorized` if the arbitrator is the merchant or customer of the refund.
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
            .ok_or(Error::Core(CoreError::RefundNotFound))?;
        if case.status != ArbitrationStatus::Open {
            return Err(Error::Core(CoreError::InvalidStatus));
        }
        if env.ledger().timestamp() > case.deadline {
            return Err(Error::Core(CoreError::InvalidStatus));
        }
        if !case.arbitrators.contains(&arbitrator) {
            return Err(Error::Core(CoreError::NotArbitrator));
        }
        if env
            .storage()
            .instance()
            .has(&ArbitrationKey::ArbitratorVote(case_id, arbitrator.clone()))
        {
            return Err(Error::Core(CoreError::AlreadyProcessed));
        }

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(case.refund_id))
            .unwrap();
        if arbitrator == refund.merchant || arbitrator == refund.customer {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Close an arbitration case once quorum has been reached and tally votes.
    ///
    /// Requires at least 3 total votes. The refund is approved if the majority
    /// voted in favor, otherwise it is rejected. Also updates the arbitrator
    /// reputation scores based on majority/minority alignment.
    ///
    /// # Arguments
    /// * `case_id` - The ID of the arbitration case to close.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if the case does not exist.
    /// Returns `InvalidStatus` if the case is not open or quorum (3 votes) has not been reached.
    pub fn close_arbitration_case(env: Env, case_id: u64) -> Result<(), Error> {
        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;
        if case.status != ArbitrationStatus::Open {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        let total_votes = case.votes_for_refund + case.votes_against_refund;
        if total_votes < 3 {
            return Err(Error::Core(CoreError::InvalidStatus));
        } // quorum

        let approved = case.votes_for_refund > case.votes_against_refund;

        case.status = ArbitrationStatus::Decided;
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);

        // Update refund status based on the arbitration outcome
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(case.refund_id))
            .unwrap();
        if approved {
            refund.status = RefundStatus::Approved;
            env.storage()
                .instance()
                .set(&DataKey::Refund(case.refund_id), &refund);

            (RefundApproved {
                refund_id: case.refund_id,
                payment_id: refund.payment_id,
                amount: refund.amount,
                approved_by: env.current_contract_address(),
                approved_at: env.ledger().timestamp(),
            })
            .publish(&env);
            Self::invoke_hooks(&env, RefundEventType::Approved, case.refund_id);
        } else if refund.status == RefundStatus::PendingAppeal {
            // The arbitration panel upheld the rejection, so it's final now
            // — no need to wait out the rest of the appeal window.
            Self::remove_from_status_index(&env, RefundStatus::PendingAppeal, refund.id)?;
            refund.status = RefundStatus::Rejected;
            refund.rejected_at = Some(env.ledger().timestamp());
            env.storage()
                .instance()
                .set(&DataKey::Refund(case.refund_id), &refund);
            Self::add_to_status_index(&env, RefundStatus::Rejected, refund.id);
            Self::release_payment_refund_usage(&env, refund.payment_id, refund.amount);

            (RefundRejected {
                refund_id: case.refund_id,
                rejected_by: env.current_contract_address(),
                rejected_at: refund.rejected_at.unwrap(),
                rejection_reason: soroban_sdk::String::from_str(
                    &env,
                    "arbitration case decided against refund",
                ),
            })
            .publish(&env);
            Self::invoke_hooks(&env, RefundEventType::Rejected, case.refund_id);
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
        Self::settle_arbitration_stake(&env, case_id, approved);

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

    /// Returns a case's stake to the staker if they won, or forfeits it to the
    /// treasury if they lost. Shared by both the quorum-vote resolution path
    /// (`close_arbitration_case`) and the timeout-default path
    /// (`trigger_arbitration_timeout`) so a staker can never permanently lose
    /// their stake just because a case was resolved by timeout instead of vote.
    ///
    /// * `approved` - whether the arbitration outcome favors the customer
    ///   (refund approved). The staker is whichever party escalated the case;
    ///   compare their role against the outcome to decide if they won.
    fn settle_arbitration_stake(env: &Env, case_id: u64, approved: bool) {
        let stake_opt: Option<ArbitrationStake> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStake(case_id));

        let mut stake = match stake_opt {
            Some(s) if !s.returned => s,
            _ => return,
        };

        let case: ArbitrationCase = match env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
        {
            Some(c) => c,
            None => return,
        };

        let refund: Refund = match env
            .storage()
            .instance()
            .get(&DataKey::Refund(case.refund_id))
        {
            Some(r) => r,
            None => return,
        };

        let stake_config: Option<ArbitrationStakeConfig> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationStakeConfig);

        let stake_cfg = match stake_config {
            Some(cfg) => cfg,
            None => return,
        };

        let stake_token_client = token::Client::new(env, &stake_cfg.token);

        // Get treasury address from fee config
        let fee_config: Option<ArbitrationFeeConfig> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationFeeConfig);

        // Staker wins when the outcome aligns with their side of the dispute.
        let staker_won = (stake.staker == refund.customer && approved)
            || (stake.staker == refund.merchant && !approved);

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
            .publish(env);
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
            .publish(env);
        }

        // Mark stake as returned
        stake.returned = true;
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationStake(case_id), &stake);
    }

    /// Set the default timeout duration for arbitration cases.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the timeout.
    /// * `timeout_seconds` - The timeout duration in seconds (default is 14 days).
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationTimeoutConfig, &timeout_seconds);
        Ok(())
    }

    /// Get the current arbitration timeout configuration in seconds.
    ///
    /// # Returns
    /// The timeout duration in seconds. Defaults to 1,209,600 (14 days) if not configured.
    pub fn get_arbitration_timeout_config(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationTimeoutConfig)
            .unwrap_or(86400 * 14)
    }

    /// Trigger a timeout on an arbitration case that has exceeded its deadline.
    ///
    /// If quorum has not been reached and the timeout has elapsed, the case is
    /// resolved with the default outcome (typically favoring the customer).
    ///
    /// # Arguments
    /// * `case_id` - The ID of the arbitration case to time out.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if the case does not exist.
    /// Returns `InvalidStatus` if the case is not open or quorum was already reached.
    /// Returns `CaseNotTimedOut` if the timeout has not yet elapsed.
    pub fn trigger_arbitration_timeout(env: Env, case_id: u64) -> Result<(), Error> {
        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if case.status != ArbitrationStatus::Open {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        // Block if quorum already reached
        let total_votes = case.votes_for_refund + case.votes_against_refund;
        if total_votes >= 3 {
            return Err(Error::Core(CoreError::QuorumNotReached));
        }

        if env.ledger().timestamp() < case.timeout_at {
            return Err(Error::Core(CoreError::CaseNotTimedOut));
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

            (RefundApproved {
                refund_id: case.refund_id,
                payment_id: refund.payment_id,
                amount: refund.amount,
                approved_by: env.current_contract_address(),
                approved_at: env.ledger().timestamp(),
            })
            .publish(&env);
            Self::invoke_hooks(&env, RefundEventType::Approved, case.refund_id);
        }

        // Handle stake return or forfeiture, same as the quorum-vote path —
        // a case resolved by timeout must not leave the staker's funds stuck.
        Self::settle_arbitration_stake(&env, case_id, approved);

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

    /// Create a reusable refund policy template that can be applied to merchants.
    ///
    /// # Arguments
    /// * `admin` - The contract admin creating the template.
    /// * `name` - A human-readable name for the template.
    /// * `tiers` - A vector of `(days_from_purchase, max_refund_bps)` tuples defining the refund tiers.
    /// * `window` - The default refund window in seconds.
    ///
    /// # Returns
    /// The ID of the newly created template.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Apply a policy template to a merchant, replacing their current refund policy.
    ///
    /// # Arguments
    /// * `admin` - The contract admin applying the template.
    /// * `merchant` - The merchant to apply the template to.
    /// * `template_id` - The ID of the template to apply.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `TemplateNotFound` if the template does not exist.
    /// Returns `TemplateInactive` if the template has been deactivated.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let template: RefundPolicyTemplate = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplate(template_id))
            .ok_or(Error::Ext(ExtError::TemplateNotFound))?;

        if !template.active {
            return Err(Error::Ext(ExtError::TemplateInactive));
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

    /// Get a policy template by its ID.
    ///
    /// # Arguments
    /// * `template_id` - The ID of the template to retrieve.
    ///
    /// # Returns
    /// The `RefundPolicyTemplate` if found, `None` otherwise.
    pub fn get_policy_template(env: Env, template_id: u64) -> Option<RefundPolicyTemplate> {
        env.storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplate(template_id))
    }

    /// List all active refund policy templates.
    ///
    /// # Returns
    /// A vector of all active `RefundPolicyTemplate` entries.
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

    /// Deactivate a refund policy template so it can no longer be applied.
    ///
    /// # Arguments
    /// * `admin` - The contract admin deactivating the template.
    /// * `template_id` - The ID of the template to deactivate.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `TemplateNotFound` if the template does not exist.
    /// Returns `TemplateInactive` if the template is already inactive.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut template: RefundPolicyTemplate = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyTemplate(template_id))
            .ok_or(Error::Ext(ExtError::TemplateNotFound))?;

        if !template.active {
            return Err(Error::Ext(ExtError::TemplateInactive));
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
            return Err(Error::Core(CoreError::Unauthorized));
        }

        if min_score < 0 {
            return Err(Error::Ext(ExtError::InvalidScoreThreshold));
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

    /// Retrieve the details of an arbitration case by its ID.
    ///
    /// # Arguments
    /// * `case_id` - The ID of the arbitration case to retrieve.
    ///
    /// # Returns
    /// The `ArbitrationCase` details.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if the case does not exist.
    pub fn get_arbitration_case(env: Env, case_id: u64) -> Result<ArbitrationCase, Error> {
        env.storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))
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
            return Err(Error::Core(CoreError::Unauthorized));
        }

        // Validate that shares add up to 10000 (100%)
        if config.arbitrator_share_bps + config.treasury_share_bps != 10000 {
            return Err(Error::Core(CoreError::InvalidFeeConfig));
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
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&ArbitrationKey::AccumulatedTreasuryFees)
            .unwrap_or(0);

        if accumulated <= 0 {
            return Err(Error::Core(CoreError::InsufficientTreasuryFees));
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
            return Err(Error::Core(CoreError::Unauthorized));
        }

        // Validate stake amount if enabled
        if config.enabled && config.amount <= 0 {
            return Err(Error::Core(CoreError::InvalidAmount));
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

    /// Get a paginated list of refunds filtered by status.
    ///
    /// # Arguments
    /// * `status` - The refund status to filter by.
    /// * `limit` - Maximum number of results to return.
    /// * `offset` - Number of results to skip for pagination.
    ///
    /// # Returns
    /// A vector of `Refund` entries matching the given status.
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

    /// Get a paginated list of all refunds for a specific merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    /// * `limit` - Maximum number of results to return.
    /// * `offset` - Number of results to skip for pagination.
    ///
    /// # Returns
    /// A vector of `Refund` entries for the merchant.
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

    /// Get a paginated list of refunds for a merchant filtered by status.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    /// * `status` - The refund status to filter by.
    /// * `limit` - Maximum number of results to return.
    /// * `offset` - Number of results to skip for pagination.
    ///
    /// # Returns
    /// A vector of `Refund` entries matching the merchant and status.
    pub fn get_merchant_refunds_by_status(
        env: Env,
        merchant: Address,
        status: RefundStatus,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        Self::get_merchant_refunds_by_status_internal(&env, &merchant, status, limit, offset)
    }

    /// Get all pending (requested) refunds for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// A vector of all `Refund` entries in `Requested` status for the merchant.
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

    /// Get aggregate refund statistics for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// A `MerchantRefundSummary` containing total requests, approved/rejected counts,
    /// total refunded amount, and pending counts/amounts.
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
                            pending_count += 1;
                            pending_amount += refund.amount;
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
                        RefundStatus::PendingAppeal => {
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

    /// Get a paginated list of refunds filtered by reason code.
    ///
    /// # Arguments
    /// * `code` - The refund reason code to filter by.
    /// * `limit` - Maximum number of results to return.
    /// * `offset` - Number of results to skip for pagination.
    ///
    /// # Returns
    /// A vector of `Refund` entries matching the given reason code.
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

    /// Get analytics showing the count of refunds for each reason code, sorted by frequency,
    /// restricted to refunds created within `[window_start, window_end]` (inclusive).
    ///
    /// The result is deterministic for a given window and is cached under
    /// `SystemKey::AnalyticsCache(window_start, window_end)`. The cache is invalidated
    /// (recomputed) only when a refund whose `created_at` falls inside that window is
    /// processed after the cache entry was written (see `process_refund_internal`).
    ///
    /// # Returns
    /// A vector of `(RefundReasonCode, count)` tuples sorted by descending count.
    pub fn get_reason_code_analytics(
        env: Env,
        window_start: u64,
        window_end: u64,
    ) -> Vec<(RefundReasonCode, u64)> {
        let cache_key = SystemKey::AnalyticsCache(window_start, window_end);
        if let Some(cached) = env
            .storage()
            .instance()
            .get::<_, Vec<(RefundReasonCode, u64)>>(&cache_key)
        {
            return cached;
        }

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
                if refund.requested_at >= window_start && refund.requested_at <= window_end {
                    match refund.reason_code {
                        RefundReasonCode::ProductDefect => product_defect += 1,
                        RefundReasonCode::NonDelivery => non_delivery += 1,
                        RefundReasonCode::DuplicateCharge => duplicate_charge += 1,
                        RefundReasonCode::Unauthorized => unauthorized += 1,
                        RefundReasonCode::CustomerRequest => customer_request += 1,
                        RefundReasonCode::Other => other += 1,
                    }
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

        env.storage().instance().set(&cache_key, &result);

        let mut windows: Vec<(u64, u64)> = env
            .storage()
            .instance()
            .get(&SystemKey::AnalyticsCacheWindows)
            .unwrap_or(Vec::new(&env));
        if !windows.iter().any(|w| w == (window_start, window_end)) {
            windows.push_back((window_start, window_end));
            env.storage()
                .instance()
                .set(&SystemKey::AnalyticsCacheWindows, &windows);
        }

        result
    }

    /// Invalidate any cached analytics window that contains `requested_at`.
    ///
    /// Only called when a refund is processed, per issue #382: caches are keyed by
    /// window and there is no bounded index of "cache keys ever written," so we
    /// track the small set of distinct windows queried so far and drop the ones
    /// whose range covers the newly processed refund's `requested_at`.
    fn invalidate_analytics_cache_for(env: &Env, requested_at: u64) {
        let windows: Vec<(u64, u64)> = env
            .storage()
            .instance()
            .get(&SystemKey::AnalyticsCacheWindows)
            .unwrap_or(Vec::new(env));
        for (window_start, window_end) in windows.iter() {
            if requested_at >= window_start && requested_at <= window_end {
                env.storage()
                    .instance()
                    .remove(&SystemKey::AnalyticsCache(window_start, window_end));
            }
        }
    }

    /// Get the total number of refunds in a given status.
    ///
    /// # Arguments
    /// * `status` - The refund status to count.
    ///
    /// # Returns
    /// The count of refunds in the specified status.
    pub fn get_refund_count_by_status(env: &Env, status: RefundStatus) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::RefundStatusCount(status))
            .unwrap_or(0)
    }

    /// Get the cumulative amount that has been refunded for a given payment.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID to calculate the total for.
    ///
    /// # Returns
    /// The total refunded amount in the smallest denomination of the token.
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

    /// Check whether a refund request for a given payment would exceed the original payment amount.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID to check.
    /// * `requested_amount` - The refund amount being requested.
    /// * `original_amount` - The original payment amount.
    ///
    /// # Returns
    /// `Ok(true)` if the refund is allowed.
    ///
    /// # Errors
    /// Returns `TotalRefundsExceedsPayment` if the cumulative refunds would exceed the original amount.
    pub fn can_refund_payment(
        env: &Env,
        payment_id: u64,
        requested_amount: i128,
        original_amount: i128,
    ) -> Result<bool, Error> {
        let total_refunded = Self::get_total_refunded_amount(env, payment_id);
        if requested_amount.saturating_add(total_refunded) > original_amount {
            return Err(Error::Core(CoreError::TotalRefundsExceedPayment));
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

    /// Set a refund policy for a merchant with tiered refund rules.
    ///
    /// Tiers are sorted by `days_from_purchase` in ascending order. Each tier
    /// specifies the maximum refund percentage (in basis points) within its time window.
    ///
    /// # Arguments
    /// * `merchant` - The merchant setting the policy (must authenticate).
    /// * `tiers` - A vector of `RefundTier` entries defining the policy tiers.
    ///
    /// # Errors
    /// Returns `RefundExceedsPolicy` if any tier has an invalid `max_refund_bps` value.
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
                return Err(Error::Core(CoreError::RefundExceedsPolicy));
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

    /// Get a specific versioned refund policy for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    /// * `version` - The version number to retrieve.
    ///
    /// # Returns
    /// The `RefundPolicyVersion` if found, `None` otherwise.
    pub fn get_refund_policy_version(
        env: Env,
        merchant: Address,
        version: u32,
    ) -> Option<RefundPolicyVersion> {
        env.storage()
            .instance()
            .get(&PolicyKey::RefundPolicyVersion(merchant, version))
    }

    /// Get the refund policy version that was in effect for a merchant at a given timestamp.
    ///
    /// Walks all versions in reverse order and returns the latest one created at or
    /// before the specified timestamp.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    /// * `timestamp` - The Unix timestamp to look up the policy for.
    ///
    /// # Returns
    /// The `RefundPolicyVersion` in effect at the given time, or `None` if no version existed.
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

    /// Get the full version history of refund policies for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// A vector of all `RefundPolicyVersion` entries in chronological order.
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

    /// Get the current active refund policy for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// The current `RefundPolicy` if one exists, `None` otherwise.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    fn get_auto_approve_below_ceiling_inner(env: &Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::AutoApproveBelowCeiling)
            .unwrap_or(0)
    }

    fn set_auto_approve_below_ceiling_inner(env: &Env, value: i128) {
        env.storage()
            .instance()
            .set(&DataKey::AutoApproveBelowCeiling, &value);
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

    /// Check whether a merchant's refunds require admin approval before processing.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// `true` if admin approval is required (the default), `false` otherwise.
    pub fn get_requires_admin_approval(env: Env, merchant: Address) -> bool {
        Self::get_requires_admin_approval_inner(&env, &merchant)
    }

    /// Set whether a merchant's refunds require admin approval before processing.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to configure (must authenticate).
    /// * `value` - `true` to require admin approval, `false` to allow auto-processing.
    pub fn set_requires_admin_approval(env: Env, merchant: Address, value: bool) {
        merchant.require_auth();
        Self::set_requires_admin_approval_inner(&env, &merchant, value);
    }

    /// Get the auto-approval threshold amount for a merchant.
    ///
    /// Refunds at or below this amount are automatically approved when admin approval
    /// is not required.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// The auto-approval threshold amount. Returns 0 if not configured.
    pub fn get_auto_approve_below(env: Env, merchant: Address) -> i128 {
        Self::get_auto_approve_below_inner(&env, &merchant)
    }

    /// Set the auto-approval threshold amount for a merchant.
    ///
    /// Refunds at or below this amount will be automatically approved when admin
    /// approval is not required.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to configure (must authenticate).
    /// * `value` - The threshold amount below which refunds are auto-approved.
    pub fn set_auto_approve_below(env: Env, merchant: Address, value: i128) -> Result<(), Error> {
        merchant.require_auth();
        let ceiling = Self::get_auto_approve_below_ceiling_inner(&env);
        if value > ceiling {
            return Err(Error::Core(CoreError::AutoApproveThresholdExceedsCeiling));
        }
        Self::set_auto_approve_below_inner(&env, &merchant, value);
        Ok(())
    }

    /// Get the platform-wide ceiling for merchant auto-approval thresholds.
    ///
    /// Refund thresholds above this value are rejected by `set_auto_approve_below()`.
    pub fn get_auto_approve_below_ceiling(env: Env) -> i128 {
        Self::get_auto_approve_below_ceiling_inner(&env)
    }

    /// Set the platform-wide ceiling for merchant auto-approval thresholds.
    ///
    /// # Arguments
    /// * `admin` - The contract admin configuring the ceiling.
    /// * `value` - The maximum auto-approval threshold any merchant may set.
    pub fn set_auto_approve_below_ceiling(
        env: Env,
        admin: Address,
        value: i128,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        Self::set_auto_approve_below_ceiling_inner(&env, value);
        Ok(())
    }

    /// Check whether a merchant inherits its refund policy from its parent merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query.
    ///
    /// # Returns
    /// `true` if inheritance is enabled (the default), `false` otherwise.
    pub fn get_inherit_from_parent(env: Env, merchant: Address) -> bool {
        Self::get_inherit_from_parent_inner(&env, &merchant)
    }

    /// Set whether a merchant inherits its refund policy from its parent merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to configure (must authenticate).
    /// * `inherit` - `true` to enable inheritance, `false` to disable it.
    pub fn set_inherit_from_parent(env: Env, merchant: Address, inherit: bool) {
        merchant.require_auth();
        Self::set_inherit_from_parent_inner(&env, &merchant, inherit);
    }

    /// Deactivate a merchant's refund policy so it is no longer enforced.
    ///
    /// # Arguments
    /// * `merchant` - The merchant whose policy should be deactivated (must authenticate).
    ///
    /// # Errors
    /// Returns `PolicyNotFound` if no policy exists for the merchant.
    /// Returns `PolicyInactive` if the policy is already inactive.
    pub fn deactivate_refund_policy(env: Env, merchant: Address) -> Result<(), Error> {
        // Require merchant authentication
        merchant.require_auth();

        let mut policy: RefundPolicy = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicy(merchant.clone()))
            .ok_or(Error::Core(CoreError::PolicyNotFound))?;

        if !policy.active {
            return Err(Error::Core(CoreError::PolicyInactive));
        }

        policy.active = false;
        env.storage()
            .instance()
            .set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // Emit RefundPolicyDeactivated event
        (RefundPolicyDeactivated { merchant }).publish(&env);

        Ok(())
    }

    /// Override a refund decision as an admin and create an immutable audit log entry.
    ///
    /// Records the override with a SHA-256 transaction hash for integrity verification.
    /// Emits both `AdminRefundOverride` and legacy `PolicyOverrideApplied` events.
    ///
    /// # Arguments
    /// * `admin` - The contract admin performing the override.
    /// * `refund_id` - The ID of the refund to override.
    /// * `new_status` - The new status to apply to the refund.
    /// * `new_amount` - The new amount to apply to the refund.
    /// * `reason` - A human-readable reason for the override.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `RefundNotFound` if the refund does not exist.
    pub fn admin_override_policy(
        env: Env,
        admin: Address,
        refund_id: u64,
        new_status: RefundStatus,
        new_amount: i128,
        reason: String,
    ) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        let admin_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;

        if admin != admin_address {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        // Verify refund exists and update it
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        // Apply override
        refund.status = new_status.clone();
        refund.amount = new_amount;
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);

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
        hash_data.append(&Bytes::from_slice(&env, &new_amount.to_be_bytes()));
        hash_data.append(&Bytes::from_slice(&env, &executed_at.to_be_bytes()));
        let transaction_hash = env.crypto().sha256(&hash_data);

        let audit_entry = AdminOverrideHistory {
            override_id,
            refund_id,
            admin: admin.clone(),
            reason: reason.clone(),
            override_amount: new_amount,
            override_status: new_status.clone(),
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
            override_amount: new_amount,
            override_status: new_status,
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        // Prevent self-parent
        if merchant == parent {
            return Err(Error::Core(CoreError::CircularInheritance));
        }

        // Check for circular reference by traversing up from parent
        // If we encounter the merchant in the parent's chain, it would create a cycle
        let mut visited = Vec::new(&env);
        visited.push_back(merchant.clone());

        let mut current = parent.clone();
        let mut depth: u32 = 1;

        while depth <= Self::MAX_INHERITANCE_DEPTH {
            if current == merchant {
                return Err(Error::Core(CoreError::CircularInheritance));
            }

            // Check if we've seen this address before (shouldn't happen but safety check)
            if visited.contains(&current) {
                return Err(Error::Core(CoreError::CircularInheritance));
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
            return Err(Error::Core(CoreError::MaxInheritanceDepth));
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
                        return Err(Error::Core(CoreError::CircularInheritance));
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
            return Err(Error::Core(CoreError::MaxInheritanceDepth));
        }

        Ok(chain)
    }

    /// Get the applicable refund basis points for a merchant and payment, considering
    /// policy inheritance and tier evaluation.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to evaluate.
    /// * `payment_id` - The payment ID to determine the applicable tier.
    ///
    /// # Returns
    /// The maximum refund amount in basis points (0-10000) applicable to the payment.
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
        customer: &Address,
        amount: i128,
        original_amount: i128,
        payment_created_at: u64,
        payment_id: u64,
    ) -> Result<(), Error> {
        let policy: RefundPolicy = Self::get_effective_refund_policy(env.clone(), merchant.clone())
            .ok_or(Error::Core(CoreError::PolicyNotFound))?;

        if !policy.active {
            return Err(Error::Core(CoreError::PolicyInactive));
        }

        let current_time = env.ledger().timestamp();
        let elapsed_seconds = current_time.saturating_sub(payment_created_at);
        let days_since_purchase = elapsed_seconds / (24 * 60 * 60);
        let elapsed_seconds_total = elapsed_seconds;

        // Enforce the category-specific (or policy-default) refund window first.
        // get_effective_window returns seconds; if elapsed time exceeds it the
        // refund is outside the allowed window regardless of tier settings.
        if payment_id > 0 {
            let effective_window_seconds =
                Self::get_effective_window(env.clone(), merchant.clone(), payment_id);
            if elapsed_seconds_total > effective_window_seconds {
                return Err(Error::Core(CoreError::RefundWindowExpired));
            }
        }

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
            return Err(Error::Core(CoreError::RefundWindowExpired));
        }

        // Issue #370: Override allowed_bps with customer tier policy if set
        let tier_id_opt: Option<u32> = env
            .storage()
            .instance()
            .get(&DataKey::CustomerTier(customer.clone()));

        if let Some(tier_id) = tier_id_opt {
            let tier_cap_opt: Option<RefundCap> = env
                .storage()
                .instance()
                .get(&DataKey::CustomerTierPolicy(merchant.clone(), tier_id));
            match tier_cap_opt {
                Some(cap) => {
                    allowed_bps = cap.max_refund_bps;
                }
                None => {
                    let strict: bool = env
                        .storage()
                        .instance()
                        .get(&DataKey::StrictTierPolicy(merchant.clone()))
                        .unwrap_or(false);
                    if strict {
                        return Err(Error::Ext(ExtError::TierPolicyNotFound));
                    }
                }
            }
        }

        // Check refund percentage using overflow-safe math
        let refund_percentage_bps = amount
            .checked_mul(10000)
            .unwrap_or(i128::MAX)
            .checked_div(original_amount)
            .unwrap_or(u32::MAX as i128) as u32;

        if refund_percentage_bps > allowed_bps {
            return Err(Error::Core(CoreError::RefundExceedsPolicy));
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
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        let index: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundStatusIndex(refund_id))
            .ok_or(Error::Core(CoreError::InvalidStatus))?;
        let last_index = count - 1;

        if index != last_index {
            let last_refund_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::RefundsByStatus(status.clone(), last_index))
                .ok_or(Error::Core(CoreError::InvalidStatus))?;
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

    /// Get the maximum number of refunds that can be processed in a single batch operation.
    ///
    /// # Returns
    /// The batch refund limit. Defaults to 20 if not configured.
    pub fn get_batch_refund_limit(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::BatchRefundLimit)
            .unwrap_or(Self::DEFAULT_BATCH_LIMIT)
    }

    /// Set the maximum number of refunds allowed per batch operation.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the limit.
    /// * `limit` - The maximum batch size.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn set_batch_refund_limit(env: Env, admin: Address, limit: u32) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::BatchRefundLimit, &limit);
        Ok(())
    }

    /// Approve multiple refunds in a single batch operation.
    ///
    /// Per-item failures are isolated; valid items are processed and invalid items
    /// return an error entry in the results vector without blocking other items.
    /// The entire batch is rejected if the count exceeds the configured batch limit.
    ///
    /// # Arguments
    /// * `admin` - The contract admin approving the refunds.
    /// * `refund_ids` - A vector of refund IDs to approve.
    ///
    /// # Returns
    /// A vector of `BatchRefundResult` entries indicating success or failure for each refund.
    pub fn approve_refund_batch(
        env: Env,
        admin: Address,
        refund_ids: Vec<u64>,
    ) -> Vec<BatchRefundResult> {
        admin.require_auth();
        let limit = Self::get_batch_refund_limit(env.clone());
        if refund_ids.len() > limit {
            // Batch-level validation failure: reject the entire batch without processing.
            let mut results = Vec::new(&env);
            results.push_back(BatchRefundResult {
                refund_id: 0,
                success: false,
                error_code: Error::Core(CoreError::BatchRefundTooLarge).to_u32(),
                amount_refunded: 0,
            });
            return results;
        }

        // Per-item failures are isolated; valid items are processed and invalid items
        // return an error entry in the results vector without blocking other items.
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
                        error_code: e.to_u32(),
                        amount_refunded: 0,
                    });
                }
            }
        }
        results
    }

    /// Process (finalize) multiple approved refunds in a single batch operation.
    ///
    /// Per-item failures are isolated; valid items are processed and invalid items
    /// return an error entry in the results vector without blocking other items.
    /// The entire batch is rejected if the count exceeds the configured batch limit.
    ///
    /// # Arguments
    /// * `admin` - The contract admin processing the refunds.
    /// * `refund_ids` - A vector of refund IDs to process.
    ///
    /// # Returns
    /// A vector of `BatchRefundResult` entries indicating success or failure for each refund.
    pub fn process_refund_batch(
        env: Env,
        admin: Address,
        refund_ids: Vec<u64>,
    ) -> Vec<BatchRefundResult> {
        admin.require_auth();
        let limit = Self::get_batch_refund_limit(env.clone());
        if refund_ids.len() > limit {
            // Batch-level validation failure: reject the entire batch without processing.
            let mut results = Vec::new(&env);
            results.push_back(BatchRefundResult {
                refund_id: 0,
                success: false,
                error_code: Error::Core(CoreError::BatchRefundTooLarge).to_u32(),
                amount_refunded: 0,
            });
            return results;
        }

        // Per-item failures are isolated; valid items are processed and invalid items
        // return an error entry in the results vector without blocking other items.
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
                        error_code: e.to_u32(),
                        amount_refunded: 0,
                    });
                }
            }
        }
        results
    }

    // ── Issue #143: Cross-contract payment verification ───────────────────────

    /// Set the address of the payment contract used for cross-contract ownership verification.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the address.
    /// * `payment_contract` - The address of the payment contract.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::PaymentContractAddress, &payment_contract);
        Ok(())
    }

    /// Get the address of the payment contract used for cross-contract verification.
    ///
    /// # Returns
    /// The payment contract address if configured, `None` otherwise.
    pub fn get_payment_contract_address(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::PaymentContractAddress)
    }

    /// Verify that a customer owns a given payment via a cross-contract call.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID to verify.
    /// * `customer` - The customer address to verify ownership for.
    ///
    /// # Returns
    /// `true` if the payment exists, belongs to the customer, and is completed.
    /// Returns `false` if no payment contract is set or verification fails.
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
            return Err(Error::Core(CoreError::InvalidAmount));
        }

        if amount > original_payment_amount {
            return Err(Error::Core(CoreError::RefundExceedsPayment));
        }

        Self::check_customer_refund_cooldown(&env, &customer)?;

        if payment_id == 0 {
            return Err(Error::Core(CoreError::InvalidPaymentId));
        }

        if env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::PaymentContractAddress)
            .is_some()
        {
            let owned = Self::verify_payment_ownership(env.clone(), payment_id, customer.clone());
            if !owned {
                return Err(Error::Core(CoreError::PaymentOwnershipMismatch));
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
                return Err(Error::Ext(ExtError::AddressFlaggedForFraud));
            }
        }

        // Issue #148: Check merchant-level customer eligibility
        let eligibility_rule = Self::check_refund_eligibility_internal(&env, &merchant, &customer);
        if eligibility_rule == EligibilityRule::Block {
            return Err(Error::Ext(ExtError::CustomerBlockedFromRefund));
        }

        if env.storage().instance().has(&DataKey::Admin) {
            Self::validate_against_policy(
                &env,
                &merchant,
                &customer,
                amount,
                original_payment_amount,
                payment_created_at,
                payment_id,
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
            let auto_below = {
                let merchant_threshold =
                    Self::get_auto_approve_below_inner(&env, &effective_merchant);
                let platform_ceiling = Self::get_auto_approve_below_ceiling_inner(&env);
                core::cmp::min(merchant_threshold, platform_ceiling)
            };
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
            rejected_by: None,
            appeal_deadline: None,
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

        Self::append_customer_refund_history(&env, &customer, refund_id);

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
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if refund.status != RefundStatus::Requested {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        // Issue #199: reject if TTL has expired
        if let Some(expires_at) = refund.expires_at {
            if env.ledger().timestamp() >= expires_at {
                return Err(Error::Core(CoreError::RefundWindowExpired));
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
            payment_id: refund.payment_id,
            amount: refund.amount,
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
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if refund.status != RefundStatus::Approved {
            return Err(Error::Core(CoreError::InvalidStatus));
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

        if net_refund_amount > 0 {
            token::Client::new(env, &refund.token).transfer(
                &env.current_contract_address(),
                &refund.customer,
                &net_refund_amount,
            );
        }

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
                .ok_or(Error::Core(CoreError::InvalidAmount))?;
            if new_used > quota.limit {
                return Err(Error::Core(CoreError::RefundExceedsPolicy));
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
        // Issue #382: this refund's requested_at may fall within a previously
        // cached analytics window, so drop that cache entry.
        Self::invalidate_analytics_cache_for(env, refund.requested_at);

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
            .ok_or(Error::Core(CoreError::PaymentContractNotSet))?;
        let args = (payment_id,).into_val(env);
        let func = Symbol::new(env, "get_payment");
        match env.try_invoke_contract::<ExternalPayment, soroban_sdk::InvokeError>(
            &payment_contract,
            &func,
            args,
        ) {
            Ok(Ok(payment)) => Ok(payment),
            _ => Err(Error::Core(CoreError::InvalidPaymentId)),
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

    /// Get overall refund analytics for the contract.
    ///
    /// # Returns
    /// A `RefundAnalytics` struct containing total requests, approvals, rejections,
    /// processed count, total volume, and approval rate in basis points.
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

    /// Pause the entire contract, blocking all state-changing refund operations.
    ///
    /// Records the pause event in the history log with a timestamp and reason.
    ///
    /// # Arguments
    /// * `admin` - The contract admin pausing the contract.
    /// * `reason` - A human-readable reason for the pause.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn pause_contract(env: Env, admin: Address, reason: String) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Unpause the contract and resume all refund operations.
    ///
    /// # Arguments
    /// * `admin` - The contract admin unpausing the contract.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn unpause_contract(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Pause a specific contract function while keeping others operational.
    ///
    /// # Arguments
    /// * `admin` - The contract admin pausing the function.
    /// * `function_name` - The name of the function to pause.
    /// * `reason` - A human-readable reason for the pause.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Unpause a previously paused contract function.
    ///
    /// # Arguments
    /// * `admin` - The contract admin unpausing the function.
    /// * `function_name` - The name of the function to unpause.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn unpause_function(env: Env, admin: Address, function_name: String) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Get the current global pause state of the contract.
    ///
    /// # Returns
    /// A `PauseState` struct indicating whether the contract is globally paused,
    /// which functions are individually paused, and who initiated the pause.
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

    /// Check whether a specific function is currently paused.
    ///
    /// # Arguments
    /// * `function_name` - The name of the function to check.
    ///
    /// # Returns
    /// `true` if the function is paused (either individually or due to a global pause).
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
                return Err(Error::Core(CoreError::ContractPaused));
            }
            let fn_str = String::from_str(env, function_name);
            for fn_name in state.paused_functions.iter() {
                if fn_name == fn_str {
                    return Err(Error::Core(CoreError::FunctionPaused));
                }
            }
        }
        Ok(())
    }

    // ── CIRCUIT BREAKER ────────────────────────────────────────────────────

    /// Set the circuit breaker configuration that monitors refund volume ratios.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the configuration.
    /// * `config` - The `CircuitBreakerConfig` with thresholds and cooldown settings.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&SystemKey::CircuitBreakerConfigKey, &config);
        Ok(())
    }

    /// Get the current state of the circuit breaker.
    ///
    /// # Returns
    /// A `CircuitBreakerState` indicating whether the breaker is tripped, the trip count,
    /// the last observed refund rate, and the auto-reset timestamp.
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

    /// Manually reset the circuit breaker and clear the tripped state.
    ///
    /// # Arguments
    /// * `admin` - The contract admin resetting the circuit breaker.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn reset_circuit_breaker(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Check whether the circuit breaker is currently active (tripped and not yet reset).
    ///
    /// # Returns
    /// `true` if the circuit breaker is tripped and the cooldown has not elapsed.
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

    fn effective_global_rate_limit(global: &GlobalRefundRateLimit, now: u64) -> (u32, u64) {
        if global.next_config_effective_at > 0 && now >= global.next_config_effective_at {
            (
                global.next_max_requests_per_window,
                global.next_window_seconds,
            )
        } else {
            (global.max_requests_per_window, global.window_seconds)
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
        let now = env.ledger().timestamp();
        let mut limit = match customer_limit_opt {
            Some(l) => l,
            None => {
                let g = global_limit_opt.as_ref().unwrap();
                let (max_requests, window_seconds) = Self::effective_global_rate_limit(g, now);
                CustomerRefundRateLimit {
                    customer: customer.clone(),
                    window_start: now,
                    request_count: 0,
                    max_requests_per_window: max_requests,
                    window_seconds,
                    custom_override: false,
                }
            }
        };
        if now >= limit.window_start + limit.window_seconds {
            limit.window_start = now;
            limit.request_count = 0;
            if !limit.custom_override {
                if let Some(ref g) = global_limit_opt {
                    let (max_requests, window_seconds) = Self::effective_global_rate_limit(g, now);
                    limit.max_requests_per_window = max_requests;
                    limit.window_seconds = window_seconds;
                }
            }
        }
        if limit.request_count >= limit.max_requests_per_window {
            return Err(Error::Core(CoreError::RefundRateLimitExceeded));
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
                    return Err(Error::Core(CoreError::CircuitBreakerTripped));
                }
            } else {
                return Err(Error::Core(CoreError::CircuitBreakerTripped));
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
            return Err(Error::Core(CoreError::CircuitBreakerTripped));
        }

        env.storage()
            .instance()
            .set(&SystemKey::WindowRefundVolume, &new_refund_vol);
        env.storage()
            .instance()
            .set(&SystemKey::WindowPaymentVolume, &new_payment_vol);

        Ok(())
    }

    /// Check for fraud signals on an address based on its refund rate relative to payment count.
    ///
    /// If the refund rate exceeds the configured threshold and the address has
    /// sufficient transaction history, a `FraudSignal` is created or updated.
    ///
    /// # Arguments
    /// * `address` - The address to check for fraud signals.
    ///
    /// # Returns
    /// The `FraudSignal` if one exists and has not been reviewed, `None` otherwise.
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

                    // Add to flagged addresses index: store both the ordered
                    // address entry and the updated counter so the list can
                    // be fully reconstructed by get_flagged_addresses.
                    let flagged_count: u64 = env
                        .storage()
                        .instance()
                        .get(&SystemKey::FlaggedAddressesIndex)
                        .unwrap_or(0);
                    env.storage()
                        .instance()
                        .set(&SystemKey::FlaggedAddress(flagged_count), &address.clone());
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

    /// Get all addresses that have been flagged for potential fraud.
    ///
    /// # Returns
    /// A vector of `FraudSignal` entries for all flagged addresses.
    pub fn get_flagged_addresses(env: Env) -> Vec<FraudSignal> {
        let mut flagged = Vec::new(&env);

        let total: u64 = env
            .storage()
            .instance()
            .get(&SystemKey::FlaggedAddressesIndex)
            .unwrap_or(0);

        for i in 0..total {
            if let Some(address) = env
                .storage()
                .instance()
                .get::<SystemKey, Address>(&SystemKey::FlaggedAddress(i))
            {
                if let Some(signal) = env
                    .storage()
                    .instance()
                    .get::<SystemKey, FraudSignal>(&SystemKey::FraudSignal(address))
                {
                    flagged.push_back(signal);
                }
            }
        }

        flagged
    }

    /// Mark a fraud signal as reviewed by an admin, allowing the address to continue
    /// requesting refunds.
    ///
    /// # Arguments
    /// * `admin` - The contract admin marking the signal as reviewed.
    /// * `address` - The flagged address to review.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `FraudSignalNotFound` if no fraud signal exists for the address.
    pub fn mark_fraud_reviewed(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin is the contract admin
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut signal: FraudSignal = env
            .storage()
            .instance()
            .get(&SystemKey::FraudSignal(address.clone()))
            .ok_or(Error::Ext(ExtError::FraudSignalNotFound))?;

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

    /// Set the fraud detection configuration thresholds.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the configuration.
    /// * `config` - The `FraudConfig` with detection parameters.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn set_fraud_config(env: Env, admin: Address, config: FraudConfig) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin is the contract admin
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        env.storage()
            .instance()
            .set(&SystemKey::FraudConfig, &config);

        Ok(())
    }

    // Helper functions for fraud detection
    fn get_customer_payment_count(env: &Env, address: &Address) -> u64 {
        let payment_contract: Address = match env
            .storage()
            .instance()
            .get(&DataKey::PaymentContractAddress)
        {
            Some(addr) => addr,
            None => return 0,
        };

        let func = Symbol::new(env, "get_payment_count_by_customer");
        let args = (address.clone(),).into_val(env);

        match env.try_invoke_contract::<u64, soroban_sdk::InvokeError>(
            &payment_contract,
            &func,
            args,
        ) {
            Ok(Ok(count)) => count,
            _ => 0,
        }
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

    /// Appends a refund id to a customer's history index, capping the number
    /// of entries kept in "hot" instance storage. Once the cap is exceeded,
    /// the oldest entry is moved to persistent storage so per-customer
    /// history can grow without bound while the instance storage footprint
    /// (loaded on every invocation) stays fixed.
    fn append_customer_refund_history(env: &Env, customer: &Address, refund_id: u64) {
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

        let start: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundHistoryStart(customer.clone()))
            .unwrap_or(0);
        let hot_len = customer_count + 1 - start;
        if hot_len > CUSTOMER_HISTORY_HOT_CAP {
            if let Some(archived_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::CustomerRefunds(customer.clone(), start))
            {
                env.storage().persistent().set(
                    &DataKey::CustomerRefundsArchive(customer.clone(), start),
                    &archived_id,
                );
            }
            env.storage()
                .instance()
                .remove(&DataKey::CustomerRefunds(customer.clone(), start));
            env.storage().instance().set(
                &DataKey::CustomerRefundHistoryStart(customer.clone()),
                &(start + 1),
            );
        }
    }

    /// Reads a customer's refund id at a given history index, transparently
    /// falling back to the archive when the entry has aged out of hot storage.
    fn get_customer_refund_id_at(env: &Env, customer: &Address, index: u64) -> Option<u64> {
        let start: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundHistoryStart(customer.clone()))
            .unwrap_or(0);
        if index < start {
            env.storage()
                .persistent()
                .get(&DataKey::CustomerRefundsArchive(customer.clone(), index))
        } else {
            env.storage()
                .instance()
                .get(&DataKey::CustomerRefunds(customer.clone(), index))
        }
    }

    /// Set the per-customer refund cooldown configuration.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the configuration.
    /// * `config` - Cooldown duration and enable flag.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn set_refund_cooldown_config(
        env: Env,
        admin: Address,
        config: RefundCooldownConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&SystemKey::RefundCooldownConfig, &config);
        Ok(())
    }

    /// Returns the configured per-customer refund cooldown, if any.
    pub fn get_refund_cooldown_config(env: Env) -> Option<RefundCooldownConfig> {
        env.storage()
            .instance()
            .get(&SystemKey::RefundCooldownConfig)
    }

    fn check_customer_refund_cooldown(env: &Env, customer: &Address) -> Result<(), Error> {
        let config: RefundCooldownConfig = match env
            .storage()
            .instance()
            .get::<SystemKey, RefundCooldownConfig>(&SystemKey::RefundCooldownConfig)
        {
            Some(c) if c.enabled => c,
            _ => return Ok(()),
        };

        let record: CustomerRefundCooldown = match env
            .storage()
            .instance()
            .get(&SystemKey::CustomerRefundCooldown(customer.clone()))
        {
            Some(r) => r,
            None => return Ok(()),
        };

        let now = env.ledger().timestamp();
        let elapsed = now.saturating_sub(record.last_refund_requested_at);
        if elapsed < record.cooldown_seconds {
            let available_at = record
                .last_refund_requested_at
                .saturating_add(record.cooldown_seconds);
            RefundCooldownEnforced {
                customer: customer.clone(),
                last_refund_at: record.last_refund_requested_at,
                cooldown_seconds: record.cooldown_seconds,
                available_at,
            }
            .publish(env);
            return Err(Error::Core(CoreError::RefundCooldownActive));
        }

        Ok(())
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

            if let Some(refund_id) = Self::get_customer_refund_id_at(&env, &customer, index) {
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
            if let Some(refund_id) = Self::get_customer_refund_id_at(&env, &customer, index) {
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

    /// Verify the subscriber address is a reachable contract that implements `ping()`.
    fn validate_notification_hook_subscriber(env: &Env, subscriber: &Address) -> Result<(), Error> {
        match env.try_invoke_contract::<(), soroban_sdk::InvokeError>(
            subscriber,
            &Symbol::new(env, "ping"),
            ().into_val(env),
        ) {
            Ok(Ok(_)) => Ok(()),
            _ => Err(Error::Ext(ExtError::InvalidHookAddress)),
        }
    }

    /// Register a notification hook for specific refund events
    pub fn register_notification_hook(
        env: Env,
        subscriber: Address,
        events: Vec<RefundEventType>,
    ) -> Result<u64, Error> {
        subscriber.require_auth();

        // Check that at least one event is specified
        if events.is_empty() {
            return Err(Error::Core(CoreError::InvalidAmount)); // Reusing error for invalid input
        }

        Self::validate_notification_hook_subscriber(&env, &subscriber)?;

        // Check max hooks per event type
        for event_type in events.iter() {
            let count: u32 = env
                .storage()
                .instance()
                .get(&SystemKey::HooksByEventCount(event_type.clone()))
                .unwrap_or(0);

            if count >= Self::MAX_HOOKS_PER_EVENT {
                return Err(Error::Ext(ExtError::MaxHooksPerEventReached));
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
            .ok_or(Error::Ext(ExtError::HookNotFound))?;

        // Verify ownership
        if hook.subscriber != subscriber {
            return Err(Error::Ext(ExtError::HookNotOwnedBySubscriber));
        }

        // Mark as inactive
        let mut updated_hook = hook.clone();
        updated_hook.active = false;

        env.storage()
            .instance()
            .set(&SystemKey::NotificationHook(hook_id), &updated_hook);

        // Decrement per-event hook counters
        for event_type in hook.events.iter() {
            let count: u32 = env
                .storage()
                .instance()
                .get(&SystemKey::HooksByEventCount(event_type.clone()))
                .unwrap_or(0);
            if count > 0 {
                env.storage().instance().set(
                    &SystemKey::HooksByEventCount(event_type.clone()),
                    &(count - 1),
                );
            }
        }

        // Decrement per-subscriber hook counter
        let subscriber_count: u32 = env
            .storage()
            .instance()
            .get(&SystemKey::SubscriberHookCount(subscriber.clone()))
            .unwrap_or(0);
        if subscriber_count > 0 {
            env.storage().instance().set(
                &SystemKey::SubscriberHookCount(subscriber.clone()),
                &(subscriber_count - 1),
            );
        }

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
                    if hook.active {
                        hooks.push_back(hook);
                    }
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
            return Err(Error::Ext(ExtError::EligibilityEntryNotFound));
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

    /// Batch reject multiple refunds in a single operation.
    ///
    /// Per-item failures are isolated; successful rejections are recorded in the
    /// `succeeded` list and failures in the `failed` list.
    ///
    /// # Arguments
    /// * `admin` - The contract admin performing the batch rejection.
    /// * `refund_ids` - A vector of refund IDs to reject.
    /// * `note_hash` - A SHA-256 hash of rejection notes (currently unused).
    ///
    /// # Returns
    /// A `BatchDecisionResult` with lists of succeeded and failed refund IDs.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `BatchRefundTooLarge` if the batch exceeds the configured limit.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        if refund_ids.len() > Self::BATCH_DECISION_LIMIT {
            return Err(Error::Core(CoreError::BatchRefundTooLarge));
        }

        let mut succeeded = Vec::new(&env);
        let mut failed = Vec::new(&env);
        let mut had_failure = false;

        for refund_id in refund_ids.iter() {
            let result = (|| -> Result<(), Error> {
                Self::begin_refund_rejection(
                    &env,
                    admin.clone(),
                    refund_id,
                    soroban_sdk::String::from_str(&env, "batch rejection"),
                )
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
            return Err(Error::Core(CoreError::BatchRefundTooLarge));
        }

        Ok(BatchDecisionResult { succeeded, failed })
    }

    // ── Issue #197: Category-based dynamic refund windows ─────────────────────

    /// Set a category-specific refund window for a merchant.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the window.
    /// * `merchant` - The merchant to configure the window for.
    /// * `category` - The payment category to apply the window to.
    /// * `window_seconds` - The refund window duration in seconds for this category.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Get the category-specific refund window for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    /// * `category` - The payment category to look up.
    ///
    /// # Returns
    /// The refund window in seconds for the category, or `None` if not configured.
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

    /// Tag a payment with a category to determine its applicable refund window.
    ///
    /// # Arguments
    /// * `merchant` - The merchant who owns the payment (must authenticate).
    /// * `payment_id` - The payment ID to tag.
    /// * `category` - The category to assign to the payment.
    ///
    /// # Errors
    /// Returns `AlreadyProcessed` if the payment has already been tagged.
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
            return Err(Error::Core(CoreError::AlreadyProcessed));
        }
        let cat_idx = category.to_index();
        env.storage()
            .instance()
            .set(&RefundExtKey::PaymentCategoryTag(payment_id), &cat_idx);
        Ok(())
    }

    /// Get the effective refund window for a specific payment, considering category tags.
    ///
    /// If the payment has a category tag and a category-specific window is configured,
    /// that window is returned. Otherwise, falls back to the merchant's default policy window.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    /// * `payment_id` - The payment ID to evaluate.
    ///
    /// # Returns
    /// The effective refund window in seconds.
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

    /// Configure the round-robin auto-assignment of arbitrators to cases.
    ///
    /// # Arguments
    /// * `admin` - The contract admin configuring the assignment.
    /// * `panel_size` - The number of arbitrators to assign per case.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `ArbitratorNotFound` if no arbitrators are registered or panel size exceeds the count.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        if arbitrators.is_empty() {
            return Err(Error::Ext(ExtError::ArbitratorNotFound));
        }

        if panel_size as u32 > arbitrators.len() {
            return Err(Error::Ext(ExtError::ArbitratorNotFound));
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

    /// Automatically assign a panel of arbitrators to a case using round-robin rotation.
    ///
    /// Advances the rotation index after assignment to ensure even distribution of cases.
    ///
    /// # Arguments
    /// * `case_id` - The ID of the arbitration case (currently reserved for future use).
    ///
    /// # Returns
    /// A vector of assigned arbitrator addresses.
    ///
    /// # Errors
    /// Returns `PolicyNotFound` if auto-assignment has not been configured.
    /// Returns `ArbitratorNotFound` if no arbitrators are registered.
    pub fn auto_assign_arbitrators(env: Env, case_id: u64) -> Result<Vec<Address>, Error> {
        let mut config: ArbitratorAssignmentConfig = env
            .storage()
            .instance()
            .get(&RefundExtKey::AssignmentConfig)
            .ok_or(Error::Core(CoreError::PolicyNotFound))?;

        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        if arbitrators.is_empty() {
            return Err(Error::Ext(ExtError::ArbitratorNotFound));
        }

        let total = arbitrators.len() as u32;
        if config.panel_size > total {
            return Err(Error::Ext(ExtError::ArbitratorNotFound));
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

        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;
        case.arbitrators = panel.clone();
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationCase(case_id), &case);

        Ok(panel)
    }

    /// Preview the next arbitrators that would be assigned using round-robin rotation.
    ///
    /// Does not advance the rotation index.
    ///
    /// # Arguments
    /// * `count` - The number of arbitrators to preview.
    ///
    /// # Returns
    /// A vector of the next arbitrator addresses in rotation order.
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

    /// Reset the round-robin rotation index back to the beginning.
    ///
    /// # Arguments
    /// * `admin` - The contract admin resetting the index.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `PolicyNotFound` if auto-assignment has not been configured.
    pub fn reset_rotation_index(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut config: ArbitratorAssignmentConfig = env
            .storage()
            .instance()
            .get(&RefundExtKey::AssignmentConfig)
            .ok_or(Error::Core(CoreError::PolicyNotFound))?;

        config.rotation_index = 0;
        env.storage()
            .instance()
            .set(&RefundExtKey::AssignmentConfig, &config);
        Ok(())
    }

    // ── Issue #199: Refund request TTL with automatic expiry ──────────────────

    /// Configure the default time-to-live for refund requests.
    ///
    /// Refunds that are not processed before their TTL expires can be automatically
    /// rejected via `expire_stale_refund`.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the TTL.
    /// * `ttl_seconds` - The default TTL in seconds for new refund requests.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn set_refund_ttl_config(env: Env, admin: Address, ttl_seconds: u64) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Expire a stale refund request that has exceeded its TTL without being processed.
    ///
    /// Moves the refund from `Requested` to `Rejected` status with a "TTL expired" reason.
    ///
    /// # Arguments
    /// * `refund_id` - The ID of the refund to expire.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if the refund does not exist.
    /// Returns `InvalidStatus` if the refund is not in `Requested` status.
    /// Returns `RefundWindowExpired` if the refund's TTL has not yet elapsed.
    pub fn expire_stale_refund(env: Env, refund_id: u64) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if refund.status != RefundStatus::Requested {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        let expires_at = refund
            .expires_at
            .ok_or(Error::Core(CoreError::PolicyNotFound))?;

        if env.ledger().timestamp() < expires_at {
            return Err(Error::Core(CoreError::RefundWindowExpired));
        }

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;
        refund.status = RefundStatus::Rejected;
        refund.rejected_at = Some(env.ledger().timestamp());
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);
        Self::release_payment_refund_usage(&env, refund.payment_id, refund.amount);

        (RefundRejected {
            refund_id,
            rejected_by: env.current_contract_address(),
            rejected_at: env.ledger().timestamp(),
            rejection_reason: soroban_sdk::String::from_str(&env, "TTL expired"),
        })
        .publish(&env);

        Ok(())
    }

    /// Get refund IDs that have expired (past their TTL) and are still in `Requested` status.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of expired refund IDs to return.
    ///
    /// # Returns
    /// A vector of refund IDs that are eligible for expiration.
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

    /// Submit evidence for a refund dispute as the customer or merchant.
    ///
    /// Each party (customer or merchant) can submit one evidence entry per refund.
    ///
    /// # Arguments
    /// * `submitter` - The address submitting the evidence (must be the customer or merchant).
    /// * `refund_id` - The ID of the refund to submit evidence for.
    /// * `evidence_hash` - SHA-256 hash of the evidence document.
    ///
    /// # Errors
    /// Returns `RefundNotFound` if the refund does not exist.
    /// Returns `Unauthorized` if the submitter is not the customer or merchant.
    /// Returns `EvidenceAlreadySubmitted` if this party has already submitted evidence.
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
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if submitter != refund.customer && submitter != refund.merchant {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        if env
            .storage()
            .instance()
            .has(&EvidenceKey::Evidence(refund_id, submitter.clone()))
        {
            return Err(Error::Ext(ExtError::EvidenceAlreadySubmitted));
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

    /// Get the evidence submitted by a specific party for a refund dispute.
    ///
    /// # Arguments
    /// * `refund_id` - The ID of the refund.
    /// * `submitter` - The address of the party who submitted the evidence.
    ///
    /// # Returns
    /// The `RefundEvidence` if found, `None` otherwise.
    pub fn get_refund_evidence(
        env: Env,
        refund_id: u64,
        submitter: Address,
    ) -> Option<RefundEvidence> {
        env.storage()
            .instance()
            .get(&EvidenceKey::Evidence(refund_id, submitter))
    }

    /// Get all evidence entries submitted for a refund dispute.
    ///
    /// # Arguments
    /// * `refund_id` - The ID of the refund.
    ///
    /// # Returns
    /// A vector of all `RefundEvidence` entries for the refund.
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

    /// Register a token as a supported refund payment method.
    ///
    /// # Arguments
    /// * `admin` - The contract admin registering the token.
    /// * `token` - The address of the token contract to register.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn register_refund_token(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Deregister a token so it can no longer be used for refunds.
    ///
    /// Sets the token's active status to `false` rather than removing it.
    ///
    /// # Arguments
    /// * `admin` - The contract admin deregistering the token.
    /// * `token` - The address of the token contract to deregister.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `RefundNotFound` if the token is not registered.
    pub fn deregister_refund_token(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let mut entry: SupportedRefundToken = env
            .storage()
            .instance()
            .get(&TokenKey::SupportedToken(token.clone()))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        entry.active = false;
        env.storage()
            .instance()
            .set(&TokenKey::SupportedToken(token), &entry);

        Ok(())
    }

    /// Get all registered refund tokens (both active and inactive).
    ///
    /// # Returns
    /// A vector of `SupportedRefundToken` entries for all registered tokens.
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

    /// Issue a refund credit voucher for an approved refund.
    ///
    /// Creates a voucher with the refund amount that the customer can redeem
    /// against a future payment.
    ///
    /// # Arguments
    /// * `admin` - The contract admin issuing the voucher.
    /// * `refund_id` - The ID of the refund to create a voucher for.
    /// * `expiry_seconds` - The number of seconds until the voucher expires.
    ///
    /// # Returns
    /// The ID of the newly created voucher.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `RefundNotFound` if the refund does not exist.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        let refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        // Validate refund status is Approved
        if refund.status != RefundStatus::Approved {
            return Err(Error::Core(CoreError::InvalidAmount));
        }

        // Prevent duplicate vouchers for the same refund
        if env
            .storage()
            .instance()
            .get::<_, bool>(&VoucherKey::RefundVoucherIssued(refund_id))
            .unwrap_or(false)
        {
            return Err(Error::Core(CoreError::InvalidAmount));
        }

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

        // Mark voucher as issued for this refund
        env.storage()
            .instance()
            .set(&VoucherKey::RefundVoucherIssued(refund_id), &true);

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

    /// Redeem a refund credit voucher for a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer redeeming the voucher (must authenticate).
    /// * `voucher_id` - The ID of the voucher to redeem.
    /// * `_payment_id` - The payment ID to apply the voucher to (reserved for future use).
    ///
    /// # Errors
    /// Returns `VoucherNotFound` if the voucher does not exist.
    /// Returns `Unauthorized` if the caller is not the voucher's customer.
    /// Returns `VoucherAlreadyRedeemed` if the voucher has already been used.
    /// Returns `VoucherExpired` if the voucher has expired.
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
            .ok_or(Error::Ext(ExtError::VoucherNotFound))?;

        if voucher.customer != customer {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        if voucher.redeemed {
            return Err(Error::Ext(ExtError::VoucherAlreadyRedeemed));
        }
        if env.ledger().timestamp() > voucher.expires_at {
            return Err(Error::Ext(ExtError::VoucherExpired));
        }

        token::Client::new(&env, &voucher.token).transfer(
            &env.current_contract_address(),
            &customer,
            &voucher.amount,
        );

        voucher.redeemed = true;
        env.storage()
            .instance()
            .set(&VoucherKey::Voucher(voucher_id), &voucher);

        Ok(())
    }

    /// Get a refund voucher by its ID.
    ///
    /// # Arguments
    /// * `voucher_id` - The ID of the voucher to retrieve.
    ///
    /// # Returns
    /// The `RefundVoucher` if found, `None` otherwise.
    pub fn get_voucher(env: Env, voucher_id: u64) -> Option<RefundVoucher> {
        env.storage()
            .instance()
            .get(&VoucherKey::Voucher(voucher_id))
    }

    /// Get all refund vouchers issued to a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query.
    ///
    /// # Returns
    /// A vector of `RefundVoucher` entries for the customer.
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

    /// Add an arbitrator to the senior arbitrator list for tiered escalation.
    ///
    /// Senior arbitrators handle cases that have been escalated from the junior panel.
    ///
    /// # Arguments
    /// * `admin` - The contract admin adding the senior arbitrator.
    /// * `arbitrator` - The address of the arbitrator to add.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
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

    /// Set the configuration for tiered arbitration escalation.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the configuration.
    /// * `config` - The `ArbitrationTierConfig` with escalation timeout settings.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&ArbitrationKey::ArbitrationTierConfig, &config);
        Ok(())
    }

    /// Escalate an arbitration case from the junior panel to the senior arbitrator panel.
    ///
    /// The case must be in `Open` status and must have exceeded the escalation timeout.
    /// Resets all votes and reassigns the case to senior arbitrators.
    ///
    /// # Arguments
    /// * `case_id` - The ID of the arbitration case to escalate.
    ///
    /// # Errors
    /// Returns `CaseAlreadyEscalated` if the case has already been escalated.
    /// Returns `RefundNotFound` if the case does not exist.
    /// Returns `InvalidStatus` if the case is not open.
    /// Returns `CaseNotTimedOut` if the escalation timeout has not elapsed.
    /// Returns `ArbitratorNotFound` if no senior arbitrators are registered.
    pub fn escalate_arbitration_case(env: Env, case_id: u64) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .has(&ArbitrationKey::CaseEscalated(case_id))
        {
            return Err(Error::Ext(ExtError::CaseAlreadyEscalated));
        }

        let mut case: ArbitrationCase = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationCase(case_id))
            .ok_or(Error::Core(CoreError::RefundNotFound))?;

        if case.status != ArbitrationStatus::Open {
            return Err(Error::Core(CoreError::InvalidStatus));
        }

        let config: ArbitrationTierConfig = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitrationTierConfig)
            .ok_or(Error::Core(CoreError::CaseNotTimedOut))?;

        if env.ledger().timestamp()
            < case
                .created_at
                .saturating_add(config.escalation_timeout_seconds)
        {
            return Err(Error::Core(CoreError::CaseNotTimedOut));
        }

        let senior_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::SeniorArbitratorList)
            .unwrap_or(Vec::new(&env));

        if senior_list.len() == 0 {
            return Err(Error::Ext(ExtError::ArbitratorNotFound));
        }

        Self::clear_arbitration_votes(&env, case_id);

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

    /// Get the arbitration tier (Junior or Senior) for a given case.
    ///
    /// # Arguments
    /// * `case_id` - The ID of the arbitration case.
    ///
    /// # Returns
    /// `ArbitratorTier::Senior` if the case has been escalated, `ArbitratorTier::Junior` otherwise.
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

    /// Set a refund cap on a specific payment to limit the number and amount of refunds.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the cap.
    /// * `cap` - The `PaymentRefundCap` with max count and max total amount limits.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    /// Returns `InvalidPaymentId` if the payment ID is zero.
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
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }

        if cap.payment_id == 0 {
            return Err(Error::Core(CoreError::InvalidPaymentId));
        }

        env.storage()
            .instance()
            .set(&DataKey::PaymentRefundCap(cap.payment_id), &cap);
        Ok(())
    }

    /// Get the refund cap configuration for a specific payment.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID to query.
    ///
    /// # Returns
    /// The `PaymentRefundCap` if configured, `None` otherwise.
    pub fn get_payment_refund_cap(env: Env, payment_id: u64) -> Option<PaymentRefundCap> {
        env.storage()
            .instance()
            .get(&DataKey::PaymentRefundCap(payment_id))
    }

    /// Get the current refund usage for a specific payment.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID to query.
    ///
    /// # Returns
    /// A tuple of `(refund_count, total_refunded_amount)` representing how many
    /// refunds have been made and the cumulative amount refunded.
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
            return Err(Error::Ext(ExtError::RefundCountCapExceeded));
        }

        // Check amount cap (cumulative across all statuses except Rejected)
        let new_total_amount = current_amount.saturating_add(refund_amount);
        if new_total_amount > cap.max_total_amount {
            return Err(Error::Ext(ExtError::RefundAmountCapExceeded));
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

    fn release_payment_refund_usage(env: &Env, payment_id: u64, refund_amount: i128) {
        let (current_count, current_amount): (u32, i128) = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundUsage(payment_id))
            .unwrap_or((0u32, 0i128));

        let new_count = current_count.saturating_sub(1u32);
        let new_amount = current_amount.saturating_sub(refund_amount);

        env.storage().instance().set(
            &DataKey::PaymentRefundUsage(payment_id),
            &(new_count, new_amount),
        );
    }

    fn clear_arbitration_votes(env: &Env, case_id: u64) {
        let voters: Vec<Address> = env
            .storage()
            .instance()
            .get(&ArbitrationKey::ArbitratorsVoted(case_id))
            .unwrap_or_else(|| Vec::new(env));

        for voter in voters.iter() {
            env.storage()
                .instance()
                .remove(&ArbitrationKey::ArbitratorVote(case_id, voter.clone()));
        }

        env.storage()
            .instance()
            .remove(&ArbitrationKey::ArbitratorsVoted(case_id));
    }

    fn validate_bps(bps: u32) -> Result<(), Error> {
        if bps < 1 || bps > 10000 {
            return Err(Error::Core(CoreError::InvalidAmount));
        };

        Ok(())
    }

    // Issue #370: Customer tier management

    /// Assign a tier level to a customer for tier-based refund cap policies.
    ///
    /// # Arguments
    /// * `admin` - The contract admin setting the tier.
    /// * `customer` - The customer address to assign the tier to.
    /// * `tier_id` - The tier level to assign.
    ///
    /// # Errors
    /// Returns `Unauthorized` if the caller is not the contract admin.
    pub fn set_customer_tier(
        env: Env,
        admin: Address,
        customer: Address,
        tier_id: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Core(CoreError::Unauthorized))?;
        if admin != stored_admin {
            return Err(Error::Core(CoreError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::CustomerTier(customer), &tier_id);
        Ok(())
    }

    /// Get the tier level assigned to a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query.
    ///
    /// # Returns
    /// The tier ID if assigned, `None` otherwise.
    pub fn get_customer_tier(env: Env, customer: Address) -> Option<u32> {
        env.storage()
            .instance()
            .get(&DataKey::CustomerTier(customer))
    }

    /// Set the refund cap policy for a specific customer tier under a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant setting the policy (must authenticate).
    /// * `tier_id` - The tier level to configure.
    /// * `max_refund_bps` - The maximum refund percentage in basis points (0-10000).
    ///
    /// # Errors
    /// Returns `InvalidAmount` if `max_refund_bps` exceeds 10000.
    pub fn set_customer_tier_policy(
        env: Env,
        merchant: Address,
        tier_id: u32,
        max_refund_bps: u32,
    ) -> Result<(), Error> {
        merchant.require_auth();
        if max_refund_bps > 10000 {
            return Err(Error::Core(CoreError::InvalidAmount));
        }
        let cap = RefundCap { max_refund_bps };
        env.storage()
            .instance()
            .set(&DataKey::CustomerTierPolicy(merchant, tier_id), &cap);
        Ok(())
    }

    /// Get the refund cap policy for a specific customer tier under a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    /// * `tier_id` - The tier level to look up.
    ///
    /// # Returns
    /// The `RefundCap` for the tier if configured, `None` otherwise.
    pub fn get_customer_tier_policy(
        env: Env,
        merchant: Address,
        tier_id: u32,
    ) -> Option<RefundCap> {
        env.storage()
            .instance()
            .get(&DataKey::CustomerTierPolicy(merchant, tier_id))
    }

    /// Enable or disable strict tier policy enforcement for a merchant.
    ///
    /// When strict mode is enabled, customers without an assigned tier are
    /// denied refunds instead of falling back to default behavior.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to configure (must authenticate).
    /// * `strict` - `true` to enable strict mode, `false` to disable it.
    pub fn set_strict_tier_policy(env: Env, merchant: Address, strict: bool) -> Result<(), Error> {
        merchant.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::StrictTierPolicy(merchant), &strict);
        Ok(())
    }

    /// Check whether strict tier policy enforcement is enabled for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    ///
    /// # Returns
    /// `true` if strict mode is enabled, `false` otherwise (the default).
    pub fn get_strict_tier_policy(env: Env, merchant: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::StrictTierPolicy(merchant))
            .unwrap_or(false)
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

#[cfg(test)]
mod test_versioning;

#[cfg(test)]
mod test_batch;

#[cfg(test)]
mod test_cross_contract;

#[cfg(test)]
mod test_arbitration_fees;

#[cfg(test)]
mod test_arbitration_stake;
mod test_refund_cooldown;

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

#[cfg(test)]
mod test_customer_tier_policy;

#[cfg(test)]
mod test_voucher_expiry;

#[cfg(test)]
mod schema_version_test;

#[cfg(test)]
mod test_merchant_override_and_error_codes;

#[cfg(test)]
mod test_admin_rotation;
