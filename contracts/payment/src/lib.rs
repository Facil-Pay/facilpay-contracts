// This contract uses a multi-level enum structure for DataKey and Error to stay within
// Soroban's 50-variant XDR limit. Each sub-enum must have <= 50 variants.
#![no_std]
use escrow::EscrowContractClient;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, xdr::ToXdr, Address,
    Bytes, BytesN, Env, String, Symbol, TryFromVal, Val, Vec,
};

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum Currency {
    XLM,
    USDC,
    USDT,
    BTC,
    ETH,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum PayoutFrequency {
    Immediate,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Clone)]
#[contracttype]
pub enum ConfigKey {
    Admin,
    MultiSigConfig,
    FeeConfig,
    RateLimitConfig,
    DunningConfig,
    LoyaltyConfig,
    RiskFeeConfig,
    FinalityConfig,
    FeeRebateConfig,
    TierThresholds,
    LargePaymentThreshold,
    GlobalMerchantCount,
    PauseStateKey,
    MinSplitAmount,
    SchemaVersion,
    AllowedTokens,
    MaxForwardDepth,
}

#[derive(Clone)]
#[contracttype]
pub enum PaymentKey {
    Data(u64),
    Counter,
    Metadata(u64),
    Memo(u64),
    MemoVersion(u64),
    Tag(u64),
    Invoice(u64),
    InvoiceCounter,
    InvoicePaymentId(u64),
    PartialPaymentCounter(u64),
    OutstandingBalance(u64),
    PendingSettlement(u64),
    AccumulatedFees,
    LargePaymentCounter,
    Discount(u64),
}

pub const MAX_MEMO_VERSIONS: u32 = 10;

#[derive(Clone)]
#[contracttype]
pub enum SubscriptionKey {
    Data(u64),
    Counter,
    Metered(u64),
    MeteredCounter,
    Group(u64),
    GroupCounter,
    GroupMembership(u64),
}

#[derive(Clone)]
#[contracttype]
pub enum FeatureKey {
    PaymentAnalytics,
    PlatformAnalyticsDaily(u64),
    PaymentForwardConfig(Address),
    OracleRateConfig(Currency),
    ConversionRate(Currency),
    MerchantRateLimit(Address),
    CustomerLoyaltyBalance(Address),
    CustomerSpendLimit(Address),
    PaymentChannel(u64),
    PaymentChannelCounter,
    SplitConfig(u64),
    SweepRecipient,
    SweepCounter,
    SweepHistory(u64),
    RouteOptions(Address, Address),
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Config(ConfigKey),
    Payment(PaymentKey),
    Subscription(SubscriptionKey),
    Feature(FeatureKey),
    Customer(CustomerDataKey),
    Merchant(MerchantDataKey),
    State(StateDataKey),
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum BasicError {
    Unauthorized = 100,
    MetadataTooLarge = 101,
    NotesTooLarge = 102,
    InvalidCurrency = 103,
    InvalidBatchSize = 104,
    BatchPartialFailure = 105,
    RateLimitExceeded = 106,
    DailyVolumeExceeded = 107,
    AddressFlagged = 108,
    AddressAlreadyFlagged = 109,
    AmountExceedsLimit = 110,
    MultiSigNotInitialized = 111,
    InsufficientAdmins = 112,
    NotAnAdmin = 113,
    AlreadyApproved = 114,
    OracleCallFailed = 115,
    ContractPaused = 116,
    FunctionPaused = 117,
    InvalidTierThresholds = 118,
    OracleFeedStale = 119,
    OracleNotConfigured = 120,
    InvalidAmount = 121,
    VerificationLevelNotFound = 122,
    TierLimitsNotConfigured = 123,
    InvalidInterval = 124,
    InvalidBps = 125,
    SchemaAlreadyAtTarget = 126,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum PaymentError {
    NotFound = 200,
    InvalidStatus = 201,
    AlreadyProcessed = 202,
    Expired = 203,
    NotExpired = 204,
    NoExpiration = 205,
    TransferFailed = 206,
    RefundExceedsPayment = 207,
    NotYetDue = 208,
    ScheduledPaymentCancelled = 209,
    MetadataAlreadySet = 210,
    MetadataNotFound = 211,
    HashMismatch = 212,
    AlreadyFullyPaid = 213,
    InstallmentExceedsRemaining = 214,
    PartialPaymentNotFound = 215,
    MerchantRateLimitExceeded = 216,
    AmountRateLimitExceeded = 217,
    PayoutScheduleNotFound = 218,
    PayoutNotYetDue = 219,
    NothingToSettle = 220,
    BillingOverflow = 221,
    InvalidLineItem = 222,
    InvalidScheduleTime = 223,
    TokenNotAllowed = 224,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum SubscriptionError {
    NotFound = 300,
    NotActive = 301,
    PaymentNotDue = 302,
    MaxRetriesExceeded = 303,
    Ended = 304,
    DunningNotFound = 305,
    NotInDunning = 306,
    RetryNotDue = 307,
    GracePeriodExpired = 308,
    RetryTooEarly = 309,
    MeteredNotFound = 310,
    BillingCapExceeded = 311,
    GroupNotFound = 312,
    AlreadyInGroup = 313,
    GroupSizeLimitExceeded = 314,
    TrialExpired = 315,
    MaxTrialDurationExceeded = 316,
    MerchantPaused = 317,
    UsageCapExceeded = 318,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum ProposalError {
    NotFound = 400,
    Expired = 401,
    AlreadyExecuted = 402,
    ThresholdNotMet = 403,
    RequiresMultiSig = 404,
    InsufficientApprovals = 405,
    ProposalExpired = 406,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum FeatureError {
    EscrowMappingNotFound = 500,
    EscrowBridgeFailed = 501,
    FeeConfigNotFound = 502,
    InsufficientFees = 503,
    ConditionNotMet = 504,
    ConditionAlreadyEvaluated = 505,
    AutoEscrowRuleNotFound = 506,
    AutoEscrowBelowMinimum = 507,
    AutoEscrowAlreadyTriggered = 508,
    ConditionEvaluationFailed = 509,
    ConditionRuntimeNotMet = 510,
    InvalidFeeConfig = 511,
    ChannelNotFound = 512,
    InvalidSignature = 513,
    InvalidNonce = 514,
    ChannelClosed = 515,
    ChannelExpired = 516,
    ChannelNotExpired = 517,
    InvalidSplitShares = 518,
    TooManyRecipients = 519,
    InvalidCounterparty = 540,
    SplitConfigNotFound = 520,
    SplitAlreadyExecuted = 521,
    LoyaltyNotConfigured = 522,
    InsufficientPoints = 523,
    PointsExpired = 524,
    NothingToSweep = 525,
    SweepRecipientNotSet = 526,
    SpendLimitExceeded = 527,
    SpendLimitNotConfigured = 528,
    SettlementNotReady = 529,
    FinalityConfigNotFound = 530,
    SettlementAlreadyFinalized = 531,
    RebateThresholdNotMet = 532,
    RebateAlreadyClaimed = 533,
    RebateConfigNotFound = 534,
    ForwardConfigNotFound = 535,
    ForwardLoop = 536,
    InvalidForwardBps = 537,
    SenderIsRecipient = 538,
    BelowMinSplitAmount = 539,
    // Issue #385: claimed settlement amounts must sum exactly to the channel deposit.
    BalanceSumMismatch = 541,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    Basic(BasicError),
    Payment(PaymentError),
    Subscription(SubscriptionError),
    Proposal(ProposalError),
    Feature(FeatureError),
}

impl Error {
    /// Converts this error variant to its raw `u32` discriminant for Soroban's
    /// `InvokeError` compatibility.
    ///
    /// # Returns
    /// The `u32` discriminant corresponding to this error variant.
    pub fn to_u32(&self) -> u32 {
        match self {
            Error::Basic(e) => *e as u32,
            Error::Payment(e) => *e as u32,
            Error::Subscription(e) => *e as u32,
            Error::Proposal(e) => *e as u32,
            Error::Feature(e) => *e as u32,
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
        if error.is_type(soroban_sdk::xdr::ScErrorType::Contract) {
            let code = error.get_code();
            if code >= 500 && code <= 540 {
                return Ok(Error::Feature(unsafe { core::mem::transmute(code) }));
            }
            if code >= 400 && code <= 406 {
                return Ok(Error::Proposal(unsafe { core::mem::transmute(code) }));
            }
            if code >= 300 && code <= 318 {
                return Ok(Error::Subscription(unsafe { core::mem::transmute(code) }));
            }
            if code >= 200 && code <= 224 {
                return Ok(Error::Payment(unsafe { core::mem::transmute(code) }));
            }
            if code >= 100 && code <= 126 {
                return Ok(Error::Basic(unsafe { core::mem::transmute(code) }));
            }
        }
        Err(error)
    }
}

// impl FromVal<Env, Error> for Val {
//     fn from_val(env: &Env, v: &Error) -> Self {
//         soroban_sdk::Error::from(v).into_val(env)
//     }
// }

impl TryFromVal<Env, Val> for Error {
    type Error = soroban_sdk::ConversionError;
    fn try_from_val(env: &Env, val: &Val) -> Result<Self, Self::Error> {
        let error: soroban_sdk::Error =
            soroban_sdk::Error::try_from_val(env, val).map_err(|_| soroban_sdk::ConversionError)?;
        Error::try_from(error).map_err(|_| soroban_sdk::ConversionError)
    }
}

// Core payment data keys (≤50 variants for Soroban XDR spec limit)

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
    PayoutSchedule(Address),
    RebateAccrual(Address),
    PendingSettlementCount(Address),
    PendingSettlementIndex(Address, u64),
    VerificationLevel(Address),
    VerificationTierLimit(MerchantVerificationLevel),
    MerchantPaymentsPage(Address, u64),
    MerchantPaused(Address),
    MerchantActiveSubscriptions(Address, u64),
    MerchantActiveSubscriptionCount(Address),
    ActiveSubscriptionIndex(u64),
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
    PauseHistoryCount,
    // Auto-escrow
    AutoEscrowRule(Address),
    AutoEscrowTriggered(u64),
    PartialPaymentRecord(u64, u32), // payment_id, installment_number
    SettlementFinalized(u64),
    ScheduledPaymentCounter,
}

#[derive(Clone)]
#[contracttype]
pub struct PayoutSchedule {
    pub merchant: Address,
    pub token: Address,
    pub frequency: PayoutFrequency,
    pub next_payout_at: u64,
    pub accumulated: i128,
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

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TestError {
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
    AutoEscrowRuleNotFound = 46,
    AutoEscrowBelowMinimum = 47,
    AutoEscrowAlreadyTriggered = 48,
    PaymentNotYetDue = 54,
    ScheduledPaymentCancelled = 55,
    OracleFeedStale = 58,
    OracleNotConfigured = 59,
    ConditionEvaluationFailed = 62,
    ConditionRuntimeNotMet = 63,
    RetryTooEarly = 56,
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
    MeteredSubscriptionNotFound = 79,
    BillingCapExceeded = 80,
    InvalidSplitShares = 85,
    TooManyRecipients = 86,
    SplitConfigNotFound = 87,
    SplitAlreadyExecuted = 88,
    LoyaltyNotConfigured = 100,
    InsufficientPoints = 101,
    PointsExpired = 102,
    // Fee sweep (#216)
    NothingToSweep = 114,
    SweepRecipientNotSet = 115,
    // Customer spend limits (#217)
    SpendLimitExceeded = 116,
    SpendLimitNotConfigured = 117,
    // Subscription groups (#218)
    GroupNotFound = 118,
    SubscriptionAlreadyInGroup = 119,
    GroupSizeLimitExceeded = 120,
    // Finality delay (#219)
    SettlementNotReady = 121,
    FinalityConfigNotFound = 122,
    SettlementAlreadyFinalized = 123,
    VerificationLevelNotFound = 95,
    TierLimitsNotConfigured = 96,
    // Fee rebate programme
    RebateThresholdNotMet = 106,
    RebateAlreadyClaimed = 107,
    RebateConfigNotFound = 108,
    PayoutScheduleNotFound = 89,
    PayoutNotYetDue = 90,
    NothingToSettle = 91,
    // Payment forwarding (#220)
    ForwardConfigNotFound = 109,
    ForwardLoop = 110,
    InvalidForwardBps = 111,
    // Arithmetic safety
    BillingOverflow = 124,
    InvalidInterval = 125,
}

// Manual trait implementations replacing #[contracterror] (105 variants exceed the 50-variant XDR spec limit)
// impl From<Error> for soroban_sdk::Error {
//     #[inline(always)]
//     fn from(val: Error) -> soroban_sdk::Error {
//         <_ as From<&Error>>::from(&val)
//     }
// }
// impl From<&Error> for soroban_sdk::Error {
//     #[inline(always)]
//     fn from(val: &Error) -> soroban_sdk::Error {
//         soroban_sdk::Error::from_contract_error(*val.to_u32())
//     }
// }
// impl TryFrom<soroban_sdk::Error> for Error {
//     type Error = soroban_sdk::Error;
//     #[inline(always)]
//     fn try_from(error: soroban_sdk::Error) -> Result<Self, soroban_sdk::Error> {
//         if error.is_type(soroban_sdk::xdr::ScErrorType::Contract) {
//             let code = error.get_code();
//             if matches!(code, 1..=21 | 23..=48 | 50..=56 | 58..=80 | 85..=91 | 95..=96 | 100..=102 | 106..=111 | 114..=125)
//             {
//                 // SAFETY: Error is #[repr(u32)] and all valid discriminants are covered by the matches! guard above
//                 Ok(unsafe { core::mem::transmute::<u32, Error>(code) })
//             } else {
//                 Err(error)
//             }
//         } else {
//             Err(error)
//         }
//     }
// }
impl TryFrom<&soroban_sdk::Error> for Error {
    type Error = soroban_sdk::Error;
    #[inline(always)]
    fn try_from(error: &soroban_sdk::Error) -> Result<Self, soroban_sdk::Error> {
        <_ as TryFrom<soroban_sdk::Error>>::try_from(*error)
    }
}
impl From<Error> for soroban_sdk::InvokeError {
    #[inline(always)]
    fn from(val: Error) -> soroban_sdk::InvokeError {
        <_ as From<&Error>>::from(&val)
    }
}
// impl From<&Error> for soroban_sdk::InvokeError {
//     #[inline(always)]
//     fn from(val: &Error) -> soroban_sdk::InvokeError {
//         soroban_sdk::InvokeError::Contract(*val as u32)
//     }
// }

impl From<&Error> for soroban_sdk::InvokeError {
    fn from(e: &Error) -> Self {
        soroban_sdk::InvokeError::Contract(e.to_u32())
    }
}

impl TryFrom<soroban_sdk::InvokeError> for Error {
    type Error = soroban_sdk::InvokeError;
    #[inline(always)]
    fn try_from(error: soroban_sdk::InvokeError) -> Result<Self, soroban_sdk::InvokeError> {
        match error {
            soroban_sdk::InvokeError::Abort => Err(error),
            soroban_sdk::InvokeError::Contract(code) => {
                soroban_sdk::Error::from_contract_error(code)
                    .try_into()
                    .map_err(|_| error)
            }
        }
    }
}
impl TryFrom<&soroban_sdk::InvokeError> for Error {
    type Error = soroban_sdk::InvokeError;
    #[inline(always)]
    fn try_from(error: &soroban_sdk::InvokeError) -> Result<Self, soroban_sdk::InvokeError> {
        <_ as TryFrom<soroban_sdk::InvokeError>>::try_from(*error)
    }
}
// impl soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val> for Error {
//     type Error = soroban_sdk::ConversionError;
//     #[inline(always)]
//     fn try_from_val(
//         env: &soroban_sdk::Env,
//         val: &soroban_sdk::Val,
//     ) -> Result<Self, soroban_sdk::ConversionError> {
//         use soroban_sdk::TryIntoVal;
//         let error: soroban_sdk::Error = val.try_into_val(env)?;
//         error.try_into().map_err(|_| soroban_sdk::ConversionError)
//     }
// }
impl soroban_sdk::TryFromVal<soroban_sdk::Env, Error> for soroban_sdk::Val {
    type Error = soroban_sdk::ConversionError;
    #[inline(always)]
    fn try_from_val(
        _env: &soroban_sdk::Env,
        val: &Error,
    ) -> Result<Self, soroban_sdk::ConversionError> {
        let error: soroban_sdk::Error = val.into();
        Ok(error.into())
    }
}
impl soroban_sdk::TryFromVal<soroban_sdk::Env, &Error> for soroban_sdk::Val {
    type Error = soroban_sdk::ConversionError;
    #[inline(always)]
    fn try_from_val(
        env: &soroban_sdk::Env,
        val: &&Error,
    ) -> Result<Self, soroban_sdk::ConversionError> {
        <_ as soroban_sdk::TryFromVal<soroban_sdk::Env, Error>>::try_from_val(env, *val)
    }
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
pub struct PaymentDisputeResolved {
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
    pub settled_nonce: u64,
    pub open: bool,
    pub expires_at: u64,
    pub customer_pk: BytesN<32>,
}

#[derive(Clone)]
#[contracttype]
pub struct MeteredSubscription {
    pub subscription_id: u64,
    pub merchant: Address,
    pub customer: Address,
    pub token: Address,
    pub price_per_unit: i128,
    pub unit_name: String,
    pub accumulated_units: u64,
    pub billing_cap: Option<i128>,
    pub last_reset_at: u64,
    pub max_units_per_period: Option<u64>,
}

#[derive(Clone)]
#[contracttype]
pub struct SplitRecipient {
    pub address: Address,
    pub share_bps: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentSplitConfig {
    pub payment_id: u64,
    pub recipients: Vec<SplitRecipient>,
    pub executed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct PaymentForwardConfig {
    pub merchant: Address,
    pub forward_to: Address,
    pub forward_bps: u32,
    pub active: bool,
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
pub struct ChannelToppedUp {
    pub channel_id: u64,
    pub amount: i128,
    pub new_deposited: i128,
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
pub struct UsageReported {
    pub subscription_id: u64,
    pub units: u64,
    pub accumulated: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeteredBillingExecuted {
    pub subscription_id: u64,
    pub amount: i128,
    pub units_billed: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillingCapReached {
    pub subscription_id: u64,
    pub cap: i128,
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
pub struct SubscriptionResumedProrated {
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

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum MerchantVerificationLevel {
    Unverified,
    Basic,
    Standard,
    Premium,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct VerificationTierLimits {
    pub level: MerchantVerificationLevel,
    pub tx_per_period: u32,
    pub volume_limit: i128,
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
pub struct AutoEscrowRule {
    pub merchant: Address,
    pub escrow_bps: u32,
    pub min_amount: i128,
    pub token: Address,
    pub active: bool,
    pub escrow_contract: Address,
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

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct LoyaltyConfig {
    pub points_per_unit: u32,
    pub redemption_rate: u32,
    pub expiry_seconds: u64,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct CustomerLoyaltyBalance {
    pub customer: Address,
    pub points: u64,
    pub last_updated: u64,
    pub expires_at: u64,
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
    /// Volume baseline set on manual tier downgrade; automatic upgrades only
    /// consider volume earned after this point.
    pub tier_volume_baseline: i128,
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

#[derive(Clone)]
#[contracttype]
pub struct FeeRebateConfig {
    pub threshold_volume: i128,
    pub rebate_bps: u32,
    pub rebate_period_seconds: u64,
    pub active: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct MerchantRebateAccrual {
    pub merchant: Address,
    pub accrued_rebate: i128,
    pub period_start: u64,
    pub period_volume: i128,
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
pub struct AutoEscrowTriggered {
    pub payment_id: u64,
    pub merchant: Address,
    pub escrow_id: u64,
    pub amount: i128,
    pub escrow_amount: i128,
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

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentMemoSet {
    pub payment_id: u64,
    pub memo_ref: String,
    pub set_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentMemoUpdated {
    pub payment_id: u64,
    pub memo_ref: String,
    pub updated_by: Address,
    pub version: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentMemoVerified {
    pub payment_id: u64,
    pub memo_hash: BytesN<32>,
    pub verified_at: u64,
    pub verified_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentForwardConfigSet {
    pub merchant: Address,
    pub forward_to: Address,
    pub forward_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentForwardConfigRemoved {
    pub merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentForwarded {
    pub payment_id: u64,
    pub merchant: Address,
    pub forward_to: Address,
    pub forward_amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentMemo {
    pub payment_id: u64,
    pub memo_ref: String,      // Reference to memo content (IPFS CID, URL, etc.)
    pub memo_hash: BytesN<32>, // SHA-256 hash of memo plaintext (immutable)
    pub reference_hash: BytesN<32>, // Hash linking memo to payment (for integrity)
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u32,
    pub created_by: Address,
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

#[contracttype]
#[derive(Clone)]
pub struct FeeSweepRecord {
    pub sweep_id: u64,
    pub amount: i128,
    pub token: Address,
    pub recipient: Address,
    pub swept_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CustomerSpendLimit {
    pub customer: Address,
    pub limit_amount: i128,
    pub period_seconds: u64,
    pub used: i128,
    pub period_start: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct SubscriptionGroup {
    pub group_id: u64,
    pub owner: Address,
    pub subscription_ids: Vec<u64>,
    pub discount_bps: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct FinalityConfig {
    pub delay_seconds: u64,
    pub min_amount_threshold: i128,
    pub active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct PendingSettlement {
    pub payment_id: u64,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub release_at: u64,
}

// Issue #118: Payment routing optimization
#[derive(Clone)]
#[contracttype]
pub struct RouteOption {
    pub input_token: Address,
    pub output_token: Address,
    pub input_amount: i128,
    pub output_amount: i128,
    pub fee_bps: u32,
    pub effective_cost: i128,
}

// Issue #210: Payment tagging system
#[derive(Clone)]
#[contracttype]
pub struct PaymentTagSet {
    pub payment_id: u64,
    pub tags: Vec<BytesN<32>>,
}

// Issue #205: Invoice-based payment with line items
#[derive(Clone)]
#[contracttype]
pub struct LineItem {
    pub description_hash: BytesN<32>,
    pub quantity: u32,
    pub unit_price: i128,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentInvoice {
    pub invoice_id: u64,
    pub payment_id: u64,
    pub items: Vec<LineItem>,
    pub subtotal: i128,
    pub tax: i128,
    pub total: i128,
    pub issued_at: u64,
}

#[contract]
pub struct PaymentContract;

// Constants for size limits
const MAX_METADATA_SIZE: u32 = 512;
const MAX_NOTES_SIZE: u32 = 1024;
const DEFAULT_MAX_RETRIES: u64 = 3;
const SECONDS_PER_DAY: u64 = 86400;
const MAX_TRIAL_DURATION: u64 = 90 * SECONDS_PER_DAY; // 90 days max trial

// Fee tier volume thresholds (raw token units)
const PREMIUM_VOLUME_THRESHOLD: i128 = 10_000;
const ENTERPRISE_VOLUME_THRESHOLD: i128 = 100_000;
const INITIAL_SCHEMA_VERSION: u32 = 1;

#[contractimpl]
impl PaymentContract {
    /// Initializes the payment contract with a single admin and default multisig config.
    ///
    /// # Arguments
    /// * `admin` - The initial admin address
    ///
    /// # Panics
    /// Panics if the contract has already been initialized.
    pub fn initialize(env: Env, admin: Address) {
        if env
            .storage()
            .instance()
            .has(&DataKey::Config(ConfigKey::MultiSigConfig))
        {
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
            .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);
        // Keep Admin key for backward compat
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::Admin), &admin);
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::SchemaVersion),
            &INITIAL_SCHEMA_VERSION,
        );
        (AdminAdded { admin }).publish(&env);
    }

    /// Returns the current schema version of the contract storage.
    ///
    /// # Returns
    /// The current schema version as a `u32`.
    pub fn get_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::SchemaVersion))
            .unwrap_or(INITIAL_SCHEMA_VERSION)
    }

    /// Migrates the contract storage schema to a target version.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing the migration (must be in the multisig admin list)
    /// * `target_version` - The schema version to migrate to
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the caller is not an admin, the target version
    /// is not greater than the current version, or multisig is not initialized.
    pub fn migrate_schema(env: Env, admin: Address, target_version: u32) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let current = Self::get_schema_version(env.clone());
        if current >= target_version {
            return Err(Error::Basic(BasicError::SchemaAlreadyAtTarget));
        }

        env.storage().instance().set(
            &DataKey::Config(ConfigKey::SchemaVersion),
            &target_version,
        );
        Ok(())
    }

    /// Sets the verification level for a specific merchant.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this change (must be in the multisig admin list)
    /// * `merchant` - The merchant whose verification level is being set
    /// * `level` - The new verification level to assign
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the caller is not an admin or multisig is not initialized.
    pub fn set_merchant_verification_level(
        env: Env,
        admin: Address,
        merchant: Address,
        level: MerchantVerificationLevel,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::VerificationLevel(merchant)),
            &level,
        );

        Ok(())
    }

    /// Retrieves the verification level for a given merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query
    ///
    /// # Returns
    /// The merchant's verification level, defaulting to `Unverified` if not set.
    pub fn get_merchant_verification_level(
        env: Env,
        merchant: Address,
    ) -> MerchantVerificationLevel {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::VerificationLevel(
                merchant,
            )))
            .unwrap_or(MerchantVerificationLevel::Unverified)
    }

    /// Configures the transaction limits for a specific verification tier.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this change (must be in the multisig admin list)
    /// * `limits` - The tier limits to store, including the tier level and its associated limits
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the caller is not an admin or multisig is not initialized.
    pub fn set_verification_tier_limits(
        env: Env,
        admin: Address,
        limits: VerificationTierLimits,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::VerificationTierLimit(limits.level.clone())),
            &limits,
        );

        Ok(())
    }

    /// Retrieves the transaction limits configured for a given verification tier.
    ///
    /// # Arguments
    /// * `level` - The verification tier level to query
    ///
    /// # Returns
    /// `Some(VerificationTierLimits)` if configured for the tier, or `None` otherwise.
    pub fn get_tier_limits(
        env: Env,
        level: MerchantVerificationLevel,
    ) -> Option<VerificationTierLimits> {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::VerificationTierLimit(
                level,
            )))
    }

    /// Returns the current multisig configuration.
    ///
    /// # Returns
    /// The `MultiSigConfig` containing admin list, required signatures, and proposal TTL.
    ///
    /// # Panics
    /// Panics if the multisig configuration has not been initialized.
    pub fn get_multisig_config(env: Env) -> MultiSigConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .expect("MultiSig not initialized")
    }

    /// Proposes a new admin action for multisig approval.
    ///
    /// # Arguments
    /// * `proposer` - The admin proposing the action (must be in the multisig admin list)
    /// * `action_type` - The type of action being proposed
    /// * `target` - The target address for the action
    /// * `data` - Arbitrary data payload for the action
    ///
    /// # Returns
    /// `Ok(proposal_id)` on success, or an error if the proposer is not an admin
    /// or multisig is not initialized.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&proposer) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::LargePaymentCounter))
            .unwrap_or(0)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::LargePaymentCounter), &counter);

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

        env.storage().instance().set(
            &DataKey::State(StateDataKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

        (ActionProposed {
            proposal_id: proposal_id.clone(),
            proposer,
            action_type,
        })
        .publish(&env);

        Ok(proposal_id)
    }

    /// Approves a pending admin proposal.
    ///
    /// # Arguments
    /// * `approver` - The admin approving the proposal (must be in the multisig admin list)
    /// * `proposal_id` - The ID of the proposal to approve
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the approver is not an admin, the proposal
    /// is not found, has already been executed/rejected, is expired, or was already approved
    /// by this admin.
    pub fn approve_action(env: Env, approver: Address, proposal_id: String) -> Result<(), Error> {
        approver.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&approver) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::AdminProposal(
                proposal_id.clone(),
            )))
            .ok_or(Error::Proposal(ProposalError::NotFound))?;

        if proposal.executed || proposal.rejected {
            return Err(Error::Proposal(ProposalError::AlreadyExecuted));
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Proposal(ProposalError::Expired));
        }

        if proposal.approvals.contains(&approver) {
            return Err(Error::Basic(BasicError::AlreadyApproved));
        }

        proposal.approvals.push_back(approver.clone());
        proposal.approval_count += 1;

        env.storage().instance().set(
            &DataKey::State(StateDataKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

        (ActionApproved {
            proposal_id,
            approver,
            approval_count: proposal.approval_count,
        })
        .publish(&env);

        Ok(())
    }

    /// Executes a fully approved admin proposal.
    ///
    /// # Arguments
    /// * `proposal_id` - The ID of the proposal to execute
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the proposal is not found, has already been
    /// executed/rejected, is expired, or has not reached the required signature threshold.
    pub fn execute_action(env: Env, proposal_id: String) -> Result<(), Error> {
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::AdminProposal(
                proposal_id.clone(),
            )))
            .ok_or(Error::Proposal(ProposalError::NotFound))?;

        if proposal.executed || proposal.rejected {
            return Err(Error::Proposal(ProposalError::AlreadyExecuted));
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Proposal(ProposalError::Expired));
        }

        if proposal.approval_count < config.required_signatures {
            return Err(Error::Proposal(ProposalError::ThresholdNotMet));
        }

        proposal.executed = true;
        env.storage().instance().set(
            &DataKey::State(StateDataKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

        PaymentContract::dispatch_action(&env, &proposal)?;

        (ActionExecuted { proposal_id }).publish(&env);

        Ok(())
    }

    /// Rejects a pending admin proposal, preventing its execution.
    ///
    /// # Arguments
    /// * `rejecter` - The admin rejecting the proposal (must be in the multisig admin list)
    /// * `proposal_id` - The ID of the proposal to reject
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the rejecter is not an admin, the proposal
    /// is not found, or has already been executed/rejected.
    pub fn reject_action(env: Env, rejecter: Address, proposal_id: String) -> Result<(), Error> {
        rejecter.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&rejecter) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::AdminProposal(
                proposal_id.clone(),
            )))
            .ok_or(Error::Proposal(ProposalError::NotFound))?;

        if proposal.executed || proposal.rejected {
            return Err(Error::Proposal(ProposalError::AlreadyExecuted));
        }

        proposal.rejected = true;
        env.storage().instance().set(
            &DataKey::State(StateDataKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

        (ActionRejected {
            proposal_id,
            rejected_by: rejecter,
        })
        .publish(&env);

        Ok(())
    }

    /// Adds a new admin to the multisig configuration.
    ///
    /// # Arguments
    /// * `caller` - The existing admin adding the new admin (must be in the multisig admin list)
    /// * `new_admin` - The address of the new admin to add
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the caller is not an admin or multisig is not initialized.
    /// No-op if the address is already an admin.
    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if !config.admins.contains(&new_admin) {
            config.admins.push_back(new_admin.clone());
            config.total_admins += 1;
            env.storage()
                .instance()
                .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);
            (AdminAdded { admin: new_admin }).publish(&env);
        }

        Ok(())
    }

    /// Removes an admin from the multisig configuration.
    ///
    /// # Arguments
    /// * `caller` - The existing admin performing the removal (must be in the multisig admin list)
    /// * `admin` - The address of the admin to remove
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the caller is not an admin, the target is not
    /// an admin, removing them would leave fewer admins than required signatures, or multisig
    /// is not initialized.
    pub fn remove_admin(env: Env, caller: Address, admin: Address) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if config.total_admins <= config.required_signatures {
            return Err(Error::Basic(BasicError::InsufficientAdmins));
        }

        let mut new_admins = Vec::new(&env);
        for a in config.admins.iter() {
            if a != admin {
                new_admins.push_back(a);
            }
        }

        if new_admins.len() == config.admins.len() {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        config.admins = new_admins;
        config.total_admins -= 1;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);
        (AdminRemoved { admin }).publish(&env);

        Ok(())
    }

    /// Updates the required number of admin signatures for multisig proposals.
    ///
    /// # Arguments
    /// * `caller` - The admin updating the requirement (must be in the multisig admin list)
    /// * `required` - The new number of required signatures (must be > 0 and <= total admins)
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the caller is not an admin, the required count
    /// is invalid, or multisig is not initialized.
    pub fn update_required_signatures(
        env: Env,
        caller: Address,
        required: u32,
    ) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if required == 0 || required > config.total_admins {
            return Err(Error::Basic(BasicError::InsufficientAdmins));
        }

        config.required_signatures = required;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);

        Ok(())
    }

    /// Creates a new payment from a customer to a merchant.
    ///
    /// # Arguments
    /// * `customer` - The customer making the payment (must authorize)
    /// * `merchant` - The merchant receiving the payment
    /// * `amount` - The payment amount in base token units
    /// * `token` - The SPL token address for the payment
    /// * `currency` - The fiat currency associated with the payment
    /// * `expiration_duration` - Seconds until the payment expires (0 for no expiration)
    /// * `metadata` - Arbitrary metadata string for the payment
    ///
    /// # Returns
    /// `Ok(payment_id)` on success, or an error if the token is not allowed, currency is
    /// invalid, metadata is too large, the customer is flagged, rate limits are exceeded,
    /// or the contract is paused.
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
        Self::require_merchant_not_paused(&env, &merchant)?;
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

    /// Schedules a future payment by escrowing tokens until the scheduled time.
    ///
    /// # Arguments
    /// * `customer` - The customer scheduling the payment (must authorize)
    /// * `merchant` - The merchant who will receive the payment
    /// * `token` - The SPL token address for the payment
    /// * `amount` - The payment amount in base token units
    /// * `scheduled_at` - The ledger timestamp at which the payment should be executed
    ///
    /// # Returns
    /// `Ok(payment_id)` on success, or an error if the amount is non-positive, the scheduled
    /// time is in the past, or the contract is paused.
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
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        let now = env.ledger().timestamp();
        if scheduled_at <= now {
            return Err(Error::Payment(PaymentError::InvalidScheduleTime));
        }

        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(&contract_address, &customer, &contract_address, &amount);

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::ScheduledPaymentCounter))
            .unwrap_or(0);
        let payment_id = counter + 1;
        let scheduled = ScheduledPayment {
            payment_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            token,
            amount,
            scheduled_at,
            executed: false,
            cancelled: false,
        };
        env.storage().instance().set(
            &DataKey::State(StateDataKey::ScheduledPayment(payment_id)),
            &scheduled,
        );
        env.storage().instance().set(
            &DataKey::State(StateDataKey::ScheduledPaymentCounter),
            &payment_id,
        );

        (PaymentCreated {
            payment_id,
            customer,
            merchant,
            amount,
        })
        .publish(&env);

        Ok(payment_id)
    }

    /// Executes a previously scheduled payment once its scheduled time has passed.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the scheduled payment to execute
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not found, has been cancelled,
    /// was already executed, is not yet due, the customer is flagged, spend limits are
    /// exceeded, or the contract is paused.
    pub fn execute_scheduled_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "execute_scheduled_payment")?;
        let mut scheduled: ScheduledPayment = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::ScheduledPayment(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;
        if scheduled.cancelled {
            return Err(Error::Payment(PaymentError::ScheduledPaymentCancelled));
        }
        if scheduled.executed {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }
        if env.ledger().timestamp() < scheduled.scheduled_at {
            return Err(Error::Payment(PaymentError::NotYetDue));
        }

        // Check customer spend limit (#282)
        PaymentContract::check_and_update_spend_limit(&env, &scheduled.customer, scheduled.amount)?;

        Self::settle_or_accumulate(
            &env,
            scheduled.merchant.clone(),
            scheduled.token.clone(),
            scheduled.amount,
        )?;
        scheduled.executed = true;
        env.storage().instance().set(
            &DataKey::State(StateDataKey::ScheduledPayment(payment_id)),
            &scheduled,
        );
        Ok(())
    }

    /// Cancels a scheduled payment and refunds the escrowed tokens to the customer.
    ///
    /// # Arguments
    /// * `caller` - The customer who scheduled the payment, or an admin (must authorize)
    /// * `payment_id` - The ID of the scheduled payment to cancel
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not found, has already been
    /// executed or cancelled, the caller is unauthorized, or the contract is paused.
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
            .get(&DataKey::State(StateDataKey::ScheduledPayment(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;
        if scheduled.executed {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }
        if scheduled.cancelled {
            return Err(Error::Payment(PaymentError::ScheduledPaymentCancelled));
        }
        if caller != scheduled.customer {
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Config(ConfigKey::Admin))
                .ok_or(Error::Basic(BasicError::Unauthorized))?;
            if caller != stored_admin {
                return Err(Error::Basic(BasicError::Unauthorized));
            }
        }

        let token_client = token::Client::new(&env, &scheduled.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &scheduled.customer, &scheduled.amount);
        scheduled.cancelled = true;
        env.storage().instance().set(
            &DataKey::State(StateDataKey::ScheduledPayment(payment_id)),
            &scheduled,
        );
        Ok(())
    }

    /// Retrieves the details of a scheduled payment.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the scheduled payment to retrieve
    ///
    /// # Returns
    /// `Ok(ScheduledPayment)` on success, or `Error::Payment(NotFound)` if not found.
    pub fn get_scheduled_payment(env: Env, payment_id: u64) -> Result<ScheduledPayment, Error> {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::ScheduledPayment(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))
    }

    fn settle_or_accumulate(
        env: &Env,
        merchant: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        // If merchant has a payout schedule for this token and it's not Immediate, accumulate
        if let Some(mut schedule) =
            env.storage()
                .instance()
                .get::<DataKey, PayoutSchedule>(&DataKey::Merchant(
                    MerchantDataKey::PayoutSchedule(merchant.clone()),
                ))
        {
            if schedule.token == token && schedule.frequency != PayoutFrequency::Immediate {
                schedule.accumulated += amount;
                env.storage().instance().set(
                    &DataKey::Merchant(MerchantDataKey::PayoutSchedule(merchant.clone())),
                    &schedule,
                );
                return Ok(());
            }
        }

        // Otherwise perform immediate transfer
        let token_client = token::Client::new(env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &merchant, &amount);
        Ok(())
    }

    /// Configures a merchant's payout schedule for accumulating payments before settlement.
    ///
    /// # Arguments
    /// * `merchant` - The merchant configuring the payout schedule (must authorize)
    /// * `frequency` - How often payouts should occur (Immediate, Daily, Weekly, Monthly)
    /// * `token` - The SPL token address to accumulate for payouts
    ///
    /// # Returns
    /// `Ok(())` on success.
    pub fn set_payout_schedule(
        env: Env,
        merchant: Address,
        frequency: PayoutFrequency,
        token: Address,
    ) -> Result<(), Error> {
        merchant.require_auth();
        let now = env.ledger().timestamp();
        let next = match frequency {
            PayoutFrequency::Immediate => now,
            PayoutFrequency::Daily => now + SECONDS_PER_DAY,
            PayoutFrequency::Weekly => now + SECONDS_PER_DAY * 7,
            PayoutFrequency::Monthly => now + SECONDS_PER_DAY * 30,
        };
        let schedule = PayoutSchedule {
            merchant: merchant.clone(),
            token: token.clone(),
            frequency,
            next_payout_at: next,
            accumulated: 0,
        };
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::PayoutSchedule(merchant)),
            &schedule,
        );
        Ok(())
    }

    /// Retrieves the payout schedule configuration for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query
    ///
    /// # Returns
    /// `Some(PayoutSchedule)` if configured, or `None` otherwise.
    pub fn get_payout_schedule(env: Env, merchant: Address) -> Option<PayoutSchedule> {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PayoutSchedule(
                merchant,
            )))
    }

