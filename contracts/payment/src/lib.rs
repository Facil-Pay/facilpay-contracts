#![no_std]
use escrow::EscrowContractClient;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes,
    BytesN, Env, IntoVal, String, Symbol, Vec,
};


// Core payment data keys
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Payment(u64),
    PaymentCounter,
    PaymentMetadata(u64),
    SubscriptionCounter,
    Subscription(u64),
    ProposalCounter,
    LargePaymentCounter,
    ScheduledPaymentCounter,
    GlobalMerchantCount,
    PauseHistoryCount,
    PaymentAnalyticsKey,
    PauseStateKey,
    RateLimitConfig,
    FeeConfig,
    TierThresholds,
    AddressRateLimit(Address),
    MerchantRateLimit(Address),
    AddressFlagReason(Address),
    AddressAllowlist(Address),
    DunningConfig,
    MultiSigConfig,
    LargePaymentThreshold,
    RiskFeeConfig,
    ConversionRate(Currency),
    PlatformAnalyticsDaily(u64),
    // Partial payment support (#112)
    PartialPaymentCounter(u64),
    OutstandingBalance(u64),
    // Oracle data
    OracleRateConfig(Currency),
    // Payment Channel support (#125)
    PaymentChannel(u64),
    PaymentChannelCounter,
}

// Customer-specific data keys
#[derive(Clone)]
#[contracttype]
pub enum CustomerDataKey {
    Payments(Address, u64),
    PaymentCount(Address),
    Subscriptions(Address, u64),
    SubscriptionCount(Address),
    Analytics(Address),
    RateLimit(Address),
    FlagReason(Address),
    Allowlist(Address),
    FeeWaiver(Address),
    MerchantVolume(Address, Address),
    MerchantList(Address, u64),
    MerchantCount(Address),
    MonthlyVolume(Address, u64),
    HourCount(Address, u32),
}

// Merchant-specific data keys
#[derive(Clone)]
#[contracttype]
pub enum MerchantDataKey {
    Payments(Address, u64),
    PaymentCount(Address),
    Subscriptions(Address, u64),
    SubscriptionCount(Address),
    Analytics(Address),
    FeeRecord(Address),
    AnalyticsBucket(Address, u64),
    GlobalList(u64),
}

