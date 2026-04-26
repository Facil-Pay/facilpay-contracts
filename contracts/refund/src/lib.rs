#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address,
    BytesN, Env, String, Vec,
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
    DefaultRefundPolicy,
    RefundPolicy(Address),
    // Policy versioning (#134)
    RefundPolicyVersion(Address, u32),
    RefundPolicyVersionCount(Address),
    // Payment contract address (#143)
    PaymentContractAddress,
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

        // Validate amount is positive
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if amount > original_payment_amount {
            return Err(Error::RefundExceedsPayment);
        }

        // Validate payment_id is valid (greater than 0)
        if payment_id == 0 {
            return Err(Error::InvalidPaymentId);
        }

        // ── Issue #143: Cross-contract verification (if payment contract is set) ──
        if let Some(_payment_contract_addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::PaymentContractAddress)
        {
            // Verify ownership via cross-contract call; skip if call fails (backward-compatible)
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

        // Circuit breaker check
        Self::check_and_update_circuit_breaker(&env, amount, original_payment_amount)?;

        // Validate against refund policy
        Self::validate_against_policy(
            &env,
            &merchant,
            amount,
            original_payment_amount,
            payment_created_at
        )?;

        // Get and increment refund counter
        let counter: u64 = env.storage().instance().get(&DataKey::RefundCounter).unwrap_or(0);
        let refund_id = counter + 1;

        // Determine initial status based on policy (merchant → global default → Requested)
        let initial_status = {
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

        // Create Refund struct
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

        // Store refund in contract storage
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        env.storage().instance().set(&DataKey::RefundCounter, &refund_id);
        Self::add_to_status_index(&env, initial_status.clone(), refund_id);

        // Index refund by merchant
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

        // Index refund by customer
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

        // Index refund by payment
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

        // Emit RefundRequested event
        (RefundRequested {
            refund_id,
            payment_id,
            merchant,
            customer,
            amount,
            token,
        }).publish(&env);

        // Emit AutoApproved event if applicable
        if initial_status == RefundStatus::Approved {
            (AutoApproved {
                refund_id,
                amount,
            }).publish(&env);
        }

        // Return the new refund ID
        Ok(refund_id)
    }

    pub fn get_refund(env: &Env, refund_id: u64) -> Result<Refund, Error> {
        // Retrieve refund from storage by ID
        env.storage().instance().get(&DataKey::Refund(refund_id)).ok_or(Error::RefundNotFound)
    }

    pub fn approve_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
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

        // Update refund status to Approved
        refund.status = RefundStatus::Approved;

        // Store updated refund back to storage
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Approved, refund_id);

        // Emit RefundApproved event
        (RefundApproved {
            refund_id,
            approved_by: admin,
            approved_at: env.ledger().timestamp(),
        }).publish(&env);

        Ok(())
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

        // Emit RefundRejected event
        (RefundRejected {
            refund_id,
            rejected_by: admin,
            rejected_at: env.ledger().timestamp(),
            rejection_reason,
        }).publish(&env);

        Ok(())
    }

    pub fn process_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.status != RefundStatus::Approved {
            return Err(Error::InvalidStatus);
        }

        Self::can_refund_payment(
            &env,
            refund.payment_id,
            refund.amount,
            refund.original_payment_amount
        )?;

        Self::remove_from_status_index(&env, RefundStatus::Approved, refund_id)?;
        refund.status = RefundStatus::Processed;

        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Processed, refund_id);

        Ok(())
    }

    pub fn register_arbitrator(env: Env, admin: Address, arbitrator: Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin no set");
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
        list.push_back(arbitrator);
        env.storage()
            .instance()
            .set(&DataKey::ArbitratorList, &list);
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

        // Distribute fees equally to voting arbitrators
        let num_voters = total_votes as i128;
        if num_voters > 0 {
            let pool_token: Address = env
                .storage()
                .instance()
                .get(&DataKey::PoolToken(case_id))
                .unwrap();
            let token_client = token::Client::new(&env, &pool_token);

            let arbitrators: Vec<Address> = env
                .storage()
                .instance()
                .get(&DataKey::ArbitratorsVoted(case_id))
                .unwrap_or_else(|| Vec::new(&env));
            let arbitrator_fee = case.fee_pool / (arbitrators.len() as i128);

            for arbitrator in arbitrators {
                token_client.transfer(&env.current_contract_address(), arbitrator, &arbitrator_fee);
            }
            ArbitrationFeesDistributed {
                case_id,
                per_arbitrator: arbitrator_fee,
            }
            .publish(&env);
        }

        ArbitrationCaseDecided { case_id, approved }.publish(&env);

        Ok(())
    }

    pub fn get_arbitration_case(env: Env, case_id: u64) -> Result<ArbitrationCase, Error> {
        env.storage()
            .instance()
            .get(&DataKey::ArbitrationCase(case_id))
            .ok_or(Error::RefundNotFound)
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
            let result = Self::approve_refund(env.clone(), admin.clone(), refund_id);
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
            let result = Self::process_refund(env.clone(), admin.clone(), refund_id);
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
        use soroban_sdk::{Symbol, IntoVal};
        let func = Symbol::new(&env, "check_payment_customer");
        let args = (payment_id, customer).into_val(&env);
        match env.try_invoke_contract::<bool, soroban_sdk::InvokeError>(&payment_contract, &func, args) {
            Ok(Ok(result)) => result,
            _ => false,
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
}

mod test;
mod test_process;
mod test_policy;

#[cfg(test)]
mod test_circuit_breaker;

#[cfg(test)]
mod test_versioning;

#[cfg(test)]
mod test_batch;

#[cfg(test)]
mod test_cross_contract;