    /// Triggers an accumulated payout for a merchant, transferring held funds.
    ///
    /// # Arguments
    /// * `merchant` - The merchant requesting the payout (must authorize)
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payout schedule is not found, the payout
    /// period has not elapsed, or there is nothing to settle.
    pub fn trigger_scheduled_payout(env: Env, merchant: Address) -> Result<(), Error> {
        merchant.require_auth();
        let mut schedule: PayoutSchedule = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PayoutSchedule(
                merchant.clone(),
            )))
            .ok_or(Error::Payment(PaymentError::PayoutScheduleNotFound))?;
        let now = env.ledger().timestamp();
        if now < schedule.next_payout_at {
            return Err(Error::Payment(PaymentError::PayoutNotYetDue));
        }
        if schedule.accumulated == 0 {
            return Err(Error::Payment(PaymentError::NothingToSettle));
        }
        let token_client = token::Client::new(&env, &schedule.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &merchant, &schedule.accumulated);
        schedule.accumulated = 0;
        let period = match schedule.frequency {
            PayoutFrequency::Immediate => SECONDS_PER_DAY,
            PayoutFrequency::Daily => SECONDS_PER_DAY,
            PayoutFrequency::Weekly => SECONDS_PER_DAY * 7,
            PayoutFrequency::Monthly => SECONDS_PER_DAY * 30,
        };
        schedule.next_payout_at = schedule.next_payout_at + period;
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::PayoutSchedule(merchant)),
            &schedule,
        );
        Ok(())
    }

    /// Returns the accumulated token balance for a merchant's payout schedule.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query
    ///
    /// # Returns
    /// The accumulated balance in base token units, or 0 if no payout schedule exists.
    pub fn get_accumulated_balance(env: Env, merchant: Address) -> i128 {
        env.storage()
            .instance()
            .get::<DataKey, PayoutSchedule>(&DataKey::Merchant(MerchantDataKey::PayoutSchedule(
                merchant,
            )))
            .map(|s: PayoutSchedule| s.accumulated)
            .unwrap_or(0)
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
        Self::require_merchant_not_paused(env, &merchant)?;
        if !PaymentContract::is_token_allowed(env, &token) {
            return Err(Error::Payment(PaymentError::TokenNotAllowed));
        }

        // Validate currency
        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::Basic(BasicError::InvalidCurrency));
        }

        // Validate metadata size
        if metadata.len() > MAX_METADATA_SIZE {
            return Err(Error::Basic(BasicError::MetadataTooLarge));
        }

        // Enforce sanctions/flag checks at creation entry point.
        if PaymentContract::is_address_flagged(env.clone(), customer.clone())
            && !PaymentContract::is_allowlisted(env, &customer)
        {
            return Err(Error::Basic(BasicError::AddressFlagged));
        }

        // Check rate limits and anti-fraud before processing
        PaymentContract::check_rate_limit_internal(env, &customer, amount)?;

        // Check merchant rate limits
        PaymentContract::check_merchant_rate_limit(env, &merchant, amount)?;

        // Check customer spend limit (#217)
        PaymentContract::check_and_update_spend_limit(env, &customer, amount)?;

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Counter))
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
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Counter), &payment_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::PaymentCount(
                customer.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Payments(customer.clone(), customer_count)),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::PaymentCount(customer)),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PaymentCount(
                merchant.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::Payments(merchant.clone(), merchant_count)),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::PaymentCount(merchant.clone())),
            &(merchant_count + 1),
        );

        // Paged merchant payment index (100 entries per page)
        {
            const PAGE_SIZE: u64 = 100;
            let page_num = merchant_count / PAGE_SIZE;
            let mut page: Vec<u64> = env
                .storage()
                .instance()
                .get(&DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(
                    merchant.clone(),
                    page_num,
                )))
                .unwrap_or_else(|| Vec::new(&env));
            page.push_back(payment_id);
            env.storage().instance().set(
                &DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(merchant, page_num)),
                &page,
            );
        }

        // Update global analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
                .get(&DataKey::Config(ConfigKey::GlobalMerchantCount))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Merchant(MerchantDataKey::GlobalList(global_count)),
                &payment.merchant.clone(),
            );
            env.storage().instance().set(
                &DataKey::Config(ConfigKey::GlobalMerchantCount),
                &(global_count + 1),
            );
        }
        env.storage()
            .instance()
            .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);

        // Update merchant analytics
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::Analytics(
                payment.merchant.clone(),
            )))
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
            &DataKey::Merchant(MerchantDataKey::Analytics(payment.merchant.clone())),
            &m_analytics,
        );

        // Update customer analytics
        let mut c_analytics: CustomerAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::Analytics(
                payment.customer.clone(),
            )))
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
            .get(&DataKey::Customer(CustomerDataKey::HourCount(
                payment.customer.clone(),
                hour,
            )))
            .unwrap_or(0)
            + 1;
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::HourCount(payment.customer.clone(), hour)),
            &hour_count,
        );
        let peak_hour_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::HourCount(
                payment.customer.clone(),
                c_analytics.peak_hour,
            )))
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
            .get(&DataKey::Customer(CustomerDataKey::MerchantVolume(
                payment.customer.clone(),
                payment.merchant.clone(),
            )))
            .unwrap_or(0);
        if prev_merchant_vol == 0 {
            // New merchant for this customer — add to list
            let m_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::Customer(CustomerDataKey::MerchantCount(
                    payment.customer.clone(),
                )))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Customer(CustomerDataKey::MerchantList(
                    payment.customer.clone(),
                    m_count,
                )),
                &payment.merchant,
            );
            env.storage().instance().set(
                &DataKey::Customer(CustomerDataKey::MerchantCount(payment.customer.clone())),
                &(m_count + 1),
            );
        }
        let new_merchant_vol = prev_merchant_vol + amount;
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::MerchantVolume(
                payment.customer.clone(),
                payment.merchant.clone(),
            )),
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
            .get(&DataKey::Customer(CustomerDataKey::MonthlyVolume(
                payment.customer.clone(),
                month_bucket,
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::MonthlyVolume(
                payment.customer.clone(),
                month_bucket,
            )),
            &(prev_monthly + amount),
        );

        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Analytics(payment.customer.clone())),
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

    /// Retrieves a payment by its ID.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to retrieve
    ///
    /// # Returns
    /// The `Payment` record.
    ///
    /// # Panics
    /// Panics if the payment is not found.
    pub fn get_payment(env: &Env, payment_id: u64) -> Payment {
        env.storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Data(payment_id)))
            .expect("Payment not found")
    }

    /// Used by the refund contract for cross-contract ownership verification (#143).
    /// Returns true if the payment exists, belongs to `customer`, and is Completed.
    pub fn check_payment_customer(env: Env, payment_id: u64, customer: Address) -> bool {
        let payment: Option<Payment> = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Data(payment_id)));
        match payment {
            Some(p) => p.customer == customer && p.status == PaymentStatus::Completed,
            None => false,
        }
    }

    /// Creates a payment backed by an escrow contract for dispute protection.
    ///
    /// # Arguments
    /// * `customer` - The customer making the payment
    /// * `merchant` - The merchant receiving the payment
    /// * `amount` - The payment amount in base token units
    /// * `token` - The SPL token address for the payment
    /// * `currency` - The fiat currency associated with the payment
    /// * `escrow_contract` - The address of the escrow contract to hold funds
    /// * `release_timestamp` - The timestamp when funds can be released from escrow
    /// * `min_hold_period` - Minimum time funds must remain in escrow
    /// * `metadata` - Arbitrary metadata string for the payment
    /// * `auto_release_on_complete` - Whether to auto-release escrow when payment completes
    ///
    /// # Returns
    /// `Ok((payment_id, escrow_id))` on success, or an error if the underlying payment
    /// creation fails, the escrow contract invocation fails, or the token transfer fails.
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
        Self::require_not_paused(&env, "create_escrowed_payment")?;
        Self::require_merchant_not_paused(&env, &merchant)?;
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
        env.storage().instance().set(
            &DataKey::State(StateDataKey::EscrowedPayment(payment_id)),
            &bridge,
        );

        // Custody is already shifted to the escrow contract by `invoke_escrow_create`
        // above, which transfers `amount` from `customer` as part of creating the escrow.

        (EscrowedPaymentCreated {
            payment_id,
            escrow_id,
            escrow_contract,
        })
        .publish(&env);

        Ok((payment_id, escrow_id))
    }

    /// Completes an escrowed payment by releasing funds from the escrow contract.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing the completion (must be in the multisig admin list)
    /// * `payment_id` - The ID of the escrowed payment to complete
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized, the payment is not
    /// found, is not in Pending status, has an unresolved dispute, or the escrow release fails.
    pub fn complete_escrowed_payment(
        env: Env,
        admin: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "complete_escrowed_payment")?;
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let mut payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        Self::require_merchant_not_paused(&env, &payment.merchant)?;
        PaymentContract::require_no_unresolved_escrowed_payment_dispute(&env, payment_id)?;

        // Released as this contract's own identity (not the human `admin`),
        // so the escrow contract can recognize it as a registered trusted
        // bridge and allow an immediate release. See
        // EscrowContract::add_trusted_bridge / is_trusted_bridge.
        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_release_escrow(
                &env.current_contract_address(),
                &bridge.escrow_id,
                &bridge.auto_release_on_complete,
            )
            .is_err()
        {
            return Err(Error::Feature(FeatureError::EscrowBridgeFailed));
        }

        payment.status = PaymentStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        (EscrowedPaymentCompleted {
            payment_id,
            escrow_id: bridge.escrow_id,
        })
        .publish(&env);
        Ok(())
    }

    /// Cancels an escrowed payment and refunds the escrowed funds to the customer.
    ///
    /// # Arguments
    /// * `caller` - The customer or merchant who authorized this payment (must authorize)
    /// * `payment_id` - The ID of the escrowed payment to cancel
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not found, is not in Pending
    /// status, the caller is not the customer or merchant, has an unresolved dispute,
    /// or the escrow refund fails.
    pub fn cancel_escrowed_payment(
        env: Env,
        caller: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "cancel_escrowed_payment")?;
        caller.require_auth();
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let mut payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        if payment.customer != caller && payment.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        PaymentContract::require_no_unresolved_escrowed_payment_dispute(&env, payment_id)?;

        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_refund_escrow(&caller, &bridge.escrow_id)
            .is_err()
        {
            return Err(Error::Feature(FeatureError::EscrowBridgeFailed));
        }

        payment.status = PaymentStatus::Cancelled;
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        (EscrowedPaymentCancelled {
            payment_id,
            escrow_id: bridge.escrow_id,
        })
        .publish(&env);
        Ok(())
    }

    /// Raises a dispute on an escrowed payment, freezing the escrow funds.
    ///
    /// # Arguments
    /// * `caller` - The customer or merchant raising the dispute (must authorize)
    /// * `payment_id` - The ID of the escrowed payment to dispute
    /// * `reason` - A text description of the dispute reason
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not found, is not in Pending
    /// status, the caller is not the customer or merchant, there is already an unresolved
    /// dispute, or the escrow dispute invocation fails.
    pub fn dispute_escrowed_payment(
        env: Env,
        caller: Address,
        payment_id: u64,
        reason: String,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "dispute_escrowed_payment")?;
        caller.require_auth();
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        if payment.customer != caller && payment.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if let Some(dispute) =
            PaymentContract::get_escrowed_payment_dispute(env.clone(), payment_id)
        {
            if !dispute.resolved {
                return Err(Error::Payment(PaymentError::AlreadyProcessed));
            }
        }

        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_dispute_escrow(&caller, &bridge.escrow_id)
            .is_err()
        {
            return Err(Error::Feature(FeatureError::EscrowBridgeFailed));
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
        env.storage().instance().set(
            &DataKey::State(StateDataKey::EscrowedPaymentDispute(payment_id)),
            &dispute,
        );

        (EscrowedPaymentDisputed {
            payment_id,
            raised_by: caller,
        })
        .publish(&env);
        Ok(())
    }

    /// Resolves an escrowed payment dispute by ruling in favor of the customer or merchant.
    ///
    /// # Arguments
    /// * `admin` - The admin resolving the dispute (must be in the multisig admin list)
    /// * `payment_id` - The ID of the escrowed payment with the dispute
    /// * `favor_customer` - If true, funds are returned to the customer; otherwise, released to merchant
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized, the payment is not
    /// found, the dispute is already resolved, the payment is not in Pending status,
    /// or the escrow resolution fails.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let bridge = PaymentContract::get_escrowed_payment(env.clone(), payment_id)?;
        let mut dispute = PaymentContract::get_escrowed_payment_dispute(env.clone(), payment_id)
            .ok_or(Error::Payment(PaymentError::InvalidStatus))?;
        if dispute.resolved {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        Self::require_merchant_not_paused(&env, &payment.merchant)?;

        let release_to_merchant = !favor_customer;
        let escrow_client = EscrowContractClient::new(&env, &bridge.escrow_contract);
        if escrow_client
            .try_resolve_dispute(&admin, &bridge.escrow_id, &release_to_merchant)
            .is_err()
        {
            return Err(Error::Feature(FeatureError::EscrowBridgeFailed));
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
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);
        env.storage().instance().set(
            &DataKey::State(StateDataKey::EscrowedPaymentDispute(payment_id)),
            &dispute,
        );

        (PaymentDisputeResolved {
            payment_id,
            favor_customer,
        })
        .publish(&env);
        Ok(())
    }

    /// Retrieves the escrowed payment bridge record for a given payment.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to look up
    ///
    /// # Returns
    /// `Ok(EscrowedPayment)` on success, or an error if the escrow mapping is not found.
    pub fn get_escrowed_payment(env: Env, payment_id: u64) -> Result<EscrowedPayment, Error> {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::EscrowedPayment(payment_id)))
            .ok_or(Error::Feature(FeatureError::EscrowMappingNotFound))
    }

    /// Retrieves the dispute record for an escrowed payment, if one exists.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to check for disputes
    ///
    /// # Returns
    /// `Some(EscrowedPaymentDispute)` if a dispute record exists, or `None` otherwise.
    pub fn get_escrowed_payment_dispute(
        env: Env,
        payment_id: u64,
    ) -> Option<EscrowedPaymentDispute> {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::EscrowedPaymentDispute(
                payment_id,
            )))
    }

    fn require_no_unresolved_escrowed_payment_dispute(
        env: &Env,
        payment_id: u64,
    ) -> Result<(), Error> {
        if let Some(dispute) = env
            .storage()
            .instance()
            .get::<DataKey, EscrowedPaymentDispute>(&DataKey::State(
                StateDataKey::EscrowedPaymentDispute(payment_id),
            ))
        {
            if !dispute.resolved {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
        }
        Ok(())
    }

    /// Updates the notes field on a payment.
    ///
    /// # Arguments
    /// * `merchant` - The merchant who owns the payment (must authorize)
    /// * `payment_id` - The ID of the payment to update
    /// * `notes` - The new notes text to set on the payment
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the notes are too large, the payment is not
    /// found, or the caller is not the payment's merchant.
    pub fn update_payment_notes(
        env: Env,
        merchant: Address,
        payment_id: u64,
        notes: String,
    ) -> Result<(), Error> {
        merchant.require_auth();

        // Validate notes size
        if notes.len() > MAX_NOTES_SIZE {
            return Err(Error::Basic(BasicError::NotesTooLarge));
        }

        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Verify caller is the merchant
        if payment.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Update notes
        payment.notes = notes;

        // Save updated payment
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        Ok(())
    }

    /// Checks whether a payment has expired based on its expiration timestamp.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to check
    ///
    /// # Returns
    /// `true` if the payment exists, has a non-zero expiration, and the current time
    /// exceeds the expiration; `false` otherwise.
    pub fn is_payment_expired(env: &Env, payment_id: u64) -> bool {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return false;
        }
        let payment = PaymentContract::get_payment(env, payment_id);
        payment.expires_at > 0 && env.ledger().timestamp() > payment.expires_at
    }

    /// Expires a payment that has passed its expiration time, refunding the customer.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to expire
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not found, is not in Pending
    /// status, has no expiration set, or has not yet expired.
    pub fn expire_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        // Retrieve payment from storage
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }
        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Check payment status allows expiry (only allow Pending)
        match payment.status {
            PaymentStatus::Pending => {
                // Allow expiry
            }
            PaymentStatus::Refunded | PaymentStatus::PartialRefunded | PaymentStatus::Cancelled => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
            PaymentStatus::Completed => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
        }

        // Check payment has expiration set
        if payment.expires_at == 0 {
            return Err(Error::Payment(PaymentError::NoExpiration));
        }

        // Check current time is past expires_at
        if env.ledger().timestamp() <= payment.expires_at {
            return Err(Error::Payment(PaymentError::NotExpired));
        }

        // Refund only what was actually deposited for this payment (installments),
        // not the contract's shared pooled balance.
        let deposited = PaymentContract::get_payment_deposited_amount(&env, payment_id);
        let token_client = token::Client::new(&env, &payment.token);
        let contract_address = env.current_contract_address();
        let contract_balance = token_client.balance(&contract_address);
        let refund_amount = if deposited <= contract_balance {
            deposited
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
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

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

    /// Completes a pending payment, transferring funds from the customer to the merchant.
    ///
    /// If the payment exceeds the large payment threshold, a multisig proposal is
    /// automatically created instead of completing immediately.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing the completion (must be in the multisig admin list)
    /// * `payment_id` - The ID of the payment to complete
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized, the payment is not
    /// found, is not in Pending status, is expired, or the multisig threshold is not met.
    /// Returns `ProposalError::RequiresMultiSig` if a multisig proposal was auto-created.
    pub fn complete_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "complete_payment")?;
        admin.require_auth();

        // Verify caller is in the multisig admin list
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if payment requires multi-sig approval
        let payment = PaymentContract::get_payment(&env, payment_id);
        let threshold = PaymentContract::get_large_payment_threshold(env.clone());

        if threshold > 0 && payment.amount > threshold {
            // Check if there's already a proposal for this payment
            if env
                .storage()
                .instance()
                .get::<DataKey, LargePaymentProposal>(&DataKey::State(
                    StateDataKey::LargePaymentProposal(payment_id),
                ))
                .is_some()
            {
                return Err(Error::Proposal(ProposalError::RequiresMultiSig));
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

            env.storage().instance().set(
                &DataKey::State(StateDataKey::LargePaymentProposal(payment_id)),
                &proposal,
            );

            (LargePaymentProposed {
                payment_id,
                proposer: admin,
                required_approvals: config.required_signatures,
                expires_at,
            })
            .publish(&env);

            return Err(Error::Proposal(ProposalError::RequiresMultiSig));
        }

        PaymentContract::do_complete_payment(&env, payment_id)
    }

    fn do_complete_payment(env: &Env, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let mut payment = PaymentContract::get_payment(env, payment_id);

        // Before updating status, check if payment is expired
        if PaymentContract::is_payment_expired(env, payment_id) {
            return Err(Error::Payment(PaymentError::Expired));
        }

        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Completed;
            }
            PaymentStatus::Completed => {
                return Err(Error::Payment(PaymentError::AlreadyProcessed));
            }
            PaymentStatus::Refunded | PaymentStatus::PartialRefunded => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
            PaymentStatus::Cancelled => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
        }

        // Subtract any loyalty-point discount redeemed against this payment (#490)
        let discount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Discount(payment_id)))
            .unwrap_or(0);
        let charge_amount = if discount > 0 {
            env.storage()
                .instance()
                .remove(&DataKey::Payment(PaymentKey::Discount(payment_id)));
            let capped_discount = if discount > payment.amount {
                payment.amount
            } else {
                discount
            };
            payment.amount - capped_discount
        } else {
            payment.amount
        };

        // Deduct platform fee (if configured) and get net amount for merchant
        let (net_amount, fee_amount) = PaymentContract::deduct_fee(
            env,
            payment_id,
            charge_amount,
            payment.merchant.clone(),
            &payment.token,
            &payment.customer,
            payment.currency.clone(),
        );

        // Check finality delay config (#219)
        let finality: Option<FinalityConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FinalityConfig));
        if let Some(ref fc) = finality {
            if fc.active && payment.amount >= fc.min_amount_threshold {
                // Hold funds — create PendingSettlement instead of transferring
                let release_at = env.ledger().timestamp() + fc.delay_seconds;
                let settlement = PendingSettlement {
                    payment_id,
                    merchant: payment.merchant.clone(),
                    amount: net_amount,
                    token: payment.token.clone(),
                    release_at,
                };
                env.storage().instance().set(
                    &DataKey::Payment(PaymentKey::PendingSettlement(payment_id)),
                    &settlement,
                );
                let idx: u64 = env
                    .storage()
                    .instance()
                    .get(&DataKey::Merchant(MerchantDataKey::PendingSettlementCount(
                        payment.merchant.clone(),
                    )))
                    .unwrap_or(0);
                env.storage().instance().set(
                    &DataKey::Merchant(MerchantDataKey::PendingSettlementIndex(
                        payment.merchant.clone(),
                        idx,
                    )),
                    &payment_id,
                );
                env.storage().instance().set(
                    &DataKey::Merchant(MerchantDataKey::PendingSettlementCount(
                        payment.merchant.clone(),
                    )),
                    &(idx + 1),
                );
                env.storage()
                    .instance()
                    .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);
                PaymentContract::update_merchant_fee_record_post_completion(
                    env,
                    payment.merchant.clone(),
                    payment.amount,
                    fee_amount,
                );
                let mut analytics: PaymentAnalytics = env
                    .storage()
                    .instance()
                    .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
                    .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);
                (PaymentCompleted {
                    payment_id,
                    merchant: payment.merchant.clone(),
                    amount: payment.amount,
                })
                .publish(env);
                return Ok(());
            }
        }

        // Carve out the portion (if any) that an active auto-escrow rule will route
        // to the escrow contract, so the merchant is only paid what remains. The
        // carved-out amount is transferred to escrow later by `trigger_auto_escrow`.
        let escrow_carveout = PaymentContract::get_auto_escrow_carveout(env, &payment);
        let merchant_amount = net_amount - escrow_carveout;

        // Pull merchant proceeds into the contract, then honor payout schedule.
        let token_client = token::Client::new(env, &payment.token);
        let contract_address = env.current_contract_address();

        token_client.transfer_from(
            &contract_address,
            &payment.customer,
            &contract_address,
            &merchant_amount,
        );

        PaymentContract::settle_or_accumulate(
            env,
            payment.merchant.clone(),
            payment.token.clone(),
            merchant_amount,
        )?;

        // Forwarding only applies when funds were paid out immediately.
        let payout_deferred = env
            .storage()
            .instance()
            .get::<DataKey, PayoutSchedule>(&DataKey::Merchant(
                MerchantDataKey::PayoutSchedule(payment.merchant.clone()),
            ))
            .map(|schedule: PayoutSchedule| {
                schedule.token == payment.token && schedule.frequency != PayoutFrequency::Immediate
            })
            .unwrap_or(false);

        // Check if merchant has an active payment forward config
        if !payout_deferred {
            if let Ok(forward_config) =
                PaymentContract::get_forward_config(env.clone(), payment.merchant.clone())
            {
                if forward_config.active {
                    // Calculate the forward amount based on forward_bps
                    let forward_amount =
                        (merchant_amount * (forward_config.forward_bps as i128)) / 10000;

                    // Transfer the forward amount from merchant to forward_to address
                    if forward_amount > 0 {
                        token_client.transfer(
                            &payment.merchant,
                            &forward_config.forward_to,
                            &forward_amount,
                        );

                        (PaymentForwarded {
                            payment_id,
                            merchant: payment.merchant.clone(),
                            forward_to: forward_config.forward_to,
                            forward_amount,
                        })
                        .publish(env);
                    }
                }
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);
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
            .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
            .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::Analytics(
                payment.merchant.clone(),
            )))
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
            &DataKey::Merchant(MerchantDataKey::Analytics(payment.merchant.clone())),
            &m_analytics,
        );
        (PaymentCompleted {
            payment_id,
            merchant: payment.merchant.clone(),
            amount: payment.amount,
        })
        .publish(env);

        // Accrue loyalty points for completed payments if loyalty is configured.
        PaymentContract::maybe_accrue_loyalty_points(
            &env,
            payment.customer.clone(),
            payment.amount,
        );

        // Accrue fee rebate for merchant if rebate programme is active
        PaymentContract::maybe_accrue_fee_rebate(
            env,
            payment.merchant.clone(),
            payment.amount,
            fee_amount,
        );

        // Attempt to trigger auto-escrow if a rule exists
        // Ignore errors - if there's no rule, payment is below minimum, or already triggered,
        // we just skip the auto-escrow (the payment completion still succeeds)
        let _ = PaymentContract::trigger_auto_escrow(env, payment_id);
        let now = env.ledger().timestamp();
        PaymentContract::update_merchant_bucket(env, payment.merchant.clone(), now, 0, 0, 0, 0);
        PaymentContract::update_platform_daily_bucket(env, now, 0, 0, 0);

        Ok(())
    }

    /// Configures automatic payment forwarding for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant setting up forwarding (must authorize)
    /// * `forward_to` - The address to forward a portion of payments to
    /// * `forward_bps` - The percentage to forward in basis points (1-10000)
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the forward_bps is invalid, a forwarding loop
    /// is detected, or the contract is paused.
    pub fn set_payment_forward(
        env: Env,
        merchant: Address,
        forward_to: Address,
        forward_bps: u32,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "set_payment_forward")?;
        merchant.require_auth();

        // Validate forward_bps: must be between 1 and 10000
        if let Err(_) = Self::validate_bps(forward_bps) {
            return Err(Error::Feature(FeatureError::InvalidForwardBps));
        }

        // Get configured max depth (default 5)
        let max_depth: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MaxForwardDepth))
            .unwrap_or(5);

        // Detect cycles and enforce depth limit by walking the forward chain
        {
            let mut current = forward_to.clone();
            for _depth in 0..max_depth {
                if current == merchant {
                    return Err(Error::Feature(FeatureError::ForwardLoop));
                }
                match env
                    .storage()
                    .instance()
                    .get::<DataKey, PaymentForwardConfig>(&DataKey::Feature(
                        FeatureKey::PaymentForwardConfig(current.clone()),
                    )) {
                    Some(next) => current = next.forward_to,
                    None => break,
                }
            }
            // If we exhausted max_depth iterations, the chain is too long
            if env
                .storage()
                .instance()
                .has(&DataKey::Feature(FeatureKey::PaymentForwardConfig(
                    current.clone(),
                )))
            {
                return Err(Error::Feature(FeatureError::ForwardLoop));
            }
        }

        // Create and store the forward config
        let config = PaymentForwardConfig {
            merchant: merchant.clone(),
            forward_to: forward_to.clone(),
            forward_bps,
            active: true,
        };

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PaymentForwardConfig(merchant.clone())),
            &config,
        );

        (PaymentForwardConfigSet {
            merchant: merchant.clone(),
            forward_to,
            forward_bps,
        })
        .publish(&env);

        Ok(())
    }

    /// Removes the payment forwarding configuration for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant removing forwarding (must authorize)
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if no forwarding config exists or the contract is paused.
    pub fn remove_payment_forward(env: Env, merchant: Address) -> Result<(), Error> {
        Self::require_not_paused(&env, "remove_payment_forward")?;
        merchant.require_auth();

        // Check if the forward config exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Feature(FeatureKey::PaymentForwardConfig(
                merchant.clone(),
            )))
        {
            return Err(Error::Feature(FeatureError::ForwardConfigNotFound));
        }

        // Remove the forward config
        env.storage()
            .instance()
            .remove(&DataKey::Feature(FeatureKey::PaymentForwardConfig(
                merchant.clone(),
            )));

        (PaymentForwardConfigRemoved {
            merchant: merchant.clone(),
        })
        .publish(&env);

        Ok(())
    }

    /// Retrieves the payment forwarding configuration for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query
    ///
    /// # Returns
    /// `Ok(PaymentForwardConfig)` if configured, or an error if no config is found.
    pub fn get_forward_config(env: Env, merchant: Address) -> Result<PaymentForwardConfig, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentForwardConfig(
                merchant,
            )))
            .ok_or(Error::Feature(FeatureError::ForwardConfigNotFound))
    }

    /// Configures the loyalty points program settings.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this change (must be in the multisig admin list)
    /// * `config` - The loyalty configuration including points per unit, redemption rate,
    ///   and expiry duration
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized, the config values
    /// are invalid (zero points_per_unit or zero expiry_seconds), or the contract is paused.
    pub fn configure_loyalty(env: Env, admin: Address, config: LoyaltyConfig) -> Result<(), Error> {
        Self::require_not_paused(&env, "configure_loyalty")?;
        admin.require_auth();

        let multisig_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !multisig_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if config.points_per_unit == 0 || config.expiry_seconds == 0 {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::LoyaltyConfig), &config);
        Ok(())
    }

    /// Returns the loyalty points balance for a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query
    ///
    /// # Returns
    /// `Some(CustomerLoyaltyBalance)` if the customer has a loyalty balance, or `None` otherwise.
    pub fn get_loyalty_balance(env: Env, customer: Address) -> Option<CustomerLoyaltyBalance> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::CustomerLoyaltyBalance(
                customer,
            )))
    }

    /// Redeems loyalty points as a discount on a specific payment.
    ///
    /// # Arguments
    /// * `customer` - The customer redeeming points (must authorize)
    /// * `points` - The number of loyalty points to redeem
    /// * `payment_id` - The payment to apply the discount to
    ///
    /// # Returns
    /// `Ok(discount_amount)` on success, where `discount_amount` is the token amount
    /// to deduct from the payment (capped at 50% of the payment amount), or an error
    /// if loyalty is not configured, points are insufficient or expired, the payment is
    /// not found, or the caller is not the payment's customer.
    pub fn redeem_points(
        env: Env,
        customer: Address,
        points: u64,
        payment_id: u64,
    ) -> Result<i128, Error> {
        Self::require_not_paused(&env, "redeem_points")?;
        customer.require_auth();

        let config: LoyaltyConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::LoyaltyConfig))
            .ok_or(Error::Feature(FeatureError::LoyaltyNotConfigured))?;
        if !config.active {
            return Err(Error::Feature(FeatureError::LoyaltyNotConfigured));
        }

        let mut balance: CustomerLoyaltyBalance = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::CustomerLoyaltyBalance(
                customer.clone(),
            )))
            .ok_or(Error::Feature(FeatureError::InsufficientPoints))?;

        let now = env.ledger().timestamp();
        if balance.expires_at != 0 && now > balance.expires_at {
            return Err(Error::Feature(FeatureError::PointsExpired));
        }

        if balance.points < points {
            return Err(Error::Feature(FeatureError::InsufficientPoints));
        }

        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }
        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.customer != customer {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let existing_discount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Discount(payment_id)))
            .unwrap_or(0);

        let max_discount = payment.amount / 2;
        if existing_discount >= max_discount {
            return Err(Error::Feature(FeatureError::InsufficientPoints));
        }

        let mut discount = (points as i128) * (config.redemption_rate as i128);
        let remaining_capacity = max_discount - existing_discount;
        if discount > remaining_capacity {
            discount = remaining_capacity;
        }
        if discount <= 0 {
            return Err(Error::Feature(FeatureError::InsufficientPoints));
        }

        balance.points -= points;
        balance.last_updated = now;
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::CustomerLoyaltyBalance(customer.clone())),
            &balance,
        );

        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::Discount(payment_id)),
            &(existing_discount + discount),
        );

        Ok(discount)
    }

    fn get_loyalty_config(env: &Env) -> Option<LoyaltyConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::LoyaltyConfig))
    }

    fn maybe_accrue_loyalty_points(env: &Env, customer: Address, amount: i128) {
        if amount <= 0 {
            return;
        }
        let config = match PaymentContract::get_loyalty_config(env) {
            Some(c) if c.active && c.points_per_unit > 0 => c,
            _ => return,
        };

        let points = (amount / i128::from(config.points_per_unit)) as u64;
        if points == 0 {
            return;
        }

        let now = env.ledger().timestamp();
        let mut balance: CustomerLoyaltyBalance = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::CustomerLoyaltyBalance(
                customer.clone(),
            )))
            .unwrap_or(CustomerLoyaltyBalance {
                customer: customer.clone(),
                points: 0,
                last_updated: 0,
                expires_at: 0,
            });

        if balance.expires_at != 0 && now > balance.expires_at {
            balance.points = points;
        } else {
            balance.points = balance.points.saturating_add(points);
        }
        balance.last_updated = now;
        balance.expires_at = now + config.expiry_seconds;

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::CustomerLoyaltyBalance(customer)),
            &balance,
        );
    }

    /// Pays an installment toward a pending payment.
    ///
    /// # Arguments
    /// * `customer` - The customer making the installment payment (must authorize)
    /// * `payment_id` - The ID of the payment to pay toward
    /// * `amount` - The installment amount in base token units
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the amount is non-positive, the payment is expired
    /// or not in Pending status, the installment exceeds the outstanding balance, or the
    /// contract is paused. Automatically finalizes the payment if fully paid.
    pub fn pay_installment(
        env: Env,
        customer: Address,
        payment_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "pay_installment")?;
        customer.require_auth();

        if amount <= 0 {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Check if payment is expired
        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::Payment(PaymentError::Expired));
        }

        // Only allow installments on Pending payments
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        // Get current outstanding balance
        let outstanding_balance = PaymentContract::get_outstanding_balance(env.clone(), payment_id);

        if outstanding_balance <= 0 {
            return Err(Error::Payment(PaymentError::AlreadyFullyPaid));
        }

        if amount > outstanding_balance {
            return Err(Error::Payment(PaymentError::InstallmentExceedsRemaining));
        }

        // Get current installment counter
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::PartialPaymentCounter(
                payment_id,
            )))
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
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PartialPaymentRecord(
                payment_id,
                new_installment_number,
            )),
            &partial_payment,
        );

        // Update installment counter
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::PartialPaymentCounter(payment_id)),
            &new_installment_number,
        );

        // Update outstanding balance
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::OutstandingBalance(payment_id)),
            &remaining,
        );

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

    /// Returns the full installment payment history for a given payment.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to retrieve installment history for
    ///
    /// # Returns
    /// A `Vec<PartialPaymentRecord>` of all installments made toward the payment.
    pub fn get_installment_history(env: Env, payment_id: u64) -> Vec<PartialPaymentRecord> {
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::PartialPaymentCounter(
                payment_id,
            )))
            .unwrap_or(0);

        let mut history = Vec::new(&env);
        for i in 1..=installment_counter {
            if let Some(record) =
                env.storage()
                    .instance()
                    .get(&DataKey::State(StateDataKey::PartialPaymentRecord(
                        payment_id, i,
                    )))
            {
                history.push_back(record);
            }
        }
        history
    }

    /// Returns the outstanding balance remaining on a payment.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to check
    ///
    /// # Returns
    /// The remaining amount in base token units. Returns the full payment amount if no
    /// partial payments have been made.
    pub fn get_outstanding_balance(env: Env, payment_id: u64) -> i128 {
        // First check if we have an outstanding balance stored
        if let Some(balance) =
            env.storage()
                .instance()
                .get(&DataKey::Payment(PaymentKey::OutstandingBalance(
                    payment_id,
                )))
        {
            return balance;
        }

        // If not, calculate from payment amount and partial payments
        let payment = PaymentContract::get_payment(&env, payment_id);
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::PartialPaymentCounter(
                payment_id,
            )))
            .unwrap_or(0);

        let mut total_paid = 0i128;
        for i in 1..=installment_counter {
            if let Some(record) = env
                .storage()
                .instance()
                .get::<DataKey, PartialPaymentRecord>(&DataKey::State(
                    StateDataKey::PartialPaymentRecord(payment_id, i),
                ))
            {
                total_paid += record.amount_paid;
            }
        }

        let outstanding = payment.amount - total_paid;

        // Cache the calculated balance
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::OutstandingBalance(payment_id)),
            &outstanding,
        );

        outstanding
    }

    /// Finalizes a payment that has been fully paid through installments.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to finalize
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not in Pending status or
    /// the outstanding balance is not zero.
    pub fn finalize_installment_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        let mut payment = PaymentContract::get_payment(&env, payment_id);

        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        let outstanding_balance = PaymentContract::get_outstanding_balance(env.clone(), payment_id);
        if outstanding_balance != 0 {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        // Get installment counter for event
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::PartialPaymentCounter(
                payment_id,
            )))
            .unwrap_or(0);

        // Update payment status to Completed
        payment.status = PaymentStatus::Completed;
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        // Transfer or accumulate all collected funds to merchant
        Self::settle_or_accumulate(
            &env,
            payment.merchant.clone(),
            payment.token.clone(),
            payment.amount,
        )?;

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

    /// Refunds a pending payment in full, returning funds to the customer.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing the refund (must be in the multisig admin list)
    /// * `payment_id` - The ID of the payment to refund
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized, the payment is not
    /// found, is expired, is not in Pending status, or the contract is paused.
    pub fn refund_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "refund_payment")?;
        admin.require_auth();

        // Verify caller is in the multisig admin list
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        PaymentContract::do_refund_payment(&env, payment_id)
    }

    fn do_refund_payment(env: &Env, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let mut payment = PaymentContract::get_payment(env, payment_id);

        // Before updating status, check if payment is expired
        if PaymentContract::is_payment_expired(env, payment_id) {
            return Err(Error::Payment(PaymentError::Expired));
        }

        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Refunded;
            }
            PaymentStatus::Completed | PaymentStatus::PartialRefunded => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
            PaymentStatus::Refunded => {
                return Err(Error::Payment(PaymentError::AlreadyProcessed));
            }
            PaymentStatus::Cancelled => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        // Update analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
            .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::Analytics(
                payment.merchant.clone(),
            )))
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
            &DataKey::Merchant(MerchantDataKey::Analytics(payment.merchant.clone())),
            &m_analytics,
        );
        let mut c_analytics: CustomerAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::Analytics(
                payment.customer.clone(),
            )))
            .unwrap_or(PaymentContract::default_customer_analytics());
        c_analytics.total_refunds += payment.amount;
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Analytics(payment.customer.clone())),
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

    /// Refunds a partial amount of a pending or partially-refunded payment.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing the refund (must be in the multisig admin list)
    /// * `payment_id` - The ID of the payment to partially refund
    /// * `refund_amount` - The amount to refund in base token units
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized, the payment is not
    /// found, is expired, is not in a refundable status, or the refund exceeds the payment amount.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::Payment(PaymentError::Expired));
        }

        match payment.status {
            PaymentStatus::Pending | PaymentStatus::PartialRefunded => {
                let new_refunded = payment.refunded_amount + refund_amount;
                if new_refunded > payment.amount {
                    return Err(Error::Payment(PaymentError::RefundExceedsPayment));
                }
                payment.refunded_amount = new_refunded;
                payment.status = if new_refunded == payment.amount {
                    PaymentStatus::Refunded
                } else {
                    PaymentStatus::PartialRefunded
                };
            }
            _ => {
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        (PaymentRefunded {
            payment_id,
            customer: payment.customer,
            amount: refund_amount,
        })
        .publish(&env);

        Ok(())
    }

    /// Cancels a pending payment.
    ///
    /// # Arguments
    /// * `caller` - The customer or merchant who authorized the payment (must authorize)
    /// * `payment_id` - The ID of the payment to cancel
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the payment is not found, is not in Pending
    /// status, or the caller is not the customer or merchant.
    pub fn cancel_payment(env: Env, caller: Address, payment_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "cancel_payment")?;
        caller.require_auth();
        PaymentContract::do_cancel_payment(&env, caller, payment_id)
    }

    fn do_cancel_payment(env: &Env, caller: Address, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let mut payment = PaymentContract::get_payment(env, payment_id);

        // Check authorization: caller must be customer, merchant, or admin
        let is_authorized = payment.customer == caller || payment.merchant == caller;
        if !is_authorized {
            return Err(Error::Basic(BasicError::Unauthorized));
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
                return Err(Error::Payment(PaymentError::InvalidStatus));
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);

        // Update analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
            .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::Analytics(
                payment.merchant.clone(),
            )))
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
            &DataKey::Merchant(MerchantDataKey::Analytics(payment.merchant.clone())),
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

    /// Returns a paginated list of payments made by a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query
    /// * `limit` - Maximum number of payments to return
    /// * `offset` - The number of payments to skip (for pagination)
    ///
    /// # Returns
    /// A `Vec<Payment>` containing the requested page of payments.
    pub fn get_payments_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Payment> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::PaymentCount(
                customer.clone(),
            )))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) =
                env.storage()
                    .instance()
                    .get::<DataKey, u64>(&DataKey::Customer(CustomerDataKey::Payments(
                        customer.clone(),
                        i,
                    )))
            {
                if let Some(payment) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Payment>(&DataKey::Payment(PaymentKey::Data(payment_id)))
                {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    /// Returns the total number of payments made by a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query
    ///
    /// # Returns
    /// The total payment count for the customer, or 0 if none.
    pub fn get_payment_count_by_customer(env: Env, customer: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::PaymentCount(customer)))
            .unwrap_or(0)
    }

    /// Returns a paginated list of payments received by a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query
    /// * `limit` - Maximum number of payments to return
    /// * `offset` - The number of payments to skip (for pagination)
    ///
    /// # Returns
    /// A `Vec<Payment>` containing the requested page of payments.
    pub fn get_payments_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Payment> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PaymentCount(
                merchant.clone(),
            )))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) =
                env.storage()
                    .instance()
                    .get::<DataKey, u64>(&DataKey::Merchant(MerchantDataKey::Payments(
                        merchant.clone(),
                        i,
                    )))
            {
                if let Some(payment) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Payment>(&DataKey::Payment(PaymentKey::Data(payment_id)))
                {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    /// Returns the total number of payments received by a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query
    ///
    /// # Returns
    /// The total payment count for the merchant, or 0 if none.
    pub fn get_payment_count_by_merchant(env: Env, merchant: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PaymentCount(merchant)))
            .unwrap_or(0)
    }

    /// Returns the payment IDs for the given merchant on the requested page (100 per page).
    pub fn get_merchant_payments(env: Env, merchant: Address, page: u64) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(
                merchant, page,
            )))
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Adds a token to the list of allowed tokens for payments.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this change (must be in the multisig admin list)
    /// * `token` - The SPL token address to add to the allowed list
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized or multisig is not initialized.
    pub fn add_allowed_token(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let mut tokens: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AllowedTokens))
            .unwrap_or_else(|| Vec::new(&env));
        if !tokens.contains(&token) {
            tokens.push_back(token);
            env.storage()
                .instance()
                .set(&DataKey::Config(ConfigKey::AllowedTokens), &tokens);
        }

        Ok(())
    }

    /// Removes a token from the list of allowed tokens for payments.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this change (must be in the multisig admin list)
    /// * `token` - The SPL token address to remove from the allowed list
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if the admin is unauthorized or multisig is not initialized.
    pub fn remove_allowed_token(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let tokens: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AllowedTokens))
            .unwrap_or_else(|| Vec::new(&env));
        let mut updated = Vec::new(&env);
        for entry in tokens.iter() {
            if entry != token {
                updated.push_back(entry);
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::AllowedTokens), &updated);

        Ok(())
    }

    /// Returns the list of allowed payment tokens.
    ///
    /// If no tokens have been configured, returns an empty list, which means all tokens are permitted.
    ///
    /// # Returns
    /// A vector of token addresses that have been explicitly allowed by an admin.
    /// Returns the list of all allowed tokens for payments.
    ///
    /// # Returns
    /// A `Vec<Address>` of allowed token addresses. Returns an empty vector if none are configured.
    pub fn get_allowed_tokens(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AllowedTokens))
            .unwrap_or_else(|| Vec::new(&env))
    }

    // Mirrors the refund contract's token-registry bypass (Issue #191): an
    // empty allowlist means the allowlist hasn't been configured yet, so all
    // tokens are permitted until an admin opts in by calling
    // `add_allowed_token`.
    fn is_token_allowed(env: &Env, token: &Address) -> bool {
        let tokens: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AllowedTokens))
            .unwrap_or_else(|| Vec::new(env));
        tokens.is_empty() || tokens.contains(token)
    }

    const ACTIVE_SUBSCRIPTION_PAGE_SIZE: u64 = 100;

    fn merchant_active_subscription_count(env: &Env, merchant: &Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(
                MerchantDataKey::MerchantActiveSubscriptionCount(merchant.clone()),
            ))
            .unwrap_or(0)
    }

    fn set_active_subscription_at(
        env: &Env,
        merchant: &Address,
        flat_index: u64,
        subscription_id: u64,
    ) {
        let page_num = flat_index / Self::ACTIVE_SUBSCRIPTION_PAGE_SIZE;
        let page_offset = flat_index % Self::ACTIVE_SUBSCRIPTION_PAGE_SIZE;
        let mut page: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptions(
                merchant.clone(),
                page_num,
            )))
            .unwrap_or_else(|| Vec::new(env));

        let page_offset_u32 = page_offset as u32;
        while page.len() <= page_offset_u32 {
            page.push_back(0);
        }
        page.set(page_offset_u32, subscription_id);
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptions(
                merchant.clone(),
                page_num,
            )),
            &page,
        );
    }

    fn active_subscription_at(env: &Env, merchant: &Address, flat_index: u64) -> Option<u64> {
        let page_num = flat_index / Self::ACTIVE_SUBSCRIPTION_PAGE_SIZE;
        let page_offset = flat_index % Self::ACTIVE_SUBSCRIPTION_PAGE_SIZE;
        let page: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptions(
                merchant.clone(),
                page_num,
            )))?;
        let page_offset_u32 = page_offset as u32;
        if page_offset_u32 < page.len() {
            Some(page.get(page_offset_u32).unwrap())
        } else {
            None
        }
    }

    fn remove_active_subscription_page_entry(env: &Env, merchant: &Address, flat_index: u64) {
        let page_num = flat_index / Self::ACTIVE_SUBSCRIPTION_PAGE_SIZE;
        let page_offset = flat_index % Self::ACTIVE_SUBSCRIPTION_PAGE_SIZE;
        let page_offset_u32 = page_offset as u32;
        let mut page: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptions(
                merchant.clone(),
                page_num,
            )))
            .unwrap_or_else(|| Vec::new(env));

        if page_offset_u32 < page.len() {
            let mut rebuilt = Vec::new(env);
            for i in 0..page.len() {
                if i != page_offset_u32 {
                    rebuilt.push_back(page.get(i).unwrap());
                }
            }
            if rebuilt.is_empty() {
                env.storage().instance().remove(&DataKey::Merchant(
                    MerchantDataKey::MerchantActiveSubscriptions(merchant.clone(), page_num),
                ));
            } else {
                env.storage().instance().set(
                    &DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptions(
                        merchant.clone(),
                        page_num,
                    )),
                    &rebuilt,
                );
            }
        }
    }

    fn add_to_merchant_active_subscriptions(env: &Env, merchant: &Address, subscription_id: u64) {
        let count = Self::merchant_active_subscription_count(env, merchant);
        Self::set_active_subscription_at(env, merchant, count, subscription_id);
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::ActiveSubscriptionIndex(subscription_id)),
            &count,
        );
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptionCount(
                merchant.clone(),
            )),
            &(count + 1),
        );
    }

    fn remove_from_merchant_active_subscriptions(
        env: &Env,
        merchant: &Address,
        subscription_id: u64,
    ) {
        let count = Self::merchant_active_subscription_count(env, merchant);
        if count == 0 {
            return;
        }

        let index: u64 = match env.storage().instance().get(&DataKey::Merchant(
            MerchantDataKey::ActiveSubscriptionIndex(subscription_id),
        )) {
            Some(i) => i,
            None => return,
        };
        let last_index = count - 1;

        if index != last_index {
            if let Some(last_id) = Self::active_subscription_at(env, merchant, last_index) {
                Self::set_active_subscription_at(env, merchant, index, last_id);
                env.storage().instance().set(
                    &DataKey::Merchant(MerchantDataKey::ActiveSubscriptionIndex(last_id)),
                    &index,
                );
            }
        }

        Self::remove_active_subscription_page_entry(env, merchant, last_index);
        env.storage().instance().remove(&DataKey::Merchant(
            MerchantDataKey::ActiveSubscriptionIndex(subscription_id),
        ));
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptionCount(
                merchant.clone(),
            )),
            &last_index,
        );
    }

    /// Returns active subscription IDs for a merchant (100 per page).
    pub fn get_merchant_subscriptions(env: Env, merchant: Address, page: u64) -> Vec<u64> {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantActiveSubscriptions(
                merchant, page,
            )))
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn is_valid_currency(currency: &Currency) -> bool {
        matches!(
            currency,
            Currency::XLM | Currency::USDC | Currency::USDT | Currency::BTC | Currency::ETH
        )
    }

    /// Sets the manual conversion rate for a given currency.
    ///
    /// This rate is used when oracle-based rate fetching is disabled or unavailable.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `currency` - The fiat currency to set a conversion rate for.
    /// * `rate` - The conversion rate in base units (e.g., 1_0000000 = 1:1).
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized or the currency is invalid.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::Basic(BasicError::InvalidCurrency));
        }

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::ConversionRate(currency)),
            &rate,
        );

        Ok(())
    }

    /// Returns the stored conversion rate for a given currency.
    ///
    /// Defaults to `1_0000000` (1:1) if no rate has been set.
    ///
    /// # Arguments
    /// * `currency` - The fiat currency to query the conversion rate for.
    ///
    /// # Returns
    /// The conversion rate in base units.
    pub fn get_conversion_rate(env: Env, currency: Currency) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::ConversionRate(currency)))
            .unwrap_or(1_0000000)
    }

    /// Configures the oracle-based price feed for a given currency.
    ///
    /// When enabled, the contract will fetch live conversion rates from the oracle
    /// contract instead of using the manual conversion rate.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `config` - The oracle rate configuration including oracle address, price feed ID,
    ///   staleness threshold, and currency.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized.
    pub fn set_oracle_rate_config(
        env: Env,
        admin: Address,
        config: OracleRateConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::OracleRateConfig(config.currency.clone())),
            &config,
        );
        Ok(())
    }

    /// Returns the oracle rate configuration for a given currency.
    ///
    /// # Arguments
    /// * `currency` - The fiat currency to query the oracle config for.
    ///
    /// # Returns
    /// `Some(OracleRateConfig)` if configured, `None` otherwise.
    pub fn get_oracle_rate_config(env: Env, currency: Currency) -> Option<OracleRateConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::OracleRateConfig(currency)))
    }

    /// Fetches a fresh conversion rate from the oracle and stores it.
    ///
    /// If the oracle is disabled for this currency, returns the stored manual rate.
    /// If the oracle feed data is stale (older than `max_staleness_seconds`), returns an error.
    ///
    /// # Arguments
    /// * `currency` - The fiat currency to refresh the conversion rate for.
    ///
    /// # Returns
    /// The updated conversion rate on success.
    ///
    /// # Errors
    /// - `OracleNotConfigured` if no oracle config exists for the currency.
    /// - `OracleCallFailed` if the oracle contract call fails.
    /// - `OracleFeedStale` if the fetched data exceeds the staleness threshold.
    pub fn refresh_conversion_rate(env: Env, currency: Currency) -> Result<i128, Error> {
        let cfg: OracleRateConfig = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::OracleRateConfig(
                currency.clone(),
            )))
            .ok_or(Error::Basic(BasicError::OracleNotConfigured))?;

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
            .map_err(|_| Error::Basic(BasicError::OracleCallFailed))?
            .map_err(|_| Error::Basic(BasicError::OracleCallFailed))?;

        let now = env.ledger().timestamp();
        if now.saturating_sub(fetched.1) > cfg.max_staleness_seconds {
            return Err(Error::Basic(BasicError::OracleFeedStale));
        }

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::ConversionRate(currency)),
            &fetched.0,
        );
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
        Self::require_not_paused(&env, "create_subscription")?;
        Self::require_merchant_not_paused(&env, &merchant)?;
        customer.require_auth();

        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::Basic(BasicError::InvalidCurrency));
        }
        if metadata.len() > MAX_METADATA_SIZE {
            return Err(Error::Basic(BasicError::MetadataTooLarge));
        }
        if interval == 0 {
            // return Err(Error::InvalidInterval);
            return Err(Error::Basic(BasicError::InvalidInterval));
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Counter))
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
            .set(&DataKey::Subscription(SubscriptionKey::Data(sub_id)), &sub);
        env.storage()
            .instance()
            .set(&DataKey::Subscription(SubscriptionKey::Counter), &sub_id);

        // Index by customer
        let c_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::SubscriptionCount(
                customer.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Subscriptions(customer.clone(), c_count)),
            &sub_id,
        );
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::SubscriptionCount(customer)),
            &(c_count + 1),
        );

        // Index by merchant
        let m_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::SubscriptionCount(
                merchant.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::Subscriptions(merchant.clone(), m_count)),
            &sub_id,
        );
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::SubscriptionCount(merchant.clone())),
            &(m_count + 1),
        );

        Self::add_to_merchant_active_subscriptions(&env, &merchant, sub_id);

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

    /// Extend the trial period for a subscription. Only callable by the merchant before the trial expires.
    /// The total trial duration cannot exceed `MAX_TRIAL_DURATION`.
    pub fn extend_trial(
        env: Env,
        merchant: Address,
        subscription_id: u64,
        additional_seconds: u64,
    ) -> Result<(), Error> {
        merchant.require_auth();

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .ok_or(Error::Subscription(SubscriptionError::NotFound))?;

        if sub.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if sub.trial_data.ends_at == 0 {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let now = env.ledger().timestamp();
        if now >= sub.trial_data.ends_at {
            return Err(Error::Subscription(SubscriptionError::TrialExpired));
        }

        let new_total = sub
            .trial_data
            .period_seconds
            .checked_add(additional_seconds)
            .ok_or(Error::Subscription(
                SubscriptionError::MaxTrialDurationExceeded,
            ))?;

        if new_total > MAX_TRIAL_DURATION {
            return Err(Error::Subscription(
                SubscriptionError::MaxTrialDurationExceeded,
            ));
        }

        sub.trial_data.ends_at = sub
            .trial_data
            .ends_at
            .checked_add(additional_seconds)
            .ok_or(Error::Subscription(
                SubscriptionError::MaxTrialDurationExceeded,
            ))?;
        sub.trial_data.period_seconds = new_total;

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
            &sub,
        );

        Ok(())
    }

    /// Execute the next recurring payment for a subscription.
    /// Anyone (typically an off-chain keeper / cron) may call this once the
    /// payment is due. It handles retry logic internally.
    pub fn execute_recurring_payment(env: Env, subscription_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        let now = env.ledger().timestamp();

        // InDunning path: enforce on-chain backoff before retrying
        if sub.status == SubscriptionStatus::InDunning {
            let mut dunning: DunningState = env
                .storage()
                .instance()
                .get(&DataKey::State(StateDataKey::DunningState(subscription_id)))
                .ok_or(Error::Subscription(SubscriptionError::DunningNotFound))?;

            if now < dunning.next_retry_at {
                return Err(Error::Subscription(SubscriptionError::RetryTooEarly));
            }

            // Check merchant account is not paused
            let merchant_paused: bool = env
                .storage()
                .instance()
                .get(&DataKey::Merchant(MerchantDataKey::MerchantPaused(
                    sub.merchant.clone(),
                )))
                .unwrap_or(false);
            if merchant_paused {
                return Err(Error::Subscription(SubscriptionError::MerchantPaused));
            }

            // Check customer spend limit (#282)
            let charge_amount =
                PaymentContract::get_discounted_subscription_amount(&env, subscription_id, sub.amount);
            if let Err(_) =
                PaymentContract::check_and_update_spend_limit(&env, &sub.customer, charge_amount)
            {
                return Err(Error::Feature(FeatureError::SpendLimitExceeded));
            }

            let token_client = token::Client::new(&env, &sub.token);
            let contract_address = env.current_contract_address();
            let transfer_ok = token_client
                .try_transfer_from(
                    &contract_address,
                    &sub.customer,
                    &sub.merchant,
                    &charge_amount,
                )
                .is_ok();

            if transfer_ok {
                sub.payment_count += 1;
                sub.retry_count = 0;
                sub.next_payment_at = sub.next_payment_at + sub.interval;
                sub.status = SubscriptionStatus::Active;

                if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                    sub.status = SubscriptionStatus::Expired;
                }

                env.storage().instance().set(
                    &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                    &sub,
                );
                env.storage()
                    .instance()
                    .remove(&DataKey::State(StateDataKey::DunningState(subscription_id)));

                (DunningResolved {
                    subscription_id,
                    resolved_at: now,
                })
                .publish(&env);

                (RecurringPaymentExecuted {
                    subscription_id,
                    payment_count: sub.payment_count,
                    amount: charge_amount,
                    next_payment_at: sub.next_payment_at,
                })
                .publish(&env);
            } else {
                dunning.retry_count += 1;
                dunning.last_failed_at = now;

                if dunning.retry_count >= dunning.max_retries {
                    sub.status = SubscriptionStatus::Suspended;
                    env.storage().instance().set(
                        &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                        &sub,
                    );
                    env.storage().instance().set(
                        &DataKey::State(StateDataKey::DunningState(subscription_id)),
                        &dunning,
                    );

                    (SubscriptionSuspended {
                        subscription_id,
                        reason: String::from_str(&env, "Maximum retries exceeded"),
                    })
                    .publish(&env);

                    return Ok(());
                }

                // Exponential backoff: backoff_seconds * 2^retry_count
                dunning.next_retry_at = now + (dunning.backoff_seconds << dunning.retry_count);
                env.storage().instance().set(
                    &DataKey::State(StateDataKey::DunningState(subscription_id)),
                    &dunning,
                );

                (RecurringPaymentFailed {
                    subscription_id,
                    retry_count: dunning.retry_count as u64,
                })
                .publish(&env);

                return Ok(());
            }

            return Ok(());
        }

        // Must be Active for the normal payment path
        if sub.status != SubscriptionStatus::Active {
            return Err(Error::Subscription(SubscriptionError::NotActive));
        }

        // Check subscription has not ended
        if sub.ends_at > 0 && now >= sub.ends_at {
            sub.status = SubscriptionStatus::Expired;
            env.storage().instance().set(
                &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                &sub,
            );
            return Err(Error::Subscription(SubscriptionError::Ended));
        }

        // Check merchant account is not paused
        let merchant_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantPaused(
                sub.merchant.clone(),
            )))
            .unwrap_or(false);
        if merchant_paused {
            return Err(Error::Subscription(SubscriptionError::MerchantPaused));
        }

        // Check payment is due
        if now < sub.next_payment_at {
            return Err(Error::Subscription(SubscriptionError::PaymentNotDue));
        }

        // Skip charge if still within trial period
        if sub.trial_data.ends_at > 0 && now < sub.trial_data.ends_at {
            sub.next_payment_at = sub.next_payment_at + sub.interval;
            env.storage().instance().set(
                &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                &sub,
            );
            return Ok(());
        }

        // Check customer spend limit (#282)
        let charge_amount =
            PaymentContract::get_discounted_subscription_amount(&env, subscription_id, sub.amount);
        if let Err(_) =
            PaymentContract::check_and_update_spend_limit(&env, &sub.customer, charge_amount)
        {
            return Err(Error::Feature(FeatureError::SpendLimitExceeded));
        }

        // Attempt token transfer
        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();

        let transfer_ok = token_client
            .try_transfer_from(
                &contract_address,
                &sub.customer,
                &sub.merchant,
                &charge_amount,
            )
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
            sub.next_payment_at = sub.next_payment_at + sub.interval;

            // Auto-expire when duration is reached
            if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                sub.status = SubscriptionStatus::Expired;
            }

            env.storage().instance().set(
                &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                &sub,
            );

            (RecurringPaymentExecuted {
                subscription_id,
                payment_count: sub.payment_count,
                amount: charge_amount,
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

            return Ok(());
        }

        Ok(())
    }

    /// Creates a new metered (usage-based) subscription.
    ///
    /// The merchant authorises creation. Usage is reported separately via `report_usage`
    /// and billed via `execute_metered_billing`.
    ///
    /// # Arguments
    /// * `merchant` - The merchant creating the subscription.
    /// * `customer` - The customer being subscribed.
    /// * `price_per_unit` - The cost per usage unit in token base units.
    /// * `unit_name` - A human-readable name for the usage unit (e.g., "API calls").
    /// * `token` - The payment token address.
    /// * `billing_cap` - Optional maximum billable amount per billing cycle.
    /// * `max_units_per_period` - Optional maximum number of units that can be accumulated per period.
    ///
    /// # Returns
    /// The metered subscription ID on success.
    ///
    /// # Errors
    /// - `Unauthorized` if the caller is not the merchant.
    pub fn create_metered_subscription(
        env: Env,
        merchant: Address,
        customer: Address,
        price_per_unit: i128,
        unit_name: String,
        token: Address,
        billing_cap: Option<i128>,
        max_units_per_period: Option<u64>,
    ) -> Result<u64, Error> {
        Self::require_not_paused(&env, "create_metered_subscription")?;
        Self::require_merchant_not_paused(&env, &merchant)?;
        merchant.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::MeteredCounter))
            .unwrap_or(0);
        let sub_id = counter + 1;

        let now = env.ledger().timestamp();

        let sub = MeteredSubscription {
            subscription_id: sub_id,
            merchant,
            customer,
            token,
            price_per_unit,
            unit_name,
            accumulated_units: 0,
            billing_cap,
            last_reset_at: now,
            max_units_per_period,
        };

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Metered(sub_id)),
            &sub,
        );
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::MeteredCounter),
            &sub_id,
        );

        Ok(sub_id)
    }

    /// Reports usage units for a metered subscription.
    ///
    /// The reported units are accumulated until the next billing cycle.
    ///
    /// # Arguments
    /// * `merchant` - The merchant authorizing the usage report (must own the subscription).
    /// * `subscription_id` - The metered subscription to report usage for.
    /// * `units` - The number of usage units to add to the accumulator.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// - `MeteredNotFound` if the subscription does not exist.
    /// - `Unauthorized` if the caller is not the subscription's merchant.
    /// - `UsageCapExceeded` if adding the units would exceed `max_units_per_period`.
    pub fn report_usage(
        env: Env,
        merchant: Address,
        subscription_id: u64,
        units: u64,
    ) -> Result<(), Error> {
        merchant.require_auth();

        let mut sub: MeteredSubscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Metered(
                subscription_id,
            )))
            .ok_or(Error::Subscription(SubscriptionError::MeteredNotFound))?;

        if sub.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if let Some(max_units) = sub.max_units_per_period {
            if sub.accumulated_units.saturating_add(units) > max_units {
                return Err(Error::Subscription(SubscriptionError::UsageCapExceeded));
            }
        }

        sub.accumulated_units = sub.accumulated_units.saturating_add(units);

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Metered(subscription_id)),
            &sub,
        );

        (UsageReported {
            subscription_id,
            units,
            accumulated: sub.accumulated_units,
        })
        .publish(&env);

        Ok(())
    }

    /// Returns the current usage state of a metered subscription.
    ///
    /// # Arguments
    /// * `subscription_id` - The metered subscription to query.
    ///
    /// # Returns
    /// The `MeteredSubscription` containing accumulated units, billing cap, and other usage data.
    ///
    /// # Panics
    /// Panics if the metered subscription is not found.
    pub fn get_current_usage(env: Env, subscription_id: u64) -> MeteredSubscription {
        env.storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Metered(
                subscription_id,
            )))
            .expect("MeteredSubscription not found")
    }

    /// Sets or updates the billing cap for a metered subscription.
    ///
    /// The billing cap limits the maximum amount that can be charged per billing cycle.
    ///
    /// # Arguments
    /// * `merchant` - The merchant authorizing the update (must own the subscription).
    /// * `subscription_id` - The metered subscription to update.
    /// * `cap` - The new billing cap amount in token base units.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// - `MeteredNotFound` if the subscription does not exist.
    /// - `Unauthorized` if the caller is not the subscription's merchant.
    pub fn set_billing_cap(
        env: Env,
        merchant: Address,
        subscription_id: u64,
        cap: i128,
    ) -> Result<(), Error> {
        merchant.require_auth();

        let mut sub: MeteredSubscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Metered(
                subscription_id,
            )))
            .ok_or(Error::Subscription(SubscriptionError::MeteredNotFound))?;

        if sub.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        sub.billing_cap = Some(cap);

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Metered(subscription_id)),
            &sub,
        );

        Ok(())
    }

    /// Executes a billing cycle for a metered subscription.
    ///
    /// Calculates the total charge from accumulated units × price per unit (capped by `billing_cap` if set),
    /// transfers the amount from the customer to the merchant, and resets the usage accumulator.
    ///
    /// # Arguments
    /// * `subscription_id` - The metered subscription to bill.
    ///
    /// # Returns
    /// The total amount charged on success, or `0` if no usage was accumulated.
    ///
    /// # Errors
    /// - `MeteredNotFound` if the subscription does not exist.
    /// - `BillingOverflow` if the multiplication of units × price overflows.
    pub fn execute_metered_billing(env: Env, subscription_id: u64) -> Result<i128, Error> {
        let mut sub: MeteredSubscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Metered(
                subscription_id,
            )))
            .ok_or(Error::Subscription(SubscriptionError::MeteredNotFound))?;

        Self::require_merchant_not_paused(&env, &sub.merchant)?;

        let units_billed = sub.accumulated_units;
        if units_billed == 0 {
            return Ok(0);
        }

        let mut amount = (units_billed as i128)
            .checked_mul(sub.price_per_unit)
            // .ok_or(Error::BillingOverflow)?;
            .ok_or(Error::Payment(PaymentError::BillingOverflow))?;

        let mut cap_hit = false;

        if let Some(cap) = sub.billing_cap {
            if amount > cap {
                amount = cap;
                cap_hit = true;
            }
        }

        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(&contract_address, &sub.customer, &sub.merchant, &amount);

        sub.accumulated_units = 0;
        sub.last_reset_at = env.ledger().timestamp();

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Metered(subscription_id)),
            &sub,
        );

        if cap_hit {
            (BillingCapReached {
                subscription_id,
                cap: sub.billing_cap.unwrap_or(0),
            })
            .publish(&env);
        }

        (MeteredBillingExecuted {
            subscription_id,
            amount,
            units_billed,
        })
        .publish(&env);

        Ok(amount)
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
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        let config: Option<MultiSigConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig));

        let is_authorized = sub.customer == caller
            || sub.merchant == caller
            || config.map_or(false, |c| c.admins.contains(&caller));

        if !is_authorized {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if sub.status == SubscriptionStatus::Cancelled || sub.status == SubscriptionStatus::Expired
        {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        sub.status = SubscriptionStatus::Cancelled;
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
            &sub,
        );

        Self::remove_from_merchant_active_subscriptions(&env, &sub.merchant, subscription_id);

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
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if sub.status != SubscriptionStatus::Active {
            return Err(Error::Subscription(SubscriptionError::NotActive));
        }

        sub.status = SubscriptionStatus::Paused;
        sub.pause_data.last_paused_at = env.ledger().timestamp();
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
            &sub,
        );

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
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if sub.status != SubscriptionStatus::Paused {
            return Err(Error::Payment(PaymentError::InvalidStatus));
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

        if sub.pause_data.proration_enabled {
            // Proration formula: (Full Amount * Remaining Time in Cycle) / Cycle Duration
            let remaining_time = sub.next_payment_at - now;
            let prorated_amount =
                (sub.amount * remaining_time as i128) / sub.interval as i128;

            if prorated_amount > 0 {
                let merchant_paused: bool = env
                    .storage()
                    .instance()
                    .get(&DataKey::Merchant(MerchantDataKey::MerchantPaused(
                        sub.merchant.clone(),
                    )))
                    .unwrap_or(false);
                if merchant_paused {
                    return Err(Error::Subscription(SubscriptionError::MerchantPaused));
                }

                if let Err(_) = PaymentContract::check_and_update_spend_limit(
                    &env,
                    &sub.customer,
                    prorated_amount,
                ) {
                    return Err(Error::Feature(FeatureError::SpendLimitExceeded));
                }

                let token_client = token::Client::new(&env, &sub.token);
                let contract_address = env.current_contract_address();
                let transfer_ok = token_client
                    .try_transfer_from(
                        &contract_address,
                        &sub.customer,
                        &sub.merchant,
                        &prorated_amount,
                    )
                    .is_ok();
                if !transfer_ok {
                    return Err(Error::Payment(PaymentError::TransferFailed));
                }

                sub.payment_count += 1;
            }

            (SubscriptionResumedProrated {
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

        sub.status = SubscriptionStatus::Active;

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
            &sub,
        );

        Ok(())
    }

    /// Read a single subscription.
    pub fn get_subscription(env: Env, subscription_id: u64) -> Subscription {
        env.storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
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
            .get(&DataKey::Customer(CustomerDataKey::SubscriptionCount(
                customer.clone(),
            )))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let end = (offset + limit).min(total);

        for i in offset..end {
            if let Some(sub_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::Customer(CustomerDataKey::Subscriptions(
                    customer.clone(),
                    i,
                )))
            {
                if let Some(sub) =
                    env.storage()
                        .instance()
                        .get::<DataKey, Subscription>(&DataKey::Subscription(
                            SubscriptionKey::Data(sub_id),
                        ))
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
            .get(&DataKey::Merchant(MerchantDataKey::SubscriptionCount(
                merchant.clone(),
            )))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let end = (offset + limit).min(total);

        for i in offset..end {
            if let Some(sub_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::Merchant(MerchantDataKey::Subscriptions(
                    merchant.clone(),
                    i,
                )))
            {
                if let Some(sub) =
                    env.storage()
                        .instance()
                        .get::<DataKey, Subscription>(&DataKey::Subscription(
                            SubscriptionKey::Data(sub_id),
                        ))
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::DunningConfig), &config);

        Ok(())
    }

    /// Returns the current dunning configuration.
    /// Returns default config if not yet set.
    pub fn get_dunning_config(env: Env) -> DunningConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::DunningConfig))
            .unwrap_or(DunningConfig {
                initial_backoff_seconds: 3600, // 1 hour
                max_retries: 5,
            })
    }

    /// Returns the dunning state for a subscription, if any.
    pub fn get_dunning_state(env: Env, subscription_id: u64) -> Option<DunningState> {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::DunningState(subscription_id)))
    }

    /// Retry a failed payment for a subscription in dunning.
    /// Validates that the retry is due before attempting.
    pub fn retry_failed_payment(env: Env, subscription_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        if sub.status != SubscriptionStatus::InDunning {
            return Err(Error::Subscription(SubscriptionError::NotInDunning));
        }

        let mut dunning: DunningState = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::DunningState(subscription_id)))
            .ok_or(Error::Subscription(SubscriptionError::DunningNotFound))?;

        let now = env.ledger().timestamp();

        // Enforce backoff window
        if now < dunning.next_retry_at {
            return Err(Error::Subscription(SubscriptionError::RetryTooEarly));
        }

        // Attempt the payment
        let charge_amount =
            PaymentContract::get_discounted_subscription_amount(&env, subscription_id, sub.amount);
        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();

        let transfer_ok = token_client
            .try_transfer_from(
                &contract_address,
                &sub.customer,
                &sub.merchant,
                &charge_amount,
            )
            .is_ok();

        if transfer_ok {
            sub.payment_count += 1;
            sub.retry_count = 0;
            sub.next_payment_at = now + sub.interval;
            sub.status = SubscriptionStatus::Active;

            if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                sub.status = SubscriptionStatus::Expired;
            }

            env.storage().instance().set(
                &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                &sub,
            );
            env.storage()
                .instance()
                .remove(&DataKey::State(StateDataKey::DunningState(subscription_id)));

            (DunningResolved {
                subscription_id,
                resolved_at: now,
            })
            .publish(&env);

            (RecurringPaymentExecuted {
                subscription_id,
                payment_count: sub.payment_count,
                amount: charge_amount,
                next_payment_at: sub.next_payment_at,
            })
            .publish(&env);

            Ok(())
        } else {
            dunning.retry_count += 1;
            dunning.last_failed_at = now;

            if dunning.retry_count >= dunning.max_retries {
                sub.status = SubscriptionStatus::Suspended;
                env.storage().instance().set(
                    &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                    &sub,
                );
                env.storage().instance().set(
                    &DataKey::State(StateDataKey::DunningState(subscription_id)),
                    &dunning,
                );

                (SubscriptionSuspended {
                    subscription_id,
                    reason: String::from_str(&env, "Maximum retries exceeded"),
                })
                .publish(&env);

                return Ok(());
            }

            // Exponential backoff: backoff_seconds * 2^retry_count
            dunning.next_retry_at = now + (dunning.backoff_seconds << dunning.retry_count);
            env.storage().instance().set(
                &DataKey::State(StateDataKey::DunningState(subscription_id)),
                &dunning,
            );

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

            Ok(())
        }
    }

    /// Admin resolves dunning for a subscription, returning it to active state.
    pub fn resolve_dunning(env: Env, admin: Address, subscription_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        if sub.status != SubscriptionStatus::InDunning
            && sub.status != SubscriptionStatus::Suspended
        {
            return Err(Error::Subscription(SubscriptionError::NotInDunning));
        }

        // Reset to active state
        sub.status = SubscriptionStatus::Active;
        sub.retry_count = 0;
        sub.next_payment_at = env.ledger().timestamp() + sub.interval;

        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
            &sub,
        );

        // Remove dunning state
        env.storage()
            .instance()
            .remove(&DataKey::State(StateDataKey::DunningState(subscription_id)));

        (DunningResolved {
            subscription_id,
            resolved_at: env.ledger().timestamp(),
        })
        .publish(&env);

        Ok(())
    }

    /// Sum of installment deposits actually received for a pending payment.
    fn get_payment_deposited_amount(env: &Env, payment_id: u64) -> i128 {
        let installment_counter: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::PartialPaymentCounter(
                payment_id,
            )))
            .unwrap_or(0);

        let mut total_deposited = 0i128;
        for i in 1..=installment_counter {
            if let Some(record) = env
                .storage()
                .instance()
                .get::<DataKey, PartialPaymentRecord>(&DataKey::State(
                    StateDataKey::PartialPaymentRecord(payment_id, i),
                ))
            {
                total_deposited += record.amount_paid;
            }
        }
        total_deposited
    }

    /// Apply an active subscription group's discount_bps to a billing amount.
    fn get_discounted_subscription_amount(env: &Env, subscription_id: u64, amount: i128) -> i128 {
        if let Some(group_id) = env.storage().instance().get::<DataKey, u64>(
            &DataKey::Subscription(SubscriptionKey::GroupMembership(subscription_id)),
        ) {
            if let Some(group) = env.storage().instance().get::<DataKey, SubscriptionGroup>(
                &DataKey::Subscription(SubscriptionKey::Group(group_id)),
            ) {
                if group.active && group.discount_bps > 0 {
                    let bps = group.discount_bps.min(10_000) as i128;
                    return amount * (10_000 - bps) / 10_000;
                }
            }
        }
        amount
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

        env.storage().instance().set(
            &DataKey::State(StateDataKey::DunningState(subscription_id)),
            &dunning_state,
        );

        // Update subscription status
        if let Some(mut sub) =
            env.storage()
                .instance()
                .get::<DataKey, Subscription>(&DataKey::Subscription(SubscriptionKey::Data(
                    subscription_id,
                )))
        {
            sub.status = SubscriptionStatus::InDunning;
            env.storage().instance().set(
                &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
                &sub,
            );
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::RateLimitConfig), &config);
        Ok(())
    }

    /// Returns the current rate limit configuration.
    /// Defaults to unlimited if not yet configured.
    pub fn get_rate_limit_config(env: Env) -> RateLimitConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::RateLimitConfig))
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
            .get(&DataKey::Customer(CustomerDataKey::RateLimit(
                address.clone(),
            )))
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let mut rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::RateLimit(
                address.clone(),
            )))
            .unwrap_or(AddressRateLimit {
                address: address.clone(),
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            });
        if rate_limit.flagged {
            return Err(Error::Basic(BasicError::AddressAlreadyFlagged));
        }
        rate_limit.flagged = true;
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::RateLimit(address.clone())),
            &rate_limit,
        );
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::FlagReason(address.clone())),
            &reason,
        );
        (AddressFlagged { address, reason }).publish(&env);
        Ok(())
    }

    /// Admin removes the flag from an address, allowing it to create payments again.
    pub fn unflag_address(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let mut rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::RateLimit(
                address.clone(),
            )))
            .unwrap_or(AddressRateLimit {
                address: address.clone(),
                payment_count: 0,
                window_start: 0,
                daily_volume: 0,
                last_payment_at: 0,
                flagged: false,
            });
        if !rate_limit.flagged {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        rate_limit.flagged = false;
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::RateLimit(address.clone())),
            &rate_limit,
        );
        env.storage()
            .instance()
            .remove(&DataKey::Customer(CustomerDataKey::FlagReason(
                address.clone(),
            )));
        (AddressUnflagged { address }).publish(&env);
        Ok(())
    }

    /// Checks whether an address has been flagged by an admin.
    ///
    /// Flagged addresses are blocked from creating payments unless explicitly allowlisted.
    ///
    /// # Arguments
    /// * `address` - The address to check.
    ///
    /// # Returns
    /// `true` if the address is flagged, `false` otherwise.
    pub fn is_address_flagged(env: Env, address: Address) -> bool {
        let rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::RateLimit(
                address.clone(),
            )))
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

    /// Returns the reason an address was flagged, if any.
    ///
    /// # Arguments
    /// * `address` - The address to query.
    ///
    /// # Returns
    /// `Some(reason)` if the address is flagged and a reason was recorded, `None` otherwise.
    pub fn get_flag_reason(env: Env, address: Address) -> Option<String> {
        env.storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::FlagReason(address)))
    }

    /// Adds an address to the allowlist, bypassing the flagged-address block.
    ///
    /// Allowlisted addresses can create payments even if they have been flagged.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `address` - The address to allowlist.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized.
    pub fn add_to_allowlist(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Allowlist(address)),
            &true,
        );
        Ok(())
    }

    /// Removes an address from the allowlist.
    ///
    /// Once removed, a flagged address will again be blocked from creating payments.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `address` - The address to remove from the allowlist.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized.
    pub fn remove_from_allowlist(env: Env, admin: Address, address: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .remove(&DataKey::Customer(CustomerDataKey::Allowlist(address)));
        Ok(())
    }

    /// Internal check called by create_payment. Validates the address against
    /// the configured rate limits and updates per-address counters.
    fn check_rate_limit_internal(env: &Env, address: &Address, amount: i128) -> Result<(), Error> {
        // If no config is set, rate limiting is disabled.
        let config: Option<RateLimitConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::RateLimitConfig));
        let config = match config {
            None => {
                return Ok(());
            }
            Some(c) => c,
        };

        let mut rate_limit: AddressRateLimit = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::RateLimit(
                address.clone(),
            )))
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
            return Err(Error::Basic(BasicError::AddressFlagged));
        }

        // Reject payment if it exceeds the single-transaction amount cap.
        if config.max_payment_amount > 0 && amount > config.max_payment_amount {
            return Err(Error::Basic(BasicError::AmountExceedsLimit));
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
            return Err(Error::Basic(BasicError::RateLimitExceeded));
        }

        // Enforce daily volume limit.
        if config.max_daily_volume > 0 {
            let new_volume = rate_limit.daily_volume.saturating_add(amount);
            if new_volume > config.max_daily_volume {
                return Err(Error::Basic(BasicError::DailyVolumeExceeded));
            }
            rate_limit.daily_volume = new_volume;
        }

        // Record successful check: increment counters and persist.
        rate_limit.payment_count += 1;
        rate_limit.last_payment_at = now;

        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::RateLimit(address.clone())),
            &rate_limit,
        );

        Ok(())
    }

    fn is_allowlisted(env: &Env, address: &Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::Allowlist(
                address.clone(),
            )))
            .unwrap_or(false)
    }

    /// Sets a custom rate limit configuration for a specific merchant.
    ///
    /// Overrides the global rate limit for this merchant. Use `reset_merchant_rate_limit`
    /// to revert to global defaults.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `merchant` - The merchant to configure the rate limit for.
    /// * `config` - The merchant-specific rate limit configuration.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::MerchantRateLimit(merchant)),
            &config,
        );
        Ok(())
    }

    /// Returns the custom rate limit configuration for a merchant, if set.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    ///
    /// # Returns
    /// `Some(MerchantRateLimit)` if a custom config exists, `None` if the merchant uses global defaults.
    pub fn get_merchant_rate_limit(env: Env, merchant: Address) -> Option<MerchantRateLimit> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::MerchantRateLimit(merchant)))
    }

    /// Removes the custom rate limit for a merchant, reverting to global defaults.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `merchant` - The merchant whose rate limit config should be removed.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized.
    pub fn reset_merchant_rate_limit(
        env: Env,
        admin: Address,
        merchant: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        let ms_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .remove(&DataKey::Feature(FeatureKey::MerchantRateLimit(merchant)));
        Ok(())
    }

    /// Checks whether a payment of the given amount would pass the merchant's rate limit.
    ///
    /// Considers the merchant's verification tier limits, custom merchant limits,
    /// and the global rate limit config. Does **not** update any counters — use
    /// this as a read-only pre-check.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to check rate limits for.
    /// * `amount` - The proposed payment amount.
    ///
    /// # Returns
    /// `true` if the payment is allowed, `false` if it would exceed a rate limit.
    pub fn check_rate_limit(env: Env, merchant: Address, amount: i128) -> bool {
        let level = Self::get_merchant_verification_level(env.clone(), merchant.clone());
        let tier_limits_opt = Self::get_tier_limits(env.clone(), level);

        let (max_tx, max_amt) = if let Some(tier_limits) = tier_limits_opt {
            (tier_limits.tx_per_period, tier_limits.volume_limit)
        } else if let Some(custom_limit) =
            env.storage()
                .instance()
                .get::<_, MerchantRateLimit>(&DataKey::Feature(FeatureKey::MerchantRateLimit(
                    merchant.clone(),
                )))
        {
            (
                custom_limit.max_transactions_per_hour,
                custom_limit.max_amount_per_hour,
            )
        } else {
            // Fallback to global config
            let config: Option<RateLimitConfig> = env
                .storage()
                .instance()
                .get(&DataKey::Config(ConfigKey::RateLimitConfig));
            if let Some(config) = config {
                if config.max_payment_amount > 0 && amount > config.max_payment_amount {
                    return false;
                }
            }
            return true;
        };

        if let Some(l) = env
            .storage()
            .instance()
            .get::<_, MerchantRateLimit>(&DataKey::Feature(FeatureKey::MerchantRateLimit(
                merchant.clone(),
            )))
        {
            let now = env.ledger().timestamp();
            let reset_needed = l.window_start > 0 && now >= l.window_start + 3600;
            let current_transactions = if reset_needed {
                0
            } else {
                l.current_transactions
            };
            let current_amount = if reset_needed { 0 } else { l.current_amount };

            if max_tx > 0 && current_transactions >= max_tx {
                return false;
            }
            if max_amt > 0 && current_amount + amount > max_amt {
                return false;
            }
            return true;
        } else {
            if max_amt > 0 && amount > max_amt {
                return false;
            }
            return true;
        }
    }

    fn check_merchant_rate_limit(env: &Env, merchant: &Address, amount: i128) -> Result<(), Error> {
        let level = Self::get_merchant_verification_level(env.clone(), merchant.clone());
        let tier_limits_opt = Self::get_tier_limits(env.clone(), level);

        let (max_tx, max_amt) = if let Some(tier_limits) = tier_limits_opt {
            (tier_limits.tx_per_period, tier_limits.volume_limit)
        } else if let Some(custom_limit) =
            env.storage()
                .instance()
                .get::<_, MerchantRateLimit>(&DataKey::Feature(FeatureKey::MerchantRateLimit(
                    merchant.clone(),
                )))
        {
            (
                custom_limit.max_transactions_per_hour,
                custom_limit.max_amount_per_hour,
            )
        } else {
            let config: Option<RateLimitConfig> = env
                .storage()
                .instance()
                .get(&DataKey::Config(ConfigKey::RateLimitConfig));
            if let Some(config) = config {
                if config.max_payment_amount > 0 && amount > config.max_payment_amount {
                    return Err(Error::Basic(BasicError::AmountExceedsLimit));
                }
            }
            return Ok(());
        };

        let mut limit = env
            .storage()
            .instance()
            .get::<_, MerchantRateLimit>(&DataKey::Feature(FeatureKey::MerchantRateLimit(
                merchant.clone(),
            )))
            .unwrap_or(MerchantRateLimit {
                merchant: merchant.clone(),
                max_transactions_per_hour: max_tx,
                max_amount_per_hour: max_amt,
                current_transactions: 0,
                current_amount: 0,
                window_start: 0,
            });

        limit.max_transactions_per_hour = max_tx;
        limit.max_amount_per_hour = max_amt;

        let now = env.ledger().timestamp();

        if limit.window_start > 0 && now >= limit.window_start + 3600 {
            limit.current_transactions = 0;
            limit.current_amount = 0;
            limit.window_start = now;
        } else if limit.window_start == 0 {
            limit.window_start = now;
        }

        if limit.max_transactions_per_hour > 0
            && limit.current_transactions >= limit.max_transactions_per_hour
        {
            return Err(Error::Payment(PaymentError::MerchantRateLimitExceeded));
        }
        if limit.max_amount_per_hour > 0
            && limit.current_amount + amount > limit.max_amount_per_hour
        {
            return Err(Error::Payment(PaymentError::AmountRateLimitExceeded));
        }

        limit.current_transactions += 1;
        limit.current_amount += amount;
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::MerchantRateLimit(merchant.clone())),
            &limit,
        );

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
        let expiry = release_timestamp + 86400 * 7;
        let call = client.try_create_escrow(
            customer,
            merchant,
            &amount,
            token,
            &release_timestamp,
            &min_hold_period,
            &expiry,
            &false,
        );
        match call {
            Ok(Ok(escrow_id)) => Ok(escrow_id),
            _ => Err(Error::Feature(FeatureError::EscrowBridgeFailed)),
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
                    .get(&DataKey::Config(ConfigKey::MultiSigConfig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if !config.admins.contains(&new_admin) {
                    config.admins.push_back(new_admin.clone());
                    config.total_admins += 1;
                    env.storage()
                        .instance()
                        .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);
                    (AdminAdded { admin: new_admin }).publish(env);
                }
            }
            ActionType::RemoveAdmin => {
                let admin_to_remove = proposal.target.clone();
                let mut config: MultiSigConfig = env
                    .storage()
                    .instance()
                    .get(&DataKey::Config(ConfigKey::MultiSigConfig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if config.total_admins <= config.required_signatures {
                    return Err(Error::Basic(BasicError::InsufficientAdmins));
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
                    .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);
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
                    .get(&DataKey::Config(ConfigKey::MultiSigConfig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if required == 0 || required > config.total_admins {
                    return Err(Error::Basic(BasicError::InsufficientAdmins));
                }
                config.required_signatures = required;
                env.storage()
                    .instance()
                    .set(&DataKey::Config(ConfigKey::MultiSigConfig), &config);
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        (FeeConfigUpdated {
            fee_bps: fee_config.fee_bps,
            treasury: fee_config.treasury.clone(),
        })
        .publish(&env);
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::FeeConfig), &fee_config);
        Ok(())
    }

    /// Returns the current fee configuration.
    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig))
            .ok_or(Error::Feature(FeatureError::FeeConfigNotFound))
    }

    /// Admin sets the maximum forward chain depth.
    /// 
    /// # Arguments
    /// * `admin` - The admin address (must be in multisig config)
    /// * `max_depth` - Maximum allowed hops in a forward chain (default: 5)
    /// 
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized or contract is paused.
    pub fn set_max_forward_depth(env: Env, admin: Address, max_depth: u32) -> Result<(), Error> {
        Self::require_not_paused(&env, "set_max_forward_depth")?;
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::MaxForwardDepth), &max_depth);
        Ok(())
    }

    /// Returns the maximum forward chain depth.
    pub fn get_max_forward_depth(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MaxForwardDepth))
            .unwrap_or(5)
    }

    /// Calculates the fee for a given amount and merchant (accounting for tier discount and waivers).
    pub fn calculate_fee(env: Env, amount: i128, merchant: Address) -> i128 {
        let config: Option<FeeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig));
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

        // Risk-based surcharge applied at completion time by `deduct_fee`.
        let risk_surcharge_bps =
            PaymentContract::risk_surcharge_bps(&env, &customer, amount, &currency);

        PaymentContract::compute_fee_amount(
            amount,
            effective_fee_bps,
            &FeeTier::Standard, // Tier already applied in get_effective_fee_bps
            config.min_fee,
            config.max_fee,
            risk_surcharge_bps,
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let mut record =
            PaymentContract::get_or_default_merchant_fee_record(&env, merchant.clone());
        let old_tier = record.fee_tier.clone();
        record.fee_tier = tier.clone();
        if PaymentContract::tier_rank(&tier) < PaymentContract::tier_rank(&old_tier) {
            record.tier_volume_baseline = record.total_volume;
        }
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::FeeRecord(merchant)),
            &record,
        );
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        PaymentContract::validate_thresholds(&thresholds)?;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::TierThresholds), &thresholds);
        Ok(())
    }

    /// Returns the total fees accumulated in the contract.
    pub fn get_accumulated_fees(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::AccumulatedFees))
            .unwrap_or(0)
    }

    /// Admin withdraws accumulated fees to the treasury address.
    pub fn withdraw_fees(env: Env, admin: Address, amount: i128) -> Result<(), Error> {
        admin.require_auth();
        let multisig: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let fee_config: FeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig))
            .ok_or(Error::Feature(FeatureError::FeeConfigNotFound))?;
        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::AccumulatedFees))
            .unwrap_or(0);
        if amount > accumulated {
            return Err(Error::Feature(FeatureError::InsufficientFees));
        }
        let token_client = token::Client::new(&env, &fee_config.fee_token);
        token_client.transfer(
            &env.current_contract_address(),
            &fee_config.treasury,
            &amount,
        );
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::AccumulatedFees),
            &(accumulated - amount),
        );
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
        currency: Currency,
    ) -> (i128, i128) {
        let config: Option<FeeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig));
        let config = match config {
            None => {
                return (amount, 0);
            }
            Some(c) if !c.active => {
                return (amount, 0);
            }
            Some(c) => c,
        };

        if !PaymentContract::is_token_allowed(env, token) {
            return (amount, 0);
        }

        let record = PaymentContract::get_or_default_merchant_fee_record(env, merchant.clone());
        let risk_config: RiskFeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::RiskFeeConfig))
            .unwrap_or(RiskFeeConfig {
                base_fee_bps: 100,
                large_amount_threshold: 1000000,
                large_amount_surcharge_bps: 50,
                new_customer_surcharge_bps: 100,
                high_risk_currency_surcharge: 200,
            });
        let risk_surcharge_bps = PaymentContract::calculate_risk_score(
            env.clone(),
            customer.clone(),
            merchant.clone(),
            amount,
            currency,
        );
        let effective_bps = (config.fee_bps as u64 + risk_surcharge_bps as u64)
            .min(1000u64) as u32;
        let fee = PaymentContract::compute_fee_amount(
            amount,
            effective_bps,
            &record.fee_tier,
            config.min_fee,
            config.max_fee,
            risk_surcharge_bps,
        );

        if fee <= 0 {
            return (amount, 0);
        }

        let net_amount = amount.saturating_sub(fee);

        // Transfer fee from customer to contract
        let token_client = token::Client::new(env, token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(&contract_address, customer, &contract_address, &fee);

        // Update accumulated fees
        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::AccumulatedFees))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::AccumulatedFees),
            &(accumulated + fee),
        );

        if risk_surcharge_bps > 0 {
            (RiskFeeApplied {
                payment_id,
                base_fee_bps: config.fee_bps,
                risk_surcharge_bps,
                total_fee_bps: effective_bps,
            })
            .publish(env);
        }

        (FeeCollected {
            payment_id,
            fee_amount: fee,
            merchant,
        })
        .publish(env);

        (net_amount, fee)
    }

    /// Computes the fee amount applying the tier discount, adding the
    /// risk-based surcharge, then clamping to min/max.
    fn compute_fee_amount(
        amount: i128,
        fee_bps: u32,
        tier: &FeeTier,
        min_fee: i128,
        max_fee: i128,
        risk_surcharge_bps: u32,
    ) -> i128 {
        let discount = PaymentContract::get_tier_discount_bps(tier);
        let effective_bps = fee_bps - (fee_bps * discount) / 10000 + risk_surcharge_bps;
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

    /// Risk-based fee surcharge (in bps) applied to a payment on top of the
    /// tier/waiver-adjusted platform fee.
    ///
    /// Opt-in: returns 0 unless an admin has explicitly configured a
    /// `RiskFeeConfig`. Both the preview path (`calculate_fee`) and the
    /// deduction path (`deduct_fee`) call this so they stay in sync. Surcharge
    /// components: large amount, new customer (fewer than 3 prior payments),
    /// and high-risk currency (BTC/ETH).
    fn risk_surcharge_bps(env: &Env, customer: &Address, amount: i128, currency: &Currency) -> u32 {
        let config: RiskFeeConfig = match env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::RiskFeeConfig))
        {
            Some(c) => c,
            None => return 0,
        };

        let mut surcharge: u32 = 0;

        if amount > config.large_amount_threshold {
            surcharge += config.large_amount_surcharge_bps;
        }

        let customer_payment_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::PaymentCount(
                customer.clone(),
            )))
            .unwrap_or(0);
        if customer_payment_count < 3 {
            surcharge += config.new_customer_surcharge_bps;
        }

        match currency {
            Currency::BTC | Currency::ETH => {
                surcharge += config.high_risk_currency_surcharge;
            }
            _ => {}
        }

        surcharge
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
            .get(&DataKey::Merchant(MerchantDataKey::FeeRecord(
                merchant.clone(),
            )))
            .unwrap_or(MerchantFeeRecord {
                merchant,
                total_fees_paid: 0,
                total_volume: 0,
                fee_tier: FeeTier::Standard,
                tier_volume_baseline: 0,
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
            .get(&DataKey::Config(ConfigKey::TierThresholds))
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
            return Err(Error::Basic(BasicError::InvalidTierThresholds));
        }

        let mut has_premium = false;
        let mut has_enterprise = false;
        let mut prev_rank: Option<u32> = None;
        let mut prev_value: Option<i128> = None;

        for pair in thresholds.iter() {
            let rank = PaymentContract::tier_rank(&pair.0);
            if let Some(r) = prev_rank {
                if rank <= r {
                    return Err(Error::Basic(BasicError::InvalidTierThresholds));
                }
            }
            if let Some(v) = prev_value {
                if pair.1 <= v {
                    return Err(Error::Basic(BasicError::InvalidTierThresholds));
                }
            }
            if pair.1 < 0 {
                return Err(Error::Basic(BasicError::InvalidTierThresholds));
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
            return Err(Error::Basic(BasicError::InvalidTierThresholds));
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
        let volume_for_tier = record
            .total_volume
            .saturating_sub(record.tier_volume_baseline);
        let computed_tier = PaymentContract::calculate_tier(env, volume_for_tier);
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

        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::FeeRecord(merchant)),
            &record,
        );
    }

    // ── FEE WAIVER FUNCTIONS ───────────────────────────────────────────────────

    /// Grants a fee waiver to a merchant, reducing their platform fee by the specified BPS.
    ///
    /// The waiver applies to all future payments until it expires or is revoked.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `merchant` - The merchant to grant the waiver to.
    /// * `waiver_bps` - The fee reduction in basis points (0–10000, where 10000 = 100% waiver).
    /// * `valid_until` - The ledger timestamp at which the waiver expires.
    /// * `reason` - A human-readable reason for granting the waiver.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized or the BPS value is invalid.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Validate waiver_bps is between 0 and 10000 (100%)
        if let Err(_) = Self::validate_bps(waiver_bps) {
            return Err(Error::Basic(BasicError::InvalidTierThresholds));
        }

        let waiver = FeeWaiver {
            merchant: merchant.clone(),
            waiver_bps,
            valid_until,
            reason,
            granted_by: admin.clone(),
        };

        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::FeeWaiver(merchant.clone())),
            &waiver,
        );

        (FeeWaiverGranted {
            merchant,
            waiver_bps,
            valid_until,
        })
        .publish(&env);

        Ok(())
    }

    /// Revokes an active fee waiver for a merchant.
    ///
    /// After revocation, the merchant reverts to their standard fee tier.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `merchant` - The merchant whose waiver should be revoked.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized or no waiver exists.
    pub fn revoke_fee_waiver(env: Env, admin: Address, merchant: Address) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if waiver exists
        let _waiver: FeeWaiver = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::FeeWaiver(
                merchant.clone(),
            )))
            .ok_or(Error::Payment(PaymentError::NotFound))?; // Reuse existing error

        // Remove the waiver
        env.storage()
            .instance()
            .remove(&DataKey::Customer(CustomerDataKey::FeeWaiver(
                merchant.clone(),
            )));

        (FeeWaiverRevoked {
            merchant,
            revoked_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    /// Returns the active fee waiver for a merchant, if any.
    ///
    /// Automatically removes and returns `None` if the waiver has expired.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    ///
    /// # Returns
    /// `Some(FeeWaiver)` if a valid waiver exists, `None` otherwise.
    pub fn get_fee_waiver(env: Env, merchant: Address) -> Option<FeeWaiver> {
        let waiver: Option<FeeWaiver> =
            env.storage()
                .instance()
                .get(&DataKey::Customer(CustomerDataKey::FeeWaiver(
                    merchant.clone(),
                )));

        // Check if waiver is expired
        if let Some(w) = waiver {
            if env.ledger().timestamp() > w.valid_until {
                // Remove expired waiver and publish expiration event
                env.storage()
                    .instance()
                    .remove(&DataKey::Customer(CustomerDataKey::FeeWaiver(
                        merchant.clone(),
                    )));

                (FeeWaiverExpired { merchant }).publish(&env);
                None
            } else {
                Some(w)
            }
        } else {
            None
        }
    }

    /// Computes the effective fee rate for a merchant after applying tier discount and waiver.
    ///
    /// Returns 0 if no fee config exists or fees are inactive.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to compute the effective fee for.
    ///
    /// # Returns
    /// The effective fee in basis points.
    pub fn get_effective_fee_bps(env: Env, merchant: Address) -> u32 {
        // Get base fee configuration
        let config: Option<FeeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig));
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

    // ── FEE REBATE FUNCTIONS ──────────────────────────────────────────────────

    /// Configures the fee rebate program for merchants.
    ///
    /// When active, merchants accrue a rebate on fees paid above a volume threshold,
    /// which they can later claim via `claim_fee_rebate`.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing this operation (must be a multisig admin).
    /// * `config` - The rebate configuration including BPS, threshold volume, and period.
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if unauthorized.
    pub fn configure_fee_rebate(
        env: Env,
        admin: Address,
        config: FeeRebateConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::FeeRebateConfig), &config);
        Ok(())
    }

    /// Returns the current rebate accrual state for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant to query.
    ///
    /// # Returns
    /// `Some(MerchantRebateAccrual)` if accrual data exists, `None` otherwise.
    pub fn get_rebate_accrual(env: Env, merchant: Address) -> Option<MerchantRebateAccrual> {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::RebateAccrual(merchant)))
    }

    /// Claims the accrued fee rebate for a merchant.
    ///
    /// Transfers the rebate amount from accumulated fees to the merchant and resets the accrual.
    ///
    /// # Arguments
    /// * `merchant` - The merchant claiming the rebate.
    ///
    /// # Returns
    /// The rebate amount transferred on success.
    ///
    /// # Errors
    /// - `RebateConfigNotFound` if the rebate program is not configured or inactive.
    /// - `RebateThresholdNotMet` if no accrual data exists for the merchant.
    /// - `RebateAlreadyClaimed` if the accrued rebate is zero (already claimed).
    pub fn claim_fee_rebate(env: Env, merchant: Address) -> Result<i128, Error> {
        merchant.require_auth();

        let config: FeeRebateConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeRebateConfig))
            .ok_or(Error::Feature(FeatureError::RebateConfigNotFound))?;

        if !config.active {
            return Err(Error::Feature(FeatureError::RebateConfigNotFound));
        }

        let accrual: MerchantRebateAccrual = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::RebateAccrual(
                merchant.clone(),
            )))
            .ok_or(Error::Feature(FeatureError::RebateThresholdNotMet))?;

        if accrual.accrued_rebate == 0 {
            return Err(Error::Feature(FeatureError::RebateAlreadyClaimed));
        }

        let rebate = accrual.accrued_rebate;

        // Deduct from accumulated fees and transfer to merchant
        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::AccumulatedFees))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::AccumulatedFees),
            &(accumulated - rebate),
        );

        let fee_config: FeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig))
            .ok_or(Error::Feature(FeatureError::FeeConfigNotFound))?;
        let token_client = token::Client::new(&env, &fee_config.fee_token);
        token_client.transfer(&env.current_contract_address(), &merchant, &rebate);

        // Reset accrual (keep period_start and period_volume, zero out rebate)
        let reset = MerchantRebateAccrual {
            merchant: accrual.merchant,
            accrued_rebate: 0,
            period_start: accrual.period_start,
            period_volume: accrual.period_volume,
        };
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::RebateAccrual(merchant)),
            &reset,
        );

        Ok(rebate)
    }

    fn maybe_accrue_fee_rebate(
        env: &Env,
        merchant: Address,
        payment_amount: i128,
        fee_amount: i128,
    ) {
        let config: Option<FeeRebateConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeRebateConfig));
        let config = match config {
            Some(c) if c.active => c,
            _ => return,
        };

        let now = env.ledger().timestamp();

        let mut accrual: MerchantRebateAccrual = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::RebateAccrual(
                merchant.clone(),
            )))
            .unwrap_or(MerchantRebateAccrual {
                merchant: merchant.clone(),
                accrued_rebate: 0,
                period_start: now,
                period_volume: 0,
            });

        // Reset period if elapsed
        if now >= accrual.period_start + config.rebate_period_seconds {
            accrual.period_start = now;
            accrual.period_volume = 0;
            accrual.accrued_rebate = 0;
        }

        accrual.period_volume += payment_amount;

        // Only accrue rebate on volume above the threshold
        if accrual.period_volume > config.threshold_volume && fee_amount > 0 {
            let rebate = (fee_amount * (config.rebate_bps as i128)) / 10000;
            accrual.accrued_rebate += rebate;
        }

        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::RebateAccrual(merchant)),
            &accrual,
        );
    }

    // ── BATCH PAYMENT OPERATIONS ──────────────────────────────────────────────

    fn validate_batch_size(len: u32) -> Result<(), Error> {
        const MAX_BATCH_SIZE: u32 = 50;
        if len == 0 || len > MAX_BATCH_SIZE {
            return Err(Error::Basic(BasicError::InvalidBatchSize));
        }
        Ok(())
    }

    /// Creates multiple payments in a single transaction.
    ///
    /// Each unique customer in the batch must authorize the transaction. Individual
    /// payment failures do not revert the entire batch — results are returned per entry.
    ///
    /// # Arguments
    /// * `entries` - A vector of `BatchPaymentEntry` structs describing each payment.
    ///
    /// # Returns
    /// A vector of `BatchResult` indicating success/failure for each entry.
    ///
    /// # Errors
    /// - `InvalidBatchSize` if the batch is empty or exceeds 50 entries.
    /// - `ContractPaused` if the contract is paused.
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
                        error_code: Some(e.to_u32()),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Completes multiple payments in a single transaction.
    ///
    /// Only admin (multisig) users may call this. Each payment is individually
    /// completed — failures do not revert the batch.
    ///
    /// # Arguments
    /// * `admin` - The admin authorizing the batch completion.
    /// * `payment_ids` - A vector of payment IDs to complete.
    ///
    /// # Returns
    /// A vector of `BatchResult` indicating success/failure for each payment.
    ///
    /// # Errors
    /// - `InvalidBatchSize` if the batch is empty or exceeds 50 entries.
    /// - `Unauthorized` if the caller is not a multisig admin.
    /// - `ContractPaused` if the contract is paused.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
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
                        error_code: Some(e.to_u32()),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Cancels multiple payments in a single transaction.
    ///
    /// The caller must be authorized for each payment (customer, merchant, or admin).
    /// Individual cancellation failures do not revert the batch.
    ///
    /// # Arguments
    /// * `caller` - The address authorizing the cancellations.
    /// * `payment_ids` - A vector of payment IDs to cancel.
    ///
    /// # Returns
    /// A vector of `BatchResult` indicating success/failure for each payment.
    ///
    /// # Errors
    /// - `InvalidBatchSize` if the batch is empty or exceeds 50 entries.
    /// - `ContractPaused` if the contract is paused.
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
                        error_code: Some(e.to_u32()),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Creates a batch of payments atomically with aggregated transfers.
    ///
    /// Validates each entry (currency, metadata, sanctions, rate limits, spend limits),
    /// creates payment records, and executes token transfers. Transfers to merchants
    /// are aggregated by token/merchant pair for gas efficiency.
    ///
    /// # Arguments
    /// * `admin` - Admin address initiating the batch (must be a multi-sig admin).
    /// * `entries` - Vector of `BatchPaymentEntry` items to process.
    ///
    /// # Returns
    /// A vector of `BatchResult` indicating success/failure and payment ID for each entry.
    ///
    /// # Errors
    /// Returns an error if the contract is paused, admin is unauthorized,
    /// or the batch size exceeds the configured maximum.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
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

        for entry in entries.iter() {
            // Validate currency
            if !PaymentContract::is_valid_currency(&entry.currency) {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(Error::Basic(BasicError::InvalidCurrency).to_u32()),
                });
                continue;
            }

            // Validate metadata size
            if entry.metadata.len() > MAX_METADATA_SIZE {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(Error::Basic(BasicError::MetadataTooLarge).to_u32()),
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
                    error_code: Some(Error::Basic(BasicError::AddressFlagged).to_u32()),
                });
                continue;
            }

            // Check rate limits
            if let Err(e) =
                PaymentContract::check_rate_limit_internal(&env, &entry.customer, entry.amount)
            {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(e.to_u32()),
                });
                continue;
            }

            // Check merchant rate limits
            if let Err(e) =
                PaymentContract::check_merchant_rate_limit(&env, &entry.merchant, entry.amount)
            {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(e.to_u32()),
                });
                continue;
            }

            // Check customer spend limit
            if let Err(e) =
                PaymentContract::check_and_update_spend_limit(&env, &entry.customer, entry.amount)
            {
                results.push_back(BatchResult {
                    payment_id: 0,
                    success: false,
                    error_code: Some(e.to_u32()),
                });
                continue;
            }

            // Create payment record
            let counter: u64 = env
                .storage()
                .instance()
                .get(&DataKey::Payment(PaymentKey::Counter))
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
                status: PaymentStatus::Pending, // completed via do_complete_payment below
                created_at: current_timestamp,
                expires_at,
                metadata: entry.metadata.clone(),
                notes: String::from_str(&env, ""),
                refunded_amount: 0,
            };

            env.storage()
                .instance()
                .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);
            env.storage()
                .instance()
                .set(&DataKey::Payment(PaymentKey::Counter), &payment_id);

            // Index by customer
            let customer_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::Customer(CustomerDataKey::PaymentCount(
                    entry.customer.clone(),
                )))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Customer(CustomerDataKey::Payments(
                    entry.customer.clone(),
                    customer_count,
                )),
                &payment_id,
            );
            env.storage().instance().set(
                &DataKey::Customer(CustomerDataKey::PaymentCount(entry.customer.clone())),
                &(customer_count + 1),
            );

            // Index by merchant
            let merchant_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::Merchant(MerchantDataKey::PaymentCount(
                    entry.merchant.clone(),
                )))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Merchant(MerchantDataKey::Payments(
                    entry.merchant.clone(),
                    merchant_count,
                )),
                &payment_id,
            );
            env.storage().instance().set(
                &DataKey::Merchant(MerchantDataKey::PaymentCount(entry.merchant.clone())),
                &(merchant_count + 1),
            );

            // Paged merchant payment index (100 entries per page)
            {
                const PAGE_SIZE: u64 = 100;
                let page_num = merchant_count / PAGE_SIZE;
                let mut page: Vec<u64> = env
                    .storage()
                    .instance()
                    .get(&DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(
                        entry.merchant.clone(),
                        page_num,
                    )))
                    .unwrap_or_else(|| Vec::new(&env));
                page.push_back(payment_id);
                env.storage().instance().set(
                    &DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(
                        entry.merchant.clone(),
                        page_num,
                    )),
                    &page,
                );
            }

            // Deduct fee
            let (net_amount, fee_amount) = PaymentContract::deduct_fee(
                &env,
                payment_id,
                entry.amount,
                entry.merchant.clone(),
                &entry.token,
                &entry.customer,
                entry.currency.clone(),
            );

            // Transfer from customer to contract
            let token_client = token::Client::new(&env, &entry.token);
            token_client.transfer_from(
                &contract_address,
                &entry.customer,
                &contract_address,
                &net_amount,
            );

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
                .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
                .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);
            let mut m_analytics: MerchantAnalytics = env
                .storage()
                .instance()
                .get(&DataKey::Merchant(MerchantDataKey::Analytics(
                    entry.merchant.clone(),
                )))
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
                &DataKey::Merchant(MerchantDataKey::Analytics(entry.merchant.clone())),
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
        }

        Ok(results)
    }

    /// Estimates the gas cost for executing a batch payment operation.
    ///
    /// Uses a rough heuristic based on entry count and unique token/merchant groups.
    ///
    /// # Arguments
    /// * `entries` - The batch payment entries to estimate gas for.
    ///
    /// # Returns
    /// Estimated gas units as a `u32`.
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

    /// Creates a conditional payment that is only completed when a specified condition is met.
    ///
    /// The payment is created in `Pending` status and stored with its condition.
    /// The condition can be time-based, oracle-based, or cross-contract state-based.
    ///
    /// # Arguments
    /// * `customer` - The customer initiating the payment (must authorize).
    /// * `merchant` - The merchant receiving the payment.
    /// * `amount` - Payment amount in base token units.
    /// * `token` - Address of the token contract.
    /// * `currency` - Currency enum for the payment.
    /// * `expiration_duration` - Seconds until the payment expires (0 for no expiration).
    /// * `metadata` - Optional descriptive metadata string.
    /// * `condition` - The `ConditionType` that must be satisfied to complete payment.
    ///
    /// # Returns
    /// The payment ID on success.
    ///
    /// # Errors
    /// Returns an error if the base payment creation fails.
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
            &DataKey::State(StateDataKey::ConditionalPayment(payment_id)),
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

    /// Evaluates the condition of a conditional payment and caches the result.
    ///
    /// For time-based conditions, compares the current ledger timestamp against
    /// the target. For cross-contract conditions, invokes the target contract
    /// to retrieve its state hash. Oracle conditions are not yet supported.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the conditional payment to evaluate.
    ///
    /// # Returns
    /// `true` if the condition is met, `false` otherwise.
    ///
    /// # Errors
    /// Returns an error if the conditional payment is not found or if
    /// oracle/cross-contract evaluation fails.
    pub fn evaluate_condition(env: Env, payment_id: u64) -> Result<bool, Error> {
        let mut conditional_payment: ConditionalPayment = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::ConditionalPayment(
                payment_id,
            )))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

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
                return Err(Error::Basic(BasicError::OracleCallFailed));
            }
            ConditionType::CrossContractState(target_contract, expected_state_hash) => {
                let fetched = env
                    .try_invoke_contract::<BytesN<32>, Error>(
                        target_contract,
                        &Symbol::new(&env, "get_state_hash"),
                        Vec::new(&env),
                    )
                    .map_err(|_| Error::Feature(FeatureError::ConditionEvaluationFailed))?
                    .map_err(|_| Error::Feature(FeatureError::ConditionEvaluationFailed))?;
                fetched == *expected_state_hash
            }
        };

        // Cache the result
        conditional_payment.condition_met = condition_met;
        conditional_payment.evaluated_at = Some(current_timestamp);

        env.storage().instance().set(
            &DataKey::State(StateDataKey::ConditionalPayment(payment_id)),
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

    /// Completes a conditional payment after evaluating its condition.
    ///
    /// Only an admin can call this function. The payment must be in `Pending` status,
    /// not expired, and the condition must evaluate to `true`.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `payment_id` - The ID of the conditional payment to complete.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the admin is unauthorized, payment not found,
    /// already processed, expired, or condition not met.
    pub fn complete_conditional_payment(
        env: Env,
        admin: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        // Check if conditional payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::State(StateDataKey::ConditionalPayment(
                payment_id,
            )))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        // Check if payment is expired
        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::Payment(PaymentError::Expired));
        }

        // Evaluate condition
        let condition_met = PaymentContract::evaluate_condition(env.clone(), payment_id)?;
        if !condition_met {
            return Err(Error::Feature(FeatureError::ConditionNotMet));
        }

        // Complete the payment
        PaymentContract::do_complete_payment(&env, payment_id)?;

        Ok(())
    }

    /// Executes a conditional payment if its condition is met.
    ///
    /// Unlike `complete_conditional_payment`, this can be called by anyone.
    /// It evaluates the condition and completes the payment atomically.
    /// If already completed, it returns `Ok(())` silently.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the conditional payment.
    ///
    /// # Returns
    /// `Ok(())` on success or if already completed.
    ///
    /// # Errors
    /// Returns an error if the payment is not found, not in pending status,
    /// or the condition is not met.
    pub fn execute_if_condition_met(env: Env, payment_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::State(StateDataKey::ConditionalPayment(
                payment_id,
            )))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }
        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.status == PaymentStatus::Completed {
            return Ok(());
        }
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        let condition_met = PaymentContract::evaluate_condition(env.clone(), payment_id)?;
        if !condition_met {
            return Err(Error::Feature(FeatureError::ConditionNotMet));
        }
        PaymentContract::do_complete_payment(&env, payment_id)
    }

    /// Retrieves the conditional payment record for a given payment ID.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the conditional payment.
    ///
    /// # Returns
    /// The `ConditionalPayment` record.
    ///
    /// # Errors
    /// Returns `PaymentError::NotFound` if no conditional payment exists.
    pub fn get_conditional_payment(env: Env, payment_id: u64) -> Result<ConditionalPayment, Error> {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::ConditionalPayment(
                payment_id,
            )))
            .ok_or(Error::Payment(PaymentError::NotFound))
    }

    // ── ANALYTICS FUNCTIONS ────────────────────────────────────────────────

    /// Returns the global payment analytics for the entire platform.
    ///
    /// # Returns
    /// A `PaymentAnalytics` struct with aggregate counts and volume totals.
    /// Returns zeroed defaults if no analytics data has been recorded.
    pub fn get_payment_analytics(env: Env) -> PaymentAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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

    /// Returns analytics data for a specific merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address to query analytics for.
    ///
    /// # Returns
    /// A `MerchantAnalytics` struct with per-merchant payment counts and volume.
    /// Returns zeroed defaults if no analytics data exists for this merchant.
    pub fn get_merchant_analytics(env: Env, merchant: Address) -> MerchantAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::Analytics(merchant)))
            .unwrap_or(MerchantAnalytics {
                total_payments: 0,
                total_volume: 0,
                total_completed: 0,
                total_cancelled: 0,
                total_refunded: 0,
                total_refunded_volume: 0,
            })
    }

    /// Returns the total transaction volume for a specific merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address.
    ///
    /// # Returns
    /// Total volume as `i128`.
    pub fn get_merchant_total_volume(env: Env, merchant: Address) -> i128 {
        PaymentContract::get_merchant_analytics(env, merchant).total_volume
    }

    /// Returns analytics data for a specific customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address to query analytics for.
    ///
    /// # Returns
    /// A `CustomerAnalytics` struct with payment counts, volume, and behavioral metrics.
    pub fn get_customer_analytics(env: Env, customer: Address) -> CustomerAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::Analytics(customer)))
            .unwrap_or(PaymentContract::default_customer_analytics())
    }

    /// Returns the top merchants by volume for a given customer, up to the specified limit.
    ///
    /// # Arguments
    /// * `customer` - The customer address.
    /// * `limit` - Maximum number of merchants to return.
    ///
    /// # Returns
    /// A vector of `(Address, i128)` tuples representing (merchant, total_volume),
    /// sorted by descending volume.
    pub fn get_customer_top_merchants(
        env: Env,
        customer: Address,
        limit: u32,
    ) -> Vec<(Address, i128)> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::MerchantCount(
                customer.clone(),
            )))
            .unwrap_or(0);

        let mut pairs: Vec<(Address, i128)> = Vec::new(&env);
        for i in 0..count {
            if let Some(merchant) =
                env.storage()
                    .instance()
                    .get::<DataKey, Address>(&DataKey::Customer(CustomerDataKey::MerchantList(
                        customer.clone(),
                        i,
                    )))
            {
                let vol: i128 = env
                    .storage()
                    .instance()
                    .get(&DataKey::Customer(CustomerDataKey::MerchantVolume(
                        customer.clone(),
                        merchant.clone(),
                    )))
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

    /// Returns the total spending volume for a customer in a specific month.
    ///
    /// # Arguments
    /// * `customer` - The customer address.
    /// * `month_timestamp` - Unix timestamp representing the start of the month.
    ///
    /// # Returns
    /// Volume as `i128` for the specified month. Returns 0 if no data exists.
    pub fn get_customer_monthly_volume(env: Env, customer: Address, month_timestamp: u64) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::MonthlyVolume(
                customer,
                month_timestamp,
            )))
            .unwrap_or(0)
    }

    /// Returns hourly analytics buckets for a merchant within a time range.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address.
    /// * `from` - Start timestamp (inclusive).
    /// * `to` - End timestamp (exclusive).
    ///
    /// # Returns
    /// A vector of `AnalyticsBucket` for each hour that has recorded data.
    ///
    /// # Errors
    /// Returns an error if `from >= to`.
    pub fn get_merchant_analytics_range(
        env: Env,
        merchant: Address,
        from: u64,
        to: u64,
    ) -> Result<Vec<AnalyticsBucket>, Error> {
        if from >= to {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        let mut out = Vec::new(&env);
        let mut bucket_start = PaymentContract::hour_bucket_start(from);
        while bucket_start < to {
            if let Some(bucket) =
                env.storage()
                    .instance()
                    .get::<DataKey, AnalyticsBucket>(&DataKey::Merchant(
                        MerchantDataKey::AnalyticsBucket(merchant.clone(), bucket_start),
                    ))
            {
                out.push_back(bucket);
            }
            bucket_start += 3600;
        }
        Ok(out)
    }

    /// Returns the platform-wide daily analytics bucket for a given day.
    ///
    /// # Arguments
    /// * `day_timestamp` - Any timestamp within the target day.
    ///
    /// # Returns
    /// An `AnalyticsBucket` with aggregated daily metrics (payments, volume, refunds, failures).
    pub fn get_platform_analytics_daily(env: Env, day_timestamp: u64) -> AnalyticsBucket {
        let day_start = PaymentContract::day_bucket_start(day_timestamp);
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PlatformAnalyticsDaily(
                day_start,
            )))
            .unwrap_or(AnalyticsBucket {
                bucket_start: day_start,
                bucket_end: day_start + 86400,
                total_payments: 0,
                total_volume: 0,
                total_refunds: 0,
                failed_count: 0,
            })
    }

    /// Returns the top merchants across the platform by total volume.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of merchants to return.
    ///
    /// # Returns
    /// A vector of `(Address, i128)` tuples representing (merchant, total_volume),
    /// sorted by descending volume.
    pub fn get_top_merchants_by_volume(env: Env, limit: u32) -> Vec<(Address, i128)> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::GlobalMerchantCount))
            .unwrap_or(0);
        let mut pairs: Vec<(Address, i128)> = Vec::new(&env);
        for i in 0..count {
            if let Some(merchant) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::Merchant(MerchantDataKey::GlobalList(i)))
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
            .get(&DataKey::Merchant(MerchantDataKey::AnalyticsBucket(
                merchant.clone(),
                bucket_start,
            )))
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
            &DataKey::Merchant(MerchantDataKey::AnalyticsBucket(merchant, bucket_start)),
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
            .get(&DataKey::Feature(FeatureKey::PlatformAnalyticsDaily(
                day_start,
            )))
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
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PlatformAnalyticsDaily(day_start)),
            &bucket,
        );
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

    /// Pauses the entire contract, blocking all guarded functions.
    ///
    /// Only callable by a multi-sig admin. Records the pause event
    /// in the pause history for audit purposes.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `reason` - Human-readable reason for pausing.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an authorized admin.
    pub fn pause_contract(env: Env, admin: Address, reason: String) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let now = env.ledger().timestamp();
        let pause_state = if let Some(mut state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::Config(ConfigKey::PauseStateKey))
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
            .set(&DataKey::Config(ConfigKey::PauseStateKey), &pause_state);
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::PauseHistoryCount))
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: String::from_str(&env, "global"),
            paused: true,
            changed_by: admin.clone(),
            changed_at: now,
            reason: reason.clone(),
        };
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryEntry(history_count)),
            &entry,
        );
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryCount),
            &(history_count + 1),
        );
        (ContractPausedEvent {
            paused_by: admin,
            reason,
            paused_at: now,
        })
        .publish(&env);
        Ok(())
    }

    /// Unpauses the entire contract, re-enabling all guarded functions.
    ///
    /// Only callable by a multi-sig admin. Records the unpause event
    /// in the pause history for audit purposes.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an authorized admin.
    pub fn unpause_contract(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if let Some(mut state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::Config(ConfigKey::PauseStateKey))
        {
            state.globally_paused = false;
            env.storage()
                .instance()
                .set(&DataKey::Config(ConfigKey::PauseStateKey), &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::PauseHistoryCount))
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: String::from_str(&env, "global"),
            paused: false,
            changed_by: admin.clone(),
            changed_at: now,
            reason: String::from_str(&env, ""),
        };
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryEntry(history_count)),
            &entry,
        );
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryCount),
            &(history_count + 1),
        );
        (ContractUnpausedEvent {
            unpaused_by: admin,
            unpaused_at: now,
        })
        .publish(&env);
        Ok(())
    }

    /// Pauses a specific function, preventing it from being called.
    ///
    /// Only callable by a multi-sig admin. The operation is idempotent;
    /// pausing an already-paused function is a no-op.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `function_name` - The name of the function to pause.
    /// * `reason` - Human-readable reason for the pause.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an authorized admin.
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
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let now = env.ledger().timestamp();
        let mut pause_state = if let Some(state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::Config(ConfigKey::PauseStateKey))
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
            .set(&DataKey::Config(ConfigKey::PauseStateKey), &pause_state);
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::PauseHistoryCount))
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: function_name.clone(),
            paused: true,
            changed_by: admin.clone(),
            changed_at: now,
            reason: reason.clone(),
        };
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryEntry(history_count)),
            &entry,
        );
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryCount),
            &(history_count + 1),
        );
        (FunctionPausedEvent {
            function_name,
            paused_by: admin,
            reason,
        })
        .publish(&env);
        Ok(())
    }

    /// Unpauses a specific function, allowing it to be called again.
    ///
    /// Only callable by a multi-sig admin. Records the unpause event
    /// in the pause history.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `function_name` - The name of the function to unpause.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an authorized admin.
    pub fn unpause_function(env: Env, admin: Address, function_name: String) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if let Some(mut state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::Config(ConfigKey::PauseStateKey))
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
                .set(&DataKey::Config(ConfigKey::PauseStateKey), &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::PauseHistoryCount))
            .unwrap_or(0);
        let entry = PauseHistory {
            index: history_count,
            function_name: function_name.clone(),
            paused: false,
            changed_by: admin.clone(),
            changed_at: now,
            reason: String::from_str(&env, ""),
        };
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryEntry(history_count)),
            &entry,
        );
        env.storage().instance().set(
            &DataKey::State(StateDataKey::PauseHistoryCount),
            &(history_count + 1),
        );
        (FunctionUnpausedEvent {
            function_name,
            unpaused_by: admin,
        })
        .publish(&env);
        Ok(())
    }

    /// Pause a merchant account, blocking all new subscriptions and recurring payments.
    pub fn pause_merchant(env: Env, admin: Address, merchant: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::MerchantPaused(merchant.clone())),
            &true,
        );
        Ok(())
    }

    /// Unpause a merchant account, allowing new subscriptions and recurring payments.
    pub fn unpause_merchant(env: Env, admin: Address, merchant: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .remove(&DataKey::Merchant(MerchantDataKey::MerchantPaused(
                merchant,
            )));
        Ok(())
    }

    /// Returns the current global pause state of the contract.
    ///
    /// # Returns
    /// A `PauseState` struct indicating whether the contract is globally paused,
    /// which individual functions are paused, and who/when it was paused.
    pub fn get_pause_state(env: Env) -> PauseState {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::PauseStateKey))
            .unwrap_or(PauseState {
                globally_paused: false,
                paused_functions: Vec::new(&env),
                paused_at: 0,
                paused_by: env.current_contract_address(),
                pause_reason: String::from_str(&env, ""),
            })
    }

    /// Checks whether a specific function is currently paused.
    ///
    /// Returns `true` if the contract is globally paused or if the given
    /// function name is in the paused functions list.
    ///
    /// # Arguments
    /// * `function_name` - The name of the function to check.
    ///
    /// # Returns
    /// `true` if the function is paused, `false` otherwise.
    pub fn is_function_paused(env: Env, function_name: String) -> bool {
        if let Some(state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::Config(ConfigKey::PauseStateKey))
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

    /// Sets an automatic escrow rule for a merchant.
    ///
    /// When enabled, qualifying payments to this merchant will trigger
    /// automatic escrow creation via the configured escrow contract.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `merchant` - The merchant this rule applies to.
    /// * `escrow_bps` - Percentage of payment to escrow, in basis points.
    /// * `min_amount` - Minimum payment amount to trigger escrow.
    /// * `token` - Token address that the rule applies to.
    /// * `escrow_contract` - Address of the escrow contract to use.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the contract is paused or the caller is not an admin.
    pub fn set_auto_escrow_rule(
        env: Env,
        admin: Address,
        merchant: Address,
        escrow_bps: u32,
        min_amount: i128,
        token: Address,
        escrow_contract: Address,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "set_auto_escrow_rule")?;
        admin.require_auth();

        // Verify caller is admin
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let rule = AutoEscrowRule {
            merchant: merchant.clone(),
            escrow_bps,
            min_amount,
            token,
            active: true,
            escrow_contract,
        };

        env.storage().instance().set(
            &DataKey::State(StateDataKey::AutoEscrowRule(merchant)),
            &rule,
        );

        Ok(())
    }

    /// Retrieves the auto-escrow rule for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address.
    ///
    /// # Returns
    /// `Some(AutoEscrowRule)` if a rule exists, `None` otherwise.
    pub fn get_auto_escrow_rule(env: Env, merchant: Address) -> Option<AutoEscrowRule> {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::AutoEscrowRule(merchant)))
    }

    /// Removes the auto-escrow rule for a merchant.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `merchant` - The merchant whose escrow rule should be removed.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the contract is paused, caller is not an admin,
    /// or no escrow rule exists for the merchant.
    pub fn remove_auto_escrow_rule(
        env: Env,
        admin: Address,
        merchant: Address,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "remove_auto_escrow_rule")?;
        admin.require_auth();

        // Verify caller is admin
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if rule exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::State(StateDataKey::AutoEscrowRule(
                merchant.clone(),
            )))
        {
            return Err(Error::Feature(FeatureError::AutoEscrowRuleNotFound));
        }

        env.storage()
            .instance()
            .remove(&DataKey::State(StateDataKey::AutoEscrowRule(merchant)));

        Ok(())
    }

    /// Triggers automatic escrow for a completed payment if an active rule exists.
    ///
    /// Calculates the escrow amount based on the merchant's escrow rule (basis points),
    /// creates an escrow via the escrow contract, and emits an event.
    /// This function can be called by anyone after a payment is completed.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment to create escrow for.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the payment is not found, escrow was already triggered,
    /// the rule is not found/inactive, or token/amount requirements are not met.
    /// Returns the amount that an active, matching, not-yet-triggered auto-escrow
    /// rule would carve out of `payment` for escrow, or 0 if no rule applies.
    fn get_auto_escrow_carveout(env: &Env, payment: &Payment) -> i128 {
        if env
            .storage()
            .instance()
            .has(&DataKey::State(StateDataKey::AutoEscrowTriggered(
                payment.id,
            )))
        {
            return 0;
        }

        let rule = match env
            .storage()
            .instance()
            .get::<DataKey, AutoEscrowRule>(&DataKey::State(StateDataKey::AutoEscrowRule(
                payment.merchant.clone(),
            ))) {
            Some(rule) => rule,
            None => return 0,
        };

        if !rule.active || payment.amount < rule.min_amount || payment.token != rule.token {
            return 0;
        }

        (payment.amount * (rule.escrow_bps as i128)) / 10000i128
    }

    pub fn trigger_auto_escrow(env: &Env, payment_id: u64) -> Result<(), Error> {
        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let payment = PaymentContract::get_payment(env, payment_id);

        // Check if auto-escrow already triggered for this payment
        if env
            .storage()
            .instance()
            .has(&DataKey::State(StateDataKey::AutoEscrowTriggered(
                payment_id,
            )))
        {
            return Err(Error::Feature(FeatureError::AutoEscrowAlreadyTriggered));
        }

        // Get auto-escrow rule for merchant
        let rule = env
            .storage()
            .instance()
            .get::<DataKey, AutoEscrowRule>(&DataKey::State(StateDataKey::AutoEscrowRule(
                payment.merchant.clone(),
            )))
            .ok_or(Error::Feature(FeatureError::AutoEscrowRuleNotFound))?;

        // Rule must be active
        if !rule.active {
            return Err(Error::Feature(FeatureError::AutoEscrowRuleNotFound));
        }

        // Check if payment amount meets minimum
        if payment.amount < rule.min_amount {
            // Silently skip if below minimum (no error)
            return Ok(());
        }

        // Check token matches the rule
        if payment.token != rule.token {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        // Calculate escrow amount based on bps (basis points)
        // escrow_bps is in basis points, so divide by 10000
        let escrow_amount = PaymentContract::get_auto_escrow_carveout(env, &payment);
        if escrow_amount == 0 {
            return Ok(());
        }

        // Create escrow using the escrow contract
        let escrow_client = EscrowContractClient::new(&env, &rule.escrow_contract);
        let release_timestamp = env.ledger().timestamp() + 86400 * 30; // 30 days
        let expiry_timestamp = release_timestamp + 86400 * 7;
        let escrow_id = escrow_client.create_escrow(
            &payment.customer,
            &payment.merchant,
            &escrow_amount,
            &payment.token,
            &release_timestamp,
            &0u64, // min_hold_period
            &expiry_timestamp,
            &true,
        );

        // Mark escrow as triggered for this payment
        env.storage().instance().set(
            &DataKey::State(StateDataKey::AutoEscrowTriggered(payment_id)),
            &escrow_id,
        );

        // Emit event
        (AutoEscrowTriggered {
            payment_id,
            merchant: payment.merchant,
            escrow_id,
            amount: payment.amount,
            escrow_amount,
        })
        .publish(&env);

        Ok(())
    }

    fn require_merchant_not_paused(env: &Env, merchant: &Address) -> Result<(), Error> {
        let merchant_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::MerchantPaused(
                merchant.clone(),
            )))
            .unwrap_or(false);
        if merchant_paused {
            return Err(Error::Subscription(SubscriptionError::MerchantPaused));
        }
        Ok(())
    }

    fn require_not_paused(env: &Env, function_name: &str) -> Result<(), Error> {
        if let Some(state) = env
            .storage()
            .instance()
            .get::<DataKey, PauseState>(&DataKey::Config(ConfigKey::PauseStateKey))
        {
            if state.globally_paused {
                return Err(Error::Basic(BasicError::ContractPaused));
            }
            let fn_str = String::from_str(env, function_name);
            for fn_name in state.paused_functions.iter() {
                if fn_name == fn_str {
                    return Err(Error::Basic(BasicError::FunctionPaused));
                }
            }
        }
        Ok(())
    }

    // ── LARGE PAYMENT MULTI-SIG FUNCTIONS ─────────────────────────────────────

    /// Sets the threshold amount above which payments require multi-sig approval.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `threshold` - The payment amount threshold. Setting to 0 disables multi-sig.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin.
    pub fn set_large_payment_threshold(
        env: Env,
        admin: Address,
        threshold: i128,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        env.storage().instance().set(
            &DataKey::Config(ConfigKey::LargePaymentThreshold),
            &threshold,
        );

        (LargePaymentThresholdUpdated {
            threshold,
            updated_by: admin,
        })
        .publish(&env);

        Ok(())
    }

    /// Returns the current large payment threshold.
    ///
    /// # Returns
    /// The threshold amount as `i128`. A value of 0 means multi-sig is disabled.
    pub fn get_large_payment_threshold(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::LargePaymentThreshold))
            .unwrap_or(0) // Default threshold of 0 disables multi-sig requirement
    }

    /// Proposes a large payment for multi-sig approval.
    ///
    /// The payment amount must exceed the configured large payment threshold.
    /// The merchant who owns the payment becomes the initial approver.
    /// The proposal expires after `proposal_ttl` seconds from the multi-sig config.
    ///
    /// # Arguments
    /// * `merchant` - The merchant who owns the payment.
    /// * `payment_id` - The ID of the payment to propose.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not the payment merchant, the payment
    /// amount does not exceed the threshold, or a proposal already exists.
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
            .get(&DataKey::Payment(PaymentKey::Data(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

        if payment.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if payment amount exceeds threshold
        let threshold: i128 = PaymentContract::get_large_payment_threshold(env.clone());
        if threshold == 0 || payment.amount <= threshold {
            return Err(Error::Proposal(ProposalError::RequiresMultiSig));
        }

        // Check if proposal already exists
        if env
            .storage()
            .instance()
            .get::<DataKey, LargePaymentProposal>(&DataKey::State(
                StateDataKey::LargePaymentProposal(payment_id),
            ))
            .is_some()
        {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

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

        env.storage().instance().set(
            &DataKey::State(StateDataKey::LargePaymentProposal(payment_id)),
            &proposal,
        );

        (LargePaymentProposed {
            payment_id,
            proposer: merchant,
            required_approvals: config.required_signatures,
            expires_at,
        })
        .publish(&env);

        Ok(())
    }

    /// Approves a large payment proposal as a multi-sig admin.
    ///
    /// Each admin can only approve once. The proposal expires after the configured TTL.
    /// Once the required number of approvals is reached, the payment can be executed.
    ///
    /// # Arguments
    /// * `approver` - Admin address (must be a multi-sig admin).
    /// * `payment_id` - The ID of the payment proposal to approve.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the approver is not an admin, the proposal is not found,
    /// already executed, expired, or the approver has already approved.
    pub fn approve_large_payment(
        env: Env,
        approver: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        approver.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&approver) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut proposal: LargePaymentProposal = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::LargePaymentProposal(
                payment_id,
            )))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

        if proposal.executed {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Proposal(ProposalError::ProposalExpired));
        }

        if proposal.approvals.contains(&approver) {
            return Err(Error::Basic(BasicError::AlreadyApproved));
        }

        proposal.approvals.push_back(approver.clone());

        env.storage().instance().set(
            &DataKey::State(StateDataKey::LargePaymentProposal(payment_id)),
            &proposal,
        );

        (LargePaymentApproved {
            payment_id,
            approver,
            approval_count: proposal.approvals.len() as u32,
        })
        .publish(&env);

        Ok(())
    }

    /// Executes an approved large payment proposal.
    ///
    /// Transfers funds from customer to merchant and marks both the payment
    /// and proposal as completed. Requires the proposal to have enough approvals
    /// and not be expired.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the large payment to execute.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the proposal is not found, expired, has insufficient
    /// approvals, or the payment is not in pending status.
    pub fn execute_large_payment(env: Env, payment_id: u64) -> Result<(), Error> {
        // Ensure multi-sig has been initialised.
        env.storage()
            .instance()
            .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        let mut proposal: LargePaymentProposal = env
            .storage()
            .instance()
            .get(&DataKey::State(StateDataKey::LargePaymentProposal(
                payment_id,
            )))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

        if proposal.executed {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Proposal(ProposalError::ProposalExpired));
        }

        if proposal.approvals.len() < proposal.required {
            return Err(Error::Proposal(ProposalError::InsufficientApprovals));
        }

        let payment: Payment = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Data(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        // Run the same completion path as a normal payment: platform fee
        // deduction (incl. risk surcharge), finality-delay hold, payment
        // forwarding, loyalty/rebate accrual, auto-escrow and analytics.
        PaymentContract::do_complete_payment(&env, payment_id)?;

        proposal.executed = true;
        env.storage().instance().set(
            &DataKey::State(StateDataKey::LargePaymentProposal(payment_id)),
            &proposal,
        );

        (LargePaymentExecuted { payment_id }).publish(&env);

        Ok(())
    }

    /// Returns the large payment proposal for a given payment ID.
    ///
    /// # Arguments
    /// * `payment_id` - The ID of the payment.
    ///
    /// # Returns
    /// The `LargePaymentProposal` record.
    ///
    /// # Panics
    /// Panics if no proposal exists for the given payment ID.
    pub fn get_large_payment_proposal(env: Env, payment_id: u64) -> LargePaymentProposal {
        env.storage()
            .instance()
            .get(&DataKey::State(StateDataKey::LargePaymentProposal(
                payment_id,
            )))
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
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let payment = PaymentContract::get_payment(&env, payment_id);

        // Verify caller is customer, merchant, or admin
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::Admin))
            .unwrap();
        if caller != payment.customer && caller != payment.merchant && caller != admin {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if metadata already exists
        let existing_metadata: Option<PaymentMetadata> = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Metadata(payment_id)));

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

            env.storage().instance().set(
                &DataKey::Payment(PaymentKey::Metadata(payment_id)),
                &updated_metadata,
            );

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

            env.storage().instance().set(
                &DataKey::Payment(PaymentKey::Metadata(payment_id)),
                &metadata,
            );

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
            .get(&DataKey::Payment(PaymentKey::Metadata(payment_id)))
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
            .get(&DataKey::Payment(PaymentKey::Metadata(payment_id)));

        match metadata {
            Some(meta) => meta.content_hash == plaintext_hash,
            None => false,
        }
    }

    /// Set structured payment memo with hash-verified reference field
    /// Memo hash is immutable after first set to ensure integrity
    pub fn set_payment_memo(
        env: Env,
        caller: Address,
        payment_id: u64,
        memo_ref: String,
        memo_hash: BytesN<32>,
    ) -> Result<(), Error> {
        caller.require_auth();

        // Check if payment exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Payment(PaymentKey::Data(payment_id)))
        {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        let payment = PaymentContract::get_payment(&env, payment_id);

        // Verify caller is customer, merchant, or admin
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::Admin))
            .unwrap();
        if caller != payment.customer && caller != payment.merchant && caller != admin {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if memo already exists
        let existing_memo: Option<PaymentMemo> = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Memo(payment_id)));

        let current_time = env.ledger().timestamp();

        if let Some(existing) = existing_memo {
            // Archive current version into sliding-window history (capped at MAX_MEMO_VERSIONS)
            let mut history: Vec<PaymentMemo> = env
                .storage()
                .instance()
                .get(&DataKey::Payment(PaymentKey::MemoVersion(payment_id)))
                .unwrap_or_else(|| Vec::new(&env));
            if history.len() >= MAX_MEMO_VERSIONS {
                // Drop oldest entry by rebuilding without index 0
                let mut trimmed = Vec::new(&env);
                for i in 1..history.len() {
                    trimmed.push_back(history.get(i).unwrap());
                }
                history = trimmed;
            }
            history.push_back(existing.clone());
            env.storage().instance().set(
                &DataKey::Payment(PaymentKey::MemoVersion(payment_id)),
                &history,
            );

            // Memo already set - emit update event with new version
            let new_version = existing.version + 1;

            // Create reference hash linking memo to payment for integrity verification
            let mut ref_bytes = Bytes::new(&env);
            ref_bytes.extend_from_array(&payment_id.to_be_bytes());
            ref_bytes.push_back(b':');
            ref_bytes.append(&existing.memo_hash.clone().into());
            let reference_hash = env.crypto().sha256(&ref_bytes);

            let updated_memo = PaymentMemo {
                payment_id,
                memo_ref: memo_ref.clone(),
                memo_hash: existing.memo_hash, // Keep original hash immutable
                reference_hash: reference_hash.into(),
                created_at: existing.created_at,
                updated_at: current_time,
                version: new_version,
                created_by: existing.created_by,
            };

            env.storage().instance().set(
                &DataKey::Payment(PaymentKey::Memo(payment_id)),
                &updated_memo,
            );

            PaymentMemoUpdated {
                payment_id,
                memo_ref,
                updated_by: caller,
                version: new_version,
            }
            .publish(&env);
        } else {
            // First time setting memo
            // Create reference hash linking memo to payment for integrity verification
            let mut ref_bytes = Bytes::new(&env);
            ref_bytes.extend_from_array(&payment_id.to_be_bytes());
            ref_bytes.push_back(b':');
            ref_bytes.append(&memo_hash.clone().into());
            let reference_hash = env.crypto().sha256(&ref_bytes);

            let memo = PaymentMemo {
                payment_id,
                memo_ref: memo_ref.clone(),
                memo_hash,
                reference_hash: reference_hash.into(),
                created_at: current_time,
                updated_at: current_time,
                version: 1,
                created_by: caller.clone(),
            };

            env.storage()
                .instance()
                .set(&DataKey::Payment(PaymentKey::Memo(payment_id)), &memo);

            PaymentMemoSet {
                payment_id,
                memo_ref,
                set_by: caller,
            }
            .publish(&env);
        }

        Ok(())
    }

    /// Get payment memo
    pub fn get_payment_memo(env: Env, payment_id: u64) -> Option<PaymentMemo> {
        env.storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Memo(payment_id)))
    }

    /// Verify memo integrity by comparing provided hash against stored memo hash
    /// Returns true if hashes match, false otherwise
    pub fn verify_memo_integrity(env: Env, payment_id: u64, plaintext_hash: BytesN<32>) -> bool {
        let memo: Option<PaymentMemo> = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Memo(payment_id)));

        match memo {
            Some(m) => m.memo_hash == plaintext_hash,
            None => false,
        }
    }

    /// Verify memo reference integrity using the reference hash
    /// Returns true if the reference hash matches the stored reference hash
    pub fn verify_memo_reference(
        env: Env,
        payment_id: u64,
        expected_reference_hash: BytesN<32>,
    ) -> bool {
        let memo: Option<PaymentMemo> = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Memo(payment_id)));

        match memo {
            Some(m) => m.reference_hash == expected_reference_hash,
            None => false,
        }
    }

    /// Calculates a risk score for a payment based on amount, customer history, and currency.
    ///
    /// Higher scores indicate higher risk. Used by `get_effective_fee_for_payment`
    /// to determine the dynamic fee surcharge.
    ///
    /// # Arguments
    /// * `customer` - The customer address.
    /// * `merchant` - The merchant address.
    /// * `amount` - The payment amount.
    /// * `currency` - The currency of the payment.
    ///
    /// # Returns
    /// Risk score as basis points (0 = no additional risk).
    pub fn calculate_risk_score(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        currency: Currency,
    ) -> u32 {
        let config: RiskFeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::RiskFeeConfig))
            .unwrap_or(RiskFeeConfig {
                base_fee_bps: 100,                 // 1%
                large_amount_threshold: 1000000,   // 10,000 USDC/USDT
                large_amount_surcharge_bps: 50,    // 0.5%
                new_customer_surcharge_bps: 100,   // 1%
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
            .get(&DataKey::Customer(CustomerDataKey::PaymentCount(customer)))
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

    /// Calculates the total effective fee for a payment including risk-based surcharges.
    ///
    /// Combines the base fee with the risk score from `calculate_risk_score`.
    /// The total fee is capped at 1000 basis points (10%).
    ///
    /// # Arguments
    /// * `customer` - The customer address.
    /// * `merchant` - The merchant address.
    /// * `amount` - The payment amount.
    /// * `currency` - The currency of the payment.
    ///
    /// # Returns
    /// Total effective fee in basis points.
    pub fn get_effective_fee_for_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        currency: Currency,
    ) -> u32 {
        let risk_config: RiskFeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::RiskFeeConfig))
            .unwrap_or(RiskFeeConfig {
                base_fee_bps: 100, // 1%
                large_amount_threshold: 1000000,
                large_amount_surcharge_bps: 50,
                new_customer_surcharge_bps: 100,
                high_risk_currency_surcharge: 200,
            });
        let fee_config: Option<FeeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig));
        let base_fee_bps = fee_config.map_or(risk_config.base_fee_bps, |cfg| cfg.fee_bps);

        let risk_surcharge = Self::calculate_risk_score(env, customer, merchant, amount, currency);
        let total_fee = base_fee_bps as u64 + risk_surcharge as u64;

        // Fee cap at 1000 bps (10%)
        if total_fee > 1000 {
            1000
        } else {
            total_fee as u32
        }
    }

    /// Updates the risk-based fee configuration for dynamic fee calculation.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be the contract admin).
    /// * `config` - The new `RiskFeeConfig` to apply.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not the admin, `base_fee_bps` exceeds 1000,
    /// or `large_amount_threshold` is not positive.
    pub fn set_risk_fee_config(
        env: Env,
        admin: Address,
        config: RiskFeeConfig,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin is the contract admin
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::Admin))
            .expect("Admin not set");
        if admin != stored_admin {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Validate configuration
        if config.base_fee_bps > 1000 {
            return Err(Error::Feature(FeatureError::InvalidFeeConfig));
        }
        if config.large_amount_threshold <= 0 {
            return Err(Error::Feature(FeatureError::InvalidFeeConfig));
        }

        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::RiskFeeConfig), &config);
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
            .has(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
        {
            return Err(Error::Subscription(SubscriptionError::NotFound));
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Data(
                subscription_id,
            )))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        sub.pause_data.proration_enabled = enabled;
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Data(subscription_id)),
            &sub,
        );

        Ok(())
    }

    // ── PAYMENT CHANNEL METHODS (#125) ──────────────────────────────────────

    /// Opens a payment channel by depositing tokens from the customer.
    ///
    /// Creates a unidirectional payment channel for micropayments. The customer
    /// deposits tokens upfront and signs off-chain updates. The merchant settles
    /// the channel on-chain with a signed settlement transaction.
    ///
    /// # Arguments
    /// * `customer` - The customer opening the channel (must authorize).
    /// * `merchant` - The merchant who will receive settlement.
    /// * `token` - The token address for the channel.
    /// * `amount` - Amount to deposit into the channel.
    /// * `expires_at` - Ledger timestamp when the channel expires (0 for no expiry).
    /// * `customer_pk` - Customer's Ed25519 public key for signature verification.
    ///
    /// # Returns
    /// The channel ID on success.
    ///
    /// # Errors
    /// Returns an error if the amount is non-positive, the merchant is the zero address,
    /// or the token transfer fails.
    pub fn open_channel(
        env: Env,
        customer: Address,
        merchant: Address,
        token: Address,
        amount: i128,
        expires_at: u64,
        customer_pk: BytesN<32>,
    ) -> Result<u64, Error> {
        Self::require_not_paused(&env, "open_channel")?;
        Self::require_merchant_not_paused(&env, &merchant)?;
        customer.require_auth();
        if amount <= 0 {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        // Validate counterparty address is not zero address
        if Self::is_zero_address(&env, &merchant) {
            return Err(Error::Feature(FeatureError::InvalidCounterparty));
        }

        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&customer, &contract_address, &amount);

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentChannelCounter))
            .unwrap_or(0);
        let channel_id = counter + 1;

        let channel = PaymentChannel {
            channel_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            token,
            deposited: amount,
            settled: 0,
            settled_nonce: 0,
            open: true,
            expires_at,
            customer_pk,
        };

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PaymentChannel(channel_id)),
            &channel,
        );
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PaymentChannelCounter),
            &channel_id,
        );

        (ChannelOpened {
            channel_id,
            customer,
            merchant,
            amount,
        })
        .publish(&env);

        Ok(channel_id)
    }

    /// Tops up an existing open payment channel with additional deposit.
    ///
    /// Allows a customer to add funds to a channel that has been drawn down,
    /// avoiding the need to close and reopen a new channel for continued
    /// micropayments.
    ///
    /// # Arguments
    /// * `customer` - The channel's customer (must authorize).
    /// * `channel_id` - The ID of the channel to top up.
    /// * `amount` - Amount to add to the channel's deposit.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the amount is non-positive, the channel is not found,
    /// the caller is not the channel's customer, the channel is closed, or the
    /// channel has expired.
    pub fn top_up_channel(
        env: Env,
        customer: Address,
        channel_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "top_up_channel")?;
        customer.require_auth();
        if amount <= 0 {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        let mut channel: PaymentChannel = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentChannel(channel_id)))
            .ok_or(Error::Feature(FeatureError::ChannelNotFound))?;

        if channel.customer != customer {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if !channel.open {
            return Err(Error::Feature(FeatureError::ChannelClosed));
        }

        if channel.expires_at > 0 && env.ledger().timestamp() > channel.expires_at {
            return Err(Error::Feature(FeatureError::ChannelExpired));
        }

        let token_client = token::Client::new(&env, &channel.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&customer, &contract_address, &amount);

        channel.deposited += amount;

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PaymentChannel(channel_id)),
            &channel,
        );

        (ChannelToppedUp {
            channel_id,
            amount,
            new_deposited: channel.deposited,
        })
        .publish(&env);

        Ok(())
    }

    /// Settles a payment channel with a signed off-chain state update.
    ///
    /// Verifies the customer's signature over (channel_id, merchant_amount, nonce),
    /// transfers funds to the merchant and refunds the remainder to the customer,
    /// then closes the channel.
    ///
    /// # Arguments
    /// * `channel_id` - The ID of the channel to settle.
    /// * `merchant_amount` - Amount to send to the merchant.
    /// * `nonce` - Strictly increasing nonce to prevent replay attacks.
    /// * `signature` - Ed25519 signature from the customer.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the channel is not found, closed, expired, nonce is stale,
    /// amount exceeds deposit, or signature verification fails.
    pub fn settle_channel(
        env: Env,
        channel_id: u64,
        merchant_amount: i128,
        nonce: u64,
        signature: BytesN<64>,
    ) -> Result<(), Error> {
        Self::require_not_paused(&env, "settle_channel")?;
        
        let mut channel: PaymentChannel = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentChannel(channel_id)))
            .ok_or(Error::Feature(FeatureError::ChannelNotFound))?;

        Self::require_merchant_not_paused(&env, &channel.merchant)?;

        if !channel.open {
            return Err(Error::Feature(FeatureError::ChannelClosed));
        }

        if channel.expires_at > 0 && env.ledger().timestamp() > channel.expires_at {
            return Err(Error::Feature(FeatureError::ChannelExpired));
        }

        // Enforce strictly increasing nonce to prevent stale off-chain state replay
        if nonce <= channel.settled_nonce {
            return Err(Error::Feature(FeatureError::InvalidNonce));
        }

        if merchant_amount < 0 || merchant_amount > channel.deposited {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        // Verify signature over (channel_id, merchant_amount, nonce)
        let mut msg = Bytes::new(&env);
        msg.append(&channel_id.to_xdr(&env));
        msg.append(&merchant_amount.to_xdr(&env));
        msg.append(&nonce.to_xdr(&env));

        env.crypto()
            .ed25519_verify(&channel.customer_pk, &msg, &signature);

        let customer_refund = channel.deposited - merchant_amount;

        // Issue #385: assert the claimed settlement conserves the original deposit —
        // the merchant's claim plus the customer's derived refund must sum exactly
        // to what was deposited, never more.
        if merchant_amount
            .checked_add(customer_refund)
            .ok_or(Error::Feature(FeatureError::BalanceSumMismatch))?
            != channel.deposited
        {
            return Err(Error::Feature(FeatureError::BalanceSumMismatch));
        }
        let token_client = token::Client::new(&env, &channel.token);
        let contract_address = env.current_contract_address();

        if merchant_amount > 0 {
            token_client.transfer(&contract_address, &channel.merchant, &merchant_amount);
        }
        if customer_refund > 0 {
            token_client.transfer(&contract_address, &channel.customer, &customer_refund);
        }

        channel.settled = merchant_amount;
        channel.settled_nonce = nonce;
        channel.open = false;

        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PaymentChannel(channel_id)),
            &channel,
        );

        (ChannelSettled {
            channel_id,
            merchant_amount,
            customer_refund,
        })
        .publish(&env);

        Ok(())
    }

    /// Closes an expired payment channel and refunds the full deposit to the customer.
    ///
    /// Only the customer or merchant can call this. The channel must be open
    /// and past its expiration timestamp.
    ///
    /// # Arguments
    /// * `caller` - The address initiating the close (must be customer or merchant).
    /// * `channel_id` - The ID of the expired channel.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not a channel participant, the channel
    /// is not found, already closed, or not yet expired.
    pub fn close_channel_expired(env: Env, caller: Address, channel_id: u64) -> Result<(), Error> {
        Self::require_not_paused(&env, "close_channel_expired")?;
        caller.require_auth();

        let mut channel: PaymentChannel = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentChannel(channel_id)))
            .ok_or(Error::Feature(FeatureError::ChannelNotFound))?;

        if caller != channel.customer && caller != channel.merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if !channel.open {
            return Err(Error::Feature(FeatureError::ChannelClosed));
        }

        if channel.expires_at == 0 || env.ledger().timestamp() <= channel.expires_at {
            return Err(Error::Feature(FeatureError::ChannelNotExpired));
        }

        let refund_amount = channel.deposited;
        let token_client = token::Client::new(&env, &channel.token);
        let contract_address = env.current_contract_address();

        if refund_amount > 0 {
            token_client.transfer(&contract_address, &channel.customer, &refund_amount);
        }

        channel.open = false;
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::PaymentChannel(channel_id)),
            &channel,
        );

        (ChannelExpiredClosed {
            channel_id,
            refunded_to: channel.customer.clone(),
        })
        .publish(&env);

        Ok(())
    }

    /// Retrieves a payment channel by its ID.
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID.
    ///
    /// # Returns
    /// The `PaymentChannel` record.
    ///
    /// # Errors
    /// Returns an error if the channel is not found.
    pub fn get_channel(env: Env, channel_id: u64) -> Result<PaymentChannel, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentChannel(channel_id)))
            .ok_or(Error::Feature(FeatureError::ChannelNotFound))
    }

    fn extract_public_key(env: &Env, address: &Address) -> BytesN<32> {
        let xdr = address.to_xdr(env);
        // ScVal XDR layout: [0,0,0,18](ScvAddress) [0,0,0,0](Account) [0,0,0,0](Ed25519) [32 bytes PK]
        // PK starts at offset 12
        let mut pk = [0u8; 32];
        for i in 0..32 {
            pk[i] = xdr.get(12 + (i as u32)).unwrap();
        }
        BytesN::from_array(env, &pk)
    }

    fn is_zero_address(env: &Env, address: &Address) -> bool {
        let xdr = address.to_xdr(env);
        // Check if it's a Contract address with all-zero bytes
        // Contract address type is 0x00 in the XDR
        if xdr.get(0).unwrap() == 0 && xdr.get(4).unwrap() == 0 {
            // Check if all 32 bytes are zero
            for i in 0..32 {
                if xdr.get(12 + (i as u32)).unwrap() != 0 {
                    return false;
                }
            }
            return true;
        }
        false
    }

    /// Creates a split payment that distributes funds to multiple recipients.
    ///
    /// Each recipient specifies their share in basis points; total shares must equal 10000.
    /// Up to 10 recipients are allowed. The payment is created in `Pending` status.
    ///
    /// # Arguments
    /// * `customer` - The customer initiating the payment (must authorize).
    /// * `merchant` - The primary merchant for the payment.
    /// * `amount` - Total payment amount in base token units.
    /// * `token` - Address of the token contract.
    /// * `recipients` - Vector of `SplitRecipient` with addresses and share_bps.
    ///
    /// # Returns
    /// The payment ID on success.
    ///
    /// # Errors
    /// Returns an error if there are too many recipients, shares don't sum to 10000,
    /// the customer is a recipient, a recipient share is below the minimum,
    /// or the customer's spend limit is exceeded.
    pub fn create_split_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        recipients: Vec<SplitRecipient>,
    ) -> Result<u64, Error> {
        Self::require_merchant_not_paused(&env, &merchant)?;
        customer.require_auth();

        if recipients.len() > 10 {
            return Err(Error::Feature(FeatureError::TooManyRecipients));
        }

        let total_bps: u32 = recipients.iter().map(|r| r.share_bps).sum();
        if total_bps != 10000 {
            return Err(Error::Feature(FeatureError::InvalidSplitShares));
        }

        for r in recipients.iter() {
            if r.address == customer {
                return Err(Error::Feature(FeatureError::SenderIsRecipient));
            }
        }

        let min_split: Option<i128> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MinSplitAmount));
        if let Some(min_amount) = min_split {
            for r in recipients.iter() {
                let share = (amount * r.share_bps as i128) / 10000;
                if share < min_amount {
                    return Err(Error::Feature(FeatureError::BelowMinSplitAmount));
                }
            }
        }

        // Check customer spend limit (#282)
        PaymentContract::check_and_update_spend_limit(&env, &customer, amount)?;

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Counter))
            .unwrap_or(0);
        let payment_id = counter + 1;

        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&customer, &contract_address, &amount);

        let current_timestamp = env.ledger().timestamp();
        let payment = Payment {
            id: payment_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token,
            currency: Currency::USDC,
            status: PaymentStatus::Pending,
            created_at: current_timestamp,
            expires_at: 0,
            metadata: String::from_str(&env, ""),
            notes: String::from_str(&env, ""),
            refunded_amount: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Data(payment_id)), &payment);
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::Counter), &payment_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::PaymentCount(
                customer.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Payments(customer.clone(), customer_count)),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::PaymentCount(customer.clone())),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PaymentCount(
                merchant.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::Payments(merchant.clone(), merchant_count)),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::Merchant(MerchantDataKey::PaymentCount(merchant.clone())),
            &(merchant_count + 1),
        );

        // Paged merchant payment index (100 entries per page)
        {
            const PAGE_SIZE: u64 = 100;
            let page_num = merchant_count / PAGE_SIZE;
            let mut page: Vec<u64> = env
                .storage()
                .instance()
                .get(&DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(
                    merchant.clone(),
                    page_num,
                )))
                .unwrap_or_else(|| Vec::new(&env));
            page.push_back(payment_id);
            env.storage().instance().set(
                &DataKey::Merchant(MerchantDataKey::MerchantPaymentsPage(merchant.clone(), page_num)),
                &page,
            );
        }

        // Update global analytics
        let mut analytics: PaymentAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::PaymentAnalytics))
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
                .get(&DataKey::Config(ConfigKey::GlobalMerchantCount))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Merchant(MerchantDataKey::GlobalList(global_count)),
                &merchant,
            );
            env.storage().instance().set(
                &DataKey::Config(ConfigKey::GlobalMerchantCount),
                &(global_count + 1),
            );
        }
        env.storage()
            .instance()
            .set(&DataKey::Feature(FeatureKey::PaymentAnalytics), &analytics);

        // Update merchant analytics (per-merchant total volume)
        let mut m_analytics: MerchantAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::Analytics(
                merchant.clone(),
            )))
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
            &DataKey::Merchant(MerchantDataKey::Analytics(merchant.clone())),
            &m_analytics,
        );

        // Update customer analytics
        let mut c_analytics: CustomerAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::Analytics(
                customer.clone(),
            )))
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

        // Track per-merchant volume
        let prev_merchant_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Customer(CustomerDataKey::MerchantVolume(
                customer.clone(),
                merchant.clone(),
            )))
            .unwrap_or(0);
        let new_merchant_vol = prev_merchant_vol + amount;
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::MerchantVolume(
                customer.clone(),
                merchant.clone(),
            )),
            &new_merchant_vol,
        );
        if prev_merchant_vol == 0 {
            let m_count: u64 = env
                .storage()
                .instance()
                .get(&DataKey::Customer(CustomerDataKey::MerchantCount(
                    customer.clone(),
                )))
                .unwrap_or(0);
            env.storage().instance().set(
                &DataKey::Customer(CustomerDataKey::MerchantList(customer.clone(), m_count)),
                &merchant,
            );
            env.storage().instance().set(
                &DataKey::Customer(CustomerDataKey::MerchantCount(customer.clone())),
                &(m_count + 1),
            );
        }
        if new_merchant_vol > c_analytics.top_merchant_volume {
            c_analytics.top_merchant_volume = new_merchant_vol;
            c_analytics.top_merchant = Some(merchant.clone());
        }
        env.storage().instance().set(
            &DataKey::Customer(CustomerDataKey::Analytics(customer.clone())),
            &c_analytics,
        );

        let config = PaymentSplitConfig {
            payment_id,
            recipients,
            executed: false,
        };
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::SplitConfig(payment_id)),
            &config,
        );

        (PaymentCreated {
            payment_id,
            customer,
            merchant,
            amount,
        })
        .publish(&env);

        Ok(payment_id)
    }

    /// Executes the settlement of a split payment, distributing funds to recipients.
    ///
    /// Only callable by an admin. Each recipient receives their pro-rata share
    /// based on the `share_bps` values. The split config is marked as executed.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must authorize).
    /// * `payment_id` - The ID of the split payment to settle.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the split config is not found, already executed,
    /// or the payment is not found.
    pub fn execute_split_settlement(
        env: Env,
        admin: Address,
        payment_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let mut config: PaymentSplitConfig = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::SplitConfig(payment_id)))
            .ok_or(Error::Feature(FeatureError::SplitConfigNotFound))?;

        if config.executed {
            return Err(Error::Feature(FeatureError::SplitAlreadyExecuted));
        }

        let mut payment: Payment = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::Data(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

        // Prevent double-spending: ensure payment is still Pending
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }

        let token_client = token::Client::new(&env, &payment.token);
        let contract_address = env.current_contract_address();

        for recipient in config.recipients.iter() {
            let share = (payment.amount * recipient.share_bps as i128) / 10000;
            token_client.transfer(&contract_address, &recipient.address, &share);
        }

        config.executed = true;
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::SplitConfig(payment_id)),
            &config,
        );

        // Mark payment as Completed to prevent subsequent complete_payment calls
        payment.status = PaymentStatus::Completed;
        env.storage().instance().set(
            &DataKey::Payment(PaymentKey::Data(payment_id)),
            &payment,
        );

        Ok(())
    }

    /// Returns the split configuration for a payment.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID.
    ///
    /// # Returns
    /// `Some(PaymentSplitConfig)` if a split was configured, `None` otherwise.
    pub fn get_split_config(env: Env, payment_id: u64) -> Option<PaymentSplitConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::SplitConfig(payment_id)))
    }

    /// Sets the minimum amount per split recipient for split payments.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `min_amount` - Minimum amount each recipient must receive.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin.
    pub fn set_min_split_amount(env: Env, admin: Address, min_amount: i128) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::MinSplitAmount), &min_amount);
        Ok(())
    }

    /// Returns the minimum amount configured for split payments.
    ///
    /// # Returns
    /// `Some(i128)` if configured, `None` if no minimum is set.
    pub fn get_min_split_amount(env: Env) -> Option<i128> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MinSplitAmount))
    }

    // ── FEE SWEEP (#216) ─────────────────────────────────────────────────────

    /// Sets the recipient address for platform fee sweeps.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `recipient` - The address that will receive swept fees.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin.
    pub fn set_sweep_recipient(env: Env, admin: Address, recipient: Address) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Feature(FeatureKey::SweepRecipient), &recipient);
        Ok(())
    }

    /// Sweeps all accumulated platform fees to the configured recipient.
    ///
    /// Transfers the full accumulated fee balance, resets the counter,
    /// and records the sweep in history.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    ///
    /// # Returns
    /// The amount swept as `i128`.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin, the sweep recipient
    /// is not set, the fee config is missing, or there are no fees to sweep.
    pub fn sweep_platform_fees(env: Env, admin: Address) -> Result<i128, Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let recipient: Address = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::SweepRecipient))
            .ok_or(Error::Feature(FeatureError::SweepRecipientNotSet))?;
        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::AccumulatedFees))
            .unwrap_or(0);
        if accumulated <= 0 {
            return Err(Error::Feature(FeatureError::NothingToSweep));
        }
        let fee_config: FeeConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig))
            .ok_or(Error::Feature(FeatureError::FeeConfigNotFound))?;
        let token_client = token::Client::new(&env, &fee_config.fee_token);
        token_client.transfer(&env.current_contract_address(), &recipient, &accumulated);
        env.storage()
            .instance()
            .set(&DataKey::Payment(PaymentKey::AccumulatedFees), &0i128);
        let sweep_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::SweepCounter))
            .unwrap_or(0)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::Feature(FeatureKey::SweepCounter), &sweep_id);
        let record = FeeSweepRecord {
            sweep_id,
            amount: accumulated,
            token: fee_config.fee_token,
            recipient,
            swept_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::SweepHistory(sweep_id)),
            &record,
        );
        Ok(accumulated)
    }

    /// Returns the most recent fee sweep records, up to the specified limit.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of sweep records to return.
    ///
    /// # Returns
    /// A vector of `FeeSweepRecord`, ordered from oldest to newest within the limit.
    pub fn get_sweep_history(env: Env, limit: u32) -> Vec<FeeSweepRecord> {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::SweepCounter))
            .unwrap_or(0);
        let mut result = Vec::new(&env);
        let start = if total > limit as u64 {
            total - limit as u64 + 1
        } else {
            1
        };
        for i in start..=total {
            if let Some(record) = env
                .storage()
                .instance()
                .get(&DataKey::Feature(FeatureKey::SweepHistory(i)))
            {
                result.push_back(record);
            }
        }
        result
    }

    /// Returns the total accumulated platform fees available for sweeping.
    ///
    /// # Returns
    /// Accumulated fees as `i128`.
    pub fn get_sweepable_balance(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::AccumulatedFees))
            .unwrap_or(0)
    }

    // ── CUSTOMER SPEND LIMITS (#217) ─────────────────────────────────────────

    /// Sets a spending limit for a customer over a rolling time period.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `customer` - The customer to set the limit for.
    /// * `limit` - Maximum spending amount within the period.
    /// * `period_seconds` - Duration of the rolling period in seconds.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin.
    pub fn set_customer_spend_limit(
        env: Env,
        admin: Address,
        customer: Address,
        limit: i128,
        period_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let now = env.ledger().timestamp();
        let spend_limit = CustomerSpendLimit {
            customer: customer.clone(),
            limit_amount: limit,
            period_seconds,
            used: 0,
            period_start: now,
        };
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::CustomerSpendLimit(customer)),
            &spend_limit,
        );
        Ok(())
    }

    /// Returns the current spend limit configuration for a customer.
    ///
    /// # Arguments
    /// * `customer` - The customer address.
    ///
    /// # Returns
    /// `Some(CustomerSpendLimit)` if a limit is configured, `None` otherwise.
    pub fn get_spend_limit(env: Env, customer: Address) -> Option<CustomerSpendLimit> {
        env.storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::CustomerSpendLimit(customer)))
    }

    /// Removes the spending limit for a customer.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `customer` - The customer whose limit should be removed.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin.
    pub fn remove_customer_spend_limit(
        env: Env,
        admin: Address,
        customer: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .remove(&DataKey::Feature(FeatureKey::CustomerSpendLimit(customer)));
        Ok(())
    }

    /// Checks whether a customer can spend the given amount without exceeding their limit.
    ///
    /// Returns `true` if no limit is configured or if the amount fits within the
    /// remaining allowance for the current period.
    ///
    /// # Arguments
    /// * `customer` - The customer address.
    /// * `amount` - The amount to check.
    ///
    /// # Returns
    /// `true` if the spend is allowed, `false` if it would exceed the limit.
    pub fn check_spend_allowance(env: Env, customer: Address, amount: i128) -> bool {
        let limit: Option<CustomerSpendLimit> = env
            .storage()
            .instance()
            .get(&DataKey::Feature(FeatureKey::CustomerSpendLimit(customer)));
        match limit {
            None => true,
            Some(l) => {
                let now = env.ledger().timestamp();
                let used = if now > l.period_start + l.period_seconds {
                    0
                } else {
                    l.used
                };
                used + amount <= l.limit_amount
            }
        }
    }

    fn check_and_update_spend_limit(
        env: &Env,
        customer: &Address,
        amount: i128,
    ) -> Result<(), Error> {
        let limit: Option<CustomerSpendLimit> =
            env.storage()
                .instance()
                .get(&DataKey::Feature(FeatureKey::CustomerSpendLimit(
                    customer.clone(),
                )));
        let mut limit = match limit {
            None => return Ok(()),
            Some(l) => l,
        };
        let now = env.ledger().timestamp();
        if now > limit.period_start + limit.period_seconds {
            limit.used = 0;
            limit.period_start = now;
        }
        if limit.used + amount > limit.limit_amount {
            return Err(Error::Feature(FeatureError::SpendLimitExceeded));
        }
        limit.used += amount;
        env.storage().instance().set(
            &DataKey::Feature(FeatureKey::CustomerSpendLimit(customer.clone())),
            &limit,
        );
        Ok(())
    }

    // ── SUBSCRIPTION GROUPS (#218) ────────────────────────────────────────────

    /// Creates a new subscription group with a discount.
    ///
    /// # Arguments
    /// * `owner` - The owner of the group (must authorize).
    /// * `discount_bps` - Discount applied to group subscriptions, in basis points.
    ///
    /// # Returns
    /// The group ID on success.
    pub fn create_subscription_group(
        env: Env,
        owner: Address,
        discount_bps: u32,
    ) -> Result<u64, Error> {
        owner.require_auth();
        let group_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::GroupCounter))
            .unwrap_or(0)
            + 1;
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::GroupCounter),
            &group_id,
        );
        let group = SubscriptionGroup {
            group_id,
            owner,
            subscription_ids: Vec::new(&env),
            discount_bps,
            active: true,
        };
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Group(group_id)),
            &group,
        );
        Ok(group_id)
    }

    /// Adds a subscription to a group.
    ///
    /// Maximum 20 subscriptions per group. A subscription cannot belong
    /// to multiple groups simultaneously.
    ///
    /// # Arguments
    /// * `owner` - The group owner (must authorize).
    /// * `group_id` - The group to add the subscription to.
    /// * `subscription_id` - The subscription to add.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the group is not found, caller is not the owner,
    /// the group is full, or the subscription is already in a group.
    pub fn add_to_group(
        env: Env,
        owner: Address,
        group_id: u64,
        subscription_id: u64,
    ) -> Result<(), Error> {
        owner.require_auth();
        let mut group: SubscriptionGroup = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Group(group_id)))
            .ok_or(Error::Subscription(SubscriptionError::GroupNotFound))?;
        if group.owner != owner {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if group.subscription_ids.len() >= 20 {
            return Err(Error::Subscription(
                SubscriptionError::GroupSizeLimitExceeded,
            ));
        }
        if env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::Subscription(SubscriptionKey::GroupMembership(
                subscription_id,
            )))
            .is_some()
        {
            return Err(Error::Subscription(SubscriptionError::AlreadyInGroup));
        }
        group.subscription_ids.push_back(subscription_id);
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Group(group_id)),
            &group,
        );
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::GroupMembership(subscription_id)),
            &group_id,
        );
        Ok(())
    }

    /// Removes a subscription from a group.
    ///
    /// # Arguments
    /// * `owner` - The group owner (must authorize).
    /// * `group_id` - The group to remove the subscription from.
    /// * `subscription_id` - The subscription to remove.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the group is not found or the caller is not the owner.
    pub fn remove_from_group(
        env: Env,
        owner: Address,
        group_id: u64,
        subscription_id: u64,
    ) -> Result<(), Error> {
        owner.require_auth();
        let mut group: SubscriptionGroup = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Group(group_id)))
            .ok_or(Error::Subscription(SubscriptionError::GroupNotFound))?;
        if group.owner != owner {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let mut new_ids = Vec::new(&env);
        for id in group.subscription_ids.iter() {
            if id != subscription_id {
                new_ids.push_back(id);
            }
        }
        group.subscription_ids = new_ids;
        env.storage().instance().set(
            &DataKey::Subscription(SubscriptionKey::Group(group_id)),
            &group,
        );
        env.storage()
            .instance()
            .remove(&DataKey::Subscription(SubscriptionKey::GroupMembership(
                subscription_id,
            )));
        Ok(())
    }

    /// Returns the subscription group record for a given group ID.
    ///
    /// # Arguments
    /// * `group_id` - The group ID.
    ///
    /// # Returns
    /// `Some(SubscriptionGroup)` if the group exists, `None` otherwise.
    pub fn get_subscription_group(env: Env, group_id: u64) -> Option<SubscriptionGroup> {
        env.storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Group(group_id)))
    }

    /// Returns the earliest next billing timestamp across all subscriptions in a group.
    ///
    /// # Arguments
    /// * `group_id` - The group ID.
    ///
    /// # Returns
    /// The earliest `next_payment_at` timestamp, or 0 if the group has no subscriptions.
    pub fn get_group_next_billing(env: Env, group_id: u64) -> u64 {
        let group: Option<SubscriptionGroup> = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(SubscriptionKey::Group(group_id)));
        let group = match group {
            None => return 0,
            Some(g) => g,
        };
        let mut earliest: u64 = u64::MAX;
        for sub_id in group.subscription_ids.iter() {
            if let Some(sub) = env
                .storage()
                .instance()
                .get::<DataKey, Subscription>(&DataKey::Subscription(SubscriptionKey::Data(sub_id)))
            {
                if sub.next_payment_at < earliest {
                    earliest = sub.next_payment_at;
                }
            }
        }
        if earliest == u64::MAX {
            0
        } else {
            earliest
        }
    }

    // ── FINALITY DELAY (#219) ─────────────────────────────────────────────────

    /// Configures the finality delay settings for pending settlements.
    ///
    /// # Arguments
    /// * `admin` - Admin address (must be a multi-sig admin).
    /// * `config` - The `FinalityConfig` to apply.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not an admin.
    pub fn configure_finality_delay(
        env: Env,
        admin: Address,
        config: FinalityConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let ms_config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::MultiSigConfig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !ms_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::FinalityConfig), &config);
        Ok(())
    }

    /// Returns the current finality delay configuration.
    ///
    /// # Returns
    /// `Some(FinalityConfig)` if configured, `None` otherwise.
    pub fn get_finality_config(env: Env) -> Option<FinalityConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FinalityConfig))
    }

    /// Finalizes a pending settlement by transferring funds to the merchant.
    ///
    /// The settlement's release time must have passed. After finalization,
    /// the settlement is marked as finalized and cannot be finalized again.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID of the pending settlement.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the settlement is already finalized, not found,
    /// or the release time has not yet passed.
    pub fn finalize_pending_settlement(env: Env, payment_id: u64) -> Result<(), Error> {
        if env
            .storage()
            .instance()
            .has(&DataKey::State(StateDataKey::SettlementFinalized(
                payment_id,
            )))
        {
            return Err(Error::Feature(FeatureError::SettlementAlreadyFinalized));
        }
        let settlement: PendingSettlement = env
            .storage()
            .instance()
            .get(&DataKey::Payment(PaymentKey::PendingSettlement(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;
        let now = env.ledger().timestamp();
        if now < settlement.release_at {
            return Err(Error::Feature(FeatureError::SettlementNotReady));
        }
        let token_client = token::Client::new(&env, &settlement.token);
        token_client.transfer(
            &env.current_contract_address(),
            &settlement.merchant,
            &settlement.amount,
        );
        env.storage().instance().set(
            &DataKey::State(StateDataKey::SettlementFinalized(payment_id)),
            &true,
        );
        Ok(())
    }

    /// Returns all pending (non-finalized) settlements for a merchant.
    ///
    /// # Arguments
    /// * `merchant` - The merchant address.
    ///
    /// # Returns
    /// A vector of `PendingSettlement` records that have not yet been finalized.
    pub fn get_pending_settlements(env: Env, merchant: Address) -> Vec<PendingSettlement> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Merchant(MerchantDataKey::PendingSettlementCount(
                merchant.clone(),
            )))
            .unwrap_or(0);
        let mut result = Vec::new(&env);
        for i in 0..count {
            if let Some(payment_id) =
                env.storage()
                    .instance()
                    .get::<DataKey, u64>(&DataKey::Merchant(
                        MerchantDataKey::PendingSettlementIndex(merchant.clone(), i),
                    ))
            {
                if !env.storage().instance().has(&DataKey::State(
                    StateDataKey::SettlementFinalized(payment_id),
                )) {
                    if let Some(s) = env
                        .storage()
                        .instance()
                        .get(&DataKey::Payment(PaymentKey::PendingSettlement(payment_id)))
                    {
                        result.push_back(s);
                    }
                }
            }
        }
        result
    }

    // ── Issue #127: Dunning automation aliases ────────────────────────────

    /// Alias for resolve_dunning — resets retry count and transitions back to Active.
    pub fn manually_resolve_dunning(
        env: Env,
        admin: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        PaymentContract::resolve_dunning(env, admin, subscription_id)
    }

    /// Alias for set_dunning_config — updates the global DunningConfig.
    pub fn update_dunning_config(
        env: Env,
        admin: Address,
        config: DunningConfig,
    ) -> Result<(), Error> {
        PaymentContract::set_dunning_config(env, admin, config)
    }

    // ── Issue #118: Payment routing optimization ──────────────────────────

    /// Returns up to 3 candidate routes sorted by effective_cost (ascending).
    /// Routes are derived from stored conversion rates for the given token pair.
    pub fn get_optimal_route(
        env: Env,
        input_token: Address,
        output_token: Address,
        amount: i128,
    ) -> Vec<RouteOption> {
        let mut routes: Vec<RouteOption> = Vec::new(&env);

        // Direct route: input → output at 1:1 with no fee
        let direct = RouteOption {
            input_token: input_token.clone(),
            output_token: output_token.clone(),
            input_amount: amount,
            output_amount: amount,
            fee_bps: 0,
            effective_cost: 0,
        };
        routes.push_back(direct);

        // Fee-bearing route using configured fee (if any)
        let fee_config: Option<FeeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig));
        if let Some(fc) = fee_config {
            if fc.active && fc.fee_bps > 0 {
                let fee = (amount * fc.fee_bps as i128) / 10000;
                let route_with_fee = RouteOption {
                    input_token: input_token.clone(),
                    output_token: output_token.clone(),
                    input_amount: amount,
                    output_amount: amount - fee,
                    fee_bps: fc.fee_bps,
                    effective_cost: fee,
                };
                routes.push_back(route_with_fee);
            }
        }

        // Sort by effective_cost ascending (bubble sort, max 3 elements)
        let len = routes.len();
        for i in 0..len {
            for j in 0..(len.saturating_sub(i + 1)) {
                let a = routes.get(j).unwrap();
                let b = routes.get(j + 1).unwrap();
                if a.effective_cost > b.effective_cost {
                    routes.set(j, b);
                    routes.set(j + 1, a);
                }
            }
        }

        // Cap at 3
        let mut result: Vec<RouteOption> = Vec::new(&env);
        let cap = core::cmp::min(routes.len(), 3u32);
        for i in 0..cap {
            result.push_back(routes.get(i).unwrap());
        }
        result
    }

    /// Executes a payment using the provided route.
    /// Validates the route is still valid (fee_bps matches current config) before executing.
    pub fn // Fixed issue 565
    // execute_routed_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        route: RouteOption,
        payment_id: u64,
    ) -> Result<(), Error> {
        customer.require_auth();

        // Validate route is still valid at execution time
        let fee_config: Option<FeeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::FeeConfig));
        let current_fee_bps = fee_config
            .filter(|fc| fc.active)
            .map(|fc| fc.fee_bps)
            .unwrap_or(0);

        if route.fee_bps != current_fee_bps {
            return Err(Error::Feature(FeatureError::InvalidFeeConfig));
        }

        // Verify payment exists and belongs to customer
        let payment = PaymentContract::get_payment(&env, payment_id);
        if payment.customer != customer {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if payment.status != PaymentStatus::Pending {
            return Err(Error::Payment(PaymentError::InvalidStatus));
        }
        if payment.amount != route.input_amount {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        // Execute the transfer
        let token_client = token::Client::new(&env, &route.input_token);
        let contract_address = env.current_contract_address();
        token_client.transfer_from(
            &contract_address,
            &customer,
            &merchant,
            &route.output_amount,
        );

        Ok(())
    }

    /// Tags a payment with up to 10 arbitrary tag hashes.
    ///
    /// Tags can be used for categorization, filtering, or accounting.
    /// Once set, tags cannot be overwritten; use `remove_payment_tag` first.
    ///
    /// # Arguments
    /// * `merchant` - The merchant who owns the payment (must authorize).
    /// * `payment_id` - The payment to tag.
    /// * `tags` - A vector of `BytesN<32>` tag hashes (max 10).
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not the merchant, too many tags,
    /// or tags have already been set on this payment.
    pub fn tag_payment(
        env: Env,
        merchant: Address,
        payment_id: u64,
        tags: Vec<BytesN<32>>,
    ) -> Result<(), Error> {
        merchant.require_auth();

        // Check if payment exists and merchant is authorized
        let payment = Self::get_payment(&env, payment_id);
        if payment.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Validate tag count (max 10)
        if tags.len() > 10 {
            return Err(Error::Basic(BasicError::InvalidAmount));
        }

        if env
            .storage()
            .persistent()
            .has(&DataKey::Payment(PaymentKey::Tag(payment_id)))
        {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }

        env.storage()
            .persistent()
            .set(&DataKey::Payment(PaymentKey::Tag(payment_id)), &tags);

        Ok(())
    }

    /// Returns all tags associated with a payment.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID.
    ///
    /// # Returns
    /// A vector of `BytesN<32>` tag hashes. Returns an empty vector if no tags exist.
    pub fn get_payment_tags(env: Env, payment_id: u64) -> Vec<BytesN<32>> {
        match env
            .storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&DataKey::Payment(PaymentKey::Tag(payment_id)))
        {
            Some(tags) => tags,
            None => Vec::new(&env),
        }
    }

    /// Removes a specific tag from a payment.
    ///
    /// # Arguments
    /// * `merchant` - The merchant who owns the payment (must authorize).
    /// * `payment_id` - The payment ID.
    /// * `tag` - The tag hash to remove.
    ///
    /// # Returns
    /// `Ok(())` on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not the merchant, no tags exist,
    /// or the specified tag is not found.
    pub fn remove_payment_tag(
        env: Env,
        merchant: Address,
        payment_id: u64,
        tag: BytesN<32>,
    ) -> Result<(), Error> {
        merchant.require_auth();

        // Check if payment exists and merchant is authorized
        let payment = Self::get_payment(&env, payment_id);
        if payment.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let mut tags: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&DataKey::Payment(PaymentKey::Tag(payment_id)))
            .ok_or(Error::Payment(PaymentError::NotFound))?;

        // Find and remove the tag
        let mut found = false;
        let mut new_tags = Vec::new(&env);
        for existing_tag in tags.iter() {
            if existing_tag != tag {
                new_tags.push_back(existing_tag);
            } else {
                found = true;
            }
        }

        if !found {
            return Err(Error::Payment(PaymentError::NotFound));
        }

        env.storage()
            .persistent()
            .set(&DataKey::Payment(PaymentKey::Tag(payment_id)), &new_tags);

        Ok(())
    }

    /// Attaches an invoice with line items to a payment.
    ///
    /// Validates that the sum of line item amounts plus tax equals the payment amount.
    /// Each payment can only have one invoice attached.
    ///
    /// # Arguments
    /// * `merchant` - The merchant who owns the payment (must authorize).
    /// * `payment_id` - The payment to attach the invoice to.
    /// * `items` - Vector of `LineItem` with description, quantity, unit_price, and amount.
    /// * `tax` - Total tax amount.
    ///
    /// # Returns
    /// The invoice ID on success.
    ///
    /// # Errors
    /// Returns an error if the caller is not the merchant, an invoice already exists,
    /// line items are invalid, or the total does not match the payment amount.
    pub fn attach_invoice(
        env: Env,
        merchant: Address,
        payment_id: u64,
        items: Vec<LineItem>,
        tax: i128,
    ) -> Result<u64, Error> {
        merchant.require_auth();

        // Check if payment exists and merchant is authorized
        let payment = Self::get_payment(&env, payment_id);
        if payment.merchant != merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check if invoice already attached
        if env
            .storage()
            .persistent()
            .has(&DataKey::Payment(PaymentKey::Invoice(payment_id)))
        {
            return Err(Error::Payment(PaymentError::AlreadyProcessed));
        }

        // Validate line items and calculate subtotal
        let mut subtotal: i128 = 0;
        for item in items.iter() {
            if item.quantity == 0 || item.unit_price <= 0 || item.amount <= 0 {
                return Err(Error::Payment(PaymentError::InvalidLineItem));
            }
            subtotal = subtotal
                .checked_add(item.amount)
                .ok_or(Error::Basic(BasicError::InvalidAmount))?;
        }

        // Verify total
        let total = subtotal
            .checked_add(tax)
            .ok_or(Error::Basic(BasicError::InvalidAmount))?;
        if total != payment.amount {
            return Err(Error::Payment(PaymentError::RefundExceedsPayment));
        }

        // Get next invoice ID
        let invoice_id: u64 = env
            .storage()
            .persistent()
            .get::<_, u64>(&DataKey::Payment(PaymentKey::InvoiceCounter))
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::Basic(BasicError::InvalidAmount))?;

        // Create and store invoice
        let invoice = PaymentInvoice {
            invoice_id,
            payment_id,
            items,
            subtotal,
            tax,
            total,
            issued_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Payment(PaymentKey::Invoice(payment_id)), &invoice);
        env.storage()
            .persistent()
            .set(&DataKey::Payment(PaymentKey::InvoiceCounter), &invoice_id);
        env.storage().persistent().set(
            &DataKey::Payment(PaymentKey::InvoicePaymentId(invoice_id)),
            &payment_id,
        );

        Ok(invoice_id)
    }

    /// Returns an invoice by its invoice ID.
    ///
    /// # Arguments
    /// * `invoice_id` - The invoice ID.
    ///
    /// # Returns
    /// `Some(PaymentInvoice)` if the invoice exists, `None` otherwise.
    pub fn get_invoice(env: Env, invoice_id: u64) -> Option<PaymentInvoice> {
        let payment_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Payment(PaymentKey::InvoicePaymentId(invoice_id)))?;
        env.storage()
            .persistent()
            .get(&DataKey::Payment(PaymentKey::Invoice(payment_id)))
    }

    /// Returns the invoice attached to a specific payment.
    ///
    /// # Arguments
    /// * `payment_id` - The payment ID.
    ///
    /// # Returns
    /// `Some(PaymentInvoice)` if an invoice is attached, `None` otherwise.
    pub fn get_payment_invoice(env: Env, payment_id: u64) -> Option<PaymentInvoice> {
        env.storage()
            .persistent()
            .get(&DataKey::Payment(PaymentKey::Invoice(payment_id)))
    }

    /// Verifies that an invoice's stored total matches a recalculation of its line items.
    ///
    /// Useful for detecting tampering or data corruption.
    ///
    /// # Arguments
    /// * `invoice_id` - The invoice ID.
    ///
    /// # Returns
    /// `true` if the recalculation matches the stored total, `false` if
    /// the invoice is not found or the totals do not match.
    pub fn verify_invoice_total(env: Env, invoice_id: u64) -> bool {
        if let Some(invoice) = Self::get_invoice(env, invoice_id) {
            let mut calculated_subtotal: i128 = 0;
            for item in invoice.items.iter() {
                calculated_subtotal = calculated_subtotal.saturating_add(item.amount);
            }
            let calculated_total = calculated_subtotal.saturating_add(invoice.tax);
            calculated_total == invoice.total
        } else {
            false
        }
    }

    fn validate_bps(bps: u32) -> Result<(), Error> {
        if bps < 1 || bps > 10000 {
            return Err(Error::Basic(BasicError::InvalidBps));
        };

        Ok(())
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

#[cfg(test)]
mod test_metered_billing;

mod test_fee_sweep;
#[cfg(test)]
mod test_split_payment;

#[cfg(test)]
mod test_spend_limits;

#[cfg(test)]
mod test_subscription_groups;

#[cfg(test)]
mod test_finality_delay;

#[cfg(test)]
mod test_fee_rebate;

#[cfg(test)]
mod test_payment_forward;

#[cfg(test)]
mod test_scheduled_payment;

#[cfg(test)]
mod schema_version_test;
