#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes,
    BytesN, Env, IntoVal, String, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Refund(u64),
    RefundCounter,
    RefundsByStatus(RefundStatus, u64),
    RefundStatusCount(RefundStatus),
    RefundStatusIndex(u64),
    MerchantRefunds(Address, u64),
    MerchantRefundCount(Address),
    CustomerRefunds(Address, u64),
    CustomerRefundCount(Address),
    PaymentRefunds(u64, u64),
    PaymentRefundCount(u64),
    ArbitrationCase(u64),
    ArbitrationCounter,
    ArbitratorList,
    ArbitratorsVoted(u64),        // case_id -> Vec<Address>
    ArbitratorVote(u64, Address), // case_id, arbitrator
    PoolToken(u64),
    ArbitrationFeeConfig,
    AccumulatedTreasuryFees,
    ArbitrationStakeConfig,
    ArbitrationStake(u64), // case_id -> ArbitrationStake
    ArbitratorReputation(Address), // arbitrator -> ArbitratorReputation
    ArbitratorScoreIndex(i128, u64), // score -> index for sorting
    ArbitratorScoreCount,
    DefaultRefundPolicy,
    RefundPolicy(Address),
    // Policy versioning (#134)
    RefundPolicyVersion(Address, u32),
    RefundPolicyVersionCount(Address),
    // Payment contract address (#143)
    PaymentContractAddress,
    AutoRefundTrigger(u64),
    AutoRefundTriggerCounter,
    // Batch refund limit (#135)
    BatchRefundLimit,
    // Analytics
    RefundAnalyticsKey,
    // Pause system
    PauseStateKey,
    PauseHistoryEntry(u64),
    PauseHistoryCount,
    // Circuit breaker
    CircuitBreakerConfigKey,
    CircuitBreakerStateKey,
    WindowStart,
    WindowRefundVolume,
    WindowPaymentVolume,
    // Fraud detection (#137)
    FraudSignal(Address),
    FraudConfig,
    FlaggedAddressesIndex,
    RefundRejectedAt(u64),
    Appeal(u64),
    AppealCounter,
    AppealByRefund(u64),
    AppealByCustomer(Address, u64),
    AppealByCustomerCount(Address),
}

#[derive(Clone, Debug, PartialEq)]
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
    TransferFailed = 5,
    NotApproved = 6,
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
    BatchRefundTooLarge = 20,
    PaymentContractNotSet = 27,
    PaymentOwnershipMismatch = 28,
    CircuitBreakerTripped = 29,
    InvalidFeeConfig = 30,
    InsufficientTreasuryFees = 31,
    StakeRequired = 32,
    StakeAlreadyReturned = 33,
    ArbitratorNotFound = 34,
    InvalidScoreThreshold = 35,
    AutoRefundTriggerNotFound = 36,
    DuplicateAutoRefundTrigger = 37,
    AddressFlaggedForFraud = 38,
    FraudConfigNotSet = 39,
    FraudSignalNotFound = 40,
    RefundNotRejected = 23,
    AppealWindowExpired = 24,
    AppealAlreadyFiled = 25,
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
    pub status: RefundStatus,
    pub requested_at: u64,
    pub reason: String,
    pub reason_code: RefundReasonCode,
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
pub struct RefundPolicy {
    pub merchant: Address,
    pub refund_window: u64,
    pub max_refund_percentage: u32,
    pub requires_admin_approval: bool,
    pub auto_approve_below: i128,
    pub active: bool,
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

// ── Issue #135: Batch refund result struct ─────────────────────────────────
#[derive(Clone)]
#[contracttype]
pub struct BatchRefundResult {
    pub refund_id: u64,
    pub success: bool,
    pub error_code: u32,
    pub amount_refunded: i128,
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
    pub refund_window: u64,
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
    pub refund_window: u64,
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

#[contract]
pub struct RefundContract;

#[contractimpl]
impl RefundContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Set default refund policy (30 days, 100% refund, requires approval, no auto-approve)
        let default_policy = RefundPolicy {
            merchant: admin.clone(), // Placeholder, will be overridden per merchant
            refund_window: 30 * 24 * 60 * 60, // 30 days in seconds
            max_refund_percentage: 10000, // 100%
            requires_admin_approval: true,
            auto_approve_below: 0, // No auto-approve by default
            active: true,
        };
        env.storage().instance().set(&DataKey::DefaultRefundPolicy, &default_policy);
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
        payment_created_at: u64
    ) -> Result<u64, Error> {
        // Require merchant authentication
        merchant.require_auth();

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
        env.storage().instance().get(&DataKey::Refund(refund_id)).ok_or(Error::RefundNotFound)
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
        rejection_reason: String
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

        // Store updated refund back to storage
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);
        env.storage().instance().set(&DataKey::RefundRejectedAt(refund_id), &env.ledger().timestamp());

