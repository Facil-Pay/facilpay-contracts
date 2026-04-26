#![no_std]
use escrow::EscrowContractClient;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes,
    BytesN, Env, String, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Payment(u64),
    PaymentCounter,
    CustomerPayments(Address, u64),
    MerchantPayments(Address, u64),
    CustomerPaymentCount(Address),
    MerchantPaymentCount(Address),
    PaymentNotes(u64),
    ConversionRate(Currency),
    SubscriptionCounter,
    Subscription(u64),
    CustomerSubscriptions(Address, u64),
    CustomerSubscriptionCount(Address),
    MerchantSubscriptions(Address, u64),
    MerchantSubscriptionCount(Address),
    RateLimitConfig,
    AddressRateLimit(Address),
    DunningConfig,
    DunningState(u64),
    EscrowedPayment(u64),
    ConditionalPayment(u64),
    MultiSigConfig,
    AdminProposal(String),
    ProposalCounter,
    FeeConfig,
    MerchantFeeRecord(Address),
    TierThresholds,
    AccumulatedFees,
    // Analytics
    PaymentAnalyticsKey,
    MerchantAnalytics(Address),
    CustomerAnalytics(Address),
    // Extended customer analytics
    CustomerMerchantVolume(Address, Address),
    CustomerMerchantList(Address, u64),
    CustomerMerchantCount(Address),
    CustomerMonthlyVolume(Address, u64),
    CustomerHourCount(Address, u32),
    // Pause system
    PauseStateKey,
    PauseHistoryEntry(u64),
    PauseHistoryCount,
    LargePaymentProposal(u64),
    LargePaymentThreshold,
    LargePaymentCounter,
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
    pub trial_period_seconds: u64, // 0 = no trial
    pub trial_ends_at: u64,        // 0 = no trial
    pub converted: bool,           // true after first post-trial charge
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
    AddressFlagged = 22,
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
    PaymentRequiresMultiSig = 64,
    InsufficientPaymentApprovals = 65,
    PaymentProposalExpired = 66,
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
    pub expiration_timestamp: u64,
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
    pub admin: Address,
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
pub struct DunningConfig {
    pub grace_period: u64,
    pub retry_intervals: Vec<u64>,
    pub max_dunning_attempts: u32,
    pub suspend_after_attempts: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct DunningState {
    pub subscription_id: u64,
    pub attempts: u32,
    pub next_retry_at: u64,
    pub grace_period_ends_at: u64,
    pub suspended: bool,
    pub last_failure_reason: String,
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
pub struct ConditionalPayment {
    pub payment_id: u64,
    pub condition: ConditionType,
    pub condition_met: bool,
    pub evaluated_at: Option<u64>,
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
            .set(&DataKey::AdminProposal(proposal_id.clone()), &proposal);

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
            .get(&DataKey::AdminProposal(proposal_id.clone()))
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
            .set(&DataKey::AdminProposal(proposal_id.clone()), &proposal);

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
            .get(&DataKey::AdminProposal(proposal_id.clone()))
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
            .set(&DataKey::AdminProposal(proposal_id.clone()), &proposal);

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
            .get(&DataKey::AdminProposal(proposal_id.clone()))
            .ok_or(Error::ProposalNotFound)?;

        if proposal.executed || proposal.rejected {
            return Err(Error::ProposalAlreadyExecuted);
        }

        proposal.rejected = true;
        env.storage()
            .instance()
            .set(&DataKey::AdminProposal(proposal_id.clone()), &proposal);

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

        // Check rate limits and anti-fraud before processing
        PaymentContract::check_rate_limit(env, &customer, amount)?;

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
            .get(&DataKey::CustomerPaymentCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerPayments(customer.clone(), customer_count),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerPaymentCount(customer),
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
        c_analytics.avg_transaction_size = c_analytics.total_volume / (c_analytics.total_payments as i128);
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
            .get(&DataKey::CustomerHourCount(payment.customer.clone(), c_analytics.peak_hour))
            .unwrap_or(0);
        if hour_count > peak_hour_count || (hour_count == peak_hour_count && hour == c_analytics.peak_hour) {
            c_analytics.peak_hour = hour;
        }

        // Track per-merchant volume and update top merchant
        let prev_merchant_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerMerchantVolume(payment.customer.clone(), payment.merchant.clone()))
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
            .get(&DataKey::CustomerMonthlyVolume(payment.customer.clone(), month_bucket))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerMonthlyVolume(payment.customer.clone(), month_bucket),
            &(prev_monthly + amount),
        );

        env.storage().instance().set(
            &DataKey::CustomerAnalytics(payment.customer.clone()),
            &c_analytics,
        );

        (PaymentCreated {
            payment_id,
            customer: payment.customer,
            merchant: payment.merchant,
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
        let payment: Option<Payment> = env
            .storage()
            .instance()
            .get(&DataKey::Payment(payment_id));
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
            .set(&DataKey::EscrowedPayment(payment_id), &bridge);

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

    pub fn get_escrowed_payment(env: Env, payment_id: u64) -> Result<EscrowedPayment, Error> {
        env.storage()
            .instance()
            .get(&DataKey::EscrowedPayment(payment_id))
            .ok_or(Error::EscrowMappingNotFound)
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

        // Check payment status is Pending
        if payment.status != PaymentStatus::Pending {
            return Err(Error::InvalidStatus);
        }

        // Check payment has expiration set
        if payment.expires_at == 0 {
            return Err(Error::NoExpiration);
        }

        // Check current time is past expires_at
        if env.ledger().timestamp() <= payment.expires_at {
            return Err(Error::NotExpired);
        }

        // Update payment status to Cancelled
        payment.status = PaymentStatus::Cancelled;

        // Store updated payment back to storage
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        // Emit PaymentExpired event
        (PaymentExpired {
            payment_id,
            expiration_timestamp: payment.expires_at,
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
                .get::<DataKey, LargePaymentProposal>(&DataKey::LargePaymentProposal(payment_id))
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
                .set(&DataKey::LargePaymentProposal(payment_id), &proposal);

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
            merchant: payment.merchant,
            amount: payment.amount,
        })
        .publish(env);

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
            .get(&DataKey::CustomerPaymentCount(customer.clone()))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::CustomerPayments(customer.clone(), i))
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
            .get(&DataKey::CustomerPaymentCount(customer))
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
            trial_period_seconds,
            trial_ends_at,
            converted: false,
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
        if sub.trial_ends_at > 0 {
            (TrialStarted {
                subscription_id: sub_id,
                trial_ends_at: sub.trial_ends_at,
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

        // Must be Active
        if sub.status != SubscriptionStatus::Active {
            return Err(Error::SubscriptionNotActive);
        }

        let now = env.ledger().timestamp();

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
        if sub.trial_ends_at > 0 && now < sub.trial_ends_at {
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
            if sub.trial_ends_at > 0 && !sub.converted {
                sub.converted = true;
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
        if sub.trial_ends_at > 0 && now < sub.trial_ends_at {
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
        sub.next_payment_at = now + sub.interval;
        sub.status = SubscriptionStatus::Active;

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
                grace_period: 7 * 24 * 60 * 60, // 7 days
                retry_intervals: Vec::from_array(
                    &env,
                    [
                        60 * 60,          // 1 hour
                        6 * 60 * 60,      // 6 hours
                        24 * 60 * 60,     // 1 day
                        3 * 24 * 60 * 60, // 3 days
                    ],
                ),
                max_dunning_attempts: 5,
                suspend_after_attempts: 4,
            })
    }

    /// Returns the dunning state for a subscription, if any.
    pub fn get_dunning_state(env: Env, subscription_id: u64) -> Option<DunningState> {
        env.storage()
            .instance()
            .get(&DataKey::DunningState(subscription_id))
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

        let mut dunning_state: DunningState = env
            .storage()
            .instance()
            .get(&DataKey::DunningState(subscription_id))
            .ok_or(Error::DunningNotFound)?;

        let now = env.ledger().timestamp();

        // Check if retry is due
        if now < dunning_state.next_retry_at {
            return Err(Error::RetryNotDue);
        }

        // Check if grace period has expired
        if now > dunning_state.grace_period_ends_at {
            // Move to suspended state
            sub.status = SubscriptionStatus::Suspended;
            dunning_state.suspended = true;

            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
            env.storage()
                .instance()
                .set(&DataKey::DunningState(subscription_id), &dunning_state);

            (SubscriptionSuspended {
                subscription_id,
                reason: String::from_str(&env, "Grace period expired"),
            })
            .publish(&env);

            return Err(Error::GracePeriodExpired);
        }

        // Attempt the payment
        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();

        let transfer_ok = token_client
            .try_transfer_from(&contract_address, &sub.customer, &sub.merchant, &sub.amount)
            .is_ok();

        if transfer_ok {
            // Payment successful - resolve dunning
            sub.payment_count += 1;
            sub.retry_count = 0;
            sub.next_payment_at = now + sub.interval;
            sub.status = SubscriptionStatus::Active;

            // Auto-expire when duration is reached
            if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                sub.status = SubscriptionStatus::Expired;
            }

            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);

            // Remove dunning state
            env.storage()
                .instance()
                .remove(&DataKey::DunningState(subscription_id));

            (RecurringPaymentExecuted {
                subscription_id,
                payment_count: sub.payment_count,
                amount: sub.amount,
                next_payment_at: sub.next_payment_at,
            })
            .publish(&env);

            Ok(())
        } else {
            // Payment failed - update dunning state
            dunning_state.attempts += 1;

            let config = PaymentContract::get_dunning_config(env.clone());

            if dunning_state.attempts >= config.max_dunning_attempts {
                // Max attempts reached - suspend subscription
                sub.status = SubscriptionStatus::Suspended;
                dunning_state.suspended = true;

                env.storage()
                    .instance()
                    .set(&DataKey::Subscription(subscription_id), &sub);
                env.storage()
                    .instance()
                    .set(&DataKey::DunningState(subscription_id), &dunning_state);

                (SubscriptionSuspended {
                    subscription_id,
                    reason: String::from_str(&env, "Maximum dunning attempts exceeded"),
                })
                .publish(&env);

                return Err(Error::MaxRetriesExceeded);
            } else if dunning_state.attempts >= config.suspend_after_attempts {
                // Suspend after configured attempts
                sub.status = SubscriptionStatus::Suspended;
                dunning_state.suspended = true;

                env.storage()
                    .instance()
                    .set(&DataKey::Subscription(subscription_id), &sub);
                env.storage()
                    .instance()
                    .set(&DataKey::DunningState(subscription_id), &dunning_state);

                (SubscriptionSuspended {
                    subscription_id,
                    reason: String::from_str(&env, "Suspend threshold reached"),
                })
                .publish(&env);

                (DunningRetryScheduled {
                    subscription_id,
                    retry_at: dunning_state.next_retry_at,
                })
                .publish(&env);

                return Err(Error::TransferFailed);
            } else {
                // This should not happen, but handle gracefully
                return Err(Error::TransferFailed);
            }
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
            .remove(&DataKey::DunningState(subscription_id));

        (DunningResolved {
            subscription_id,
            admin,
        })
        .publish(&env);

        Ok(())
    }

    /// Internal function to enter dunning for a subscription.
    fn enter_dunning(env: &Env, subscription_id: u64, reason: String) {
        let config = PaymentContract::get_dunning_config(env.clone());
        let now = env.ledger().timestamp();

        let first_interval = if config.retry_intervals.len() > 0 {
            config.retry_intervals.get(0).unwrap()
        } else {
            3600u64
        };

        let dunning_state = DunningState {
            subscription_id,
            attempts: 1,
            next_retry_at: now + first_interval,
            grace_period_ends_at: now + config.grace_period,
            suspended: false,
            last_failure_reason: reason,
        };

        env.storage()
            .instance()
            .set(&DataKey::DunningState(subscription_id), &dunning_state);

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
        rate_limit.flagged = true;
        env.storage()
            .instance()
            .set(&DataKey::AddressRateLimit(address.clone()), &rate_limit);
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
        rate_limit.flagged = false;
        env.storage()
            .instance()
            .set(&DataKey::AddressRateLimit(address.clone()), &rate_limit);
        (AddressUnflagged { address }).publish(&env);
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

        // Block flagged addresses immediately.
        if rate_limit.flagged {
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

    /// Calculates the fee for a given amount and merchant (accounting for tier discount).
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
        let record: MerchantFeeRecord = env
            .storage()
            .instance()
            .get(&DataKey::MerchantFeeRecord(merchant.clone()))
            .unwrap_or(MerchantFeeRecord {
                merchant,
                total_fees_paid: 0,
                total_volume: 0,
                fee_tier: FeeTier::Standard,
            });
        PaymentContract::compute_fee_amount(
            amount,
            config.fee_bps,
            &record.fee_tier,
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

        let mut record = PaymentContract::get_or_default_merchant_fee_record(&env, merchant.clone());
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
        if PaymentContract::tier_rank(&computed_tier) > PaymentContract::tier_rank(&record.fee_tier) {
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
            &DataKey::ConditionalPayment(payment_id),
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
            .get(&DataKey::ConditionalPayment(payment_id))
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
            ConditionType::CrossContractState(_target_contract, _expected_state_hash) => {
                // Mock cross-contract state check
                // In real implementation, this would call the target contract and compare state
                // For now, return false to indicate check failed
                false
            }
        };

        // Cache the result
        conditional_payment.condition_met = condition_met;
        conditional_payment.evaluated_at = Some(current_timestamp);

        env.storage().instance().set(
            &DataKey::ConditionalPayment(payment_id),
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
            .has(&DataKey::ConditionalPayment(payment_id))
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

    pub fn get_conditional_payment(env: Env, payment_id: u64) -> Result<ConditionalPayment, Error> {
        env.storage()
            .instance()
            .get(&DataKey::ConditionalPayment(payment_id))
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
                    .get(&DataKey::CustomerMerchantVolume(customer.clone(), merchant.clone()))
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

    pub fn get_customer_monthly_volume(
        env: Env,
        customer: Address,
        month_timestamp: u64,
    ) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::CustomerMonthlyVolume(customer, month_timestamp))
            .unwrap_or(0)
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
            .set(&DataKey::PauseHistoryEntry(history_count), &entry);
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
            .set(&DataKey::PauseHistoryEntry(history_count), &entry);
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
            .set(&DataKey::PauseHistoryEntry(history_count), &entry);
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
            .set(&DataKey::PauseHistoryEntry(history_count), &entry);
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

    pub fn set_large_payment_threshold(env: Env, admin: Address, threshold: i128) -> Result<(), Error> {
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

    pub fn propose_large_payment(env: Env, merchant: Address, payment_id: u64) -> Result<(), Error> {
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
            .get::<DataKey, LargePaymentProposal>(&DataKey::LargePaymentProposal(payment_id))
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
            .set(&DataKey::LargePaymentProposal(payment_id), &proposal);

        (LargePaymentProposed {
            payment_id,
            proposer: merchant,
            required_approvals: config.required_signatures,
            expires_at,
        })
        .publish(&env);

        Ok(())
    }

    pub fn approve_large_payment(env: Env, approver: Address, payment_id: u64) -> Result<(), Error> {
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
            .get(&DataKey::LargePaymentProposal(payment_id))
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
            .set(&DataKey::LargePaymentProposal(payment_id), &proposal);

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
            .get(&DataKey::LargePaymentProposal(payment_id))
            .ok_or(Error::PaymentNotFound)?;

        if proposal.executed {
            return Err(Error::AlreadyProcessed);
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::PaymentProposalExpired);
        }

        if proposal.approvals.len() < proposal.required as usize {
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
            .set(&DataKey::LargePaymentProposal(payment_id), &proposal);

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
            .get(&DataKey::LargePaymentProposal(payment_id))
            .expect("Large payment proposal not found")
    }
}

mod test;

#[cfg(test)]
mod test_analytics;

#[cfg(test)]
mod test_trial;