// State and proposal data keys
#[derive(Clone)]
#[contracttype]
pub enum StateDataKey {
    DunningState(u64),
    EscrowedPayment(u64),
    EscrowedPaymentDispute(u64),
    ConditionalPayment(u64),
    ScheduledPayment(u64),
    AdminProposal(String),
    LargePaymentProposal(u64),
    PauseHistoryEntry(u64),
    PartialPaymentRecord(u64, u32), // payment_id, installment_number
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum Currency {
    XLM,
    USDC,
    USDT,
    BTC,
    ETH,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PaymentStatus {
    Pending,
    Completed,
    Refunded,
    PartialRefunded,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SubscriptionStatus {
    Active,
    Paused,
    Cancelled,
    Expired,
    InDunning,
    Suspended,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ConditionType {
    TimestampAfter(u64),
    TimestampBefore(u64),
    OraclePrice(Address, String, i128, PriceComparison),
    CrossContractState(Address, BytesN<32>),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PriceComparison {
    GreaterThan,
    LessThan,
    EqualTo,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct SubscriptionTrialData {
    pub period_seconds: u64,
    pub ends_at: u64,
    pub converted: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct SubscriptionPauseData {
    pub last_paused_at: u64,
    pub total_pause_duration: u64,
    pub proration_enabled: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct Subscription {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub currency: Currency,
    pub interval: u64, // seconds between payments
    pub duration: u64, // total seconds the subscription lives (0 = indefinite)
    pub status: SubscriptionStatus,
    pub created_at: u64,
    pub next_payment_at: u64,
    pub ends_at: u64,       // 0 = no hard end
    pub payment_count: u64, // successful executions so far
    pub retry_count: u64,   // consecutive failed attempts on current cycle
    pub max_retries: u64,   // max retries before marking failed cycle skipped
    pub metadata: String,
    pub trial_data: SubscriptionTrialData,
    pub pause_data: SubscriptionPauseData,
}

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    PaymentNotFound = 1,
    InvalidStatus = 2,
    AlreadyProcessed = 3,
    Unauthorized = 4,
    PaymentExpired = 5,
    NotExpired = 6,
    NoExpiration = 7,
    TransferFailed = 8,
    MetadataTooLarge = 9,
    NotesTooLarge = 10,
    InvalidCurrency = 11,
    RefundExceedsPayment = 12,
    SubscriptionNotFound = 13,
    SubscriptionNotActive = 14,
    PaymentNotDue = 15,
    MaxRetriesExceeded = 16,
    SubscriptionEnded = 17,
    InvalidBatchSize = 18,
    BatchPartialFailure = 19,
    RateLimitExceeded = 20,
    DailyVolumeExceeded = 21,
    AddressFlagged = 60,
    AddressAlreadyFlagged = 61,
    AmountExceedsLimit = 23,
    DunningNotFound = 24,
    SubscriptionNotInDunning = 25,
    RetryNotDue = 26,
    GracePeriodExpired = 27,
    EscrowMappingNotFound = 28,
    EscrowBridgeFailed = 29,
    MultiSigNotInitialized = 30,
    ProposalNotFound = 31,
    ProposalExpired = 32,
    ProposalAlreadyExecuted = 33,
    MultiSigThresholdNotMet = 34,
    InsufficientAdmins = 35,
    NotAnAdmin = 36,
    AlreadyApproved = 37,
    FeeConfigNotFound = 38,
    InsufficientFees = 39,
    ConditionNotMet = 40,
    ConditionAlreadyEvaluated = 41,
    OracleCallFailed = 42,
    ContractPaused = 43,
    FunctionPaused = 44,
    InvalidTierThresholds = 45,
    PaymentNotYetDue = 54,
    ScheduledPaymentCancelled = 55,
    OracleFeedStale = 58,
    OracleNotConfigured = 59,
    ConditionEvaluationFailed = 62,
    ConditionRuntimeNotMet = 63,
    RetryTooEarly = 46,
    PaymentRequiresMultiSig = 64,
    InsufficientPaymentApprovals = 65,
    PaymentProposalExpired = 66,
    MetadataAlreadySet = 67,
    MetadataNotFound = 68,
    HashMismatch = 69,
    PaymentAlreadyFullyPaid = 52,
    InstallmentExceedsRemaining = 53,
    PartialPaymentNotFound = 70,
    MerchantRateLimitExceeded = 50,
    AmountRateLimitExceeded = 51,
    InvalidFeeConfig = 71,
    InvalidAmount = 72,
    ChannelNotFound = 73,
    InvalidSignature = 74,
    InvalidNonce = 75,
    ChannelClosed = 76,
    ChannelExpired = 77,
    ChannelNotExpired = 78,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentCreated {
    pub payment_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentCompleted {
    pub payment_id: u64,
    pub merchant: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentRefunded {
    pub payment_id: u64,
    pub customer: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentCancelled {
    pub payment_id: u64,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentExpired {
    pub payment_id: u64,
    pub customer: Address,
    pub refunded_amount: i128,
    pub expired_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallmentPaid {
    pub payment_id: u64,
    pub installment_number: u32,
    pub amount: i128,
    pub remaining: i128,
    pub payer: Address,
    pub paid_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentFullyPaid {
    pub payment_id: u64,
    pub total_installments: u32,
    pub completed_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowedPaymentCreated {
    pub payment_id: u64,
    pub escrow_id: u64,
    pub escrow_contract: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowedPaymentCompleted {
    pub payment_id: u64,
    pub escrow_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowedPaymentCancelled {
    pub payment_id: u64,
    pub escrow_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowedPaymentDisputed {
    pub payment_id: u64,
    pub raised_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowedPaymentDisputeResolved {
    pub payment_id: u64,
    pub favor_customer: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionCreated {
    pub subscription_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub interval: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecurringPaymentExecuted {
    pub subscription_id: u64,
    pub payment_count: u64,
    pub amount: i128,
    pub next_payment_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecurringPaymentFailed {
    pub subscription_id: u64,
    pub retry_count: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionCancelled {
    pub subscription_id: u64,
    pub cancelled_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskFeeApplied {
    pub payment_id: u64,
    pub base_fee_bps: u32,
    pub risk_surcharge_bps: u32,
    pub total_fee_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentChannel {
    pub channel_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub token: Address,
    pub deposited: i128,
    pub settled: i128,
    pub nonce: u64,
    pub open: bool,
    pub expires_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelOpened {
    pub channel_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelSettled {
    pub channel_id: u64,
    pub merchant_amount: i128,
    pub customer_refund: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelExpiredClosed {
    pub channel_id: u64,
    pub refunded_to: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionPaused {
    pub subscription_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionResumed {
    pub subscription_id: u64,
    pub next_payment_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionResumedWithProration {
    pub subscription_id: u64,
    pub pause_duration: u64,
    pub new_next_billing_date: u64,
    pub prorated_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialStarted {
    pub subscription_id: u64,
    pub trial_ends_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialConverted {
    pub subscription_id: u64,
    pub converted_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialCancelled {
    pub subscription_id: u64,
    pub cancelled_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressFlagged {
    pub address: Address,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressUnflagged {
    pub address: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitBreached {
    pub address: Address,
    pub payment_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionEnteredDunning {
    pub subscription_id: u64,
    pub attempt: u32,
    pub next_retry_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DunningRetryScheduled {
    pub subscription_id: u64,
    pub retry_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionSuspended {
    pub subscription_id: u64,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DunningResolved {
    pub subscription_id: u64,
    pub resolved_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct RateLimitConfig {
    pub max_payments_per_window: u32,
    pub window_duration: u64,
    pub max_payment_amount: i128,
    pub max_daily_volume: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct AddressRateLimit {
    pub address: Address,
    pub payment_count: u32,
    pub window_start: u64,
    pub daily_volume: i128,
    pub last_payment_at: u64,
    pub flagged: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct MerchantRateLimit {
    pub merchant: Address,
    pub max_transactions_per_hour: u32,
    pub max_amount_per_hour: i128,
    pub current_transactions: u32,
    pub current_amount: i128,
    pub window_start: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct DunningConfig {
    pub initial_backoff_seconds: u64,
    pub max_retries: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct DunningState {
    pub subscription_id: u64,
    pub retry_count: u32,
    pub next_retry_at: u64,
    pub backoff_seconds: u64,
    pub max_retries: u32,
    pub last_failed_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct PartialPaymentRecord {
    pub payment_id: u64,
    pub installment_number: u32,
    pub amount_paid: i128,
    pub total_amount: i128,
    pub remaining: i128,
    pub paid_at: u64,
    pub payer: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct Payment {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub currency: Currency,
    pub status: PaymentStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub metadata: String,
    pub notes: String,
    pub refunded_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowedPayment {
    pub payment_id: u64,
    pub escrow_id: u64,
    pub escrow_contract: Address,
    pub auto_release_on_complete: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowedPaymentDispute {
    pub payment_id: u64,
    pub raised_by: Address,
    pub reason: String,
    pub raised_at: u64,
    pub resolved: bool,
    pub resolved_at: Option<u64>,
    pub favor_customer: Option<bool>,
}

#[derive(Clone)]
#[contracttype]
pub struct ConditionalPayment {
    pub payment_id: u64,
    pub condition: ConditionType,
    pub condition_met: bool,
    pub evaluated_at: Option<u64>,
}

#[derive(Clone)]
#[contracttype]
pub struct ScheduledPayment {
    pub payment_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub scheduled_at: u64,
    pub executed: bool,
    pub cancelled: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct OracleRateConfig {
    pub oracle_address: Address,
    pub currency: Currency,
    pub price_feed_id: BytesN<32>,
    pub max_staleness_seconds: u64,
    pub enabled: bool,
}

// Dynamic fee calculation structures (#124)
#[derive(Clone)]
#[contracttype]
pub struct RiskFeeConfig {
    pub base_fee_bps: u32,
    pub large_amount_threshold: i128,
    pub large_amount_surcharge_bps: u32,
    pub new_customer_surcharge_bps: u32,
    pub high_risk_currency_surcharge: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct AnalyticsBucket {
    pub bucket_start: u64,
    pub bucket_end: u64,
    pub total_payments: u64,
    pub total_volume: i128,
    pub total_refunds: i128,
    pub failed_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ActionType {
    ReleaseEscrow,
    ResolveDispute,
    CompletePayment,
    RefundPayment,
    AddAdmin,
    RemoveAdmin,
    UpdateRequiredSignatures,
}

#[derive(Clone)]
#[contracttype]
pub struct MultiSigConfig {
    pub admins: Vec<Address>,
    pub required_signatures: u32,
    pub total_admins: u32,
    pub proposal_ttl: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct AdminProposal {
    pub id: String,
    pub proposer: Address,
    pub action_type: ActionType,
    pub target: Address,
    pub data: Bytes,
    pub approvals: Vec<Address>,
    pub approval_count: u32,
    pub executed: bool,
    pub rejected: bool,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct LargePaymentProposal {
    pub payment_id: u64,
    pub approvals: Vec<Address>,
    pub required: u32,
    pub proposed_at: u64,
    pub expires_at: u64,
    pub executed: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchPaymentEntry {
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub currency: Currency,
    pub expiration_duration: u64,
    pub metadata: String,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchResult {
    pub payment_id: u64,
    pub success: bool,
    pub error_code: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum FeeTier {
    Standard,
    Premium,    // >= configured premium volume
    Enterprise, // >= configured enterprise volume
}

#[derive(Clone)]
#[contracttype]
pub struct FeeConfig {
    pub fee_bps: u32,
    pub min_fee: i128,
    pub max_fee: i128,
    pub treasury: Address,
    pub fee_token: Address,
    pub active: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct MerchantFeeRecord {
    pub merchant: Address,
    pub total_fees_paid: i128,
    pub total_volume: i128,
    pub fee_tier: FeeTier,
}

#[derive(Clone)]
#[contracttype]
pub struct FeeWaiver {
    pub merchant: Address,
    pub waiver_bps: u32, // reduction in basis points
    pub valid_until: u64,
    pub reason: String,
    pub granted_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionProposed {
    pub proposal_id: String,
    pub proposer: Address,
    pub action_type: ActionType,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionApproved {
    pub proposal_id: String,
    pub approver: Address,
    pub approval_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionExecuted {
    pub proposal_id: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRejected {
    pub proposal_id: String,
    pub rejected_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAdded {
    pub admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRemoved {
    pub admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    pub payment_id: u64,
    pub fee_amount: i128,
    pub merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeesWithdrawn {
    pub amount: i128,
    pub treasury: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerchantTierUpgraded {
    pub merchant: Address,
    pub old_tier: FeeTier,
    pub new_tier: FeeTier,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeWaiverGranted {
    pub merchant: Address,
    pub waiver_bps: u32,
    pub valid_until: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeWaiverRevoked {
    pub merchant: Address,
    pub revoked_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeWaiverExpired {
    pub merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionEvaluated {
    pub payment_id: u64,
    pub met: bool,
    pub evaluated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalPaymentCreated {
    pub payment_id: u64,
    pub condition_type: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigUpdated {
    pub fee_bps: u32,
    pub treasury: Address,
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

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LargePaymentProposed {
    pub payment_id: u64,
    pub proposer: Address,
    pub required_approvals: u32,
    pub expires_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LargePaymentApproved {
    pub payment_id: u64,
    pub approver: Address,
    pub approval_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LargePaymentExecuted {
    pub payment_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LargePaymentThresholdUpdated {
    pub threshold: i128,
    pub updated_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentMetadataSet {
    pub payment_id: u64,
    pub content_ref: String,
    pub encrypted: bool,
    pub set_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentMetadataUpdated {
    pub payment_id: u64,
    pub content_ref: String,
    pub updated_by: Address,
    pub version: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentMetadata {
    pub payment_id: u64,
    pub content_ref: String,      // IPFS CID or similar
    pub content_hash: BytesN<32>, // SHA-256 of plaintext for verification
    pub encrypted: bool,
    pub updated_at: u64,
    pub version: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentAnalytics {
    pub total_payments_created: u64,
    pub total_payments_completed: u64,
    pub total_payments_cancelled: u64,
    pub total_payments_refunded: u64,
    pub total_volume: i128,
    pub total_refunded_volume: i128,
    pub unique_customers: u64,
    pub unique_merchants: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct MerchantAnalytics {
    pub total_payments: u64,
    pub total_volume: i128,
    pub total_completed: u64,
    pub total_cancelled: u64,
    pub total_refunded: u64,
    pub total_refunded_volume: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct CustomerAnalytics {
    pub total_payments: u64,
    pub total_volume: i128,
    pub total_refunds: i128,
    pub avg_transaction_size: i128,
    pub peak_hour: u32,
    pub top_merchant: Option<Address>,
    pub top_merchant_volume: i128,
    pub first_payment_at: u64,
    pub last_payment_at: u64,
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

#[contract]
pub struct PaymentContract;

// Constants for size limits
const MAX_METADATA_SIZE: u32 = 512;
const MAX_NOTES_SIZE: u32 = 1024;
const DEFAULT_MAX_RETRIES: u64 = 3;
const SECONDS_PER_DAY: u64 = 86400;

// Fee tier volume thresholds (raw token units)
const PREMIUM_VOLUME_THRESHOLD: i128 = 10_000;
const ENTERPRISE_VOLUME_THRESHOLD: i128 = 100_000;

#[contractimpl]
impl PaymentContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::MultiSigConfig) {
            panic!("already initialized");
        }
        let config = MultiSigConfig {
            admins: Vec::from_array(&env, [admin.clone()]),
            required_signatures: 1,
            total_admins: 1,
            proposal_ttl: 604800,
        };
        env.storage()
            .instance()
            .set(&DataKey::MultiSigConfig, &config);
        // Keep Admin key for backward compat
        env.storage().instance().set(&DataKey::Admin, &admin);
        (AdminAdded { admin }).publish(&env);
    }

    pub fn get_multisig_config(env: Env) -> MultiSigConfig {
        env.storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .expect("MultiSig not initialized")
    }

    pub fn propose_action(
        env: Env,
        proposer: Address,
        action_type: ActionType,
        target: Address,
        data: Bytes,
    ) -> Result<String, Error> {
        proposer.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&proposer) {
            return Err(Error::NotAnAdmin);
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCounter)
            .unwrap_or(0)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &counter);

        let proposal_id = PaymentContract::u64_to_string(&env, counter);
        let now = env.ledger().timestamp();

        let mut approvals = Vec::new(&env);
        approvals.push_back(proposer.clone());

        let proposal = AdminProposal {
            id: proposal_id.clone(),
            proposer: proposer.clone(),
            action_type: action_type.clone(),
            target,
            data,
            approvals,
            approval_count: 1,
            executed: false,
            rejected: false,
            created_at: now,
            expires_at: now + config.proposal_ttl,
        };

        env.storage()
            .instance()
            .set(&StateDataKey::AdminProposal(proposal_id.clone()), &proposal);

        (ActionProposed {
            proposal_id: proposal_id.clone(),
            proposer,
            action_type,
        })
        .publish(&env);

        Ok(proposal_id)
    }

    pub fn approve_action(env: Env, approver: Address, proposal_id: String) -> Result<(), Error> {
        approver.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&approver) {
            return Err(Error::NotAnAdmin);
        }

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&StateDataKey::AdminProposal(proposal_id.clone()))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.executed || proposal.rejected {
            return Err(Error::ProposalAlreadyExecuted);
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::ProposalExpired);
        }

        if proposal.approvals.contains(&approver) {
            return Err(Error::AlreadyApproved);
        }

        proposal.approvals.push_back(approver.clone());
        proposal.approval_count += 1;

        env.storage()
            .instance()
            .set(&StateDataKey::AdminProposal(proposal_id.clone()), &proposal);

        (ActionApproved {
            proposal_id,
            approver,
            approval_count: proposal.approval_count,
        })
        .publish(&env);

        Ok(())
    }

    pub fn execute_action(env: Env, proposal_id: String) -> Result<(), Error> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&StateDataKey::AdminProposal(proposal_id.clone()))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.executed || proposal.rejected {
            return Err(Error::ProposalAlreadyExecuted);
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::ProposalExpired);
        }

        if proposal.approval_count < config.required_signatures {
            return Err(Error::MultiSigThresholdNotMet);
        }

        proposal.executed = true;
        env.storage()
            .instance()
            .set(&StateDataKey::AdminProposal(proposal_id.clone()), &proposal);

        PaymentContract::dispatch_action(&env, &proposal)?;

        (ActionExecuted { proposal_id }).publish(&env);

        Ok(())
    }

    pub fn reject_action(env: Env, rejecter: Address, proposal_id: String) -> Result<(), Error> {
        rejecter.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&rejecter) {
            return Err(Error::NotAnAdmin);
        }

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&StateDataKey::AdminProposal(proposal_id.clone()))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.executed || proposal.rejected {
            return Err(Error::ProposalAlreadyExecuted);
        }

        proposal.rejected = true;
        env.storage()
            .instance()
            .set(&StateDataKey::AdminProposal(proposal_id.clone()), &proposal);

        (ActionRejected {
            proposal_id,
            rejected_by: rejecter,
        })
        .publish(&env);

        Ok(())
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&caller) {
            return Err(Error::NotAnAdmin);
        }

        if !config.admins.contains(&new_admin) {
            config.admins.push_back(new_admin.clone());
            config.total_admins += 1;
            env.storage()
                .instance()
                .set(&DataKey::MultiSigConfig, &config);
            (AdminAdded { admin: new_admin }).publish(&env);
        }

        Ok(())
    }

    pub fn remove_admin(env: Env, caller: Address, admin: Address) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&caller) {
            return Err(Error::NotAnAdmin);
        }

        if config.total_admins <= config.required_signatures {
            return Err(Error::InsufficientAdmins);
        }

        let mut new_admins = Vec::new(&env);
        for a in config.admins.iter() {
            if a != admin {
                new_admins.push_back(a);
            }
        }

        if new_admins.len() == config.admins.len() {
            return Err(Error::NotAnAdmin);
        }

        config.admins = new_admins;
        config.total_admins -= 1;
        env.storage()
            .instance()
            .set(&DataKey::MultiSigConfig, &config);
        (AdminRemoved { admin }).publish(&env);

        Ok(())
    }

    pub fn update_required_signatures(
        env: Env,
        caller: Address,
        required: u32,
    ) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&caller) {
            return Err(Error::NotAnAdmin);
        }

        if required == 0 || required > config.total_admins {
            return Err(Error::InsufficientAdmins);
        }

        config.required_signatures = required;
        env.storage()
            .instance()
            .set(&DataKey::MultiSigConfig, &config);

        Ok(())
    }

    pub fn create_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        expiration_duration: u64,
        metadata: String,
    ) -> Result<u64, Error> {
        Self::require_not_paused(&env, "create_payment")?;
        customer.require_auth();
        PaymentContract::do_create_payment(
            &env,
            customer,
            merchant,
            amount,
            token,
            currency,
            expiration_duration,
            metadata,
        )
    }

    pub fn schedule_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        token: Address,
        amount: i128,
        scheduled_at: u64,
    ) -> Result<u64, Error> {
        Self::require_not_paused(&env, "schedule_payment")?;
        customer.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidStatus);
        }
        let now = env.ledger().timestamp();
        if scheduled_at <= now {
            return Err(Error::PaymentNotYetDue);
        }

        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(&contract_address, &customer, &contract_address, &amount);

        let counter: u64 = env
            .storage()
            .instance()
            .get(&StateDataKey::ScheduledPaymentCounter)
            .unwrap_or(0);
        let payment_id = counter + 1;
        let scheduled = ScheduledPayment {
            payment_id,
            customer,
            merchant,
            token,
            amount,
            scheduled_at,
            executed: false,
            cancelled: false,
        };
        env.storage()
            .instance()
            .set(&StateDataKey::ScheduledPayment(payment_id), &scheduled);
        env.storage()
            .instance()
            .set(&StateDataKey::ScheduledPaymentCounter, &payment_id);
        Ok(payment_id)
    }

    pub fn execute_scheduled_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "execute_scheduled_payment")?;
        let mut scheduled: ScheduledPayment = env
            .storage()
            .instance()
            .get(&StateDataKey::ScheduledPayment(payment_id))
            .ok_or(Error::PaymentNotFound)?;
        if scheduled.cancelled {
            return Err(Error::ScheduledPaymentCancelled);
        }
        if scheduled.executed {
            return Err(Error::AlreadyProcessed);
        }
        if env.ledger().timestamp() < scheduled.scheduled_at {
            return Err(Error::PaymentNotYetDue);
        }

        let token_client = token::Client::new(&env, &scheduled.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &scheduled.merchant, &scheduled.amount);
        scheduled.executed = true;
        env.storage()
            .instance()
            .set(&StateDataKey::ScheduledPayment(payment_id), &scheduled);
        Ok(())
    }

    pub fn cancel_scheduled_payment(
        env: Env,
        caller: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "cancel_scheduled_payment")?;
        caller.require_auth();
        let mut scheduled: ScheduledPayment = env
            .storage()
            .instance()
            .get(&StateDataKey::ScheduledPayment(payment_id))
            .ok_or(Error::PaymentNotFound)?;
        if scheduled.executed {
            return Err(Error::AlreadyProcessed);
        }
        if scheduled.cancelled {
            return Err(Error::ScheduledPaymentCancelled);
        }
        if caller != scheduled.customer {
            return Err(Error::Unauthorized);
        }

        let token_client = token::Client::new(&env, &scheduled.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &scheduled.customer, &scheduled.amount);
        scheduled.cancelled = true;
        env.storage()
            .instance()
            .set(&StateDataKey::ScheduledPayment(payment_id), &scheduled);
        Ok(())
    }

    pub fn get_scheduled_payment(env: Env, payment_id: u64) -> Result<ScheduledPayment, Error> {
        env.storage()
            .instance()
            .get(&StateDataKey::ScheduledPayment(payment_id))
            .ok_or(Error::PaymentNotFound)
    }

    fn do_create_payment(
        env: &Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        expiration_duration: u64,
        metadata: String,
    ) -> Result<u64, Error> {
        // Validate currency
        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::InvalidCurrency);
        }

        // Validate metadata size
        if metadata.len() > MAX_METADATA_SIZE {
            return Err(Error::MetadataTooLarge);
        }

        // Enforce sanctions/flag checks at creation entry point.
        if PaymentContract::is_address_flagged(env.clone(), customer.clone())
            && !PaymentContract::is_allowlisted(env, &customer)
        {
            return Err(Error::AddressFlagged);
        }

        // Check rate limits and anti-fraud before processing
        PaymentContract::check_rate_limit(env, &customer, amount)?;

        // Check merchant rate limits
        PaymentContract::check_merchant_rate_limit(env, &merchant, amount)?;

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentCounter)
            .unwrap_or(0);
        let payment_id = counter + 1;

        let current_timestamp = env.ledger().timestamp();
        let expires_at = if expiration_duration > 0 {
            current_timestamp + expiration_duration
        } else {
            0
        };

        let payment = Payment {
            id: payment_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token,
            currency,
            status: PaymentStatus::Pending,
            created_at: current_timestamp,
            expires_at,
            metadata,
            notes: String::from_str(&env, ""),
            refunded_amount: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);
        env.storage()
            .instance()
            .set(&DataKey::PaymentCounter, &payment_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&CustomerDataKey::PaymentCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &CustomerDataKey::Payments(customer.clone(), customer_count),
            &payment_id,
        );
        env.storage().instance().set(
            &CustomerDataKey::PaymentCount(customer),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantPaymentCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantPayments(merchant.clone(), merchant_count),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantPaymentCount(merchant),
            &(merchant_count + 1),
        );

        // Update global analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::PaymentAnalyticsKey)
            .unwrap_or(PaymentAnalytics {
                total_payments_created: 0,
                total_payments_completed: 0,
                total_payments_cancelled: 0,
                total_payments_refunded: 0,
                total_volume: 0,
                total_refunded_volume: 0,
                unique_customers: 0,
                unique_merchants: 0,
            });
        analytics.total_payments_created += 1;
        analytics.total_volume += amount;
        if customer_count == 0 {
            analytics.unique_customers += 1;
        }
        if merchant_count == 0 {
            analytics.unique_merchants += 1;
            let global_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::GlobalMerchantCount)
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::GlobalMerchantList(global_count),
                &payment.merchant.clone(),
            );
            env.storage()
                .instance()
                .set(&DataKey::GlobalMerchantCount, &(global_count + 1));
        }
        env.storage()
            .instance()
            .set(&DataKey::PaymentAnalyticsKey, &analytics);

        // Update merchant analytics
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(payment.merchant.clone()))
            .unwrap_or(MerchantAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_completed: 0,
                total_cancelled: 0,
                total_refunded: 0,
                total_refunded_volume: 0,
            });
        m_analytics.total_payments += 1;
        m_analytics.total_volume += amount;
        env.storage().instance().set(
            &DataKey::MerchantAnalytics(payment.merchant.clone()),
            &m_analytics,
        );

        // Update customer analytics
        let mut c_analytics: CustomerAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::CustomerAnalytics(payment.customer.clone()))
            .unwrap_or(CustomerAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_refunds: 0,
                avg_transaction_size: 0,
                peak_hour: 0,
                top_merchant: None,
                top_merchant_volume: 0,
                first_payment_at: 0,
                last_payment_at: 0,
            });
        c_analytics.total_payments += 1;
        c_analytics.total_volume += amount;
        c_analytics.avg_transaction_size =
            c_analytics.total_volume / (c_analytics.total_payments as i128);
        if c_analytics.first_payment_at == 0 {
            c_analytics.first_payment_at = current_timestamp;
        }
        c_analytics.last_payment_at = current_timestamp;

        // Track peak hour (UTC hour 0-23)
        let hour = ((current_timestamp / 3600) % 24) as u32;
        let hour_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerHourCount(payment.customer.clone(), hour))
            .unwrap_or(0)
            + 1;
        env.storage().instance().set(
            &DataKey::CustomerHourCount(payment.customer.clone(), hour),
            &hour_count,
        );
        let peak_hour_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerHourCount(
                payment.customer.clone(),
                c_analytics.peak_hour,
            ))
            .unwrap_or(0);
        if hour_count > peak_hour_count
            || (hour_count == peak_hour_count && hour == c_analytics.peak_hour)
        {
            c_analytics.peak_hour = hour;
        }

        // Track per-merchant volume and update top merchant
        let prev_merchant_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerMerchantVolume(
                payment.customer.clone(),
                payment.merchant.clone(),
            ))
            .unwrap_or(0);
        if prev_merchant_vol == 0 {
            // New merchant for this customer — add to list
            let m_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::CustomerMerchantCount(payment.customer.clone()))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::CustomerMerchantList(payment.customer.clone(), m_count),
                &payment.merchant,
            );
            env.storage().instance().set(
                &DataKey::CustomerMerchantCount(payment.customer.clone()),
                &(m_count + 1),
            );
        }
        let new_merchant_vol = prev_merchant_vol + amount;
        env.storage().instance().set(
            &DataKey::CustomerMerchantVolume(payment.customer.clone(), payment.merchant.clone()),
            &new_merchant_vol,
        );
        if new_merchant_vol > c_analytics.top_merchant_volume {
            c_analytics.top_merchant = Some(payment.merchant.clone());
            c_analytics.top_merchant_volume = new_merchant_vol;
        }

        // Track monthly volume (30-day bucket)
        let month_bucket = (current_timestamp / 2_592_000) * 2_592_000;
        let prev_monthly: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerMonthlyVolume(
                payment.customer.clone(),
                month_bucket,
            ))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerMonthlyVolume(payment.customer.clone(), month_bucket),
            &(prev_monthly + amount),
        );

        env.storage().instance().set(
            &DataKey::CustomerAnalytics(payment.customer.clone()),
            &c_analytics,
        );

        PaymentContract::update_merchant_bucket(
            env,
            payment.merchant.clone(),
            current_timestamp,
            1,
            amount,
            0,
            0,
        );
        PaymentContract::update_platform_daily_bucket(env, current_timestamp, amount, 0, 0);

        (PaymentCreated {
            payment_id,
            customer: payment.customer,
            merchant: payment.merchant.clone(),
            amount: payment.amount,
        })
        .publish(&env);

        Ok(payment_id)
    }

    pub fn get_payment(env: &Env, payment_id: u64) -> Payment {
        env.storage()
            .instance()
            .get(&DataKey::Payment(payment_id))
            .expect("Payment not found")
    }

    /// Used by the refund contract for cross-contract ownership verification (#143).
    /// Returns true if the payment exists, belongs to `customer`, and is Completed.
    pub fn check_payment_customer(env: Env, payment_id: u64, customer: Address) -> bool {
        let payment: Option<Payment> = env.storage().instance().get(&DataKey::Payment(payment_id));
        match payment {
            Some(p) => p.customer == customer && p.status == PaymentStatus::Completed,
            None => false,
        }
    }

    pub fn create_escrowed_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        escrow_contract: Address,
        release_timestamp: u64,
        min_hold_period: u64,
        metadata: String,
        auto_release_on_complete: bool,
    ) -> Result<(u64, u64), Error> {
        let payment_id = PaymentContract::create_payment(
            env.clone(),
            customer.clone(),
            merchant.clone(),
            amount,
            token.clone(),
            currency,
            0,
            metadata,
        )?;

        let escrow_id = PaymentContract::invoke_escrow_create(
            &env,
            &escrow_contract,
            &customer,
            &merchant,
            amount,
            &token,
            release_timestamp,
            min_hold_period,
        )?;

        let bridge = EscrowedPayment {
            payment_id,
            escrow_id,
            escrow_contract: escrow_contract.clone(),
            auto_release_on_complete,
        };
        env.storage()
            .instance()
            .set(&StateDataKey::EscrowedPayment(payment_id), &bridge);

        // Custody is shifted to escrow contract account on creation.
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(&contract_address, &customer, &escrow_contract, &amount);

        (EscrowedPaymentCreated {
            payment_id,
            escrow_id,
            escrow_contract,
        })
        .publish(&env);

        Ok((payment_id, escrow_id))
    }

    pub fn complete_escrowed_payment(
        env: Env,
        admin: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let mut payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }
        PaymentContract::require_no_unresolved_escrowed_payment_dispute(&env, payment_id)?;

        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_release_escrow(&admin, &bridge.escrow_id, &bridge.auto_release_on_complete)
            .is_err()
        {
            return Err(Error::EscrowBridgeFailed);
        }

        payment.status = PaymentStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        (EscrowedPaymentCompleted {
            payment_id,
            escrow_id: bridge.escrow_id,
        })
        .publish(&env);
        Ok(())
    }

    pub fn cancel_escrowed_payment(
        env: Env,
        caller: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let mut payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }
        if payment.customer != caller && payment.merchant != caller {
            return Err(Error::Unauthorized);
        }
        PaymentContract::require_no_unresolved_escrowed_payment_dispute(&env, payment_id)?;

        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_refund_escrow(&caller, &bridge.escrow_id)
            .is_err()
        {
            return Err(Error::EscrowBridgeFailed);
        }

        payment.status = PaymentStatus::Cancelled;
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        (EscrowedPaymentCancelled {
            payment_id,
            escrow_id: bridge.escrow_id,
        })
        .publish(&env);
        Ok(())
    }

    pub fn dispute_escrowed_payment(
        env: Env,
        caller: Address,
        payment_id: u64,
        reason: String,
    ) -> Result<(), Error> {
        caller.require_auth();
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }
        if payment.customer != caller && payment.merchant != caller {
            return Err(Error::Unauthorized);
        }
        if let Some(dispute) =
            PaymentContract::get_escrowed_payment_dispute(env.clone(), payment_id)
        {
            if !dispute.resolved {
                return Err(Error::AlreadyProcessed);
            }
        }

        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_dispute_escrow(&caller, &bridge.escrow_id)
            .is_err()
        {
            return Err(Error::EscrowBridgeFailed);
        }

        let dispute = EscrowedPaymentDispute {
            payment_id,
            raised_by: caller.clone(),
            reason,
            raised_at: env.ledger().timestamp(),
            resolved: false,
            resolved_at: None,
            favor_customer: None,
        };
        env.storage()
            .instance()
            .set(&DataKey::EscrowedPaymentDispute(payment_id), &dispute);

        (EscrowedPaymentDisputed {
            payment_id,
            raised_by: caller,
        })
        .publish(&env);
        Ok(())
    }

    pub fn resolve_escrowed_payment_dispute(
        env: Env,
        admin: Address,
        payment_id: u64,
        favor_customer: bool,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let mut dispute = PaymentContract::get_escrowed_payment_dispute(env.clone(), payment_id)
            .ok_or(Error::InvalidStatus)?;
        if dispute.resolved {
            return Err(Error::AlreadyProcessed);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        let release_to_merchant = !favor_customer;
        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_resolve_dispute(&admin, &bridge.escrow_id, &release_to_merchant)
            .is_err()
        {
            return Err(Error::EscrowBridgeFailed);
        }

        if favor_customer {
            payment.status = PaymentStatus::Refunded;
            payment.refunded_amount = payment.amount;
        } else {
            payment.status = PaymentStatus::Completed;
        }
        dispute.resolved = true;
        dispute.resolved_at = Some(env.ledger().timestamp());
        dispute.favor_customer = Some(favor_customer);

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);
        env.storage()
            .instance()
            .set(&DataKey::EscrowedPaymentDispute(payment_id), &dispute);

        (EscrowedPaymentDisputeResolved {
            payment_id,
            favor_customer,
        })
        .publish(&env);
        Ok(())
    }

    pub fn get_escrowed_payment(env: Env, payment_id: u64) -> Result<EscrowedPayment, Error> {
        env.storage()
            .instance()
            .get(&StateDataKey::EscrowedPayment(payment_id))
            .ok_or(Error::EscrowMappingNotFound)
    }

    pub fn get_escrowed_payment_dispute(
        env: Env,
        payment_id: u64,
    ) -> Option<EscrowedPaymentDispute> {
        env.storage()
            .instance()
            .get(&DataKey::EscrowedPaymentDispute(payment_id))
    }

    fn require_no_unresolved_escrowed_payment_dispute(
        env: &Env,
        payment_id: u64,
    ) -> Result<(), Error> {
        if let Some(dispute) = env
            .storage()
            .instance()
            .get::<DataKey, EscrowedPaymentDispute>(&DataKey::EscrowedPaymentDispute(payment_id))
        {
            if !dispute.resolved {
                return Err(Error::InvalidStatus);
            }
        }
        Ok(())
    }

    pub fn update_payment_notes(
        env: Env,
        merchant: Address,
        payment_id: u64,
        notes: String,
    ) -> Result<(), Error> {
        merchant.require_auth();

        // Validate notes size
        if notes.len() > MAX_NOTES_SIZE {
            return Err(Error::NotesTooLarge);
        }

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Verify caller is the merchant
        if payment.merchant != merchant {
            return Err(Error::Unauthorized);
        }

        // Update notes
        payment.notes = notes;

        // Save updated payment
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        Ok(())
    }

    pub fn is_payment_expired(env: &Env, payment_id: u64) -> bool {
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return false;
        }
        let payment = PaymentContract::get_payment(env, payment_id);
        payment.expires_at > 0 && env.ledger().timestamp() > payment.expires_at
    }

    pub fn expire_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        // Retrieve payment from storage
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }
        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Check payment status allows expiry (only allow Pending)
        match payment.status {
            PaymentStatus::Pending => {
                // Allow expiry
            }
            PaymentStatus::Refunded | PaymentStatus::PartialRefunded | PaymentStatus::Cancelled => {
                return Err(Error::InvalidStatus);
            }
            PaymentStatus::Completed => {
                return Err(Error::InvalidStatus);
            }
        }

        // Check payment has expiration set
        if payment.expires_at == 0 {
            return Err(Error::NoExpiration);
        }

        // Check current time is past expires_at
        if env.ledger().timestamp() <= payment.expires_at {
            return Err(Error::NotExpired);
        }

        // Transfer token amount back to customer (automatic refund)
        let token_client = token::Client::new(&env, &payment.token);
        let contract_address = env.current_contract_address();
        
        // Check if contract has sufficient balance (in case of partial payments)
        let contract_balance = token_client.balance(&contract_address);
        let refund_amount = if contract_balance >= payment.amount {
            payment.amount
        } else {
            contract_balance
        };

        if refund_amount > 0 {
            token_client.transfer(&contract_address, &payment.customer, &refund_amount);
        }

        // Update payment status to Cancelled
        payment.status = PaymentStatus::Cancelled;

        // Store updated payment back to storage
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        // Emit PaymentExpired event with refund info
        (PaymentExpired {
            payment_id,
            customer: payment.customer,
            refunded_amount: refund_amount,
            expired_at: env.ledger().timestamp(),
        })
        .publish(&env);

        Ok(())
    }

    pub fn complete_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "complete_payment")?;
        admin.require_auth();

        // Verify caller is in the multisig admin list
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        // Check if payment requires multi-sig approval
        let payment = PaymentContract::get_payment(&env, payment_id);
        let threshold = PaymentContract::get_large_payment_threshold(env.clone());

        if threshold > 0 && payment.amount > threshold {
            // Check if there's already a proposal for this payment
            if env
                .storage()
                .instance()
                .get::<DataKey, LargePaymentProposal>(&StateDataKey::LargePaymentProposal(payment_id))
                .is_some()
            {
                return Err(Error::PaymentRequiresMultiSig);
            }

            // Auto-create proposal for large payment
            let now = env.ledger().timestamp();
            let expires_at = now + config.proposal_ttl;

            let mut approvals = Vec::new(&env);
            approvals.push_back(admin.clone());

            let proposal = LargePaymentProposal {
                payment_id,
                approvals,
                required: config.required_signatures,
                proposed_at: now,
                expires_at,
                executed: false,
            };

            env.storage()
                .instance()
                .set(&StateDataKey::LargePaymentProposal(payment_id), &proposal);

            (LargePaymentProposed {
                payment_id,
                proposer: admin,
                required_approvals: config.required_signatures,
                expires_at,
            })
            .publish(&env);

            return Err(Error::PaymentRequiresMultiSig);
        }

        PaymentContract::do_complete_payment(&env, payment_id)
    }

    fn do_complete_payment(env: &Env, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(env, payment_id);

        // Before updating status, check if payment is expired
        if PaymentContract::is_payment_expired(env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Completed;
            }
            PaymentStatus::Completed => {
                return Err(Error::AlreadyProcessed);
            }
            PaymentStatus::Refunded | PaymentStatus::PartialRefunded => {
                return Err(Error::InvalidStatus);
            }
            PaymentStatus::Cancelled => {
                return Err(Error::InvalidStatus);
            }
        }

        // Deduct platform fee (if configured) and get net amount for merchant
        let (net_amount, fee_amount) = PaymentContract::deduct_fee(
            env,
            payment_id,
            payment.amount,
            payment.merchant.clone(),
            &payment.token,
            &payment.customer,
        );

        // Token transfer: net amount from customer to merchant
        let token_client = token::Client::new(env, &payment.token);
        let contract_address = env.current_contract_address();

        token_client.transfer_from(
            &contract_address,
            &payment.customer,
            &payment.merchant,
            &net_amount,
        );

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);
        PaymentContract::update_merchant_fee_record_post_completion(
            env,
            payment.merchant.clone(),
            payment.amount,
            fee_amount,
        );

        // Update analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::PaymentAnalyticsKey)
            .unwrap_or(PaymentAnalytics {
                total_payments_created: 0,
                total_payments_completed: 0,
                total_payments_cancelled: 0,
                total_payments_refunded: 0,
                total_volume: 0,
                total_refunded_volume: 0,
                unique_customers: 0,
                unique_merchants: 0,
            });
        analytics.total_payments_completed += 1;
        env.storage()
            .instance()
            .set(&DataKey::PaymentAnalyticsKey, &analytics);
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(payment.merchant.clone()))
            .unwrap_or(MerchantAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_completed: 0,
                total_cancelled: 0,
                total_refunded: 0,
                total_refunded_volume: 0,
            });
        m_analytics.total_completed += 1;
        env.storage().instance().set(
            &DataKey::MerchantAnalytics(payment.merchant.clone()),
            &m_analytics,
        );
        (PaymentCompleted {
            payment_id,
            merchant: payment.merchant.clone(),
            amount: payment.amount,
        })
        .publish(env);

        let now = env.ledger().timestamp();
        PaymentContract::update_merchant_bucket(env, payment.merchant.clone(), now, 0, 0, 0, 0);
        PaymentContract::update_platform_daily_bucket(env, now, 0, 0, 0);

        Ok(())
    }

    // Partial payment functions (#112)
    pub fn pay_installment(env: Env, customer: Address, payment_id: u64, amount: i128) -> Result<(), Error> {
        Self::require_not_paused(&env, "pay_installment")?;
        customer.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Check if payment is expired
        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        // Only allow installments on Pending payments
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        // Get current outstanding balance
        let outstanding_balance = PaymentContract::get_outstanding_balance(env.clone(), payment_id);
        
        if outstanding_balance <= 0 {
            return Err(Error::PaymentAlreadyFullyPaid);
        }

        if amount > outstanding_balance {
            return Err(Error::InstallmentExceedsRemaining);
        }

        // Get current installment counter
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PartialPaymentCounter(payment_id))
            .unwrap_or(0);
        let new_installment_number = installment_counter + 1;

        // Transfer tokens from customer to contract
        let token_client = token::Client::new(&env, &payment.token);
        token_client.transfer(&customer, &env.current_contract_address(), &amount);

        // Create partial payment record
        let remaining = outstanding_balance - amount;
        let partial_payment = PartialPaymentRecord {
            payment_id,
            installment_number: new_installment_number,
            amount_paid: amount,
            total_amount: payment.amount,
            remaining,
            paid_at: env.ledger().timestamp(),
            payer: customer.clone(),
        };

        // Store partial payment record
        env.storage()
            .instance()
            .set(&StateDataKey::PartialPaymentRecord(payment_id, new_installment_number), &partial_payment);
        
        // Update installment counter
        env.storage()
            .instance()
            .set(&DataKey::PartialPaymentCounter(payment_id), &new_installment_number);

        // Update outstanding balance
        env.storage()
            .instance()
            .set(&DataKey::OutstandingBalance(payment_id), &remaining);

        // Emit installment paid event
        (InstallmentPaid {
            payment_id,
            installment_number: new_installment_number,
            amount,
            remaining,
            payer: customer,
            paid_at: partial_payment.paid_at,
        })
        .publish(&env);

        // Check if payment is now fully paid
        if remaining == 0 {
            PaymentContract::finalize_installment_payment(env.clone(), payment_id)?;
        }

        Ok(())
    }

    pub fn get_installment_history(env: Env, payment_id: u64) -> Vec<PartialPaymentRecord> {
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PartialPaymentCounter(payment_id))
            .unwrap_or(0);

        let mut history = Vec::new(&env);
        for i in 1..=installment_counter {
            if let Some(record) = env
                .storage()
                .instance()
                .get(&StateDataKey::PartialPaymentRecord(payment_id, i))
            {
                history.push_back(record);
            }
        }
        history
    }

    pub fn get_outstanding_balance(env: Env, payment_id: u64) -> i128 {
        // First check if we have an outstanding balance stored
        if let Some(balance) = env.storage().instance().get(&DataKey::OutstandingBalance(payment_id)) {
            return balance;
        }

        // If not, calculate from payment amount and partial payments
        let payment = PaymentContract::get_payment(&env, payment_id);
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PartialPaymentCounter(payment_id))
            .unwrap_or(0);

        let mut total_paid = 0i128;
        for i in 1..=installment_counter {
            if let Some(record) = env
                .storage()
                .instance()
                .get(&StateDataKey::PartialPaymentRecord(payment_id, i))
            {
                total_paid += record.amount_paid;
            }
        }

        let outstanding = payment.amount - total_paid;
        
        // Cache the calculated balance
        env.storage()
            .instance()
            .set(&DataKey::OutstandingBalance(payment_id), &outstanding);

        outstanding
    }

    pub fn finalize_installment_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        let mut payment = PaymentContract::get_payment(&env, payment_id);
        
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        let outstanding_balance = PaymentContract::get_outstanding_balance(env.clone(), payment_id);
        if outstanding_balance != 0 {
            return Err(Error::InvalidStatus);
        }

        // Get installment counter for event
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::PartialPaymentCounter(payment_id))
            .unwrap_or(0);

        // Update payment status to Completed
        payment.status = PaymentStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        // Transfer all collected funds to merchant
        let token_client = token::Client::new(&env, &payment.token);
        let contract_address = env.current_contract_address();
        
        // Calculate total amount held by contract (should equal payment amount)
        token_client.transfer(&contract_address, &payment.merchant, &payment.amount);

        // Emit payment fully paid event
        (PaymentFullyPaid {
            payment_id,
            total_installments: installment_counter,
            completed_at: env.ledger().timestamp(),
        })
        .publish(&env);

        // Also emit standard payment completed event for compatibility
        (PaymentCompleted {
            payment_id,
            merchant: payment.merchant,
            amount: payment.amount,
        })
        .publish(&env);

        Ok(())
    }

    pub fn refund_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "refund_payment")?;
        admin.require_auth();

        // Verify caller is in the multisig admin list
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        PaymentContract::do_refund_payment(&env, payment_id)
    }

    fn do_refund_payment(env: &Env, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(env, payment_id);

        // Before updating status, check if payment is expired
        if PaymentContract::is_payment_expired(env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Refunded;
            }
            PaymentStatus::Completed | PaymentStatus::PartialRefunded => {
                return Err(Error::InvalidStatus);
            }
            PaymentStatus::Refunded => {
                return Err(Error::AlreadyProcessed);
            }
            PaymentStatus::Cancelled => {
                return Err(Error::InvalidStatus);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        // Update analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::PaymentAnalyticsKey)
            .unwrap_or(PaymentAnalytics {
                total_payments_created: 0,
                total_payments_completed: 0,
                total_payments_cancelled: 0,
                total_payments_refunded: 0,
                total_volume: 0,
                total_refunded_volume: 0,
                unique_customers: 0,
                unique_merchants: 0,
            });
        analytics.total_payments_refunded += 1;
        analytics.total_refunded_volume += payment.amount;
        env.storage()
            .instance()
            .set(&DataKey::PaymentAnalyticsKey, &analytics);
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(payment.merchant.clone()))
            .unwrap_or(MerchantAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_completed: 0,
                total_cancelled: 0,
                total_refunded: 0,
                total_refunded_volume: 0,
            });
        m_analytics.total_refunded += 1;
        m_analytics.total_refunded_volume += payment.amount;
        env.storage().instance().set(
            &DataKey::MerchantAnalytics(payment.merchant.clone()),
            &m_analytics,
        );
        let mut c_analytics: CustomerAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::CustomerAnalytics(payment.customer.clone()))
            .unwrap_or(PaymentContract::default_customer_analytics());
        c_analytics.total_refunds += payment.amount;
        env.storage().instance().set(
            &DataKey::CustomerAnalytics(payment.customer.clone()),
            &c_analytics,
        );

        (PaymentRefunded {
            payment_id,
            customer: payment.customer,
            amount: payment.amount,
        })
        .publish(env);

        let now = env.ledger().timestamp();
        PaymentContract::update_merchant_bucket(
            env,
            payment.merchant.clone(),
            now,
            0,
            0,
            payment.amount,
            0,
        );
        PaymentContract::update_platform_daily_bucket(env, now, 0, payment.amount, 0);

        Ok(())
    }

    pub fn partial_refund(
        env: Env,
        admin: Address,
        payment_id: u64,
        refund_amount: i128,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        match payment.status {
            PaymentStatus::Pending | PaymentStatus::PartialRefunded => {
                let new_refunded = payment.refunded_amount + refund_amount;
                if new_refunded > payment.amount {
                    return Err(Error::RefundExceedsPayment);
                }
                payment.refunded_amount = new_refunded;
                payment.status = if new_refunded == payment.amount {
                    PaymentStatus::Refunded
                } else {
                    PaymentStatus::PartialRefunded
                };
            }
            _ => {
                return Err(Error::InvalidStatus);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        (PaymentRefunded {
            payment_id,
            customer: payment.customer,
            amount: refund_amount,
        })
        .publish(&env);

        Ok(())
    }

    pub fn cancel_payment(env: Env, caller: Address, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "cancel_payment")?;
        caller.require_auth();
        PaymentContract::do_cancel_payment(&env, caller, payment_id)
    }

    fn do_cancel_payment(env: &Env, caller: Address, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(env, payment_id);

        // Check authorization: caller must be customer, merchant, or admin
        let is_authorized = payment.customer == caller || payment.merchant == caller;
        if !is_authorized {
            return Err(Error::Unauthorized);
        }

        // Check payment status is Pending
        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Cancelled;
            }
            PaymentStatus::Completed
            | PaymentStatus::Refunded
            | PaymentStatus::PartialRefunded
            | PaymentStatus::Cancelled => {
                return Err(Error::InvalidStatus);
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        // Update analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::PaymentAnalyticsKey)
            .unwrap_or(PaymentAnalytics {
                total_payments_created: 0,
                total_payments_completed: 0,
                total_payments_cancelled: 0,
                total_payments_refunded: 0,
                total_volume: 0,
                total_refunded_volume: 0,
                unique_customers: 0,
                unique_merchants: 0,
            });
        analytics.total_payments_cancelled += 1;
        env.storage()
            .instance()
            .set(&DataKey::PaymentAnalyticsKey, &analytics);
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(payment.merchant.clone()))
            .unwrap_or(MerchantAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_completed: 0,
                total_cancelled: 0,
                total_refunded: 0,
                total_refunded_volume: 0,
            });
        m_analytics.total_cancelled += 1;
        env.storage().instance().set(
            &DataKey::MerchantAnalytics(payment.merchant.clone()),
            &m_analytics,
        );
        let timestamp = env.ledger().timestamp();
        (PaymentCancelled {
            payment_id,
            cancelled_by: caller,
            timestamp,
        })
        .publish(env);

        PaymentContract::update_merchant_bucket(
            env,
            payment.merchant.clone(),
            timestamp,
            0,
            0,
            0,
            1,
        );
        PaymentContract::update_platform_daily_bucket(env, timestamp, 0, 0, 1);

        Ok(())
    }

    pub fn get_payments_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Payment> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&CustomerDataKey::PaymentCount(customer.clone()))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&CustomerDataKey::Payments(customer.clone(), i))
            {
                if let Some(payment) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Payment>(&DataKey::Payment(payment_id))
                {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    pub fn get_payment_count_by_customer(env: Env, customer: Address) -> u64 {
        env.storage()
            .instance()
            .get(&CustomerDataKey::PaymentCount(customer))
            .unwrap_or(0)
    }

    pub fn get_payments_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Payment> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantPaymentCount(merchant.clone()))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::MerchantPayments(merchant.clone(), i))
            {
                if let Some(payment) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Payment>(&DataKey::Payment(payment_id))
                {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    pub fn get_payment_count_by_merchant(env: Env, merchant: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MerchantPaymentCount(merchant))
            .unwrap_or(0)
    }

    fn is_valid_currency(currency: &Currency) -> bool {
        matches!(
            currency,
            Currency::XLM | Currency::USDC | Currency::USDT | Currency::BTC | Currency::ETH
        )
    }

    pub fn set_conversion_rate(
        env: Env,
        admin: Address,
        currency: Currency,
        rate: i128,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::InvalidCurrency);
        }

        env.storage()
            .instance()
            .set(&DataKey::ConversionRate(currency), &rate);

        Ok(())
    }

    pub fn get_conversion_rate(env: Env, currency: Currency) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::ConversionRate(currency))
            .unwrap_or(1_0000000)
    }

    pub fn set_oracle_rate_config(
        env: Env,
        admin: Address,
        config: OracleRateConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !multisig.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::OracleRateConfig(config.currency.clone()), &config);
        Ok(())
    }

    pub fn get_oracle_rate_config(env: Env, currency: Currency) -> Option<OracleRateConfig> {
        env.storage()
            .instance()
            .get(&DataKey::OracleRateConfig(currency))
    }

    pub fn refresh_conversion_rate(env: Env, currency: Currency) -> Result<i128, Error> {
        let cfg: OracleRateConfig = env
            .storage()
            .instance()
            .get(&DataKey::OracleRateConfig(currency.clone()))
            .ok_or(Error::OracleNotConfigured)?;

        if !cfg.enabled {
            return Ok(PaymentContract::get_conversion_rate(env, currency));
        }

        let args = (cfg.price_feed_id.clone(),).into_val(&env);
        let fetched = env
            .try_invoke_contract::<(i128, u64), Error>(
                &cfg.oracle_address,
                &Symbol::new(&env, "get_price"),
                args,
            )
            .map_err(|_| Error::OracleCallFailed)?
            .map_err(|_| Error::OracleCallFailed)?;

        let now = env.ledger().timestamp();
        if now.saturating_sub(fetched.1) > cfg.max_staleness_seconds {
            return Err(Error::OracleFeedStale);
        }

        env.storage()
            .instance()
            .set(&DataKey::ConversionRate(currency), &fetched.0);
        Ok(fetched.0)
    }

    // ── RECURRING / SUBSCRIPTION METHODS ────────────────────────────────────

    /// Create a new subscription. The customer authorises the creation.
    /// `interval`             – seconds between each automatic payment
    /// `duration`             – total lifetime in seconds (0 = indefinite)
    /// `max_retries`          – how many times to retry a failed cycle (0 uses DEFAULT)
    /// `trial_period_seconds` – free trial duration in seconds (0 = no trial)
    pub fn create_subscription(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        interval: u64,
        duration: u64,
        max_retries: u64,
        metadata: String,
        trial_period_seconds: u64,
    ) -> Result<u64, Error> {
        customer.require_auth();

        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::InvalidCurrency);
        }
        if metadata.len() > MAX_METADATA_SIZE {
            return Err(Error::MetadataTooLarge);
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SubscriptionCounter)
            .unwrap_or(0);
        let sub_id = counter + 1;

        let now = env.ledger().timestamp();
        let ends_at = if duration > 0 { now + duration } else { 0 };
        let retries = if max_retries == 0 {
            DEFAULT_MAX_RETRIES
        } else {
            max_retries
        };

        let trial_ends_at = if trial_period_seconds > 0 {
            now + trial_period_seconds
        } else {
            0
        };

        let sub = Subscription {
            id: sub_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token,
            currency,
            interval,
            duration,
            status: SubscriptionStatus::Active,
            created_at: now,
            next_payment_at: now + interval,
            ends_at,
            payment_count: 0,
            retry_count: 0,
            max_retries: retries,
            metadata,
            trial_data: SubscriptionTrialData {
                period_seconds: trial_period_seconds,
                ends_at: trial_ends_at,
                converted: false,
            },
            pause_data: SubscriptionPauseData {
                last_paused_at: 0,
                total_pause_duration: 0,
                proration_enabled: false,
            },
        };

        env.storage()
            .instance()
            .set(&DataKey::Subscription(sub_id), &sub);
        env.storage()
            .instance()
            .set(&DataKey::SubscriptionCounter, &sub_id);

        // Index by customer
        let c_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerSubscriptionCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerSubscriptions(customer.clone(), c_count),
            &sub_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerSubscriptionCount(customer),
            &(c_count + 1),
        );

        // Index by merchant
        let m_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantSubscriptionCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantSubscriptions(merchant.clone(), m_count),
            &sub_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantSubscriptionCount(merchant.clone()),
            &(m_count + 1),
        );

        (SubscriptionCreated {
            subscription_id: sub_id,
            customer: sub.customer.clone(),
            merchant: sub.merchant.clone(),
            amount: sub.amount,
            interval: sub.interval,
        })
        .publish(&env);

        // Emit TrialStarted if trial is active
        if sub.trial_data.ends_at > 0 {
            (TrialStarted {
                subscription_id: sub_id,
                trial_ends_at: sub.trial_data.ends_at,
            })
            .publish(&env);
        }

        Ok(sub_id)
    }

    /// Execute the next recurring payment for a subscription.
    /// Anyone (typically an off-chain keeper / cron) may call this once the
    /// payment is due. It handles retry logic internally.
    pub fn execute_recurring_payment(env: Env, subscription_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        let now = env.ledger().timestamp();

        // InDunning path: enforce on-chain backoff before retrying
        if sub.status == SubscriptionStatus::InDunning {
            let mut dunning: DunningState = env
                .storage()
                .instance()
                .get(&StateDataKey::DunningState(subscription_id))
                .ok_or(Error::DunningNotFound)?;

            if now < dunning.next_retry_at {
                return Err(Error::RetryTooEarly);
            }

            let token_client = token::Client::new(&env, &sub.token);
            let contract_address = env.current_contract_address();
            let transfer_ok = token_client
                .try_transfer_from(&contract_address, &sub.customer, &sub.merchant, &sub.amount)
                .is_ok();

            if transfer_ok {
                sub.payment_count += 1;
                sub.retry_count = 0;
                sub.next_payment_at = now + sub.interval;
                sub.status = SubscriptionStatus::Active;

                if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                    sub.status = SubscriptionStatus::Expired;
                }

                env.storage()
                    .instance()
                    .set(&DataKey::Subscription(subscription_id), &sub);
                env.storage()
                    .instance()
                    .remove(&StateDataKey::DunningState(subscription_id));

                (DunningResolved {
                    subscription_id,
                    resolved_at: now,
                })
                .publish(&env);

                (RecurringPaymentExecuted {
                    subscription_id,
                    payment_count: sub.payment_count,
                    amount: sub.amount,
                    next_payment_at: sub.next_payment_at,
                })
                .publish(&env);
            } else {
                dunning.retry_count += 1;
                dunning.last_failed_at = now;

                if dunning.retry_count >= dunning.max_retries {
                    sub.status = SubscriptionStatus::Suspended;
                    env.storage()
                        .instance()
                        .set(&DataKey::Subscription(subscription_id), &sub);
                    env.storage()
                        .instance()
                        .set(&StateDataKey::DunningState(subscription_id), &dunning);

                    (SubscriptionSuspended {
                        subscription_id,
                        reason: String::from_str(&env, "Maximum retries exceeded"),
                    })
                    .publish(&env);

                    return Err(Error::MaxRetriesExceeded);
                }

                // Exponential backoff: backoff_seconds * 2^retry_count
                dunning.next_retry_at = now + (dunning.backoff_seconds << dunning.retry_count);
                env.storage()
                    .instance()
                    .set(&StateDataKey::DunningState(subscription_id), &dunning);

                (RecurringPaymentFailed {
                    subscription_id,
                    retry_count: dunning.retry_count as u64,
                })
                .publish(&env);

                return Err(Error::TransferFailed);
            }

            return Ok(());
        }

        // Must be Active for the normal payment path
        if sub.status != SubscriptionStatus::Active {
            return Err(Error::SubscriptionNotActive);
        }

        // Check subscription has not ended
        if sub.ends_at > 0 && now >= sub.ends_at {
            sub.status = SubscriptionStatus::Expired;
            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
            return Err(Error::SubscriptionEnded);
        }

        // Check payment is due
        if now < sub.next_payment_at {
            return Err(Error::PaymentNotDue);
        }

        // Skip charge if still within trial period
        if sub.trial_data.ends_at > 0 && now < sub.trial_data.ends_at {
            sub.next_payment_at = now + sub.interval;
            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
            return Ok(());
        }

        // Attempt token transfer
        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();

        let transfer_ok = token_client
            .try_transfer_from(&contract_address, &sub.customer, &sub.merchant, &sub.amount)
            .is_ok();

        if transfer_ok {
            // Mark converted on first post-trial charge
            if sub.trial_data.ends_at > 0 && !sub.trial_data.converted {
                sub.trial_data.converted = true;
                (TrialConverted {
                    subscription_id,
                    converted_at: now,
                })
                .publish(&env);
            }

            sub.payment_count += 1;
            sub.retry_count = 0;
            sub.next_payment_at = now + sub.interval;

            // Auto-expire when duration is reached
            if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                sub.status = SubscriptionStatus::Expired;
            }

            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);

            (RecurringPaymentExecuted {
                subscription_id,
                payment_count: sub.payment_count,
                amount: sub.amount,
                next_payment_at: sub.next_payment_at,
            })
            .publish(&env);
        } else {
            // Failed payment — enter dunning instead of immediate cancellation
            PaymentContract::enter_dunning(
                &env,
                subscription_id,
                String::from_str(&env, "Payment transfer failed"),
            );

            (RecurringPaymentFailed {
                subscription_id,
                retry_count: sub.retry_count + 1,
            })
            .publish(&env);

            return Err(Error::TransferFailed);
        }

        Ok(())
    }

    /// Cancel a subscription. Only the customer, merchant, or admin may call this.
    pub fn cancel_subscription(
        env: Env,
        caller: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        let config: Option<MultiSigConfig> = env.storage().instance().get(&DataKey::MultiSigConfig);

        let is_authorized = sub.customer == caller
            || sub.merchant == caller
            || config.map_or(false, |c| c.admins.contains(&caller));

        if !is_authorized {
            return Err(Error::Unauthorized);
        }

        if sub.status == SubscriptionStatus::Cancelled || sub.status == SubscriptionStatus::Expired
        {
            return Err(Error::InvalidStatus);
        }

        sub.status = SubscriptionStatus::Cancelled;
        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        // Emit TrialCancelled if cancelled during trial
        let now = env.ledger().timestamp();
        if sub.trial_data.ends_at > 0 && now < sub.trial_data.ends_at {
            (TrialCancelled {
                subscription_id,
                cancelled_at: now,
            })
            .publish(&env);
        }

        (SubscriptionCancelled {
            subscription_id,
            cancelled_by: caller,
        })
        .publish(&env);

        Ok(())
    }

    /// Pause an active subscription. Only the customer may pause.
    pub fn pause_subscription(
        env: Env,
        customer: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        customer.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Unauthorized);
        }

        if sub.status != SubscriptionStatus::Active {
            return Err(Error::SubscriptionNotActive);
        }

        sub.status = SubscriptionStatus::Paused;
        sub.pause_data.last_paused_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        (SubscriptionPaused { subscription_id }).publish(&env);

        Ok(())
    }

    /// Resume a paused subscription. Resets `next_payment_at` from now.
    pub fn resume_subscription(
        env: Env,
        customer: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        customer.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Unauthorized);
        }

        if sub.status != SubscriptionStatus::Paused {
            return Err(Error::InvalidStatus);
        }

        let now = env.ledger().timestamp();
        let pause_duration = now - sub.pause_data.last_paused_at;
        sub.pause_data.total_pause_duration += pause_duration;

        // Shift next billing date by pause duration
        sub.next_payment_at += pause_duration;

        // Shift ends_at if it's a fixed-duration subscription
        if sub.ends_at > 0 {
            sub.ends_at += pause_duration;
        }

        sub.status = SubscriptionStatus::Active;

        if sub.pause_data.proration_enabled {
            // Proration formula: (Full Amount * Remaining Time in Cycle) / Cycle Duration
            let remaining_time = sub.next_payment_at - now;
            let prorated_amount = (sub.amount * remaining_time as i128) / sub.interval as i128;

            (SubscriptionResumedWithProration {
                subscription_id,
                pause_duration,
                new_next_billing_date: sub.next_payment_at,
                prorated_amount,
            })
            .publish(&env);
        } else {
            (SubscriptionResumed {
                subscription_id,
                next_payment_at: sub.next_payment_at,
            })
            .publish(&env);
        }

        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        (SubscriptionResumed {
            subscription_id,
            next_payment_at: sub.next_payment_at,
        })
        .publish(&env);

        Ok(())
    }

    /// Read a single subscription.
    pub fn get_subscription(env: Env, subscription_id: u64) -> Subscription {
        env.storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .expect("Subscription not found")
    }

    /// Paginated list of subscriptions for a customer.
    pub fn get_subscriptions_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Subscription> {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerSubscriptionCount(customer.clone()))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let end = (offset + limit).min(total);

        for i in offset..end {
            if let Some(sub_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::CustomerSubscriptions(customer.clone(), i))
            {
                if let Some(sub) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Subscription>(&DataKey::Subscription(sub_id))
                {
                    result.push_back(sub);
                }
            }
        }

        result
    }

    /// Paginated list of subscriptions for a merchant.
    pub fn get_subscriptions_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Subscription> {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantSubscriptionCount(merchant.clone()))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let end = (offset + limit).min(total);

        for i in offset..end {
            if let Some(sub_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::MerchantSubscriptions(merchant.clone(), i))
            {
                if let Some(sub) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Subscription>(&DataKey::Subscription(sub_id))
                {
                    result.push_back(sub);
                }
            }
        }

        result
    }

    // ── DUNNING MANAGEMENT METHODS ─────────────────────────────────────

    /// Admin sets the dunning configuration for the contract.
    pub fn set_dunning_config(
        env: Env,
        admin: Address,
        config: DunningConfig,
    ) -> Result<(), Error> {
        admin.require_auth();

        let ms_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&DataKey::DunningConfig, &config);

        Ok(())
    }