        // Emit RefundRejected event
        (RefundRejected {
            refund_id,
            rejected_by: admin,
            rejected_at: env.ledger().timestamp(),
            rejection_reason,
        }).publish(&env);

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
        if env.storage().instance().has(&DataKey::AppealByRefund(refund_id)) {
            return Err(Error::AppealAlreadyFiled);
        }

        let rejected_at: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundRejectedAt(refund_id))
            .ok_or(Error::RefundNotRejected)?;
        let now = env.ledger().timestamp();
        if now > rejected_at.saturating_add(72 * 60 * 60) {
            return Err(Error::AppealWindowExpired);
        }

        let counter: u64 = env.storage().instance().get(&DataKey::AppealCounter).unwrap_or(0);
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
        env.storage().instance().set(&DataKey::Appeal(appeal_id), &appeal);
        env.storage().instance().set(&DataKey::AppealCounter, &appeal_id);
        env.storage().instance().set(&DataKey::AppealByRefund(refund_id), &appeal_id);

        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AppealByCustomerCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::AppealByCustomer(customer.clone(), customer_count),
            &appeal_id,
        );
        env.storage().instance().set(
            &DataKey::AppealByCustomerCount(customer.clone()),
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
            .get(&DataKey::Appeal(appeal_id))
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
            env.storage().instance().set(&DataKey::Refund(refund.id), &refund);
            Self::add_to_status_index(&env, RefundStatus::Approved, refund.id);

            Self::process_refund_internal(&env, admin.clone(), refund.id)?;
        }

        appeal.resolved = true;
        appeal.outcome = Some(uphold);
        env.storage().instance().set(&DataKey::Appeal(appeal_id), &appeal);

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
            .get(&DataKey::Appeal(appeal_id))
            .ok_or(Error::RefundNotFound)
    }

    pub fn get_appeals_by_customer(env: Env, customer: Address) -> Vec<RefundAppeal> {
        let mut appeals = Vec::new(&env);
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AppealByCustomerCount(customer.clone()))
            .unwrap_or(0);

        let mut index = 0u64;
        while index < count {
            if let Some(appeal_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::AppealByCustomer(customer.clone(), index))
            {
                if let Some(appeal) = env
                    .storage()
                    .instance()
                    .get::<_, RefundAppeal>(&DataKey::Appeal(appeal_id))
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
        if refund_bps == 0 || refund_bps > 10000 {
            return Err(Error::RefundExceedsPolicy);
        }

        let payment = Self::get_external_payment(&env, payment_id)?;
        if payment.merchant != merchant {
            return Err(Error::Unauthorized);
        }

        let trigger_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AutoRefundTriggerCounter)
            .unwrap_or(0);

        let mut trigger_id = 1u64;
        while trigger_id <= trigger_count {
            if let Some(existing) = env
                .storage()
                .instance()
                .get::<DataKey, AutoRefundTrigger>(&DataKey::AutoRefundTrigger(trigger_id))
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
            .set(&DataKey::AutoRefundTrigger(new_trigger_id), &trigger);
        env.storage()
            .instance()
            .set(&DataKey::AutoRefundTriggerCounter, &new_trigger_id);

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
            .set(&DataKey::AutoRefundTrigger(trigger_id), &trigger);

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
            .get(&DataKey::AutoRefundTrigger(trigger_id))
            .ok_or(Error::AutoRefundTriggerNotFound)
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
            .get(&DataKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));
        if list.contains(&arbitrator) {
            return Err(Error::Unauthorized);
        }
        list.push_back(arbitrator.clone());
        env.storage()
            .instance()
            .set(&DataKey::ArbitratorList, &list);

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
        env.storage()
            .instance()
            .set(&DataKey::ArbitratorReputation(arbitrator), &reputation);

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
            .get(&DataKey::ArbitrationCounter)
            .unwrap_or(0);
        let case_id = counter + 1;

        let arbitrators = env
            .storage()
            .instance()
            .get(&DataKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));
        if arbitrators.len() < 3 {
            return Err(Error::QuorumNotReached);
        }

        // Handle staking if enabled
        let stake_config: Option<ArbitrationStakeConfig> = env
            .storage()
            .instance()
            .get(&DataKey::ArbitrationStakeConfig);

        if let Some(config) = stake_config {
            if config.enabled {
                if config.amount <= 0 {
                    return Err(Error::InvalidAmount);
                }

                // Transfer stake from caller to contract
                let stake_token_client = token::Client::new(&env, &config.token);
                stake_token_client.transfer(&caller, &env.current_contract_address(), &config.amount);

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
                    .set(&DataKey::ArbitrationStake(case_id), &stake);

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
        };

        env.storage()
            .instance()
            .set(&DataKey::ArbitrationCase(case_id), &case);
        env.storage()
            .instance()
            .set(&DataKey::ArbitrationCounter, &case_id);

        RefundEscalatedToArbitration {
            refund_id,
            case_id,
            fee_pool: fee_amount,
        }
        .publish(&env);

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
            .get(&DataKey::ArbitrationCase(case_id))
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
            .has(&DataKey::ArbitratorVote(case_id, arbitrator.clone()))
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
        env.storage()
            .instance()
            .set(&DataKey::ArbitratorVote(case_id, arbitrator.clone()), &vote);

        let mut voted: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::ArbitratorsVoted(case_id))
            .unwrap_or_else(|| Vec::new(&env));
        if !voted.contains(&arbitrator) {
            voted.push_back(arbitrator.clone());
            env.storage()
                .instance()
                .set(&DataKey::ArbitratorsVoted(case_id), &voted);
        }

        if vote_for_refund {
            case.votes_for_refund += 1;
        } else {
            case.votes_against_refund += 1;
        }
        env.storage()
            .instance()
            .set(&DataKey::ArbitrationCase(case_id), &case);

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
            .get(&DataKey::ArbitrationCase(case_id))
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
            .set(&DataKey::ArbitrationCase(case_id), &case);

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
            .get(&DataKey::ArbitratorsVoted(case_id))
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
                .get(&DataKey::ArbitrationFeeConfig);

            let (arbitrator_share, treasury_share, treasury_address) = if let Some(ref config) = fee_config {
                // Calculate shares based on basis points
                let arbitrator_amount = (case.fee_pool * config.arbitrator_share_bps as i128) / 10000;
                let treasury_amount = (case.fee_pool * config.treasury_share_bps as i128) / 10000;
                (arbitrator_amount, treasury_amount, Some(config.treasury_address.clone()))
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
                    .get(&DataKey::ArbitratorVote(case_id, voter.clone()))
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
                        .get(&DataKey::AccumulatedTreasuryFees)
                        .unwrap_or(0);
                    env.storage()
                        .instance()
                        .set(&DataKey::AccumulatedTreasuryFees, &(accumulated + treasury_share));
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
            .get(&DataKey::ArbitrationStake(case_id));

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
                    .get(&DataKey::ArbitrationStakeConfig);

                if let Some(stake_cfg) = stake_config {
                    let stake_token_client = token::Client::new(&env, &stake_cfg.token);

                    // Get treasury address from fee config
                    let fee_config: Option<ArbitrationFeeConfig> = env
                        .storage()
                        .instance()
                        .get(&DataKey::ArbitrationFeeConfig);

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
                        .set(&DataKey::ArbitrationStake(case_id), &stake);
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
                .get(&DataKey::ArbitratorVote(case_id, voter.clone()))
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
                .get(&DataKey::ArbitratorReputation(voter.clone()))
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
                reputation.avg_resolution_time = (total_time + case_duration) / reputation.total_cases;
            }

            // Update last active timestamp
            reputation.last_active = current_time;

            // Store updated reputation
            env.storage()
                .instance()
                .set(&DataKey::ArbitratorReputation(voter.clone()), &reputation);

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

    /// Get the reputation information for a specific arbitrator
    pub fn get_arbitrator_reputation(env: Env, arbitrator: Address) -> Option<ArbitratorReputation> {
        env.storage()
            .instance()
            .get(&DataKey::ArbitratorReputation(arbitrator))
    }

    /// Get the top arbitrators sorted by score (highest first)
    /// Returns up to `limit` arbitrators
    pub fn get_top_arbitrators(env: Env, limit: u32) -> Vec<ArbitratorReputation> {
        let mut results = Vec::new(&env);
        
        // Get all arbitrators from the arbitrator list
        let arbitrators: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::ArbitratorList)
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
                .get::<DataKey, ArbitratorReputation>(&DataKey::ArbitratorReputation(arbitrator.clone()))
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
            .get(&DataKey::ArbitratorList)
            .unwrap_or(Vec::new(&env));

        let mut removed_count: u32 = 0;
        let mut new_arbitrators = Vec::new(&env);

        for arbitrator in arbitrators.iter() {
            let reputation: Option<ArbitratorReputation> = env
                .storage()
                .instance()
                .get(&DataKey::ArbitratorReputation(arbitrator.clone()));

            let should_remove = if let Some(rep) = reputation {
                rep.score < min_score
            } else {
                false
            };

            if should_remove {
                // Remove reputation data
                env.storage()
                    .instance()
                    .remove(&DataKey::ArbitratorReputation(arbitrator.clone()));

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
            .set(&DataKey::ArbitratorList, &new_arbitrators);

        Ok(removed_count)
    }

    pub fn get_arbitration_case(env: Env, case_id: u64) -> Result<ArbitrationCase, Error> {
        env.storage()
            .instance()
            .get(&DataKey::ArbitrationCase(case_id))
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
            .set(&DataKey::ArbitrationFeeConfig, &config);

        Ok(())
    }

    /// Get the current arbitration fee configuration
    pub fn get_arbitration_fee_config(env: Env) -> Option<ArbitrationFeeConfig> {
        env.storage()
            .instance()
            .get(&DataKey::ArbitrationFeeConfig)
    }

    /// Get the accumulated treasury fees from arbitration cases
    pub fn get_accumulated_arbitration_fees(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::AccumulatedTreasuryFees)
            .unwrap_or(0)
    }

    /// Withdraw accumulated treasury fees
    /// Requires admin authorization
    /// Returns the amount withdrawn
    pub fn withdraw_treasury_fees(env: Env, admin: Address) -> Result<i128, Error> {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let accumulated: i128 = env
            .storage()
            .instance()
            .get(&DataKey::AccumulatedTreasuryFees)
            .unwrap_or(0);

        if accumulated <= 0 {
            return Err(Error::InsufficientTreasuryFees);
        }

        // Reset accumulated fees
        env.storage()
            .instance()
            .set(&DataKey::AccumulatedTreasuryFees, &0i128);

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
            .set(&DataKey::ArbitrationStakeConfig, &config);

        Ok(())
    }

    /// Get the current arbitration stake configuration
    pub fn get_arbitration_stake_config(env: Env) -> Option<ArbitrationStakeConfig> {
        env.storage()
            .instance()
            .get(&DataKey::ArbitrationStakeConfig)
    }

    /// Get the stake information for a specific arbitration case
    pub fn get_arbitration_stake(env: Env, case_id: u64) -> Option<ArbitrationStake> {
        env.storage()
            .instance()
            .get(&DataKey::ArbitrationStake(case_id))
    }

    pub fn get_refunds_by_status(
        env: &Env,
        status: RefundStatus,
        limit: u64,
        offset: u64
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(env);
        let total = Self::get_refund_count_by_status(env, status.clone());

        if limit == 0 || offset >= total {
            return results;
        }

        let end = core::cmp::min(total, offset.saturating_add(limit));
        let mut index = offset;
        while index < end {
            if
                let Some(refund_id) = env
                    .storage()
                    .instance()
                    .get::<_, u64>(&DataKey::RefundsByStatus(status.clone(), index))
            {
                if
                    let Some(refund) = env
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

    pub fn get_refunds_by_reason_code(
        env: &Env,
        code: RefundReasonCode,
        limit: u64,
        offset: u64
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
            if let Some(refund) = env.storage().instance().get::<_, Refund>(&DataKey::Refund(id)) {
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
            if let Some(refund) = env.storage().instance().get::<_, Refund>(&DataKey::Refund(id)) {
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
        env.storage().instance().get(&DataKey::RefundStatusCount(status)).unwrap_or(0)
    }

    pub fn get_total_refunded_amount(env: &Env, payment_id: u64) -> i128 {
        let total_refunds: u64 = env.storage().instance().get(&DataKey::RefundCounter).unwrap_or(0);
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
        original_amount: i128
    ) -> Result<bool, Error> {
        let total_refunded = Self::get_total_refunded_amount(env, payment_id);
        if requested_amount.saturating_add(total_refunded) > original_amount {
            return Err(Error::TotalRefundsExceedPayment);
        }

        Ok(true)
    }

    pub fn set_refund_policy(
        env: Env,
        merchant: Address,
        refund_window: u64,
        max_refund_percentage: u32,
        requires_admin_approval: bool,
        auto_approve_below: i128
    ) -> Result<(), Error> {
        // Require merchant authentication
        merchant.require_auth();

        // Validate max_refund_percentage is within bounds (0-10000 basis points)
        if max_refund_percentage > 10000 {
            return Err(Error::RefundExceedsPolicy);
        }

        let policy = RefundPolicy {
            merchant: merchant.clone(),
            refund_window,
            max_refund_percentage,
            requires_admin_approval,
            auto_approve_below,
            active: true,
        };

        env.storage().instance().set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // ── Issue #134: version the policy ──────────────────────────────────
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
            created_by: merchant.clone(),
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
            refund_window,
        }).publish(&env);

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
            .get(&DataKey::RefundPolicyVersion(merchant, version))
    }

    pub fn get_refund_policy_at_time(
        env: Env,
        merchant: Address,
        timestamp: u64,
    ) -> Option<RefundPolicyVersion> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyVersionCount(merchant.clone()))
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
                .get::<DataKey, RefundPolicyVersion>(&DataKey::RefundPolicyVersion(merchant.clone(), v))
            {
                if pv.created_at <= timestamp {
                    result = Some(pv);
                }
            }
        }
        result
    }

    pub fn get_refund_policy_history(
        env: Env,
        merchant: Address,
    ) -> Vec<RefundPolicyVersion> {
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicyVersionCount(merchant.clone()))
            .unwrap_or(0);
        let mut history = Vec::new(&env);
        for v in 1..=count {
            if let Some(pv) = env
                .storage()
                .instance()
                .get::<DataKey, RefundPolicyVersion>(&DataKey::RefundPolicyVersion(merchant.clone(), v))
            {
                history.push_back(pv);
            }
        }
        history
    }

    pub fn get_refund_policy(env: &Env, merchant: Address) -> Option<RefundPolicy> {
        env.storage().instance().get(&DataKey::RefundPolicy(merchant))
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
            refund_window: policy.refund_window,
        })
        .publish(&env);
        Ok(())
    }

    /// Get the global default refund policy (returns None if not set).
    pub fn get_default_refund_policy(env: Env) -> Option<RefundPolicy> {
        env.storage()
            .instance()
            .get(&DataKey::DefaultRefundPolicy)
    }

    /// Internal helper used by request_refund / validate_against_policy.
    fn get_default_refund_policy_inner(env: &Env) -> Option<RefundPolicy> {
        env.storage()
            .instance()
            .get(&DataKey::DefaultRefundPolicy)
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
        env.storage().instance().set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // Emit RefundPolicyDeactivated event
        (RefundPolicyDeactivated { merchant }).publish(&env);

        Ok(())
    }

    pub fn admin_override_policy(
        env: Env,
        admin: Address,
        refund_id: u64,
        reason: String
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
        let _refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Emit PolicyOverrideApplied event
        (PolicyOverrideApplied {
            refund_id,
            admin,
            reason,
        }).publish(&env);

        Ok(())
    }

    fn validate_against_policy(
        env: &Env,
        merchant: &Address,
        amount: i128,
        original_amount: i128,
        payment_created_at: u64
    ) -> Result<(), Error> {
        // Fallback chain: merchant policy → global default → PolicyNotFound
        let policy: RefundPolicy = Self::get_refund_policy(env, merchant.clone())
            .or_else(|| Self::get_default_refund_policy_inner(env))
            .ok_or(Error::PolicyNotFound)?;

        // Check if policy is active
        if !policy.active {
            return Err(Error::PolicyInactive);
        }

        // Check refund window
        let current_time = env.ledger().timestamp();
        if current_time > payment_created_at.saturating_add(policy.refund_window) {
            return Err(Error::RefundWindowExpired);
        }

        // Check refund percentage using overflow-safe math
        let refund_percentage_bps = amount
            .checked_mul(10000)
            .unwrap_or(i128::MAX)
            .checked_div(original_amount)
            .unwrap_or(u32::MAX as i128) as u32;

        if refund_percentage_bps > policy.max_refund_percentage {
            return Err(Error::RefundExceedsPolicy);
        }

        Ok(())
    }

    fn add_to_status_index(env: &Env, status: RefundStatus, refund_id: u64) {
        let count = Self::get_refund_count_by_status(env, status.clone());
        env.storage().instance().set(&DataKey::RefundsByStatus(status.clone(), count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusCount(status.clone()), &(count + 1));
        env.storage().instance().set(&DataKey::RefundStatusIndex(refund_id), &count);
    }

    fn remove_from_status_index(
        env: &Env,
        status: RefundStatus,
        refund_id: u64
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

        env.storage().instance().remove(&DataKey::RefundsByStatus(status.clone(), last_index));
        env.storage().instance().remove(&DataKey::RefundStatusIndex(refund_id));
        env.storage().instance().set(&DataKey::RefundStatusCount(status), &last_index);

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
        env.storage().instance().set(&DataKey::BatchRefundLimit, &limit);
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

    pub fn verify_payment_ownership(
        env: Env,
        payment_id: u64,
        customer: Address,
    ) -> bool {
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
        match env.try_invoke_contract::<bool, soroban_sdk::InvokeError>(&payment_contract, &func, args) {
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
            let owned = Self::verify_payment_ownership(
                env.clone(),
                payment_id,
                customer.clone(),
            );
            if !owned {
                return Err(Error::PaymentOwnershipMismatch);
            }
        }

        Self::can_refund_payment(&env, payment_id, amount, original_payment_amount)?;
        Self::check_and_update_circuit_breaker(&env, amount, original_payment_amount)?;
        
        // Check for fraud signals (#137)
        if let Some(fraud_signal) = Self::check_fraud_signals(env.clone(), customer.clone()) {
            if !fraud_signal.reviewed {
                return Err(Error::AddressFlaggedForFraud);
            }
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

        let counter: u64 = env.storage().instance().get(&DataKey::RefundCounter).unwrap_or(0);
        let refund_id = counter + 1;

        let initial_status = if force_approved {
            RefundStatus::Approved
        } else {
            let policy_opt = Self::get_refund_policy(&env, merchant.clone())
                .or_else(|| Self::get_default_refund_policy_inner(&env));
            if let Some(policy) = policy_opt {
                if !policy.requires_admin_approval && amount <= policy.auto_approve_below {
                    RefundStatus::Approved
                } else {
                    RefundStatus::Requested
                }
            } else {
                RefundStatus::Requested
            }
        };

        let refund = Refund {
            id: refund_id,
            payment_id,
            merchant: merchant.clone(),
            customer: customer.clone(),
            amount,
            original_payment_amount,
            token: token.clone(),
            status: initial_status.clone(),
            requested_at: env.ledger().timestamp(),
            reason,
            reason_code,
        };

        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        env.storage().instance().set(&DataKey::RefundCounter, &refund_id);
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

        (RefundRequested {
            refund_id,
            payment_id,
            merchant,
            customer,
            amount,
            token,
        })
        .publish(&env);

        if initial_status == RefundStatus::Approved {
            (AutoApproved { refund_id, amount }).publish(&env);
        }

        Ok(refund_id)
    }

    fn approve_refund_internal(env: &Env, approved_by: Address, refund_id: u64) -> Result<(), Error> {
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.status != RefundStatus::Requested {
            return Err(Error::InvalidStatus);
        }

        Self::remove_from_status_index(env, RefundStatus::Requested, refund_id)?;
        refund.status = RefundStatus::Approved;
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(env, RefundStatus::Approved, refund_id);

        (RefundApproved {
            refund_id,
            approved_by,
            approved_at: env.ledger().timestamp(),
        })
        .publish(env);

        Ok(())
    }

    fn process_refund_internal(env: &Env, processed_by: Address, refund_id: u64) -> Result<(), Error> {
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

        Self::remove_from_status_index(env, RefundStatus::Approved, refund_id)?;
        refund.status = RefundStatus::Processed;
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
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
        env.storage().instance()
            .get(&DataKey::RefundAnalyticsKey)
            .unwrap_or(RefundAnalytics {
                total_refunds_requested: 0, total_refunds_approved: 0,
                total_refunds_rejected: 0, total_refunds_processed: 0,
                total_refund_volume: 0, approval_rate_bps: 0,
            })
    }

    // ── PAUSE FUNCTIONS ────────────────────────────────────────────────────

    pub fn pause_contract(env: Env, admin: Address, reason: String) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env.storage().instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let now = env.ledger().timestamp();
        let pause_state = if let Some(mut state) = env.storage().instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey) {
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
        env.storage().instance().set(&DataKey::PauseStateKey, &pause_state);
        let history_count: u64 = env.storage().instance()
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
        env.storage().instance().set(&DataKey::PauseHistoryEntry(history_count), &entry);
        env.storage().instance().set(&DataKey::PauseHistoryCount, &(history_count + 1));
        (ContractPausedEvent { paused_by: admin, reason, paused_at: now }).publish(&env);
        Ok(())
    }

    pub fn unpause_contract(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env.storage().instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        if let Some(mut state) = env.storage().instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey) {
            state.globally_paused = false;
            env.storage().instance().set(&DataKey::PauseStateKey, &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env.storage().instance()
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
        env.storage().instance().set(&DataKey::PauseHistoryEntry(history_count), &entry);
        env.storage().instance().set(&DataKey::PauseHistoryCount, &(history_count + 1));
        (ContractUnpausedEvent { unpaused_by: admin, unpaused_at: now }).publish(&env);
        Ok(())
    }

    pub fn pause_function(
        env: Env,
        admin: Address,
        function_name: String,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env.storage().instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let now = env.ledger().timestamp();
        let mut pause_state = if let Some(state) = env.storage().instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey) {
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
            pause_state.paused_functions.push_back(function_name.clone());
        }
        env.storage().instance().set(&DataKey::PauseStateKey, &pause_state);
        let history_count: u64 = env.storage().instance()
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
        env.storage().instance().set(&DataKey::PauseHistoryEntry(history_count), &entry);
        env.storage().instance().set(&DataKey::PauseHistoryCount, &(history_count + 1));
        (FunctionPausedEvent { function_name, paused_by: admin, reason }).publish(&env);
        Ok(())
    }

    pub fn unpause_function(
        env: Env,
        admin: Address,
        function_name: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env.storage().instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        if let Some(mut state) = env.storage().instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey) {
            let mut new_paused = Vec::new(&env);
            for fn_name in state.paused_functions.iter() {
                if fn_name != function_name {
                    new_paused.push_back(fn_name);
                }
            }
            state.paused_functions = new_paused;
            env.storage().instance().set(&DataKey::PauseStateKey, &state);
        }
        let now = env.ledger().timestamp();
        let history_count: u64 = env.storage().instance()
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
        env.storage().instance().set(&DataKey::PauseHistoryEntry(history_count), &entry);
        env.storage().instance().set(&DataKey::PauseHistoryCount, &(history_count + 1));
        (FunctionUnpausedEvent { function_name, unpaused_by: admin }).publish(&env);
        Ok(())
    }

    pub fn get_pause_state(env: Env) -> PauseState {
        env.storage().instance()
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
        if let Some(state) = env.storage().instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey) {
            if state.globally_paused { return true; }
            for fn_name in state.paused_functions.iter() {
                if fn_name == function_name { return true; }
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
        if let Some(state) = env.storage().instance()
            .get::<DataKey, PauseState>(&DataKey::PauseStateKey) {
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
            .set(&DataKey::CircuitBreakerConfigKey, &config);
        Ok(())
    }

    pub fn get_circuit_breaker_state(env: Env) -> CircuitBreakerState {
        env.storage()
            .instance()
            .get(&DataKey::CircuitBreakerStateKey)
            .unwrap_or(CircuitBreakerState {
                tripped: false,
                tripped_at: None,
                trip_count: 0,
                last_refund_rate_bps: 0,
                resets_at: None,
            })
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
            .set(&DataKey::CircuitBreakerStateKey, &state);
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
            .get(&DataKey::CircuitBreakerConfigKey)
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

    fn check_and_update_circuit_breaker(
        env: &Env,
        refund_amount: i128,
        payment_amount: i128,
    ) -> Result<(), Error> {
        let config: CircuitBreakerConfig = match env
            .storage()
            .instance()
            .get(&DataKey::CircuitBreakerConfigKey)
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
                        .set(&DataKey::CircuitBreakerStateKey, &state);
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
            .get(&DataKey::WindowStart)
            .unwrap_or(0);

        if now >= window_start + config.measurement_window_seconds || window_start == 0 {
            env.storage().instance().set(&DataKey::WindowStart, &now);
            env.storage().instance().set(&DataKey::WindowRefundVolume, &0i128);
            env.storage().instance().set(&DataKey::WindowPaymentVolume, &0i128);
        }

        let new_refund_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::WindowRefundVolume)
            .unwrap_or(0)
            + refund_amount;

        let new_payment_vol: i128 = env
            .storage()
            .instance()
            .get(&DataKey::WindowPaymentVolume)
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
                .set(&DataKey::CircuitBreakerStateKey, &state);
            CircuitBreakerTrippedEvent {
                refund_rate_bps: rate_bps,
                tripped_at: now,
            }
            .publish(env);
            return Err(Error::CircuitBreakerTripped);
        }

        env.storage()
            .instance()
            .set(&DataKey::WindowRefundVolume, &new_refund_vol);
        env.storage()
            .instance()
            .set(&DataKey::WindowPaymentVolume, &new_payment_vol);

        Ok(())
    }

    // Fraud detection functions (#137)
    pub fn check_fraud_signals(env: Env, address: Address) -> Option<FraudSignal> {
        // Get fraud config
        let config: FraudConfig = env
            .storage()
            .instance()
            .get(&DataKey::FraudConfig)
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
        let refund_rate_bps = if total_payments > 0 {
            (total_refunds * 10000) / total_payments
        } else {
            0
        };

        // Check if refund rate exceeds threshold
        if refund_rate_bps > config.max_refund_rate_bps {
            let existing_signal: Option<FraudSignal> = env
                .storage()
                .instance()
                .get(&DataKey::FraudSignal(address.clone()));

            match existing_signal {
                Some(mut signal) if !signal.reviewed => {
                    // Update existing signal
                    signal.refund_rate_bps = refund_rate_bps;
                    signal.total_payments = total_payments;
                    signal.total_refunds = total_refunds;
                    env.storage()
                        .instance()
                        .set(&DataKey::FraudSignal(address), &signal);
                    Some(signal)
                }
                None => {
                    // Create new fraud signal
                    let signal = FraudSignal {
                        address: address.clone(),
                        refund_rate_bps,
                        total_payments,
                        total_refunds,
                        flagged_at: env.ledger().timestamp(),
                        reviewed: false,
                    };
                    env.storage()
                        .instance()
                        .set(&DataKey::FraudSignal(address), &signal);

                    // Add to flagged addresses index
                    let mut flagged_count: u64 = env
                        .storage()
                        .instance()
                        .get(&DataKey::FlaggedAddressesIndex)
                        .unwrap_or(0);
                    env.storage()
                        .instance()
                        .set(&DataKey::FlaggedAddressesIndex, &flagged_count + 1);

                    // Emit fraud signal raised event
                    (FraudSignalRaised {
                        address,
                        refund_rate_bps,
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
            .get(&DataKey::FraudSignal(address.clone()))
            .ok_or(Error::FraudSignalNotFound)?;

        signal.reviewed = true;
        env.storage()
            .instance()
            .set(&DataKey::FraudSignal(address), &signal);

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
            .set(&DataKey::FraudConfig, &config);

        Ok(())
    }

    // Helper functions for fraud detection
    fn get_customer_payment_count(env: &Env, address: &Address) -> u64 {
        // This would typically call the payment contract to get actual statistics
        // For now, return a placeholder value
        10
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
}

mod test;
mod test_process;
mod test_policy;

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
