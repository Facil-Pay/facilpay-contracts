"""One-off migration helper for the payment contract refactor.

This script was used to rewrite contracts/payment/src/lib.rs by applying a set of
regex-based substitutions to produce the refactored storage shapes and error enums.
It is not part of the runtime contract logic and is not required for ordinary builds or
tests in the current tree.

Status: historical maintenance script; it has already been applied to the source. It is
safe to keep only as an archival reference, but it is not needed for normal development.
"""

import re


def refactor():
    with open("contracts/payment/src/lib.rs", "r") as f:
        content = f.read()

    # 1. Update imports
    content = content.replace("contracttype, token,", "contracterror, contracttype, token,")

    # 2. Block Removal
    content = re.sub(r'#\[derive\(Clone\)\]\s+#\[contracttype\]\s+pub enum DataKey \{.*?\}', '', content, flags=re.DOTALL)
    content = re.sub(r'#\[repr\(u32\)\]\s+#\[derive\(Clone, Copy, Debug, PartialEq\)\]\s+pub enum Error \{.*?\}', '', content, flags=re.DOTALL)
    content = re.sub(r'#\[derive\(Clone, Debug, PartialEq\)\]\s+#\[contracttype\]\s+pub enum Currency \{.*?\}', '', content, flags=re.DOTALL)
    content = re.sub(r'#\[derive\(Clone, Debug, PartialEq, Eq\)\]\s+#\[contracttype\]\s+pub enum PayoutFrequency \{.*?\}', '', content, flags=re.DOTALL)

    start_marker = "// Manual trait implementations"
    end_marker = "#[contractevent]"
    s_idx = content.find(start_marker)
    if s_idx != -1:
        e_idx = content.find(end_marker, s_idx)
        if e_idx != -1:
            content = content[:s_idx] + content[e_idx:]

    # 3. New Definitions
    new_definitions = """
#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum Currency { XLM, USDC, USDT, BTC, ETH }

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum PayoutFrequency { Immediate, Daily, Weekly, Monthly }

#[derive(Clone)]
#[contracttype]
pub enum ConfigKey {
    Admin, MultiSigConfig, FeeConfig, RateLimitConfig, DunningConfig,
    LoyaltyConfig, RiskFeeConfig, FinalityConfig, FeeRebateConfig,
    TierThresholds, LargePaymentThreshold, GlobalMerchantCount, PauseStateKey,
}

#[derive(Clone)]
#[contracttype]
pub enum PaymentKey {
    Data(u64), Counter, Metadata(u64), Memo(u64), MemoVersion(u64), Tag(u64),
    Invoice(u64), InvoiceCounter, InvoicePaymentId(u64), PartialPaymentCounter(u64),
    OutstandingBalance(u64), PendingSettlement(u64), AccumulatedFees, LargePaymentCounter,
}

#[derive(Clone)]
#[contracttype]
pub enum SubscriptionKey {
    Data(u64), Counter, Metered(u64), MeteredCounter, Group(u64),
    GroupCounter, GroupMembership(u64),
}

#[derive(Clone)]
#[contracttype]
pub enum FeatureKey {
    PaymentAnalytics, PlatformAnalyticsDaily(u64), PaymentForwardConfig(Address),
    OracleRateConfig(Currency), ConversionRate(Currency), MerchantRateLimit(Address),
    CustomerLoyaltyBalance(Address), CustomerSpendLimit(Address), PaymentChannel(u64),
    PaymentChannelCounter, SplitConfig(u64), SweepRecipient, SweepCounter,
    SweepHistory(u64), RouteOptions(Address, Address),
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Config(ConfigKey), Payment(PaymentKey), Subscription(SubscriptionKey),
    Feature(FeatureKey), Customer(CustomerDataKey), Merchant(MerchantDataKey),
    State(StateDataKey),
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracterror]
pub enum BasicError {
    Unauthorized = 100, MetadataTooLarge = 101, NotesTooLarge = 102,
    InvalidCurrency = 103, InvalidBatchSize = 104, BatchPartialFailure = 105,
    RateLimitExceeded = 106, DailyVolumeExceeded = 107, AddressFlagged = 108,
    AddressAlreadyFlagged = 109, AmountExceedsLimit = 110, MultiSigNotInitialized = 111,
    InsufficientAdmins = 112, NotAnAdmin = 113, AlreadyApproved = 114,
    OracleCallFailed = 115, ContractPaused = 116, FunctionPaused = 117,
    InvalidTierThresholds = 118, OracleFeedStale = 119, OracleNotConfigured = 120,
    InvalidAmount = 121, VerificationLevelNotFound = 122, TierLimitsNotConfigured = 123,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracterror]
pub enum PaymentError {
    NotFound = 200, InvalidStatus = 201, AlreadyProcessed = 202, Expired = 203,
    NotExpired = 204, NoExpiration = 205, TransferFailed = 206, RefundExceedsPayment = 207,
    NotYetDue = 208, ScheduledPaymentCancelled = 209, MetadataAlreadySet = 210,
    MetadataNotFound = 211, HashMismatch = 212, AlreadyFullyPaid = 213,
    InstallmentExceedsRemaining = 214, PartialPaymentNotFound = 215,
    MerchantRateLimitExceeded = 216, AmountRateLimitExceeded = 217,
    PayoutScheduleNotFound = 218, PayoutNotYetDue = 219, NothingToSettle = 220,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracterror]
pub enum SubscriptionError {
    NotFound = 300, NotActive = 301, PaymentNotDue = 302, MaxRetriesExceeded = 303,
    Ended = 304, DunningNotFound = 305, NotInDunning = 306, RetryNotDue = 307,
    GracePeriodExpired = 308, RetryTooEarly = 309, MeteredNotFound = 310,
    BillingCapExceeded = 311, GroupNotFound = 312, AlreadyInGroup = 313,
    GroupSizeLimitExceeded = 314,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracterror]
pub enum ProposalError {
    NotFound = 400, Expired = 401, AlreadyExecuted = 402, ThresholdNotMet = 403,
    RequiresMultiSig = 404, InsufficientApprovals = 405, ProposalExpired = 406,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracterror]
pub enum FeatureError {
    EscrowMappingNotFound = 500, EscrowBridgeFailed = 501, FeeConfigNotFound = 502,
    InsufficientFees = 503, ConditionNotMet = 504, ConditionAlreadyEvaluated = 505,
    AutoEscrowRuleNotFound = 506, AutoEscrowBelowMinimum = 507,
    AutoEscrowAlreadyTriggered = 508, ConditionEvaluationFailed = 509,
    ConditionRuntimeNotMet = 510, InvalidFeeConfig = 511, ChannelNotFound = 512,
    InvalidSignature = 513, InvalidNonce = 514, ChannelClosed = 515,
    ChannelExpired = 516, ChannelNotExpired = 517, InvalidSplitShares = 518,
    TooManyRecipients = 519, SplitConfigNotFound = 520, SplitAlreadyExecuted = 521,
    LoyaltyNotConfigured = 522, InsufficientPoints = 523, PointsExpired = 524,
    NothingToSweep = 525, SweepRecipientNotSet = 526, SpendLimitExceeded = 527,
    SpendLimitNotConfigured = 528, SettlementNotReady = 529, FinalityConfigNotFound = 530,
    SettlementAlreadyFinalized = 531, RebateThresholdNotMet = 532,
    RebateAlreadyClaimed = 533, RebateConfigNotFound = 534, ForwardConfigNotFound = 535,
    ForwardLoop = 536, InvalidForwardBps = 537,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[contracttype]
pub enum Error {
    Basic(BasicError), Payment(PaymentError), Subscription(SubscriptionError),
    Proposal(ProposalError), Feature(FeatureError),
}

impl Error {
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
            if code >= 500 && code <= 537 { return Ok(Error::Feature(unsafe { core::mem::transmute(code as u16) })); }
            if code >= 400 && code <= 406 { return Ok(Error::Proposal(unsafe { core::mem::transmute(code as u16) })); }
            if code >= 300 && code <= 314 { return Ok(Error::Subscription(unsafe { core::mem::transmute(code as u16) })); }
            if code >= 200 && code <= 220 { return Ok(Error::Payment(unsafe { core::mem::transmute(code as u8) })); }
            if code >= 100 && code <= 123 { return Ok(Error::Basic(unsafe { core::mem::transmute(code as u8) })); }
        }
        Err(error)
    }
}

impl IntoVal<Env, Val> for Error {
    fn into_val(self, env: &Env) -> Val {
        soroban_sdk::Error::from(self).into_val(env)
    }
}

impl IntoVal<Env, Val> for &Error {
    fn into_val(self, env: &Env) -> Val {
        soroban_sdk::Error::from(*self).into_val(env)
    }
}

impl TryFromVal<Env, Val> for Error {
    type Error = soroban_sdk::ConversionError;
    fn try_from_val(env: &Env, val: &Val) -> Result<Self, Self::Error> {
        let error: soroban_sdk::Error = soroban_sdk::Error::try_from_val(env, val).map_err(|_| soroban_sdk::ConversionError)?;
        Error::try_from(error).map_err(|_| soroban_sdk::ConversionError)
    }
}
"""
    # Insert definitions early
    imports_match = re.search(r'use soroban_sdk::\{.*?\};', content, flags=re.DOTALL)
    if imports_match:
        content = content[:imports_match.end()] + "\n" + new_definitions + "\n" + content[imports_match.end():]

    # 4. DataKey Replacements
    dk_tuple = [
        ("PaymentMetadata", "Payment", "Metadata"), ("PaymentMemoVersion", "Payment", "MemoVersion"),
        ("PaymentMemo", "Payment", "Memo"), ("PaymentTag", "Payment", "Tag"),
        ("PaymentInvoice", "Payment", "Invoice"), ("InvoicePaymentId", "Payment", "InvoicePaymentId"),
        ("PartialPaymentCounter", "Payment", "PartialPaymentCounter"), ("OutstandingBalance", "Payment", "OutstandingBalance"),
        ("PendingSettlement", "Payment", "PendingSettlement"), ("PaymentForwardConfig", "Feature", "PaymentForwardConfig"),
        ("PlatformAnalyticsDaily", "Feature", "PlatformAnalyticsDaily"), ("OracleRateConfig", "Feature", "OracleRateConfig"),
        ("ConversionRate", "Feature", "ConversionRate"), ("MerchantRateLimit", "Feature", "MerchantRateLimit"),
        ("CustomerLoyaltyBalance", "Feature", "CustomerLoyaltyBalance"), ("CustomerSpendLimit", "Feature", "CustomerSpendLimit"),
        ("PaymentChannel", "Feature", "PaymentChannel"), ("SplitConfig", "Feature", "SplitConfig"),
        ("SweepHistory", "Feature", "SweepHistory"), ("RouteOptions", "Feature", "RouteOptions"),
        ("MeteredSubscription", "Subscription", "Metered"), ("SubscriptionGroupMembership", "Subscription", "GroupMembership"),
        ("SubscriptionGroup", "Subscription", "Group"), ("Subscription", "Subscription", "Data"),
        ("Payment", "Payment", "Data"),
    ]
    for old, cat, sub in dk_tuple:
        content = re.sub(rf'\bDataKey::{old}\((.*?)\)', rf'TEMP_DK::{cat}({cat}Key::{sub}(\1))', content)

    dk_unit = {
        "Admin": "Config(ConfigKey::Admin)", "MultiSigConfig": "Config(ConfigKey::MultiSigConfig)",
        "FeeConfig": "Config(ConfigKey::FeeConfig)", "RateLimitConfig": "Config(ConfigKey::RateLimitConfig)",
        "DunningConfig": "Config(ConfigKey::DunningConfig)", "LoyaltyConfig": "Config(ConfigKey::LoyaltyConfig)",
        "RiskFeeConfig": "Config(ConfigKey::RiskFeeConfig)", "FinalityConfig": "Config(ConfigKey::FinalityConfig)",
        "FeeRebateConfig": "Config(ConfigKey::FeeRebateConfig)", "TierThresholds": "Config(ConfigKey::TierThresholds)",
        "LargePaymentThreshold": "Config(ConfigKey::LargePaymentThreshold)", "GlobalMerchantCount": "Config(ConfigKey::GlobalMerchantCount)",
        "PauseStateKey": "Config(ConfigKey::PauseStateKey)", "PaymentCounter": "Payment(PaymentKey::Counter)",
        "SubscriptionCounter": "Subscription(SubscriptionKey::Counter)", "ProposalCounter": "Payment(PaymentKey::LargePaymentCounter)",
        "LargePaymentCounter": "Payment(PaymentKey::LargePaymentCounter)", "PaymentAnalyticsKey": "Feature(FeatureKey::PaymentAnalytics)",
        "PaymentChannelCounter": "Feature(FeatureKey::PaymentChannelCounter)", "SweepRecipient": "Feature(FeatureKey::SweepRecipient)",
        "SweepCounter": "Feature(FeatureKey::SweepCounter)", "SubscriptionGroupCounter": "Subscription(SubscriptionKey::GroupCounter)",
        "MeteredSubscriptionCounter": "Subscription(SubscriptionKey::MeteredCounter)", "AccumulatedFees": "Payment(PaymentKey::AccumulatedFees)",
        "InvoiceCounter": "Payment(PaymentKey::InvoiceCounter)",
    }
    for old, new in dk_unit.items():
        content = re.sub(rf'\bDataKey::{old}\b', f'TEMP_DK::{new}', content)

    content = re.sub(r'&(State|Customer|Merchant)DataKey::(\w+)(\s*\((?:[^()]*|\([^()]*\))*\))?',
                     r'&TEMP_DK::\1(\1DataKey::\2\3)', content)
    content = content.replace("TEMP_DK::", "DataKey::")

    for key in ["StateDataKey", "CustomerDataKey", "MerchantDataKey"]:
        content = content.replace(f"get::<{key},", "get::<DataKey,")
        content = content.replace(f"has::<{key}>(", "has::<DataKey>(")
        content = content.replace(f"remove::<{key}>(", "remove::<DataKey>(")

    # 5. Error Replacements
    err_map = {
        "PaymentNotFound": "Payment(PaymentError::NotFound)", "InvalidStatus": "Payment(PaymentError::InvalidStatus)",
        "AlreadyProcessed": "Payment(PaymentError::AlreadyProcessed)", "Unauthorized": "Basic(BasicError::Unauthorized)",
        "PaymentExpired": "Payment(PaymentError::Expired)", "NotExpired": "Payment(PaymentError::NotExpired)",
        "NoExpiration": "Payment(PaymentError::NoExpiration)", "TransferFailed": "Payment(PaymentError::TransferFailed)",
        "MetadataTooLarge": "Basic(BasicError::MetadataTooLarge)", "NotesTooLarge": "Basic(BasicError::NotesTooLarge)",
        "InvalidCurrency": "Basic(BasicError::InvalidCurrency)", "RefundExceedsPayment": "Payment(PaymentError::RefundExceedsPayment)",
        "SubscriptionNotFound": "Subscription(SubscriptionError::NotFound)", "SubscriptionNotActive": "Subscription(SubscriptionError::NotActive)",
        "PaymentNotDue": "Subscription(SubscriptionError::PaymentNotDue)", "MaxRetriesExceeded": "Subscription(SubscriptionError::MaxRetriesExceeded)",
        "SubscriptionEnded": "Subscription(SubscriptionError::Ended)", "InvalidBatchSize": "Basic(BasicError::InvalidBatchSize)",
        "BatchPartialFailure": "Basic(BasicError::BatchPartialFailure)", "RateLimitExceeded": "Basic(BasicError::RateLimitExceeded)",
        "DailyVolumeExceeded": "Basic(BasicError::DailyVolumeExceeded)", "AddressFlagged": "Basic(BasicError::AddressFlagged)",
        "AddressAlreadyFlagged": "Basic(BasicError::AddressAlreadyFlagged)", "AmountExceedsLimit": "Basic(BasicError::AmountExceedsLimit)",
        "DunningNotFound": "Subscription(SubscriptionError::DunningNotFound)", "SubscriptionNotInDunning": "Subscription(SubscriptionError::NotInDunning)",
        "RetryNotDue": "Subscription(SubscriptionError::RetryNotDue)", "GracePeriodExpired": "Subscription(SubscriptionError::GracePeriodExpired)",
        "EscrowMappingNotFound": "Feature(FeatureError::EscrowMappingNotFound)", "EscrowBridgeFailed": "Feature(FeatureError::EscrowBridgeFailed)",
        "MultiSigNotInitialized": "Basic(BasicError::MultiSigNotInitialized)", "ProposalNotFound": "Proposal(ProposalError::NotFound)",
        "ProposalExpired": "Proposal(ProposalError::Expired)", "ProposalAlreadyExecuted": "Proposal(ProposalError::AlreadyExecuted)",
        "MultiSigThresholdNotMet": "Proposal(ProposalError::ThresholdNotMet)", "InsufficientAdmins": "Basic(BasicError::InsufficientAdmins)",
        "NotAnAdmin": "Basic(BasicError::NotAnAdmin)", "AlreadyApproved": "Basic(BasicError::AlreadyApproved)",
        "FeeConfigNotFound": "Feature(FeatureError::FeeConfigNotFound)", "InsufficientFees": "Feature(FeatureError::InsufficientFees)",
        "ConditionNotMet": "Feature(FeatureError::ConditionNotMet)", "ConditionAlreadyEvaluated": "Feature(FeatureError::ConditionAlreadyEvaluated)",
        "OracleCallFailed": "Basic(BasicError::OracleCallFailed)", "ContractPaused": "Basic(BasicError::ContractPaused)",
        "FunctionPaused": "Basic(BasicError::FunctionPaused)", "InvalidTierThresholds": "Basic(BasicError::InvalidTierThresholds)",
        "AutoEscrowRuleNotFound": "Feature(FeatureError::AutoEscrowRuleNotFound)", "AutoEscrowBelowMinimum": "Feature(FeatureError::AutoEscrowBelowMinimum)",
        "AutoEscrowAlreadyTriggered": "Feature(FeatureError::AutoEscrowAlreadyTriggered)", "PaymentNotYetDue": "Payment(PaymentError::NotYetDue)",
        "ScheduledPaymentCancelled": "Payment(PaymentError::ScheduledPaymentCancelled)", "OracleFeedStale": "Basic(BasicError::OracleFeedStale)",
        "OracleNotConfigured": "Basic(BasicError::OracleNotConfigured)", "ConditionEvaluationFailed": "Feature(FeatureError::ConditionEvaluationFailed)",
        "ConditionRuntimeNotMet": "Feature(FeatureError::ConditionRuntimeNotMet)", "RetryTooEarly": "Subscription(SubscriptionError::RetryTooEarly)",
        "PaymentRequiresMultiSig": "Proposal(ProposalError::RequiresMultiSig)", "InsufficientPaymentApprovals": "Proposal(ProposalError::InsufficientApprovals)",
        "PaymentProposalExpired": "Proposal(ProposalError::ProposalExpired)", "MetadataAlreadySet": "Payment(PaymentError::MetadataAlreadySet)",
        "MetadataNotFound": "Payment(PaymentError::MetadataNotFound)", "HashMismatch": "Payment(PaymentError::HashMismatch)",
        "PaymentAlreadyFullyPaid": "Payment(PaymentError::AlreadyFullyPaid)", "InstallmentExceedsRemaining": "Payment(PaymentError::InstallmentExceedsRemaining)",
        "PartialPaymentNotFound": "Payment(PaymentError::PartialPaymentNotFound)", "MerchantRateLimitExceeded": "Payment(PaymentError::MerchantRateLimitExceeded)",
        "AmountRateLimitExceeded": "Payment(PaymentError::AmountRateLimitExceeded)", "InvalidFeeConfig": "Feature(FeatureError::InvalidFeeConfig)",
        "InvalidAmount": "Basic(BasicError::InvalidAmount)", "ChannelNotFound": "Feature(FeatureError::ChannelNotFound)",
        "InvalidSignature": "Feature(FeatureError::InvalidSignature)", "InvalidNonce": "Feature(FeatureError::InvalidNonce)",
        "ChannelClosed": "Feature(FeatureError::ChannelClosed)", "ChannelExpired": "Feature(FeatureError::ChannelExpired)",
        "ChannelNotExpired": "Feature(FeatureError::ChannelNotExpired)", "MeteredSubscriptionNotFound": "Subscription(SubscriptionError::MeteredNotFound)",
        "BillingCapExceeded": "Subscription(SubscriptionError::BillingCapExceeded)", "InvalidSplitShares": "Feature(FeatureError::InvalidSplitShares)",
        "TooManyRecipients": "Feature(FeatureError::TooManyRecipients)", "SplitConfigNotFound": "Feature(FeatureError::SplitConfigNotFound)",
        "SplitAlreadyExecuted": "Feature(FeatureError::SplitAlreadyExecuted)", "LoyaltyNotConfigured": "Feature(FeatureError::LoyaltyNotConfigured)",
        "InsufficientPoints": "Feature(FeatureError::InsufficientPoints)", "PointsExpired": "Feature(FeatureError::PointsExpired)",
        "NothingToSweep": "Feature(FeatureError::NothingToSweep)", "SweepRecipientNotSet": "Feature(FeatureError::SweepRecipientNotSet)",
        "SpendLimitExceeded": "Feature(FeatureError::SpendLimitExceeded)", "SpendLimitNotConfigured": "Feature(FeatureError::SpendLimitNotConfigured)",
        "GroupNotFound": "Subscription(SubscriptionError::GroupNotFound)", "SubscriptionAlreadyInGroup": "Subscription(SubscriptionError::AlreadyInGroup)",
        "GroupSizeLimitExceeded": "Subscription(SubscriptionError::GroupSizeLimitExceeded)", "SettlementNotReady": "Feature(FeatureError::SettlementNotReady)",
        "FinalityConfigNotFound": "Feature(FeatureError::FinalityConfigNotFound)", "SettlementAlreadyFinalized": "Feature(FeatureError::SettlementAlreadyFinalized)",
        "VerificationLevelNotFound": "Basic(BasicError::VerificationLevelNotFound)", "TierLimitsNotConfigured": "Basic(BasicError::TierLimitsNotConfigured)",
        "RebateThresholdNotMet": "Feature(FeatureError::RebateThresholdNotMet)", "RebateAlreadyClaimed": "Feature(FeatureError::RebateAlreadyClaimed)",
        "RebateConfigNotFound": "Feature(FeatureError::RebateConfigNotFound)", "PayoutScheduleNotFound": "Payment(PaymentError::PayoutScheduleNotFound)",
        "PayoutNotYetDue": "Payment(PaymentError::PayoutNotYetDue)", "NothingToSettle": "Payment(PaymentError::NothingToSettle)",
        "ForwardConfigNotFound": "Feature(FeatureError::ForwardConfigNotFound)", "ForwardLoop": "Feature(FeatureError::ForwardLoop)",
        "InvalidForwardBps": "Feature(FeatureError::InvalidForwardBps)",
    }
    for old, new in err_map.items():
        content = re.sub(rf'\bError::{old}\b', f'Error::{new}', content)

    # 6. Casting fixes
    content = content.replace("(*val as u32)", "(val.to_u32())")
    content = content.replace("e as u32", "e.to_u32()")
    for te in ["InvalidCurrency", "MetadataTooLarge", "AddressFlagged"]:
        content = content.replace(f"Error::Basic(BasicError::{te}) as u32", f"Error::Basic(BasicError::{te}).to_u32()")

    with open("contracts/payment/src/lib.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    refactor()