    /// Returns the current dunning configuration.
    /// Returns default config if not yet set.
    pub fn get_dunning_config(env: Env) -> DunningConfig {
        env.storage()
            .instance()
            .get(&DataKey::DunningConfig)
            .unwrap_or(DunningConfig {
                initial_backoff_seconds: 3600, // 1 hour
                max_retries: 5,
            })
    }

    /// Returns the dunning state for a subscription, if any.
    pub fn get_dunning_state(env: Env, subscription_id: u64) -> Option<DunningState> {
        env.storage()
            .instance()
            .get(&StateDataKey::DunningState(subscription_id))
    }

    /// Retry a failed payment for a subscription in dunning.
    /// Validates that the retry is due before attempting.
    pub fn retry_failed_payment(env: Env, subscription_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.status != SubscriptionStatus::InDunning {
            return Err(Error::SubscriptionNotInDunning);
        }

        let mut dunning: DunningState = env
            .storage()
            .instance()
            .get(&StateDataKey::DunningState(subscription_id))
            .ok_or(Error::DunningNotFound)?;

        let now = env.ledger().timestamp();

        // Enforce backoff window
        if now < dunning.next_retry_at {
            return Err(Error::RetryTooEarly);
        }

        // Attempt the payment
        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();

        let transfer_ok = token_client
            .try_transfer_from(&contract_address, &sub.customer, &sub.merchant, &sub.amount)
            .is_ok();

        if transfer_ok {
            sub.payment_count += 1;
            sub.retry_count = 0;
            sub.next_payment_at = now + sub.interval;
            sub.status = SubscriptionStatus::Active;

            if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                sub.status = SubscriptionStatus::Expired;
            }

            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
            env.storage()
                .instance()
                .remove(&StateDataKey::DunningState(subscription_id));

            (DunningResolved {
                subscription_id,
                resolved_at: now,
            })
            .publish(&env);

            (RecurringPaymentExecuted {
                subscription_id,
                payment_count: sub.payment_count,
                amount: sub.amount,
                next_payment_at: sub.next_payment_at,
            })
            .publish(&env);

            Ok(())
        } else {
            dunning.retry_count += 1;
            dunning.last_failed_at = now;

            if dunning.retry_count >= dunning.max_retries {
                sub.status = SubscriptionStatus::Suspended;
                env.storage()
                    .instance()
                    .set(&DataKey::Subscription(subscription_id), &sub);
                env.storage()
                    .instance()
                    .set(&StateDataKey::DunningState(subscription_id), &dunning);

                (SubscriptionSuspended {
                    subscription_id,
                    reason: String::from_str(&env, "Maximum retries exceeded"),
                })
                .publish(&env);

                return Err(Error::MaxRetriesExceeded);
            }

            // Exponential backoff: backoff_seconds * 2^retry_count
            dunning.next_retry_at = now + (dunning.backoff_seconds << dunning.retry_count);
            env.storage()
                .instance()
                .set(&StateDataKey::DunningState(subscription_id), &dunning);

            (DunningRetryScheduled {
                subscription_id,
                retry_at: dunning.next_retry_at,
            })
            .publish(&env);

            (RecurringPaymentFailed {
                subscription_id,
                retry_count: dunning.retry_count as u64,
            })
            .publish(&env);

            Err(Error::TransferFailed)
        }
    }

    /// Admin resolves dunning for a subscription, returning it to active state.
    pub fn resolve_dunning(env: Env, admin: Address, subscription_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.status != SubscriptionStatus::InDunning
            && sub.status != SubscriptionStatus::Suspended
        {
            return Err(Error::SubscriptionNotInDunning);
        }

        // Reset to active state
        sub.status = SubscriptionStatus::Active;
        sub.retry_count = 0;
        sub.next_payment_at = env.ledger().timestamp() + sub.interval;

        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        // Remove dunning state
        env.storage()
            .instance()
            .remove(&StateDataKey::DunningState(subscription_id));

        (DunningResolved {
            subscription_id,
            resolved_at: env.ledger().timestamp(),
        })
        .publish(&env);

        Ok(())
    }

    /// Internal function to enter dunning for a subscription.
    fn enter_dunning(env: &Env, subscription_id: u64, _reason: String) {
        let config = PaymentContract::get_dunning_config(env.clone());
        let now = env.ledger().timestamp();

        // retry_count = 0: first retry uses backoff * 2^0 = backoff (1x)
        // each subsequent failure increments retry_count, giving backoff * 2^retry_count
        let dunning_state = DunningState {
            subscription_id,
            retry_count: 0,
            next_retry_at: now + config.initial_backoff_seconds,
            backoff_seconds: config.initial_backoff_seconds,
            max_retries: config.max_retries,
            last_failed_at: now,
        };

        env.storage()
            .instance()
            .set(&StateDataKey::DunningState(subscription_id), &dunning_state);

        // Update subscription status
        if let Some(mut sub) = env
            .storage()
            .instance()
            .get::<DataKey, Subscription>(&DataKey::Subscription(subscription_id))
        {
            sub.status = SubscriptionStatus::InDunning;
            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
        }

        (SubscriptionEnteredDunning {
            subscription_id,
            attempt: 1,
            next_retry_at: dunning_state.next_retry_at,
        })
        .publish(env);
    }

    // ── RATE LIMITING / ANTI-FRAUD METHODS ──────────────────────────────────

    /// Admin sets the global rate limit configuration.
    pub fn set_rate_limit_config(
        env: Env,
        admin: Address,
        config: RateLimitConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let ms_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::RateLimitConfig, &config);
        Ok(())
    }

    /// Returns the current rate limit configuration.
    /// Defaults to unlimited if not yet configured.
    pub fn get_rate_limit_config(env: Env) -> RateLimitConfig {
        env.storage()
            .instance()
            .get(&DataKey::RateLimitConfig)
            .unwrap_or(RateLimitConfig {
                max_payments_per_window: 0,
                window_duration: 0,
                max_payment_amount: 0,
                max_daily_volume: 0,
            })
    }

    /// Returns the per-address rate limit state (or a zeroed default).
    pub fn get_address_rate_limit(env: Env, address: Address) -> AddressRateLimit {
        env.storage()
            .instance()
            .get(&DataKey::AddressRateLimit(address.clone()))
            .unwrap_or(AddressRateLimit {
                address: address.clone(),
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            })
    }

    /// Admin flags a suspicious address, blocking it from creating payments.
    pub fn flag_address(
        env: Env,
        admin: Address,
        address: Address,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        let mut rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::AddressRateLimit(address.clone()))
            .unwrap_or(AddressRateLimit {
                address: address.clone(),
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            });
        if rate_limit.flagged {
            return Err(Error::AddressAlreadyFlagged);
        }
        rate_limit.flagged = true;
        env.storage()
            .instance()
            .set(&DataKey::AddressRateLimit(address.clone()), &rate_limit);
        env.storage()
            .instance()
            .set(&DataKey::AddressFlagReason(address.clone()), &reason);
        (AddressFlagged { address, reason }).publish(&env);
        Ok(())
    }

    /// Admin removes the flag from an address, allowing it to create payments again.
    pub fn unflag_address(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        let mut rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::AddressRateLimit(address.clone()))
            .unwrap_or(AddressRateLimit {
                address: address.clone(),
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            });
        if !rate_limit.flagged {
            return Err(Error::InvalidStatus);
        }
        rate_limit.flagged = false;
        env.storage()
            .instance()
            .set(&DataKey::AddressRateLimit(address.clone()), &rate_limit);
        env.storage()
            .instance()
            .remove(&DataKey::AddressFlagReason(address.clone()));
        (AddressUnflagged { address }).publish(&env);
        Ok(())
    }

    pub fn is_address_flagged(env: Env, address: Address) -> bool {
        let rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::AddressRateLimit(address.clone()))
            .unwrap_or(AddressRateLimit {
                address,
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            });
        rate_limit.flagged
    }

    pub fn get_flag_reason(env: Env, address: Address) -> Option<String> {
        env.storage()
            .instance()
            .get(&DataKey::AddressFlagReason(address))
    }

    pub fn add_to_allowlist(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::AddressAllowlist(address), &true);
        Ok(())
    }

    pub fn remove_from_allowlist(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .remove(&DataKey::AddressAllowlist(address));
        Ok(())
    }

    /// Internal check called by create_payment. Validates the address against
    /// the configured rate limits and updates per-address counters.
    fn check_rate_limit(env: &Env, address: &Address, amount: i128) -> Result<(), Error> {
        // If no config is set, rate limiting is disabled.
        let config: Option<RateLimitConfig> =
            env.storage().instance().get(&DataKey::RateLimitConfig);
        let config = match config {
            None => {
                return Ok(());
            }
            Some(c) => c,
        };

        let mut rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::AddressRateLimit(address.clone()))
            .unwrap_or(AddressRateLimit {
                address: address.clone(),
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            });

        // Block flagged addresses unless explicitly allowlisted.
        if rate_limit.flagged && !PaymentContract::is_allowlisted(env, address) {
            return Err(Error::AddressFlagged);
        }

        // Reject payment if it exceeds the single-transaction amount cap.
        if config.max_payment_amount > 0 && amount > config.max_payment_amount {
            return Err(Error::AmountExceedsLimit);
        }

        let now = env.ledger().timestamp();

        // Reset daily volume counter when a calendar-day boundary is crossed.
        if rate_limit.window_start > 0
            && now / SECONDS_PER_DAY > rate_limit.window_start / SECONDS_PER_DAY
        {
            rate_limit.daily_volume = 0;
        }

        // Reset window payment counter when the window duration has elapsed.
        if config.window_duration > 0
            && rate_limit.window_start > 0
            && now >= rate_limit.window_start + config.window_duration
        {
            rate_limit.payment_count = 0;
            rate_limit.window_start = now;
        } else if rate_limit.window_start == 0 {
            // First payment — initialise the window.
            rate_limit.window_start = now;
        }

        // Enforce per-window payment count limit.
        if config.max_payments_per_window > 0
            && rate_limit.payment_count >= config.max_payments_per_window
        {
            (RateLimitBreached {
                address: address.clone(),
                payment_count: rate_limit.payment_count,
            })
            .publish(env);
            return Err(Error::RateLimitExceeded);
        }

        // Enforce daily volume limit.
        if config.max_daily_volume > 0 {
            let new_volume = rate_limit.daily_volume.saturating_add(amount);
            if new_volume > config.max_daily_volume {
                return Err(Error::DailyVolumeExceeded);
            }
            rate_limit.daily_volume = new_volume;
        }

        // Record successful check: increment counters and persist.
        rate_limit.payment_count += 1;
        rate_limit.last_payment_at = now;

        env.storage()
            .instance()
            .set(&DataKey::AddressRateLimit(address.clone()), &rate_limit);

        Ok(())
    }

    fn is_allowlisted(env: &Env, address: &Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::AddressAllowlist(address.clone()))
            .unwrap_or(false)
    }

    pub fn set_merchant_rate_limit(
        env: Env,
        admin: Address,
        merchant: Address,
        config: MerchantRateLimit,
    ) -> Result<(), Error> {
        admin.require_auth();
        let ms_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::MerchantRateLimit(merchant), &config);
        Ok(())
    }

    pub fn get_merchant_rate_limit(env: Env, merchant: Address) -> Option<MerchantRateLimit> {
        env.storage()
            .instance()
            .get(&DataKey::MerchantRateLimit(merchant))
    }

    pub fn reset_merchant_rate_limit(
        env: Env,
        admin: Address,
        merchant: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        let ms_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .remove(&DataKey::MerchantRateLimit(merchant));
        Ok(())
    }

    pub fn check_rate_limit(env: Env, merchant: Address, amount: i128) -> bool {
        // Check merchant-specific limit first (read-only)
        if let Some(limit) = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRateLimit(merchant.clone()))
        {
            let now = env.ledger().timestamp();
            // Check if window needs reset (read-only)
            let reset_needed = limit.window_start > 0 && now >= limit.window_start + 3600;
            let current_transactions = if reset_needed { 0 } else { limit.current_transactions };
            let current_amount = if reset_needed { 0 } else { limit.current_amount };
            // Check limits
            if current_transactions >= limit.max_transactions_per_hour {
                return false;
            }
            if current_amount + amount > limit.max_amount_per_hour {
                return false;
            }
            // Would pass
            return true;
        } else {
            // Fallback to global config
            let config: Option<RateLimitConfig> =
                env.storage().instance().get(&DataKey::RateLimitConfig);
            if let Some(config) = config {
                // For merchants, use global as fallback, but since it's read-only, just check if amount exceeds
                if config.max_payment_amount > 0 && amount > config.max_payment_amount {
                    return false;
                }
                // For transactions, since no state, assume ok
                return true;
            } else {
                // No limits
                return true;
            }
        }
    }

    fn check_merchant_rate_limit(env: &Env, merchant: &Address, amount: i128) -> Result<(), Error> {
        // If no merchant limit, use global as fallback, but enforce if set
        if let Some(mut limit) = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRateLimit(merchant.clone()))
        {
            let now = env.ledger().timestamp();
            // Reset window if 1 hour passed
            if limit.window_start > 0 && now >= limit.window_start + 3600 {
                limit.current_transactions = 0;
                limit.current_amount = 0;
                limit.window_start = now;
            } else if limit.window_start == 0 {
                limit.window_start = now;
            }
            // Check limits
            if limit.current_transactions >= limit.max_transactions_per_hour {
                return Err(Error::MerchantRateLimitExceeded);
            }
            if limit.current_amount + amount > limit.max_amount_per_hour {
                return Err(Error::AmountRateLimitExceeded);
            }
            // Update counters
            limit.current_transactions += 1;
            limit.current_amount += amount;
            env.storage()
                .instance()
                .set(&DataKey::MerchantRateLimit(merchant.clone()), &limit);
        } else {
            // Fallback to global config for merchants
            let config: Option<RateLimitConfig> =
                env.storage().instance().get(&DataKey::RateLimitConfig);
            if let Some(config) = config {
                // For merchants, enforce global amount limit if set
                if config.max_payment_amount > 0 && amount > config.max_payment_amount {
                    return Err(Error::AmountExceedsLimit);
                }
                // No transaction limit for merchants in global
            }
        }
        Ok(())
    }

    fn invoke_escrow_create(
        env: &Env,
        escrow_contract: &Address,
        customer: &Address,
        merchant: &Address,
        amount: i128,
        token: &Address,
        release_timestamp: u64,
        min_hold_period: u64,
    ) -> Result<u64, Error> {
        let client = EscrowContractClient::new(env, escrow_contract);
        let call = client.try_create_escrow(
            customer,
            merchant,
            &amount,
            token,
            &release_timestamp,
            &min_hold_period,
        );
        match call {
            Ok(Ok(escrow_id)) => Ok(escrow_id),
            _ => Err(Error::EscrowBridgeFailed),
        }
    }

    fn u64_to_string(env: &Env, n: u64) -> String {
        if n == 0 {
            return String::from_str(env, "0");
        }
        let mut digits = [0u8; 20];
        let mut count = 0usize;
        let mut num = n;
        while num > 0 {
            digits[count] = b'0' + ((num % 10) as u8);
            count += 1;
            num /= 10;
        }
        // Reverse digits into a fixed buffer
        let mut buf = [0u8; 20];
        for i in 0..count {
            buf[i] = digits[count - 1 - i];
        }
        String::from_bytes(env, &buf[..count])
    }

    fn read_u64_from_bytes(data: &Bytes, offset: u32) -> u64 {
        let mut result: u64 = 0;
        for i in 0..8u32 {
            let byte = data.get(offset + i).unwrap_or(0) as u64;
            result = (result << 8) | byte;
        }
        result
    }

    fn dispatch_action(env: &Env, proposal: &AdminProposal) -> Result<(), Error> {
        match proposal.action_type {
            ActionType::CompletePayment => {
                let payment_id = PaymentContract::read_u64_from_bytes(&proposal.data, 0);
                PaymentContract::do_complete_payment(env, payment_id)?;
            }
            ActionType::RefundPayment => {
                let payment_id = PaymentContract::read_u64_from_bytes(&proposal.data, 0);
                PaymentContract::do_refund_payment(env, payment_id)?;
            }
            ActionType::AddAdmin => {
                let new_admin = proposal.target.clone();
                let mut config: MultiSigConfig = env
                    .storage()
                    .instance()
                    .get(&DataKey::MultiSigConfig)
                    .ok_or(Error::MultiSigNotInitialized)?;
                if !config.admins.contains(&new_admin) {
                    config.admins.push_back(new_admin.clone());
                    config.total_admins += 1;
                    env.storage()
                        .instance()
                        .set(&DataKey::MultiSigConfig, &config);
                    (AdminAdded { admin: new_admin }).publish(env);
                }
            }
            ActionType::RemoveAdmin => {
                let admin_to_remove = proposal.target.clone();
                let mut config: MultiSigConfig = env
                    .storage()
                    .instance()
                    .get(&DataKey::MultiSigConfig)
                    .ok_or(Error::MultiSigNotInitialized)?;
                if config.total_admins <= config.required_signatures {
                    return Err(Error::InsufficientAdmins);
                }
                let mut new_admins = Vec::new(env);
                for a in config.admins.iter() {
                    if a != admin_to_remove {
                        new_admins.push_back(a);
                    }
                }
                config.admins = new_admins;
                config.total_admins -= 1;
                env.storage()
                    .instance()
                    .set(&DataKey::MultiSigConfig, &config);
                (AdminRemoved {
                    admin: admin_to_remove,
                })
                .publish(env);
            }
            ActionType::UpdateRequiredSignatures => {
                let required = PaymentContract::read_u64_from_bytes(&proposal.data, 0) as u32;
                let mut config: MultiSigConfig = env
                    .storage()
                    .instance()
                    .get(&DataKey::MultiSigConfig)
                    .ok_or(Error::MultiSigNotInitialized)?;
                if required == 0 || required > config.total_admins {
                    return Err(Error::InsufficientAdmins);
                }
                config.required_signatures = required;
                env.storage()
                    .instance()
                    .set(&DataKey::MultiSigConfig, &config);
            }
            _ => {}
        }
        Ok(())
    }

    // ── FEE MANAGEMENT ───────────────────────────────────────────────────────

    /// Admin sets the platform fee configuration.
    pub fn set_fee_config(env: Env, admin: Address, fee_config: FeeConfig) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        (FeeConfigUpdated {
            fee_bps: fee_config.fee_bps,
            treasury: fee_config.treasury.clone(),
        })
        .publish(&env);
        env.storage()
            .instance()
            .set(&DataKey::FeeConfig, &fee_config);
        Ok(())
    }

    /// Returns the current fee configuration.
    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        env.storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .ok_or(Error::FeeConfigNotFound)
    }

    /// Calculates the fee for a given amount and merchant (accounting for tier discount and waivers).
    pub fn calculate_fee(env: Env, amount: i128, merchant: Address) -> i128 {
        let config: Option<FeeConfig> = env.storage().instance().get(&DataKey::FeeConfig);
        let config = match config {
            None => {
                return 0;
            }
            Some(c) if !c.active => {
                return 0;
            }
            Some(c) => c,
        };

        // Get effective fee BPS including tier discounts and waivers
        let effective_fee_bps =
            PaymentContract::get_effective_fee_bps(env.clone(), merchant.clone());

        PaymentContract::compute_fee_amount(
            amount,
            effective_fee_bps,
            &FeeTier::Standard, // Tier already applied in get_effective_fee_bps
            config.min_fee,
            config.max_fee,
        )
    }

    /// Returns the fee record for a given merchant.
    pub fn get_merchant_fee_record(env: Env, merchant: Address) -> MerchantFeeRecord {
        PaymentContract::get_or_default_merchant_fee_record(&env, merchant)
    }

    /// Returns only the current tier for a given merchant.
    pub fn get_merchant_tier(env: Env, merchant: Address) -> FeeTier {
        PaymentContract::get_or_default_merchant_fee_record(&env, merchant).fee_tier
    }

    /// Admin manually sets merchant tier (supports explicit downgrade/override).
    pub fn manually_set_merchant_tier(
        env: Env,
        admin: Address,
        merchant: Address,
        tier: FeeTier,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        let mut record =
            PaymentContract::get_or_default_merchant_fee_record(&env, merchant.clone());
        record.fee_tier = tier;
        env.storage()
            .instance()
            .set(&DataKey::MerchantFeeRecord(merchant), &record);
        Ok(())
    }

    /// Returns tier thresholds as (tier, minimum-volume) pairs.
    pub fn get_tier_thresholds(env: Env) -> Vec<(FeeTier, i128)> {
        PaymentContract::get_stored_or_default_thresholds(&env)
    }

    /// Admin sets ascending tier thresholds.
    pub fn set_tier_thresholds(
        env: Env,
        admin: Address,
        thresholds: Vec<(FeeTier, i128)>,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        PaymentContract::validate_thresholds(&thresholds)?;
        env.storage()
            .instance()
            .set(&DataKey::TierThresholds, &thresholds);
        Ok(())
    }

    /// Returns the total fees accumulated in the contract.
    pub fn get_accumulated_fees(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::AccumulatedFees)
            .unwrap_or(0)
    }

    /// Admin withdraws accumulated fees to the treasury address.
    pub fn withdraw_fees(env: Env, admin: Address, amount: i128) -> Result<(), Error> {
        admin.require_auth();
        let multisig: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !multisig.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        let fee_config: FeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::FeeConfig)
            .ok_or(Error::FeeConfigNotFound)?;
        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AccumulatedFees)
            .unwrap_or(0);
        if amount > accumulated {
            return Err(Error::InsufficientFees);
        }
        let token_client = token::Client::new(&env, &fee_config.fee_token);
        token_client.transfer(
            &env.current_contract_address(),
            &fee_config.treasury,
            &amount,
        );
        env.storage()
            .instance()
            .set(&DataKey::AccumulatedFees, &(accumulated - amount));
        (FeesWithdrawn {
            amount,
            treasury: fee_config.treasury.clone(),
        })
        .publish(&env);
        Ok(())
    }

    /// Internal: deducts the platform fee from a payment, transfers fee to contract,
    /// updates merchant record and accumulated fees, and returns the net amount.
    fn deduct_fee(
        env: &Env,
        payment_id: u64,
        amount: i128,
        merchant: Address,
        token: &Address,
        customer: &Address,
    ) -> (i128, i128) {
        let config: Option<FeeConfig> = env.storage().instance().get(&DataKey::FeeConfig);
        let config = match config {
            None => {
                return (amount, 0);
            }
            Some(c) if !c.active => {
                return (amount, 0);
            }
            Some(c) if c.fee_token != *token => {
                return (amount, 0);
            }
            Some(c) => c,
        };
        let record = PaymentContract::get_or_default_merchant_fee_record(env, merchant.clone());

        let fee = PaymentContract::compute_fee_amount(
            amount,
            config.fee_bps,
            &record.fee_tier,
            config.min_fee,
            config.max_fee,
        );

        if fee <= 0 {
            return (amount, 0);
        }

        let net_amount = amount - fee;

        // Transfer fee from customer to contract
        let token_client = token::Client::new(env, token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(&contract_address, customer, &contract_address, &fee);

        // Update accumulated fees
        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AccumulatedFees)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::AccumulatedFees, &(accumulated + fee));

        (FeeCollected {
            payment_id,
            fee_amount: fee,
            merchant,
        })
        .publish(env);

        (net_amount, fee)
    }

    /// Computes the fee amount applying tier discount and min/max clamping.
    fn compute_fee_amount(
        amount: i128,
        fee_bps: u32,
        tier: &FeeTier,
        min_fee: i128,
        max_fee: i128,
    ) -> i128 {
        let discount = PaymentContract::get_tier_discount_bps(tier);
        let effective_bps = fee_bps - (fee_bps * discount) / 10000;
        let raw_fee = (amount * (effective_bps as i128)) / 10000;

        let fee = if min_fee > 0 && raw_fee < min_fee {
            min_fee
        } else {
            raw_fee
        };
        let fee = if max_fee > 0 && fee > max_fee {
            max_fee
        } else {
            fee
        };

        if fee < 0 {
            0
        } else {
            fee
        }
    }

    fn get_tier_discount_bps(tier: &FeeTier) -> u32 {
        match tier {
            FeeTier::Standard => 0,
            FeeTier::Premium => 750,     // 7.5% discount on fee
            FeeTier::Enterprise => 2000, // 20% discount on fee
        }
    }

    fn get_or_default_merchant_fee_record(env: &Env, merchant: Address) -> MerchantFeeRecord {
        env.storage()
            .instance()
            .get(&DataKey::MerchantFeeRecord(merchant.clone()))
            .unwrap_or(MerchantFeeRecord {
                merchant,
                total_fees_paid: 0,
                total_volume: 0,
                fee_tier: FeeTier::Standard,
            })
    }

    fn default_tier_thresholds_for_volume() -> [(FeeTier, i128); 2] {
        [
            (FeeTier::Premium, PREMIUM_VOLUME_THRESHOLD),
            (FeeTier::Enterprise, ENTERPRISE_VOLUME_THRESHOLD),
        ]
    }

    fn get_stored_or_default_thresholds(env: &Env) -> Vec<(FeeTier, i128)> {
        env.storage()
            .instance()
            .get(&DataKey::TierThresholds)
            .unwrap_or_else(|| {
                let defaults = PaymentContract::default_tier_thresholds_for_volume();
                Vec::from_array(env, defaults)
            })
    }

    fn tier_rank(tier: &FeeTier) -> u32 {
        match tier {
            FeeTier::Standard => 0,
            FeeTier::Premium => 1,
            FeeTier::Enterprise => 2,
        }
    }

    fn calculate_tier(env: &Env, total_volume: i128) -> FeeTier {
        let thresholds = PaymentContract::get_stored_or_default_thresholds(env);
        let mut premium_threshold = PREMIUM_VOLUME_THRESHOLD;
        let mut enterprise_threshold = ENTERPRISE_VOLUME_THRESHOLD;
        for pair in thresholds.iter() {
            match pair.0 {
                FeeTier::Standard => {}
                FeeTier::Premium => premium_threshold = pair.1,
                FeeTier::Enterprise => enterprise_threshold = pair.1,
            }
        }

        if total_volume >= enterprise_threshold {
            FeeTier::Enterprise
        } else if total_volume >= premium_threshold {
            FeeTier::Premium
        } else {
            FeeTier::Standard
        }
    }

    fn validate_thresholds(thresholds: &Vec<(FeeTier, i128)>) -> Result<(), Error> {
        if thresholds.len() < 2 || thresholds.len() > 3 {
            return Err(Error::InvalidTierThresholds);
        }

        let mut has_premium = false;
        let mut has_enterprise = false;
        let mut prev_rank: Option<u32> = None;
        let mut prev_value: Option<i128> = None;

        for pair in thresholds.iter() {
            let rank = PaymentContract::tier_rank(&pair.0);
            if let Some(r) = prev_rank {
                if rank <= r {
                    return Err(Error::InvalidTierThresholds);
                }
            }
            if let Some(v) = prev_value {
                if pair.1 <= v {
                    return Err(Error::InvalidTierThresholds);
                }
            }
            if pair.1 < 0 {
                return Err(Error::InvalidTierThresholds);
            }

            match pair.0 {
                FeeTier::Premium => has_premium = true,
                FeeTier::Enterprise => has_enterprise = true,
                FeeTier::Standard => {}
            }
            prev_rank = Some(rank);
            prev_value = Some(pair.1);
        }

        if !has_premium || !has_enterprise {
            return Err(Error::InvalidTierThresholds);
        }
        Ok(())
    }

    fn update_merchant_fee_record_post_completion(
        env: &Env,
        merchant: Address,
        amount: i128,
        fee_amount: i128,
    ) {
        let mut record = PaymentContract::get_or_default_merchant_fee_record(env, merchant.clone());
        record.total_volume += amount;
        record.total_fees_paid += fee_amount;

        // Automatic tier changes are monotonic upgrades only.
        let old_tier = record.fee_tier.clone();
        let computed_tier = PaymentContract::calculate_tier(env, record.total_volume);
        if PaymentContract::tier_rank(&computed_tier) > PaymentContract::tier_rank(&record.fee_tier)
        {
            record.fee_tier = computed_tier.clone();
            (MerchantTierUpgraded {
                merchant: merchant.clone(),
                old_tier,
                new_tier: computed_tier,
            })
            .publish(env);
        }

        env.storage()
            .instance()
            .set(&DataKey::MerchantFeeRecord(merchant), &record);
    }

    // ── FEE WAIVER FUNCTIONS ───────────────────────────────────────────────────

    pub fn grant_fee_waiver(
        env: Env,
        admin: Address,
        merchant: Address,
        waiver_bps: u32,
        valid_until: u64,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        // Validate waiver_bps is between 0 and 10000 (100%)
        if waiver_bps > 10000 {
            return Err(Error::InvalidTierThresholds); // Reuse existing error
        }

        let waiver = FeeWaiver {
            merchant: merchant.clone(),
            waiver_bps,
            valid_until,
            reason,
            granted_by: admin.clone(),
        };

        env.storage()
            .instance()
            .set(&DataKey::FeeWaiver(merchant.clone()), &waiver);

        (FeeWaiverGranted {
            merchant,
            waiver_bps,
            valid_until,
        })
        .publish(&env);

        Ok(())
    }

    pub fn revoke_fee_waiver(env: Env, admin: Address, merchant: Address) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        // Check if waiver exists
        let _waiver: FeeWaiver = env
            .storage()
            .instance()
            .get(&DataKey::FeeWaiver(merchant.clone()))
            .ok_or(Error::PaymentNotFound)?; // Reuse existing error

        // Remove the waiver
        env.storage()
            .instance()
            .remove(&DataKey::FeeWaiver(merchant.clone()));

        (FeeWaiverRevoked {
            merchant,
            revoked_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    pub fn get_fee_waiver(env: Env, merchant: Address) -> Option<FeeWaiver> {
        let waiver: Option<FeeWaiver> = env
            .storage()
            .instance()
            .get(&DataKey::FeeWaiver(merchant.clone()));

        // Check if waiver is expired
        if let Some(w) = waiver {
            if env.ledger().timestamp() > w.valid_until {
                // Remove expired waiver and publish expiration event
                env.storage()
                    .instance()
                    .remove(&DataKey::FeeWaiver(merchant.clone()));

                (FeeWaiverExpired { merchant }).publish(&env);
                None
            } else {
                Some(w)
            }
        } else {
            None
        }
    }

    pub fn get_effective_fee_bps(env: Env, merchant: Address) -> u32 {
        // Get base fee configuration
        let config: Option<FeeConfig> = env.storage().instance().get(&DataKey::FeeConfig);
        let config = match config {
            None => return 0,
            Some(c) if !c.active => return 0,
            Some(c) => c,
        };

        // Get merchant's tier discount
        let record = PaymentContract::get_or_default_merchant_fee_record(&env, merchant.clone());
        let tier_discount = PaymentContract::get_tier_discount_bps(&record.fee_tier);
        let tier_adjusted_bps = config.fee_bps - (config.fee_bps * tier_discount) / 10000;

        // Get waiver discount
        let waiver = PaymentContract::get_fee_waiver(env.clone(), merchant);
        if let Some(w) = waiver {
            let waiver_adjusted_bps =
                tier_adjusted_bps - (tier_adjusted_bps * w.waiver_bps) / 10000;
            waiver_adjusted_bps
        } else {
            tier_adjusted_bps
        }
    }

    // ── BATCH PAYMENT OPERATIONS ──────────────────────────────────────────────

    fn validate_batch_size(len: u32) -> Result<(), Error> {
        const MAX_BATCH_SIZE: u32 = 50;
        if len == 0 || len > MAX_BATCH_SIZE {
            return Err(Error::InvalidBatchSize);
        }
        Ok(())
    }

    pub fn create_batch_payment(
        env: Env,
        entries: Vec<BatchPaymentEntry>,
    ) -> Result<Vec<BatchResult>, Error> {
        Self::require_not_paused(&env, "create_batch_payment")?;
        PaymentContract::validate_batch_size(entries.len())?;

        // Require auth for all unique customers in the batch
        let mut seen_customers: Vec<Address> = Vec::new(&env);
        for entry in entries.iter() {
            if !seen_customers.contains(&entry.customer) {
                entry.customer.require_auth();
                seen_customers.push_back(entry.customer.clone());
            }
        }

        let mut results = Vec::new(&env);

        for entry in entries.iter() {
            let result = PaymentContract::do_create_payment(
                &env,
                entry.customer.clone(),
                entry.merchant.clone(),
                entry.amount,
                entry.token.clone(),
                entry.currency.clone(),
                entry.expiration_duration,
                entry.metadata.clone(),
            );

            match result {
                Ok(payment_id) => {
                    results.push_back(BatchResult {
                        payment_id,
                        success: true,
                        error_code: None,
                    });
                }
                Err(e) => {
                    results.push_back(BatchResult {
                        payment_id: 0,
                        success: false,
                        error_code: Some(e as u32),
                    });
                }
            }
        }

        Ok(results)
    }

    pub fn complete_batch_payment(
        env: Env,
        admin: Address,
        payment_ids: Vec<u64>,
    ) -> Result<Vec<BatchResult>, Error> {
        Self::require_not_paused(&env, "complete_batch_payment")?;
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        PaymentContract::validate_batch_size(payment_ids.len())?;

        let mut results = Vec::new(&env);

        for payment_id in payment_ids.iter() {
            let result = PaymentContract::do_complete_payment(&env, payment_id);

            match result {
                Ok(()) => {
                    results.push_back(BatchResult {
                        payment_id,
                        success: true,
                        error_code: None,
                    });
                }
                Err(e) => {
                    results.push_back(BatchResult {
                        payment_id,
                        success: false,
                        error_code: Some(e as u32),
                    });
                }
            }
        }

        Ok(results)
    }

    pub fn cancel_batch_payment(
        env: Env,
        caller: Address,
        payment_ids: Vec<u64>,
    ) -> Result<Vec<BatchResult>, Error> {
        Self::require_not_paused(&env, "cancel_batch_payment")?;
        caller.require_auth();

        PaymentContract::validate_batch_size(payment_ids.len())?;

        let mut results = Vec::new(&env);

        for payment_id in payment_ids.iter() {
            let result = PaymentContract::do_cancel_payment(&env, caller.clone(), payment_id);

            match result {
                Ok(()) => {
                    results.push_back(BatchResult {
                        payment_id,
                        success: true,
                        error_code: None,
                    });
                }
                Err(e) => {
                    results.push_back(BatchResult {
                        payment_id,
                        success: false,
                        error_code: Some(e as u32),
                    });
                }
            }
        }

        Ok(results)
    }

    pub fn create_payment_batch_optimized(
        env: Env,
        admin: Address,
        entries: Vec<BatchPaymentEntry>,
    ) -> Result<Vec<BatchResult>, Error> {
        Self::require_not_paused(&env, "create_payment_batch_optimized")?;
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        PaymentContract::validate_batch_size(entries.len())?;

        // Require auth for all unique customers in the batch
        let mut seen_customers: Vec<Address> = Vec::new(&env);
        for entry in entries.iter() {
            if !seen_customers.contains(&entry.customer) {
                entry.customer.require_auth();
                seen_customers.push_back(entry.customer.clone());
            }
        }

        let mut results = Vec::new(&env);
        let mut groups: Vec<(Address, Address, i128)> = Vec::new(&env); // (token, merchant, total_net_amount)

        let contract_address = env.current_contract_address();

        for entry in entries.iter() {
            // Validate currency
            if !PaymentContract::is_valid_currency(&entry.currency) {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(Error::InvalidCurrency as u32),
                });
                continue;
            }

            // Validate metadata size
            if entry.metadata.len() > MAX_METADATA_SIZE {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(Error::MetadataTooLarge as u32),
                });
                continue;
            }

            // Enforce sanctions/flag checks
            if PaymentContract::is_address_flagged(env.clone(), entry.customer.clone())
                && !PaymentContract::is_allowlisted(&env, &entry.customer)
            {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(Error::AddressFlagged as u32),
                });
                continue;
            }

            // Check rate limits
            if let Err(e) = PaymentContract::check_rate_limit(&env, &entry.customer, entry.amount) {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(e as u32),
                });
                continue;
            }

            // Create payment record
            let counter: u64 = env
                .storage()
                .instance()
                .get(&DataKey::PaymentCounter)
                .unwrap_or(0);
            let payment_id = counter + 1;

            let current_timestamp = env.ledger().timestamp();
            let expires_at = if entry.expiration_duration > 0 {
                current_timestamp + entry.expiration_duration
            } else {
                0
            };

            let payment = Payment {
                id: payment_id,
                customer: entry.customer.clone(),
                merchant: entry.merchant.clone(),
                amount: entry.amount,
                token: entry.token.clone(),
                currency: entry.currency.clone(),
                status: PaymentStatus::Completed, // Completed immediately
                created_at: current_timestamp,
                expires_at,
                metadata: entry.metadata.clone(),
                notes: String::from_str(&env, ""),
                refunded_amount: 0,
            };

            env.storage()
                .instance()
                .set(&DataKey::Payment(payment_id), &payment);
            env.storage()
                .instance()
                .set(&DataKey::PaymentCounter, &payment_id);

            // Index by customer
            let customer_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::CustomerPaymentCount(entry.customer.clone()))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::CustomerPayments(entry.customer.clone(), customer_count),
                &payment_id,
            );
            env.storage().instance().set(
                &DataKey::CustomerPaymentCount(entry.customer.clone()),
                &(customer_count + 1),
            );

            // Index by merchant
            let merchant_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::MerchantPaymentCount(entry.merchant.clone()))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::MerchantPayments(entry.merchant.clone(), merchant_count),
                &payment_id,
            );
            env.storage().instance().set(
                &DataKey::MerchantPaymentCount(entry.merchant.clone()),
                &(merchant_count + 1),
            );

            // Deduct fee
            let (net_amount, fee_amount) = PaymentContract::deduct_fee(
                &env,
                payment_id,
                entry.amount,
                entry.merchant.clone(),
                &entry.token,
                &entry.customer,
            );

            // Transfer from customer to contract
            let token_client = token::Client::new(&env, &entry.token);
            match token_client.transfer_from(
                &contract_address,
                &entry.customer,
                &contract_address,
                &net_amount,
            ) {
                Ok(()) => {
                    // Update merchant fee record
                    PaymentContract::update_merchant_fee_record_post_completion(
                        &env,
                        entry.merchant.clone(),
                        entry.amount,
                        fee_amount,
                    );

                    // Update analytics
                    let mut analytics: PaymentAnalytics = env
                        .storage()
                        .instance()
                        .get(&DataKey::PaymentAnalyticsKey)
                        .unwrap_or(PaymentAnalytics {
                            total_payments_created: 0,
                            total_payments_completed: 0,
                            total_payments_cancelled: 0,
                            total_payments_refunded: 0,
                            total_volume: 0,
                            total_refunded_volume: 0,
                            unique_customers: 0,
                            unique_merchants: 0,
                        });
                    analytics.total_payments_completed += 1;
                    env.storage()
                        .instance()
                        .set(&DataKey::PaymentAnalyticsKey, &analytics);
                    let mut m_analytics: MerchantAnalytics = env
                        .storage()
                        .instance()
                        .get(&DataKey::MerchantAnalytics(entry.merchant.clone()))
                        .unwrap_or(MerchantAnalytics {
                            total_payments: 0,
                            total_volume: 0,
                            total_completed: 0,
                            total_cancelled: 0,
                            total_refunded: 0,
                            total_refunded_volume: 0,
                        });
                    m_analytics.total_completed += 1;
                    env.storage().instance().set(
                        &DataKey::MerchantAnalytics(entry.merchant.clone()),
                        &m_analytics,
                    );

                    // Add to group
                    let mut found = false;
                    for i in 0..groups.len() {
                        let (t, m, sum) = groups.get(i).unwrap();
                        if t == entry.token && m == entry.merchant {
                            groups.set(i, (t, m, sum + net_amount));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        groups.push_back((entry.token.clone(), entry.merchant.clone(), net_amount));
                    }

                    results.push_back(BatchResult {
                        payment_id,
                        success: true,
                        error_code: None,
                    });
                }
                Err(_) => {
                    // Transfer failed, mark as failed
                    results.push_back(BatchResult {
                        payment_id,
                        success: false,
                        error_code: Some(Error::TransferFailed as u32), // Assume error
                    });
                }
            }
        }

        // Now, execute aggregated transfers
        for group in groups.iter() {
            let (token, merchant, total_net) = group;
            let token_client = token::Client::new(&env, token);
            // Transfer from contract to merchant
            if let Err(_) = token_client.transfer_from(
                &contract_address,
                &contract_address,
                merchant,
                &total_net,
            ) {
                // If aggregated transfer fails, we need to handle, but for now, assume success
                // In real implementation, might need to refund
            }
        }

        Ok(results)
    }

    pub fn get_batch_gas_estimate(env: Env, entries: Vec<BatchPaymentEntry>) -> u32 {
        // Estimate based on number of entries and groups
        let mut groups: Vec<(Address, Address)> = Vec::new(&env);
        for entry in entries.iter() {
            let mut found = false;
            for g in groups.iter() {
                if g.0 == entry.token && g.1 == entry.merchant {
                    found = true;
                    break;
                }
            }
            if !found {
                groups.push_back((entry.token.clone(), entry.merchant.clone()));
            }
        }
        // Rough estimate: base cost + per entry + per group
        1000 + (entries.len() as u32) * 500 + (groups.len() as u32) * 300
    }

    pub fn create_conditional_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        expiration_duration: u64,
        metadata: String,
        condition: ConditionType,
    ) -> Result<u64, Error> {
        customer.require_auth();

        // Create the base payment first
        let payment_id = PaymentContract::do_create_payment(
            &env,
            customer.clone(),
            merchant.clone(),
            amount,
            token.clone(),
            currency,
            expiration_duration,
            metadata,
        )?;

        // Create the conditional payment
        let conditional_payment = ConditionalPayment {
            payment_id,
            condition: condition.clone(),
            condition_met: false,
            evaluated_at: None,
        };

        env.storage().instance().set(
            &StateDataKey::ConditionalPayment(payment_id),
            &conditional_payment,
        );

        // Publish event with condition type
        let condition_type_str = match &condition {
            ConditionType::TimestampAfter(_) => String::from_str(&env, "TimestampAfter"),
            ConditionType::TimestampBefore(_) => String::from_str(&env, "TimestampBefore"),
            ConditionType::OraclePrice(_, _, _, _) => String::from_str(&env, "OraclePrice"),
            ConditionType::CrossContractState(_, _) => String::from_str(&env, "CrossContractState"),
        };

        (ConditionalPaymentCreated {
            payment_id,
            condition_type: condition_type_str,
        })
        .publish(&env);

        Ok(payment_id)
    }

    pub fn evaluate_condition(env: Env, payment_id: u64) -> Result<bool, Error> {
        let mut conditional_payment: ConditionalPayment = env
            .storage()
            .instance()
            .get(&StateDataKey::ConditionalPayment(payment_id))
            .ok_or(Error::PaymentNotFound)?;

        // Return cached result if already evaluated
        if let Some(_evaluated_at) = conditional_payment.evaluated_at {
            return Ok(conditional_payment.condition_met);
        }

        let current_timestamp = env.ledger().timestamp();
        let condition_met = match &conditional_payment.condition {
            ConditionType::TimestampAfter(timestamp) => current_timestamp > *timestamp,
            ConditionType::TimestampBefore(timestamp) => current_timestamp < *timestamp,
            ConditionType::OraclePrice(_oracle_contract, _asset, _threshold, _comparison) => {
                // Mock oracle call - in real implementation this would call the oracle contract
                // For now, return false to indicate oracle call failed
                return Err(Error::OracleCallFailed);
            }
            ConditionType::CrossContractState(target_contract, expected_state_hash) => {
                let fetched = env
                    .try_invoke_contract::<BytesN<32>, Error>(
                        target_contract,
                        &Symbol::new(&env, "get_state_hash"),
                        Vec::new(&env),
                    )
                    .map_err(|_| Error::ConditionEvaluationFailed)?
                    .map_err(|_| Error::ConditionEvaluationFailed)?;
                fetched == *expected_state_hash
            }
        };

        // Cache the result
        conditional_payment.condition_met = condition_met;
        conditional_payment.evaluated_at = Some(current_timestamp);

        env.storage().instance().set(
            &StateDataKey::ConditionalPayment(payment_id),
            &conditional_payment,
        );

        (ConditionEvaluated {
            payment_id,
            met: condition_met,
            evaluated_at: current_timestamp,
        })
        .publish(&env);

        Ok(condition_met)
    }

    pub fn complete_conditional_payment(
        env: Env,
        admin: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        // Check if conditional payment exists
        if !env
            .storage()
            .instance()
            .has(&StateDataKey::ConditionalPayment(payment_id))
        {
            return Err(Error::PaymentNotFound);
        }

        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        // Check if payment is expired
        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        // Evaluate condition
        let condition_met = PaymentContract::evaluate_condition(env.clone(), payment_id)?;
        if !condition_met {
            return Err(Error::ConditionNotMet);
        }

        // Complete the payment
        PaymentContract::do_complete_payment(&env, payment_id)?;

        Ok(())
    }

    pub fn execute_if_condition_met(env: Env, payment_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&StateDataKey::ConditionalPayment(payment_id))
        {
            return Err(Error::PaymentNotFound);
        }
        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status == PaymentStatus::Completed {
            return Ok(());
        }
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        let condition_met = PaymentContract::evaluate_condition(env.clone(), payment_id)?;
        if !condition_met {
            return Err(Error::ConditionNotMet);
        }
        PaymentContract::do_complete_payment(&env, payment_id)
    }

    pub fn get_conditional_payment(env: Env, payment_id: u64) -> Result<ConditionalPayment, Error> {
        env.storage()
            .instance()
            .get(&StateDataKey::ConditionalPayment(payment_id))
            .ok_or(Error::PaymentNotFound)
    }

    // ── ANALYTICS FUNCTIONS ────────────────────────────────────────────────

    pub fn get_payment_analytics(env: Env) -> PaymentAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::PaymentAnalyticsKey)
            .unwrap_or(PaymentAnalytics {
                total_payments_created: 0,
                total_payments_completed: 0,
                total_payments_cancelled: 0,
                total_payments_refunded: 0,
                total_volume: 0,
                total_refunded_volume: 0,
                unique_customers: 0,
                unique_merchants: 0,
            })
    }

    pub fn get_merchant_analytics(env: Env, merchant: Address) -> MerchantAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(merchant))
            .unwrap_or(MerchantAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_completed: 0,
                total_cancelled: 0,
                total_refunded: 0,
                total_refunded_volume: 0,
            })
    }

    pub fn get_customer_analytics(env: Env, customer: Address) -> CustomerAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::CustomerAnalytics(customer))
            .unwrap_or(PaymentContract::default_customer_analytics())
    }

    pub fn get_customer_top_merchants(
        env: Env,
        customer: Address,
        limit: u32,
    ) -> Vec<(Address, i128)> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerMerchantCount(customer.clone()))
            .unwrap_or(0);

        let mut pairs: Vec<(Address, i128)> = Vec::new(&env);
        for i in 0..count {
            if let Some(merchant) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::CustomerMerchantList(customer.clone(), i))
            {
                let vol: i128 = env
                    .storage()
                    .instance()
                    .get(&DataKey::CustomerMerchantVolume(
                        customer.clone(),
                        merchant.clone(),
                    ))
                    .unwrap_or(0);
                pairs.push_back((merchant, vol));
            }
        }

        // Select top `limit` by descending volume (selection without in-place swap)
        let result_len = core::cmp::min(pairs.len(), limit);
        let mut result: Vec<(Address, i128)> = Vec::new(&env);
        let mut selected: Vec<bool> = Vec::new(&env);
        for _ in 0..pairs.len() {
            selected.push_back(false);
        }

        for _ in 0..result_len {
            let mut best_idx: u32 = 0;
            let mut best_vol: i128 = -1;
            for j in 0..pairs.len() {
                if !selected.get(j).unwrap_or(true) {
                    let vol = pairs.get(j).map(|(_, v)| v).unwrap_or(0);
                    if vol > best_vol {
                        best_vol = vol;
                        best_idx = j;
                    }
                }
            }
            if best_vol >= 0 {
                result.push_back(pairs.get(best_idx).unwrap());
                selected.set(best_idx, true);
            }
        }
        result
    }

    pub fn get_customer_monthly_volume(env: Env, customer: Address, month_timestamp: u64) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::CustomerMonthlyVolume(customer, month_timestamp))
            .unwrap_or(0)
    }

    pub fn get_merchant_analytics_range(
        env: Env,
        merchant: Address,
        from: u64,
        to: u64,
    ) -> Result<Vec<AnalyticsBucket>, Error> {
        if from >= to {
            return Err(Error::InvalidStatus);
        }
        let mut out = Vec::new(&env);
        let mut bucket_start = PaymentContract::hour_bucket_start(from);
        while bucket_start < to {
            if let Some(bucket) = env.storage().instance().get::<DataKey, AnalyticsBucket>(
                &DataKey::MerchantAnalyticsBucket(merchant.clone(), bucket_start),
            ) {
                out.push_back(bucket);
            }
            bucket_start += 3600;
        }
        Ok(out)
    }

    pub fn get_platform_analytics_daily(env: Env, day_timestamp: u64) -> AnalyticsBucket {
        let day_start = PaymentContract::day_bucket_start(day_timestamp);
        env.storage()
            .instance()
            .get(&DataKey::PlatformAnalyticsDaily(day_start))
            .unwrap_or(AnalyticsBucket {
                bucket_start: day_start,
                bucket_end: day_start + 86400,
                total_payments: 0,
                total_volume: 0,
                total_refunds: 0,
                failed_count: 0,
            })
    }

    pub fn get_top_merchants_by_volume(env: Env, limit: u32) -> Vec<(Address, i128)> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::GlobalMerchantCount)
            .unwrap_or(0);
        let mut pairs: Vec<(Address, i128)> = Vec::new(&env);
        for i in 0..count {
            if let Some(merchant) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::GlobalMerchantList(i))
            {
                let analytics =
                    PaymentContract::get_merchant_analytics(env.clone(), merchant.clone());
                pairs.push_back((merchant, analytics.total_volume));
            }
        }

        let result_len = core::cmp::min(pairs.len(), limit);
        let mut result: Vec<(Address, i128)> = Vec::new(&env);
        let mut selected: Vec<bool> = Vec::new(&env);
        for _ in 0..pairs.len() {
            selected.push_back(false);
        }
        for _ in 0..result_len {
            let mut best_idx: u32 = 0;
            let mut best_vol: i128 = i128::MIN;
            for j in 0..pairs.len() {
                if !selected.get(j).unwrap_or(true) {
                    let vol = pairs.get(j).map(|(_, v)| v).unwrap_or(0);
                    if vol > best_vol {
                        best_vol = vol;
                        best_idx = j;
                    }
                }
            }
            if best_vol != i128::MIN {
                result.push_back(pairs.get(best_idx).unwrap());
                selected.set(best_idx, true);
            }
        }
        result
    }

    fn hour_bucket_start(ts: u64) -> u64 {
        (ts / 3600) * 3600
    }

    fn day_bucket_start(ts: u64) -> u64 {
        (ts / 86400) * 86400
    }

    fn update_merchant_bucket(
        env: &Env,
        merchant: Address,
        ts: u64,
        payment_delta: u64,
        volume_delta: i128,
        refund_delta: i128,
        failed_delta: u64,
    ) {
        let bucket_start = PaymentContract::hour_bucket_start(ts);
        let mut bucket: AnalyticsBucket = env
            .storage()
            .instance()
            .get(&DataKey::MerchantAnalyticsBucket(
                merchant.clone(),
                bucket_start,
            ))
            .unwrap_or(AnalyticsBucket {
                bucket_start,
                bucket_end: bucket_start + 3600,
                total_payments: 0,
                total_volume: 0,
                total_refunds: 0,
                failed_count: 0,
            });
        bucket.total_payments += payment_delta;
        bucket.total_volume += volume_delta;
        bucket.total_refunds += refund_delta;
        bucket.failed_count += failed_delta;
        env.storage().instance().set(
            &DataKey::MerchantAnalyticsBucket(merchant, bucket_start),
            &bucket,
        );
    }

    fn update_platform_daily_bucket(
        env: &Env,
        ts: u64,
        volume_delta: i128,
        refund_delta: i128,
        failed_delta: u64,
    ) {
        let day_start = PaymentContract::day_bucket_start(ts);
        let mut bucket: AnalyticsBucket = env
            .storage()
            .instance()
            .get(&DataKey::PlatformAnalyticsDaily(day_start))
            .unwrap_or(AnalyticsBucket {
                bucket_start: day_start,
                bucket_end: day_start + 86400,
                total_payments: 0,
                total_volume: 0,
                total_refunds: 0,
                failed_count: 0,
            });
        if volume_delta > 0 {
            bucket.total_payments += 1;
        }
        bucket.total_volume += volume_delta;
        bucket.total_refunds += refund_delta;
        bucket.failed_count += failed_delta;
        env.storage()
            .instance()
            .set(&DataKey::PlatformAnalyticsDaily(day_start), &bucket);
    }

    fn default_customer_analytics() -> CustomerAnalytics {
        CustomerAnalytics {
            total_payments: 0,
            total_volume: 0,
            total_refunds: 0,
            avg_transaction_size: 0,
            peak_hour: 0,
            top_merchant: None,
            top_merchant_volume: 0,
            first_payment_at: 0,
            last_payment_at: 0,
        }
    }

    // ── PAUSE FUNCTIONS ────────────────────────────────────────────────────

    pub fn pause_contract(env: Env, admin: Address, reason: String) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        let now = env.ledger().timestamp();
        let pause_state = if let Some(mut state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey)
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
            .set(&DataKey::PauseStateKey, &pause_state);
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PauseHistoryCount)
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
            .set(&StateDataKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&DataKey::PauseHistoryCount, &(history_count + 1));
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
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        if let Some(mut state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey)
        {
            state.globally_paused = false;
            env.storage()
                .instance()
                .set(&DataKey::PauseStateKey, &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PauseHistoryCount)
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
            .set(&StateDataKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&DataKey::PauseHistoryCount, &(history_count + 1));
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
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        let now = env.ledger().timestamp();
        let mut pause_state = if let Some(state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey)
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
        // Idempotent: only add if not already in list
        if !pause_state.paused_functions.contains(&function_name) {
            pause_state
                .paused_functions
                .push_back(function_name.clone());
        }
        env.storage()
            .instance()
            .set(&DataKey::PauseStateKey, &pause_state);
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PauseHistoryCount)
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
            .set(&StateDataKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&DataKey::PauseHistoryCount, &(history_count + 1));
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
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;
        if !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        if let Some(mut state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey)
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
                .set(&DataKey::PauseStateKey, &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PauseHistoryCount)
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
            .set(&StateDataKey::PauseHistoryEntry(history_count), &entry);
        env.storage()
            .instance()
            .set(&DataKey::PauseHistoryCount, &(history_count + 1));
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
            .get(&DataKey::PauseStateKey)
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
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey)
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

    fn require_not_paused(env: &Env, function_name: &str) -> Result<(), Error> {
        if let Some(state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey)
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

    // ── LARGE PAYMENT MULTI-SIG FUNCTIONS ─────────────────────────────────────

    pub fn set_large_payment_threshold(
        env: Env,
        admin: Address,
        threshold: i128,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        env.storage()
            .instance()
            .set(&DataKey::LargePaymentThreshold, &threshold);

        (LargePaymentThresholdUpdated {
            threshold,
            updated_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    pub fn get_large_payment_threshold(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::LargePaymentThreshold)
            .unwrap_or(0) // Default threshold of 0 disables multi-sig requirement
    }

    pub fn propose_large_payment(
        env: Env,
        merchant: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        merchant.require_auth();

        // Verify payment exists and belongs to merchant
        let payment: Payment = env
            .storage()
            .instance()
            .get(&DataKey::Payment(payment_id))
            .ok_or(Error::PaymentNotFound)?;

        if payment.merchant != merchant {
            return Err(Error::Unauthorized);
        }

        // Check if payment amount exceeds threshold
        let threshold: i128 = PaymentContract::get_large_payment_threshold(env.clone());
        if threshold == 0 || payment.amount <= threshold {
            return Err(Error::PaymentRequiresMultiSig);
        }

        // Check if proposal already exists
        if env
            .storage()
            .instance()
            .get::<DataKey, LargePaymentProposal>(&StateDataKey::LargePaymentProposal(payment_id))
            .is_some()
        {
            return Err(Error::AlreadyProcessed);
        }

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        let now = env.ledger().timestamp();
        let expires_at = now + config.proposal_ttl;

        let mut approvals = Vec::new(&env);
        approvals.push_back(merchant.clone());

        let proposal = LargePaymentProposal {
            payment_id,
            approvals,
            required: config.required_signatures,
            proposed_at: now,
            expires_at,
            executed: false,
        };

        env.storage()
            .instance()
            .set(&StateDataKey::LargePaymentProposal(payment_id), &proposal);

        (LargePaymentProposed {
            payment_id,
            proposer: merchant,
            required_approvals: config.required_signatures,
            expires_at,
        })
        .publish(&env);

        Ok(())
    }

    pub fn approve_large_payment(
        env: Env,
        approver: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        approver.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        if !config.admins.contains(&approver) {
            return Err(Error::NotAnAdmin);
        }

        let mut proposal: LargePaymentProposal = env
            .storage()
            .instance()
            .get(&StateDataKey::LargePaymentProposal(payment_id))
            .ok_or(Error::PaymentNotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyProcessed);
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::PaymentProposalExpired);
        }

        if proposal.approvals.contains(&approver) {
            return Err(Error::AlreadyApproved);
        }

        proposal.approvals.push_back(approver.clone());

        env.storage()
            .instance()
            .set(&StateDataKey::LargePaymentProposal(payment_id), &proposal);

        (LargePaymentApproved {
            payment_id,
            approver,
            approval_count: proposal.approvals.len() as u32,
        })
        .publish(&env);

        Ok(())
    }

    pub fn execute_large_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::MultiSigConfig)
            .ok_or(Error::MultiSigNotInitialized)?;

        let mut proposal: LargePaymentProposal = env
            .storage()
            .instance()
            .get(&StateDataKey::LargePaymentProposal(payment_id))
            .ok_or(Error::PaymentNotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyProcessed);
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::PaymentProposalExpired);
        }

        if proposal.approvals.len() < proposal.required {
            return Err(Error::InsufficientPaymentApprovals);
        }

        // Get the payment and execute it
        let mut payment: Payment = env
            .storage()
            .instance()
            .get(&DataKey::Payment(payment_id))
            .ok_or(Error::PaymentNotFound)?;

        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        // Execute the payment
        let token_client = token::Client::new(&env, &payment.token);
        let contract_address = env.current_contract_address();

        token_client.transfer(&contract_address, &payment.merchant, &payment.amount);

        payment.status = PaymentStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        proposal.executed = true;
        env.storage()
            .instance()
            .set(&StateDataKey::LargePaymentProposal(payment_id), &proposal);

        (PaymentCompleted {
            payment_id,
            merchant: payment.merchant,
            amount: payment.amount,
        })
        .publish(&env);

        (LargePaymentExecuted { payment_id }).publish(&env);

        Ok(())
    }

    pub fn get_large_payment_proposal(env: Env, payment_id: u64) -> LargePaymentProposal {
        env.storage()
            .instance()
            .get(&StateDataKey::LargePaymentProposal(payment_id))
            .expect("Large payment proposal not found")
    }

    /// Set payment metadata with encrypted content reference
    /// Only payment customer, merchant, or admin can set metadata
    /// Content hash is immutable after first set
    pub fn set_payment_metadata(
        env: Env,
        caller: Address,
        payment_id: u64,
        content_ref: String,
        content_hash: BytesN<32>,
        encrypted: bool,
    ) -> Result<(), Error> {
        caller.require_auth();

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let payment = PaymentContract::get_payment(&env, payment_id);

        // Verify caller is customer, merchant, or admin
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != payment.customer && caller != payment.merchant && caller != admin {
            return Err(Error::Unauthorized);
        }

        // Check if metadata already exists
        let existing_metadata: Option<PaymentMetadata> = env
            .storage()
            .instance()
            .get(&DataKey::PaymentMetadata(payment_id));

        if let Some(existing) = existing_metadata {
            // Metadata already set - emit update event with new version
            let new_version = existing.version + 1;
            let updated_metadata = PaymentMetadata {
                payment_id,
                content_ref: content_ref.clone(),
                content_hash: existing.content_hash, // Keep original hash immutable
                encrypted,
                updated_at: env.ledger().timestamp(),
                version: new_version,
            };

            env.storage()
                .instance()
                .set(&DataKey::PaymentMetadata(payment_id), &updated_metadata);

            PaymentMetadataUpdated {
                payment_id,
                content_ref,
                updated_by: caller,
                version: new_version,
            }
            .publish(&env);
        } else {
            // First time setting metadata
            let metadata = PaymentMetadata {
                payment_id,
                content_ref: content_ref.clone(),
                content_hash,
                encrypted,
                updated_at: env.ledger().timestamp(),
                version: 1,
            };

            env.storage()
                .instance()
                .set(&DataKey::PaymentMetadata(payment_id), &metadata);

            PaymentMetadataSet {
                payment_id,
                content_ref,
                encrypted,
                set_by: caller,
            }
            .publish(&env);
        }

        Ok(())
    }

    /// Get payment metadata
    pub fn get_payment_metadata(env: Env, payment_id: u64) -> Option<PaymentMetadata> {
        env.storage()
            .instance()
            .get(&DataKey::PaymentMetadata(payment_id))
    }

    /// Verify metadata integrity by comparing provided hash against stored hash
    /// Returns true if hashes match, false otherwise
    pub fn verify_metadata_integrity(
        env: Env,
        payment_id: u64,
        plaintext_hash: BytesN<32>,
    ) -> bool {
        let metadata: Option<PaymentMetadata> = env
            .storage()
            .instance()
            .get(&DataKey::PaymentMetadata(payment_id));

        match metadata {
            Some(meta) => meta.content_hash == plaintext_hash,
            None => false,
        }
    }

    // Dynamic fee calculation functions (#124)
    pub fn calculate_risk_score(env: Env, customer: Address, merchant: Address, amount: i128, currency: Currency) -> u32 {
        let config: RiskFeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::RiskFeeConfig)
            .unwrap_or(RiskFeeConfig {
                base_fee_bps: 100, // 1%
                large_amount_threshold: 1000000, // 10,000 USDC/USDT
                large_amount_surcharge_bps: 50, // 0.5%
                new_customer_surcharge_bps: 100, // 1%
                high_risk_currency_surcharge: 200, // 2%
            });

        let mut risk_score = 0;

        // Large amount risk
        if amount > config.large_amount_threshold {
            risk_score += config.large_amount_surcharge_bps;
        }

        // New customer risk (simplified - in production would check payment history)
        let customer_payment_count: u64 = env
            .storage()
            .instance()
            .get(&CustomerDataKey::PaymentCount(customer))
            .unwrap_or(0);
        if customer_payment_count < 3 {
            risk_score += config.new_customer_surcharge_bps;
        }

        // High risk currency
        match currency {
            Currency::BTC | Currency::ETH => {
                risk_score += config.high_risk_currency_surcharge;
            }
            _ => {}
        }

        risk_score
    }

    pub fn get_effective_fee_for_payment(env: Env, customer: Address, merchant: Address, amount: i128, currency: Currency) -> u32 {
        let config: RiskFeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::RiskFeeConfig)
            .unwrap_or(RiskFeeConfig {
                base_fee_bps: 100, // 1%
                large_amount_threshold: 1000000,
                large_amount_surcharge_bps: 50,
                new_customer_surcharge_bps: 100,
                high_risk_currency_surcharge: 200,
            });

        let risk_surcharge = Self::calculate_risk_score(env, customer, merchant, amount, currency);
        let total_fee = config.base_fee_bps + risk_surcharge;

        // Fee cap at 1000 bps (10%)
        if total_fee > 1000 {
            1000
        } else {
            total_fee
        }
    }

    pub fn set_risk_fee_config(env: Env, admin: Address, config: RiskFeeConfig) -> Result<(), Error> {
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

        // Validate configuration
        if config.base_fee_bps > 1000 {
            return Err(Error::InvalidFeeConfig);
        }
        if config.large_amount_threshold <= 0 {
            return Err(Error::InvalidFeeConfig);
        }

        env.storage()
            .instance()
            .set(&DataKey::RiskFeeConfig, &config);
        Ok(())
    }

    /// Toggle proration for a subscription. Only the customer can change this.
    pub fn set_subscription_proration(
        env: Env,
        customer: Address,
        subscription_id: u64,
        enabled: bool,
    ) -> Result<(), Error> {
        customer.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Unauthorized);
        }

        sub.pause_data.proration_enabled = enabled;
        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        Ok(())
    }

    // ── PAYMENT CHANNEL METHODS (#125) ──────────────────────────────────────

    pub fn open_channel(
        env: Env,
        customer: Address,
        merchant: Address,
        token: Address,
        amount: i128,
        expires_at: u64,
    ) -> Result<u64, Error> {
        customer.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&customer, &contract_address, &amount);

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentChannelCounter)
            .unwrap_or(0);
        let channel_id = counter + 1;

        let channel = PaymentChannel {
            channel_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            token,
            deposited: amount,
            settled: 0,
            nonce: 0,
            open: true,
            expires_at,
        };

        env.storage()
            .instance()
            .set(&DataKey::PaymentChannel(channel_id), &channel);
        env.storage()
            .instance()
            .set(&DataKey::PaymentChannelCounter, &channel_id);

        (ChannelOpened {
            channel_id,
            customer,
            merchant,
            amount,
        })
        .publish(&env);

        Ok(channel_id)
    }

    pub fn settle_channel(
        env: Env,
        channel_id: u64,
        merchant_amount: i128,
        nonce: u64,
        signature: BytesN<64>,
    ) -> Result<(), Error> {
        let mut channel: PaymentChannel = env
            .storage()
            .instance()
            .get(&DataKey::PaymentChannel(channel_id))
            .ok_or(Error::ChannelNotFound)?;

        if !channel.open {
            return Err(Error::ChannelClosed);
        }

        if channel.expires_at > 0 && env.ledger().timestamp() > channel.expires_at {
            return Err(Error::ChannelExpired);
        }

        if nonce <= channel.nonce {
            return Err(Error::InvalidNonce);
        }

        if merchant_amount > channel.deposited {
            return Err(Error::InvalidAmount);
        }

        // Verify signature over (channel_id, merchant_amount, nonce)
        let mut msg = Bytes::new(&env);
        msg.append(&channel_id.to_xdr(&env));
        msg.append(&merchant_amount.to_xdr(&env));
        msg.append(&nonce.to_xdr(&env));

        let pk = Self::extract_public_key(&env, &channel.customer);
        env.crypto().ed25519_verify(&pk, &msg, &signature);

        let customer_refund = channel.deposited - merchant_amount;
        let token_client = token::Client::new(&env, &channel.token);
        let contract_address = env.current_contract_address();

        if merchant_amount > 0 {
            token_client.transfer(&contract_address, &channel.merchant, &merchant_amount);
        }
        if customer_refund > 0 {
            token_client.transfer(&contract_address, &channel.customer, &customer_refund);
        }

        channel.settled = merchant_amount;
        channel.nonce = nonce;
        channel.open = false;

        env.storage()
            .instance()
            .set(&DataKey::PaymentChannel(channel_id), &channel);

        (ChannelSettled {
            channel_id,
            merchant_amount,
            customer_refund,
        })
        .publish(&env);

        Ok(())
    }

    pub fn close_channel_expired(env: Env, channel_id: u64) -> Result<(), Error> {
        let mut channel: PaymentChannel = env
            .storage()
            .instance()
            .get(&DataKey::PaymentChannel(channel_id))
            .ok_or(Error::ChannelNotFound)?;

        if !channel.open {
            return Err(Error::ChannelClosed);
        }

        if channel.expires_at == 0 || env.ledger().timestamp() <= channel.expires_at {
            return Err(Error::ChannelNotExpired);
        }

        let refund_amount = channel.deposited;
        let token_client = token::Client::new(&env, &channel.token);
        let contract_address = env.current_contract_address();

        if refund_amount > 0 {
            token_client.transfer(&contract_address, &channel.customer, &refund_amount);
        }

        channel.open = false;
        env.storage()
            .instance()
            .set(&DataKey::PaymentChannel(channel_id), &channel);

        (ChannelExpiredClosed {
            channel_id,
            refunded_to: channel.customer.clone(),
        })
        .publish(&env);

        Ok(())
    }

    pub fn get_channel(env: Env, channel_id: u64) -> Result<PaymentChannel, Error> {
        env.storage()
            .instance()
            .get(&DataKey::PaymentChannel(channel_id))
            .ok_or(Error::ChannelNotFound)
    }

    fn extract_public_key(env: &Env, address: &Address) -> BytesN<32> {
        let xdr = address.to_xdr(env);
        // Ed25519 Account Address XDR: 0 (ScAddress Account) + 0 (AccountId Ed25519) + 32 bytes PK
        let mut pk = [0u8; 32];
        for i in 0..32 {
            pk[i] = xdr.get(8 + (i as u32)).unwrap();
        }
        BytesN::from_array(env, &pk)
    }
}

mod test;

#[cfg(test)]
mod test_analytics;

#[cfg(test)]
mod test_trial;

mod test_issue_113_115_119_120;
#[cfg(test)]
mod test_metadata;

#[cfg(test)]
mod test_payment_channel;

#[cfg(test)]
mod test_cross_contract_escrow_verification;
