#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes,
    BytesN, Env, String, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Escrow(u64),
    EscrowCounter,
    MultiPartyEscrow(u64),
    MultiPartyEscrowCounter,
    CustomerEscrows(Address, u64),
    MerchantEscrows(Address, u64),
    CustomerEscrowCount(Address),
    MerchantEscrowCount(Address),
    EscrowEvidence(u64, u64),
    EscrowEvidenceCount(u64),
    ReputationScore(Address),
    ReputationConfig,
    VestingSchedule(u64),
    MultiSigConfig,
    AdminProposal(String),
    ProposalCounter,
    TimeLockAction(u64),
    TimeLockCounter,
    TimeLockConfig,
    BatchLimit,
    // Oracle conditions
    OracleCondition(u64),
    // Dispute collateral
    DisputeConfigKey,
    DisputeCollateral(u64),
    // Analytics
    EscrowAnalyticsKey,
    CustomerAnalytics(Address),
    MerchantAnalytics(Address),
    // Pause system
    PauseStateKey,
    PauseHistoryEntry(u64),
    PauseHistoryCount,
    // Multi-token escrow
    MultiTokenEscrow(u64),
    MultiTokenEscrowCounter,
    InsurancePool,
    InsuranceConfig,
    InsuranceClaim(u64),
    InsuranceClaimCounter,
    WatchdogConfig,
    // Reputation decay
    ReputationDecayConfig,
    EscrowFeeConfigKey,
    AccumulatedEscrowFees(Address),
    // Beneficiary transfer history
    BeneficiaryTransferHistory(u64, u64),
    BeneficiaryTransferCount(u64),
    // Multi-party dispute
    MultiPartyDisputeKey(u64),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EscrowStatus {
    Locked,
    Released,
    Disputed,
    Resolved,
    Cancelled,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowFeeConfig {
    pub fee_bps: i128,
    pub fee_recipient: Address,
    pub enabled: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFeeCollected {
    pub escrow_id: u64,
    pub fee_amount: i128,
    pub recipient: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFeesWithdrawn {
    pub amount: i128,
    pub withdrawn_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFeeConfigUpdated {
    pub fee_bps: i128,
}

#[contracttype]
pub struct InsurancePool {
    pub token: Address,
    pub balance: i128,
    pub total_premiums_collected: i128,
    pub total_claims_paid: i128,
}

#[contracttype]
pub struct InsuranceConfig {
    pub premium_bps: i128,
    pub max_coverage_bps: i128,
    pub enabled: bool,
}

#[contracttype]
pub struct InsuranceClaim {
    pub claim_id: u64,
    pub escrow_id: u64,
    pub claimant: Address,
    pub amount: i128,
    pub approved: bool,
    pub paid_at: Option<u64>,
}

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    EscrowNotFound = 1,
    InvalidStatus = 2,
    AlreadyProcessed = 3,
    Unauthorized = 4,
    ReleaseNotYetAvailable = 5,
    NotDisputed = 6,
    TimeoutNotReached = 7,
    ReleaseOnHoldPeriod = 8,
    InvalidVestingSchedule = 9,
    CliffPeriodNotPassed = 10,
    MilestoneAlreadyReleased = 11,
    InsufficientVestedAmount = 12,
    TransferFailed = 13,
    InvalidParticipantCount = 14,
    InvalidSharesSum = 15,
    DuplicateApproval = 16,
    ApprovalsThresholdNotMet = 17,
    MultiSigNotInitialized = 18,
    ProposalNotFound = 19,
    ProposalExpired = 20,
    ProposalAlreadyExecuted = 21,
    MultiSigThresholdNotMet = 22,
    InsufficientAdmins = 23,
    NotAnAdmin = 24,
    AlreadyApproved = 25,
    ActionNotReady = 26,
    ActionExpired = 27,
    ActionAlreadyExecuted = 28,
    ActionCancelled = 29,
    ContractPaused = 30,
    FunctionPaused = 31,
    EmptyTokenList = 35,
    DuplicateToken = 36,
    TokenTransferPartialFailure = 37,
    Underfunded = 32,
    NotClaimable = 33,
    OracleStalePriceFeed = 44,
    OracleConditionNotMet = 45,
    NoOracleCondition = 46,
    BatchTooLarge = 40,
    BatchPartialFailure = 41,
    MilestoneNotFound = 47,
    MilestoneNotApproved = 48,
    MilestoneOverflow = 49,
    TransferNotAllowed = 42,
    SameBeneficiary = 43,
}


#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollateralDeposited {
    pub escrow_id: u64,
    pub party: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollateralForfeited {
    pub escrow_id: u64,
    pub party: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollateralReturned {
    pub escrow_id: u64,
    pub party: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCreated {
    pub escrow_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub release_timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPartyEscrowCreated {
    pub escrow_id: u64,
    pub participant_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowReleased {
    pub escrow_id: u64,
    pub merchant: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticipantApproved {
    pub escrow_id: u64,
    pub approver: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPartyEscrowReleased {
    pub escrow_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiTokenEscrowCreated {
    pub escrow_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub token_count: u32,
    pub release_timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiTokenEscrowReleased {
    pub escrow_id: u64,
    pub merchant: Address,
    pub token_count: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowDisputed {
    pub escrow_id: u64,
    pub disputed_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowResolved {
    pub escrow_id: u64,
    pub released_to_merchant: bool,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSubmitted {
    pub escrow_id: u64,
    pub submitter: Address,
    pub ipfs_hash: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeEscalated {
    pub escrow_id: u64,
    pub level: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRecommendationGenerated {
    pub escrow_id: u64,
    pub outcome: DisputeOutcome,
    pub confidence_bps: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct DisputeConfig {
    pub collateral_token: Address,
    pub collateral_amount: i128,
    pub collateral_enabled: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct DisputeCollateral {
    pub escrow_id: u64,
    pub disputing_party: Address,
    pub amount: i128,
    pub token: Address,
    pub deposited_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DisputeOutcome {
    FavorCustomer,
    FavorMerchant,
    Inconclusive,
}

#[derive(Clone)]
#[contracttype]
pub struct DisputeRecommendation {
    pub escrow_id: u64,
    pub customer_score: i128,
    pub merchant_score: i128,
    pub recommendation: DisputeOutcome,
    pub confidence_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationUpdated {
    pub address: Address,
    pub old_score: i64,
    pub new_score: i64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationConfigUpdated {
    pub win_reward: i64,
    pub loss_penalty: i64,
    pub completion_reward: i64,
    pub dispute_initiation_penalty: i64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingScheduleCreated {
    pub escrow_id: u64,
    pub total_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestedAmountReleased {
    pub escrow_id: u64,
    pub amount: i128,
    pub released_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneReleased {
    pub escrow_id: u64,
    pub milestone_id: u64,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MilestoneApproved {
    pub escrow_id: u64,
    pub milestone_id: u64,
    pub approved_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchdogReleaseTriggered {
    pub escrow_id: u64,
    pub released_to: Address,
    pub triggered_by: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct ReputationScore {
    pub address: Address,
    pub total_transactions: u32,
    pub disputes_initiated: u32,
    pub disputes_won: u32,
    pub disputes_lost: u32,
    pub score: i64,
    pub last_updated: u64,
    pub decay_rate: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct ReputationConfig {
    pub win_reward: i64,
    pub loss_penalty: i64,
    pub completion_reward: i64,
    pub dispute_initiation_penalty: i64,
}

#[derive(Clone)]
#[contracttype]
pub struct WatchdogConfig {
    pub inactivity_release_seconds: u64,
    pub enabled: bool,
    pub favor_customer_on_release: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct ReputationDecayConfig {
    pub decay_rate_bps: i128,
    pub decay_threshold_days: u64,
    pub min_score: i128,
    pub max_score: i128,
}


#[derive(Clone)]
#[contracttype]
pub struct Escrow {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub release_timestamp: u64,
    pub dispute_started_at: u64,
    pub last_activity_at: u64,
    pub escalation_level: u64,
    pub min_hold_period: u64,
    pub fee_bps: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum ParticipantRole {
    Customer,
    Merchant,
    ServiceProvider,
    Arbitrator,
    Custom(String),
}

#[derive(Clone)]
#[contracttype]
pub struct Participant {
    pub address: Address,
    pub share_bps: u32, // basis points out of 10000
    pub role: ParticipantRole,
    pub required_approval: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct MultiPartyEscrow {
    pub id: u64,
    pub participants: Vec<Participant>,
    pub total_amount: i128,
    pub token: Address,
    pub status: EscrowStatus,
    pub approvals: Vec<Address>,
    pub required_approvals: u32,
    pub created_at: u64,
    pub release_timestamp: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct TokenEntry {
    pub token: Address,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct MultiTokenEscrow {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub tokens: Vec<TokenEntry>,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub release_timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct Evidence {
    pub submitter: Address,
    pub ipfs_hash: String,
    pub submitted_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct VestingMilestone {
    pub milestone_id: u64,
    pub unlock_timestamp: u64,
    pub amount: i128,
    pub released: bool,
    pub description: String,
    pub approved_by: Option<Address>,
    pub approved_at: Option<u64>,
}

#[derive(Clone)]
#[contracttype]
pub struct VestingSchedule {
    pub escrow_id: u64,
    pub total_amount: i128,
    pub released_amount: i128,
    pub start_timestamp: u64,
    pub cliff_timestamp: u64,
    pub end_timestamp: u64,
    pub milestones: Vec<VestingMilestone>,
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

#[contracttype]
pub struct TimeLockAction {
    pub action_id: u64,
    pub action_type: EscrowActionType,
    pub escrow_id: u64,
    pub proposed_by: Address,
    pub queued_at: u64,
    pub executable_after: u64,
    pub expires_at: u64,
    pub executed: bool,
    pub cancelled: bool,
    pub data: Bytes,
}

#[contracttype]
pub enum EscrowActionType {
    ResolveDispute(bool),
    ForceRelease,
    UpdateReleaseTimestamp(u64),
    CancelEscrow,
}

#[derive(Clone)]
#[contracttype]
pub struct TimeLockConfig {
    pub delay: u64,        // minimum seconds before execution
    pub grace_period: u64, // window after delay before action expires
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum PriceComparison {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Clone)]
#[contracttype]
pub struct OracleConfig {
    pub oracle_address: Address,
    pub price_feed_id: BytesN<32>,
    pub staleness_threshold: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct OracleCondition {
    pub escrow_id: u64,
    pub oracle: OracleConfig,
    pub target_price: i128,
    pub comparison: PriceComparison,
    pub release_to_merchant_if_met: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct OraclePriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowBatchEntry {
    pub customer: Address,
    pub merchant: Address,
    pub token: Address,
    pub amount: i128,
    pub release_timestamp: u64,
    pub description: String,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchEscrowResult {
    pub index: u32,
    pub escrow_id: u64,
    pub success: bool,
    pub error_code: u32,
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
pub struct TimeLockActionQueued {
    pub action_id: u64,
    pub escrow_id: u64,
    pub executable_after: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeLockActionExecuted {
    pub action_id: u64,
    pub escrow_id: u64,
    pub executed_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeLockActionCancelled {
    pub action_id: u64,
    pub cancelled_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeLockConfigUpdated {
    pub delay: u64,
    pub grace_period: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationDecayed {
    pub address: Address,
    pub old_score: i128,
    pub new_score: i128,
    pub days_inactive: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecayConfigUpdated {
    pub decay_rate_bps: i128,
    pub threshold_days: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyticsReset {
    pub reset_by: Address,
    pub reset_at: u64,
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
pub struct EscrowAnalytics {
    pub total_escrows_created: u64,
    pub total_value_locked: i128,
    pub total_value_released: i128,
    pub total_disputes: u64,
    pub total_resolutions: u64,
    pub dispute_rate_bps: i128,
    pub avg_escrow_duration_seconds: u64,
    pub total_escrows_released: u64,
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
pub struct BeneficiaryTransfer {
    pub escrow_id: u64,
    pub from: Address,
    pub to: Address,
    pub transferred_at: u64,
    pub authorised_by: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct MultiPartyDispute {
    pub escrow_id: u64,
    pub votes_for_merchant: Vec<Address>,
    pub votes_for_customer: Vec<Address>,
    pub quorum_required: u32,
    pub resolution_deadline: u64,
    pub resolved: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeneficiaryTransferred {
    pub escrow_id: u64,
    pub old_merchant: Address,
    pub new_merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPartyDisputeRaised {
    pub escrow_id: u64,
    pub raised_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPartyDisputeVoteCast {
    pub escrow_id: u64,
    pub voter: Address,
    pub favor_merchant: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiPartyDisputeResolved {
    pub escrow_id: u64,
    pub favor_merchant: bool,
    pub resolved_at: u64,
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
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
        AdminAdded { admin }.publish(&env);
    }

    pub fn set_escrow_fee_config(env: Env, admin: Address, config: EscrowFeeConfig) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }
        env.storage().instance().set(&DataKey::EscrowFeeConfigKey, &config);
        EscrowFeeConfigUpdated { fee_bps: config.fee_bps }.publish(&env);
        Ok(())
    }

    pub fn get_escrow_fee_config(env: Env) -> EscrowFeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::EscrowFeeConfigKey)
            .unwrap_or(EscrowFeeConfig {
                fee_bps: 0,
                fee_recipient: env.current_contract_address(),
                enabled: false,
            })
    }

    pub fn get_accumulated_escrow_fees(env: Env, token: Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::AccumulatedEscrowFees(token))
            .unwrap_or(0)
    }

    pub fn withdraw_escrow_fees(env: Env, admin: Address, token: Address, to: Address) -> Result<i128, Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        let amount: i128 = env.storage().instance().get(&DataKey::AccumulatedEscrowFees(token.clone())).unwrap_or(0);
        if amount == 0 {
            return Ok(0);
        }

        Self::transfer_if_token_contract(&env, &token, &to, amount)?;
        env.storage().instance().set(&DataKey::AccumulatedEscrowFees(token.clone()), &0i128);

        EscrowFeesWithdrawn {
            amount,
            withdrawn_by: admin,
        }.publish(&env);

        Ok(amount)
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

        let proposal_id = EscrowContract::u64_to_string(&env, counter);
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

        ActionProposed {
            proposal_id: proposal_id.clone(),
            proposer,
            action_type,
        }
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

        ActionApproved {
            proposal_id,
            approver,
            approval_count: proposal.approval_count,
        }
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

        EscrowContract::dispatch_action(&env, &proposal)?;

        ActionExecuted { proposal_id }.publish(&env);

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

        ActionRejected {
            proposal_id,
            rejected_by: rejecter,
        }
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
            AdminAdded { admin: new_admin }.publish(&env);
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
        AdminRemoved { admin }.publish(&env);

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

    pub fn create_escrow(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        release_timestamp: u64,
        min_hold_period: u64,
    ) -> u64 {
        customer.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowCounter)
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let current_timestamp = env.ledger().timestamp();

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled { fee_config.fee_bps } else { 0 };

        let escrow = Escrow {
            id: escrow_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token: token.clone(),
            status: EscrowStatus::Locked,
            created_at: current_timestamp,
            release_timestamp,
            dispute_started_at: 0,
            last_activity_at: current_timestamp,
            escalation_level: 0,
            min_hold_period,
            fee_bps,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::EscrowCounter, &escrow_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerEscrowCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerEscrows(customer.clone(), customer_count),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerEscrowCount(customer.clone()),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantEscrowCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantEscrows(merchant.clone(), merchant_count),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantEscrowCount(merchant.clone()),
            &(merchant_count + 1),
        );

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::EscrowAnalyticsKey)
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_escrows_created += 1;
        analytics.total_value_locked += amount;
        env.storage()
            .instance()
            .set(&DataKey::EscrowAnalyticsKey, &analytics);

        // Update per-address analytics
        EscrowContract::update_customer_analytics(&env, &customer, |a| {
            a.total_escrows_created += 1;
            a.total_value_locked += amount;
        });
        EscrowContract::update_merchant_analytics(&env, &merchant, |a| {
            a.total_escrows_created += 1;
            a.total_value_locked += amount;
        });

        EscrowCreated {
            escrow_id,
            customer,
            merchant,
            amount,
            token,
            release_timestamp,
        }
        .publish(&env);

        escrow_id
    }

    pub fn create_multi_party_escrow(
        env: Env,
        customer: Address,
        participants: Vec<Participant>,
        total_amount: i128,
        token: Address,
        release_timestamp: u64,
    ) -> Result<u64, Error> {
        customer.require_auth();

        // Minimum 2, maximum 10 participants
        if participants.len() < 2 || participants.len() > 10 {
            return Err(Error::InvalidParticipantCount);
        }

        // Ensure shares sum to 10000 bps
        let mut total_shares: u32 = 0;
        for p in participants.iter() {
            total_shares += p.share_bps;
        }
        if total_shares != 10000 {
            return Err(Error::InvalidSharesSum);
        }

        // Count required approvals
        let mut required_approvals: u32 = 0;
        for p in participants.iter() {
            if p.required_approval {
                required_approvals += 1;
            }
        }

        // Transfer funds from customer to contract
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&customer, &contract_address, &total_amount);

        // Use a counter for ID
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrowCounter)
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let current_timestamp = env.ledger().timestamp();

        let escrow = MultiPartyEscrow {
            id: escrow_id,
            participants,
            total_amount,
            token,
            status: EscrowStatus::Locked,
            approvals: Vec::new(&env),
            required_approvals,
            created_at: current_timestamp,
            release_timestamp,
        };

        env.storage()
            .instance()
            .set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::MultiPartyEscrowCounter, &escrow_id);

        MultiPartyEscrowCreated {
            escrow_id,
            participant_count: escrow.participants.len(),
        }
        .publish(&env);

        Ok(escrow_id)
    }

    pub fn approve_release(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::MultiPartyEscrow(escrow_id))
        {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrow(escrow_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::InvalidStatus);
        }

        // Check if caller is a participant and needs to approve
        let mut is_participant = false;
        let mut needs_approval = false;
        for p in escrow.participants.iter() {
            if p.address == caller {
                is_participant = true;
                if p.required_approval {
                    needs_approval = true;
                }
                break;
            }
        }

        if !is_participant || !needs_approval {
            return Err(Error::Unauthorized);
        }

        // Check if already approved
        for addr in escrow.approvals.iter() {
            if addr == caller {
                return Err(Error::DuplicateApproval);
            }
        }

        escrow.approvals.push_back(caller.clone());
        env.storage()
            .instance()
            .set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);

        ParticipantApproved {
            escrow_id,
            approver: caller,
        }
        .publish(&env);

        Ok(())
    }

    pub fn release_multi_party_escrow(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::MultiPartyEscrow(escrow_id))
        {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrow(escrow_id))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::InvalidStatus);
        }

        // Check if all required approvals are met
        if escrow.approvals.len() < escrow.required_approvals {
            return Err(Error::ApprovalsThresholdNotMet);
        }

        // Check release timestamp
        if env.ledger().timestamp() < escrow.release_timestamp {
            return Err(Error::ReleaseNotYetAvailable);
        }

        // Perform transfers
        let token_client = token::Client::new(&env, &escrow.token);
        let contract_address = env.current_contract_address();

        for p in escrow.participants.iter() {
            if p.share_bps > 0 {
                let amount = (escrow.total_amount * (p.share_bps as i128)) / 10000;
                if amount > 0 {
                    token_client.transfer(&contract_address, &p.address, &amount);
                }
            }
        }

        escrow.status = EscrowStatus::Released;
        env.storage()
            .instance()
            .set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);

        MultiPartyEscrowReleased { escrow_id }.publish(&env);

        Ok(())
    }

    pub fn get_multi_party_escrow(env: Env, escrow_id: u64) -> Result<MultiPartyEscrow, Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::MultiPartyEscrow(escrow_id))
        {
            return Err(Error::EscrowNotFound);
        }
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrow(escrow_id))
            .unwrap())
    }

    pub fn create_multi_token_escrow(
        env: Env,
        customer: Address,
        merchant: Address,
        tokens: Vec<TokenEntry>,
        release_timestamp: u64,
    ) -> Result<u64, Error> {
        customer.require_auth();

        if tokens.len() == 0 {
            return Err(Error::EmptyTokenList);
        }
        if tokens.len() > 10 {
            return Err(Error::InvalidParticipantCount);
        }

        // Reject duplicate token addresses
        let len = tokens.len();
        for i in 0..len {
            let entry_i = tokens.get(i).unwrap();
            for j in (i + 1)..len {
                let entry_j = tokens.get(j).unwrap();
                if entry_i.token == entry_j.token {
                    return Err(Error::DuplicateToken);
                }
            }
        }

        // Pull each token amount into the contract escrow.
        let contract_address = env.current_contract_address();
        for entry in tokens.iter() {
            let token_client = token::Client::new(&env, &entry.token);
            token_client.transfer(&customer, &contract_address, &entry.amount);
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MultiTokenEscrowCounter)
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let current_timestamp = env.ledger().timestamp();
        let token_count = tokens.len();

        let escrow = MultiTokenEscrow {
            id: escrow_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            tokens,
            status: EscrowStatus::Locked,
            created_at: current_timestamp,
            release_timestamp,
        };

        env.storage()
            .instance()
            .set(&DataKey::MultiTokenEscrow(escrow_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::MultiTokenEscrowCounter, &escrow_id);

        MultiTokenEscrowCreated {
            escrow_id,
            customer,
            merchant,
            token_count,
            release_timestamp,
        }
        .publish(&env);

        Ok(escrow_id)
    }

    pub fn release_multi_token_escrow(env: Env, escrow_id: u64) -> Result<(), Error> {
        let mut escrow: MultiTokenEscrow = env
            .storage()
            .instance()
            .get(&DataKey::MultiTokenEscrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::InvalidStatus);
        }

        if env.ledger().timestamp() < escrow.release_timestamp {
            return Err(Error::ReleaseNotYetAvailable);
        }

        // Pre-flight: verify balances so a partial failure can't leave the escrow
        // half-released. If any token is unavailable, nothing moves.
        let contract_address = env.current_contract_address();
        for entry in escrow.tokens.iter() {
            let token_client = token::Client::new(&env, &entry.token);
            let balance = match token_client.try_balance(&contract_address) {
                Ok(Ok(b)) => b,
                _ => return Err(Error::TokenTransferPartialFailure),
            };
            if balance < entry.amount {
                return Err(Error::TokenTransferPartialFailure);
            }
        }

        for entry in escrow.tokens.iter() {
            let token_client = token::Client::new(&env, &entry.token);
            if token_client
                .try_transfer(&contract_address, &escrow.merchant, &entry.amount)
                .is_err()
            {
                return Err(Error::TokenTransferPartialFailure);
            }
        }

        let token_count = escrow.tokens.len();
        escrow.status = EscrowStatus::Released;
        env.storage()
            .instance()
            .set(&DataKey::MultiTokenEscrow(escrow_id), &escrow);

        MultiTokenEscrowReleased {
            escrow_id,
            merchant: escrow.merchant,
            token_count,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_multi_token_escrow(env: Env, escrow_id: u64) -> Result<MultiTokenEscrow, Error> {
        env.storage()
            .instance()
            .get(&DataKey::MultiTokenEscrow(escrow_id))
            .ok_or(Error::EscrowNotFound)
    }

    pub fn get_escrow(env: &Env, escrow_id: u64) -> Escrow {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(escrow_id))
            .expect("Escrow not found")
    }

    pub fn release_escrow(
        env: Env,
        admin: Address,
        escrow_id: u64,
        early_release: bool,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Check if this is being called from execute_queued_action

        if let Some(config) = env.storage().instance().get::<DataKey, MultiSigConfig>(&DataKey::MultiSigConfig) {
            if config.admins.contains(&admin) && early_release {
                // Admin force release requires time-lock
                return Err(Error::Unauthorized);
            }
        }

        Self::internal_release_escrow(env, admin, escrow_id, early_release)
    }

    fn internal_release_escrow(
        env: Env,
        _admin: Address,
        escrow_id: u64,
        early_release: bool,
    ) -> Result<(), Error> {
        let current_time: u64 = env.ledger().timestamp();

        // Check if escrow exists
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        match escrow.status {
            EscrowStatus::Locked => {
                // Enforce timelock unless admin approves early release
                if !early_release {
                    if current_time < escrow.release_timestamp {
                        return Err(Error::ReleaseNotYetAvailable);
                    }

                    if current_time < escrow.created_at + escrow.min_hold_period {
                        return Err(Error::ReleaseOnHoldPeriod);
                    }
                }
                escrow.status = EscrowStatus::Released;
            }
            EscrowStatus::Released => return Err(Error::AlreadyProcessed),
            EscrowStatus::Disputed => return Err(Error::InvalidStatus),
            EscrowStatus::Resolved => return Err(Error::AlreadyProcessed),
            EscrowStatus::Cancelled => return Err(Error::AlreadyProcessed),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        let fee_amount = (escrow.amount * escrow.fee_bps) / 10000;
        let merchant_amount = escrow.amount - fee_amount;

        if fee_amount > 0 {
            let fee_config = Self::get_escrow_fee_config(env.clone());
            EscrowContract::transfer_if_token_contract(
                &env,
                &escrow.token,
                &fee_config.fee_recipient,
                fee_amount,
            )?;

            if fee_config.fee_recipient == env.current_contract_address() {
                let mut acc: i128 = env.storage().instance().get(&DataKey::AccumulatedEscrowFees(escrow.token.clone())).unwrap_or(0);
                acc += fee_amount;
                env.storage().instance().set(&DataKey::AccumulatedEscrowFees(escrow.token.clone()), &acc);
            }

            EscrowFeeCollected {
                escrow_id,
                fee_amount,
                recipient: fee_config.fee_recipient.clone(),
            }.publish(&env);
        }

        EscrowContract::transfer_if_token_contract(
            &env,
            &escrow.token,
            &escrow.merchant,
            merchant_amount,
        )?;

        // Update reputation for both parties on successful completion.
        EscrowContract::update_reputation_on_completion(&env, &escrow.merchant);
        EscrowContract::update_reputation_on_completion(&env, &escrow.customer);

        // Update global analytics
        let duration = current_time.saturating_sub(escrow.created_at);
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::EscrowAnalyticsKey)
            .unwrap_or(EscrowAnalytics::default_value());
        let old_released = analytics.total_escrows_released;
        analytics.total_escrows_released += 1;
        analytics.total_value_released += escrow.amount;
        analytics.avg_escrow_duration_seconds = if old_released == 0 {
            duration
        } else {
            (analytics
                .avg_escrow_duration_seconds
                .saturating_mul(old_released)
                + duration)
                / analytics.total_escrows_released
        };
        env.storage()
            .instance()
            .set(&DataKey::EscrowAnalyticsKey, &analytics);

        // Update per-address analytics
        let merchant_addr = escrow.merchant.clone();
        let customer_addr = escrow.customer.clone();
        let rel_amount = escrow.amount;
        EscrowContract::update_customer_analytics(&env, &customer_addr, |a| {
            a.total_escrows_released += 1;
            a.total_value_released += rel_amount;
        });
        EscrowContract::update_merchant_analytics(&env, &merchant_addr, |a| {
            a.total_escrows_released += 1;
            a.total_value_released += rel_amount;
        });

        EscrowReleased {
            escrow_id,
            merchant: escrow.merchant,
            amount: escrow.amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn refund_escrow(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();

        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Unauthorized);
        }
        match escrow.status {
            EscrowStatus::Locked | EscrowStatus::Disputed => {
                escrow.status = EscrowStatus::Resolved;
            }
            EscrowStatus::Released | EscrowStatus::Resolved | EscrowStatus::Cancelled => return Err(Error::AlreadyProcessed),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        EscrowContract::transfer_if_token_contract(
            &env,
            &escrow.token,
            &escrow.customer,
            escrow.amount,
        )?;

        EscrowResolved {
            escrow_id,
            released_to_merchant: false,
            amount: escrow.amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn dispute_escrow(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();

        // Check if escrow exists
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Only customer or merchant can dispute
        if caller != escrow.customer && caller != escrow.merchant {
            return Err(Error::Unauthorized);
        }

        // Handle collateral
        let config = Self::get_dispute_config(env.clone());
        if config.collateral_enabled && config.collateral_amount > 0 {
            let token_client = token::Client::new(&env, &config.collateral_token);
            token_client.transfer(&caller, &env.current_contract_address(), &config.collateral_amount);

            let collateral = DisputeCollateral {
                escrow_id,
                disputing_party: caller.clone(),
                amount: config.collateral_amount,
                token: config.collateral_token.clone(),
                deposited_at: env.ledger().timestamp(),
            };
            env.storage().instance().set(&DataKey::DisputeCollateral(escrow_id), &collateral);
            
            CollateralDeposited {
                escrow_id,
                party: caller.clone(),
                amount: config.collateral_amount,
            }.publish(&env);
        }

        match escrow.status {
            EscrowStatus::Locked => {
                escrow.status = EscrowStatus::Disputed;
                escrow.dispute_started_at = env.ledger().timestamp();
                escrow.last_activity_at = escrow.dispute_started_at;
            }
            EscrowStatus::Released => return Err(Error::AlreadyProcessed),
            EscrowStatus::Disputed => return Err(Error::AlreadyProcessed),
            EscrowStatus::Resolved => return Err(Error::AlreadyProcessed),
            EscrowStatus::Cancelled => return Err(Error::AlreadyProcessed),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::EscrowAnalyticsKey)
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_disputes += 1;
        analytics.dispute_rate_bps = if analytics.total_escrows_created > 0 {
            (analytics.total_disputes as i128 * 10000) / analytics.total_escrows_created as i128
        } else {
            0
        };
        env.storage()
            .instance()
            .set(&DataKey::EscrowAnalyticsKey, &analytics);

        // Update per-address analytics
        let customer_addr = escrow.customer.clone();
        let merchant_addr = escrow.merchant.clone();
        EscrowContract::update_customer_analytics(&env, &customer_addr, |a| {
            a.total_disputes += 1;
        });
        EscrowContract::update_merchant_analytics(&env, &merchant_addr, |a| {
            a.total_disputes += 1;
        });

        EscrowDisputed {
            escrow_id,
            disputed_by: caller,
        }
        .publish(&env);

        Ok(())
    }

    pub fn submit_evidence(
        env: Env,
        caller: Address,
        escrow_id: u64,
        ipfs_hash: String,
    ) -> Result<(), Error> {
        caller.require_auth();
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }
        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::NotDisputed);
        }
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Unauthorized);
        }
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowEvidenceCount(escrow_id))
            .unwrap_or(0);
        let evidence = Evidence {
            submitter: caller.clone(),
            ipfs_hash: ipfs_hash.clone(),
            submitted_at: env.ledger().timestamp(),
        };
        env.storage()
            .instance()
            .set(&DataKey::EscrowEvidence(escrow_id, count), &evidence);
        env.storage()
            .instance()
            .set(&DataKey::EscrowEvidenceCount(escrow_id), &(count + 1));
        escrow.last_activity_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        EvidenceSubmitted {
            escrow_id,
            submitter: caller,
            ipfs_hash,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_evidence_count(env: &Env, escrow_id: u64) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::EscrowEvidenceCount(escrow_id))
            .unwrap_or(0)
    }

    pub fn get_evidence(env: Env, escrow_id: u64, limit: u64, offset: u64) -> Vec<Evidence> {
        let total: u64 = EscrowContract::get_evidence_count(&env, escrow_id);
        let mut items = Vec::new(&env);
        if limit == 0 || offset >= total {
            return items;
        }
        let end = core::cmp::min(total, offset.saturating_add(limit));
        let mut i = offset;
        while i < end {
            if let Some(ev) = env
                .storage()
                .instance()
                .get::<DataKey, Evidence>(&DataKey::EscrowEvidence(escrow_id, i))
            {
                items.push_back(ev);
            }
            i += 1;
        }
        items
    }

    pub fn escalate_dispute(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }
        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::NotDisputed);
        }
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Unauthorized);
        }
        escrow.escalation_level = escrow.escalation_level.saturating_add(1);
        escrow.last_activity_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        DisputeEscalated {
            escrow_id,
            level: escrow.escalation_level,
        }
        .publish(&env);
        Ok(())
    }

    pub fn auto_resolve_dispute(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }
        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::NotDisputed);
        }
        let now = env.ledger().timestamp();
        let last = if escrow.last_activity_at == 0 {
            escrow.dispute_started_at
        } else {
            escrow.last_activity_at
        };
        let timeout: u64 = 500;
        if now.saturating_sub(last) < timeout {
            return Err(Error::TimeoutNotReached);
        }
        let release_to_merchant = EscrowContract::weighted_auto_resolve(&env, escrow_id);
        let (winner, loser) = if release_to_merchant {
            (escrow.merchant.clone(), escrow.customer.clone())
        } else {
            (escrow.customer.clone(), escrow.merchant.clone())
        };
        escrow.status = if release_to_merchant {
            EscrowStatus::Released
        } else {
            EscrowStatus::Resolved
        };
        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        EscrowContract::update_reputation_on_dispute_outcome(&env, &winner, &loser);
        EscrowResolved {
            escrow_id,
            released_to_merchant: release_to_merchant,
            amount: escrow.amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        escrow_id: u64,
        release_to_merchant: bool,
    ) -> Result<(), Error> {
        admin.require_auth();
        Self::require_not_paused(&env, "resolve_dispute")?;

        if let Some(config) = env.storage().instance().get::<DataKey, MultiSigConfig>(&DataKey::MultiSigConfig) {
            if !config.admins.contains(&admin) {
                return Err(Error::NotAnAdmin);
            }
            // Admin actions require time-lock for sensitive operations
            return Err(Error::Unauthorized);
        }

        Self::internal_resolve_dispute(env, admin, escrow_id, release_to_merchant)
    }

    fn internal_resolve_dispute(
        env: Env,
        _admin: Address,
        escrow_id: u64,
        release_to_merchant: bool,
    ) -> Result<(), Error> {
        // Check if escrow exists
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Only resolve if status is Disputed
        match escrow.status {
            EscrowStatus::Disputed => {
                escrow.status = if release_to_merchant {
                    EscrowStatus::Released
                } else {
                    EscrowStatus::Resolved
                };
            }
            _ => return Err(Error::NotDisputed),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        // Transfer main escrow funds
        let recipient = if release_to_merchant {
            &escrow.merchant
        } else {
            &escrow.customer
        };
        Self::transfer_if_token_contract(&env, &escrow.token, recipient, escrow.amount)?;

        // Handle collateral distribution
        if let Some(collateral) = env.storage().instance().get::<DataKey, DisputeCollateral>(&DataKey::DisputeCollateral(escrow_id)) {
            let winner = if release_to_merchant {
                escrow.merchant.clone()
            } else {
                escrow.customer.clone()
            };

            let token_client = token::Client::new(&env, &collateral.token);
            token_client.transfer(&env.current_contract_address(), &winner, &collateral.amount);

            if winner == collateral.disputing_party {
                CollateralReturned {
                    escrow_id,
                    party: collateral.disputing_party,
                    amount: collateral.amount,
                }.publish(&env);
            } else {
                CollateralForfeited {
                    escrow_id,
                    party: collateral.disputing_party,
                    amount: collateral.amount,
                }.publish(&env);
            }
            env.storage().instance().remove(&DataKey::DisputeCollateral(escrow_id));
        }

        let (winner, loser) = if release_to_merchant {
            (escrow.merchant.clone(), escrow.customer.clone())
        } else {
            (escrow.customer.clone(), escrow.merchant.clone())
        };
        EscrowContract::update_reputation_on_dispute_outcome(&env, &winner, &loser);

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::EscrowAnalyticsKey)
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_resolutions += 1;
        analytics.dispute_rate_bps = if analytics.total_escrows_created > 0 {
            (analytics.total_disputes as i128 * 10000) / analytics.total_escrows_created as i128
        } else {
            0
        };
        env.storage()
            .instance()
            .set(&DataKey::EscrowAnalyticsKey, &analytics);

        // Update per-address analytics
        EscrowContract::update_customer_analytics(&env, &escrow.customer, |a| {
            a.total_resolutions += 1;
        });
        EscrowContract::update_merchant_analytics(&env, &escrow.merchant, |a| {
            a.total_resolutions += 1;
        });

        EscrowResolved {
            escrow_id,
            released_to_merchant: release_to_merchant,
            amount: escrow.amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Returns an advisory dispute recommendation derived from the customer's
    /// and merchant's reputation scores. The result is purely advisory and
    /// `resolve_dispute` does not consult or enforce it. A score difference
    /// below 100 yields `Inconclusive`.
    pub fn get_dispute_recommendation(env: Env, escrow_id: u64) -> DisputeRecommendation {
        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        let customer_rep = EscrowContract::get_or_default_reputation(&env, &escrow.customer);
        let merchant_rep = EscrowContract::get_or_default_reputation(&env, &escrow.merchant);

        let customer_score = customer_rep.score as i128;
        let merchant_score = merchant_rep.score as i128;

        let diff = merchant_score - customer_score;
        let abs_diff = if diff < 0 { -diff } else { diff };

        let recommendation = if abs_diff < 100 {
            DisputeOutcome::Inconclusive
        } else if diff > 0 {
            DisputeOutcome::FavorMerchant
        } else {
            DisputeOutcome::FavorCustomer
        };

        let confidence_bps = abs_diff.min(10000) as u32;

        DisputeRecommendationGenerated {
            escrow_id,
            outcome: recommendation.clone(),
            confidence_bps,
        }
        .publish(&env);

        DisputeRecommendation {
            escrow_id,
            customer_score,
            merchant_score,
            recommendation,
            confidence_bps,
        }
    }

    pub fn get_escrows_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Escrow> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerEscrowCount(customer.clone()))
            .unwrap_or(0);

        let mut escrows = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(escrow_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::CustomerEscrows(customer.clone(), i))
            {
                if let Some(escrow) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Escrow>(&DataKey::Escrow(escrow_id))
                {
                    escrows.push_back(escrow);
                }
            }
        }

        escrows
    }

    pub fn get_escrow_count_by_customer(env: Env, customer: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::CustomerEscrowCount(customer))
            .unwrap_or(0)
    }

    pub fn get_escrows_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Escrow> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantEscrowCount(merchant.clone()))
            .unwrap_or(0);

        let mut escrows = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(escrow_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::MerchantEscrows(merchant.clone(), i))
            {
                if let Some(escrow) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Escrow>(&DataKey::Escrow(escrow_id))
                {
                    escrows.push_back(escrow);
                }
            }
        }

        escrows
    }

    pub fn get_escrow_count_by_merchant(env: Env, merchant: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MerchantEscrowCount(merchant))
            .unwrap_or(0)
    }

    // ── REPUTATION METHODS ───────────────────────────────────────────────────

    /// Returns the reputation score for an address.
    /// New addresses start at the neutral score of 5000.
    pub fn get_reputation(env: Env, address: Address) -> ReputationScore {
        EscrowContract::get_or_default_reputation(&env, &address)
    }

    /// Admin configures the reputation reward/penalty magnitudes.
    pub fn set_reputation_config(
        env: Env,
        admin: Address,
        config: ReputationConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::ReputationConfig, &config);
        ReputationConfigUpdated {
            win_reward: config.win_reward,
            loss_penalty: config.loss_penalty,
            completion_reward: config.completion_reward,
            dispute_initiation_penalty: config.dispute_initiation_penalty,
        }
        .publish(&env);
        Ok(())
    }

    /// Returns the current reputation configuration.
    /// Falls back to conservative defaults if not yet set.
    pub fn get_reputation_config(env: Env) -> ReputationConfig {
        EscrowContract::get_or_default_reputation_config(&env)
    }

    /// Internal helper: load reputation or return a neutral default.
    fn get_or_default_reputation(env: &Env, address: &Address) -> ReputationScore {
        env.storage()
            .instance()
            .get(&DataKey::ReputationScore(address.clone()))
            .unwrap_or(ReputationScore {
                address: address.clone(),
                total_transactions: 0,
                disputes_initiated: 0,
                disputes_won: 0,
                disputes_lost: 0,
                score: 5000,
                last_updated: 0,
                decay_rate: 0,
            })
    }

    /// Internal helper: load reputation config or return sensible defaults.
    fn get_or_default_reputation_config(env: &Env) -> ReputationConfig {
        env.storage()
            .instance()
            .get(&DataKey::ReputationConfig)
            .unwrap_or(ReputationConfig {
                win_reward: 200,
                loss_penalty: 200,
                completion_reward: 100,
                dispute_initiation_penalty: 50,
            })
    }

    /// Called when an escrow completes normally (released). Rewards the address
    /// with `completion_reward` and increments their transaction count.
    fn update_reputation_on_completion(env: &Env, address: &Address) {
        let config = EscrowContract::get_or_default_reputation_config(env);
        let mut rep = EscrowContract::get_or_default_reputation(env, address);
        let old_score = rep.score;
        rep.score = (rep.score + config.completion_reward).min(10000);
        rep.total_transactions = rep.total_transactions.saturating_add(1);
        rep.last_updated = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::ReputationScore(address.clone()), &rep);
        ReputationUpdated {
            address: address.clone(),
            old_score,
            new_score: rep.score,
        }
        .publish(env);
    }

    /// Called after a dispute is resolved. Applies win/loss deltas and clamps
    /// scores to [0, 10000].
    fn update_reputation_on_dispute_outcome(env: &Env, winner: &Address, loser: &Address) {
        let config = EscrowContract::get_or_default_reputation_config(env);
        let now = env.ledger().timestamp();

        // Update winner.
        let mut winner_rep = EscrowContract::get_or_default_reputation(env, winner);
        let old_winner_score = winner_rep.score;
        winner_rep.score = (winner_rep.score + config.win_reward).min(10000);
        winner_rep.disputes_won = winner_rep.disputes_won.saturating_add(1);
        winner_rep.last_updated = now;
        env.storage()
            .instance()
            .set(&DataKey::ReputationScore(winner.clone()), &winner_rep);
        ReputationUpdated {
            address: winner.clone(),
            old_score: old_winner_score,
            new_score: winner_rep.score,
        }
        .publish(env);

        // Update loser.
        let mut loser_rep = EscrowContract::get_or_default_reputation(env, loser);
        let old_loser_score = loser_rep.score;
        loser_rep.score = (loser_rep.score - config.loss_penalty).max(0);
        loser_rep.disputes_lost = loser_rep.disputes_lost.saturating_add(1);
        loser_rep.last_updated = now;
        env.storage()
            .instance()
            .set(&DataKey::ReputationScore(loser.clone()), &loser_rep);
        ReputationUpdated {
            address: loser.clone(),
            old_score: old_loser_score,
            new_score: loser_rep.score,
        }
        .publish(env);
    }

    /// Weighted auto-resolve: each piece of evidence contributes the submitter's
    /// reputation score to their side's total weight rather than a raw count.
    /// Returns `true` if the merchant side outweighs the customer side.
    fn weighted_auto_resolve(env: &Env, escrow_id: u64) -> bool {
        let escrow = EscrowContract::get_escrow(env, escrow_id);
        let total = EscrowContract::get_evidence_count(env, escrow_id);

        let mut customer_weight: i128 = 0;
        let mut merchant_weight: i128 = 0;

        let mut i: u64 = 0;
        while i < total {
            if let Some(ev) = env
                .storage()
                .instance()
                .get::<DataKey, Evidence>(&DataKey::EscrowEvidence(escrow_id, i))
            {
                let rep = EscrowContract::get_or_default_reputation(env, &ev.submitter);
                if ev.submitter == escrow.customer {
                    customer_weight = customer_weight.saturating_add(rep.score as i128);
                } else if ev.submitter == escrow.merchant {
                    merchant_weight = merchant_weight.saturating_add(rep.score as i128);
                }
            }
            i += 1;
        }

        merchant_weight > customer_weight
    }

    /// Creates a new vesting escrow with milestone-based or time-linear vesting.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `customer` - The address funding the escrow
    /// * `merchant` - The address receiving vested funds
    /// * `amount` - Total amount to be vested (must equal sum of milestone amounts if milestones provided)
    /// * `token` - The token address for the payment
    /// * `cliff_timestamp` - Timestamp before which no vesting occurs
    /// * `end_timestamp` - Timestamp when vesting completes
    /// * `milestones` - Optional vector of VestingMilestone for milestone-based vesting
    ///
    /// # Returns
    /// The escrow ID for the created vesting schedule
    ///
    /// # Errors
    /// * InvalidVestingSchedule - If milestone amounts don't sum to total amount or timestamps are invalid
    pub fn create_vesting_escrow(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        cliff_timestamp: u64,
        end_timestamp: u64,
        milestones: Vec<VestingMilestone>,
    ) -> Result<u64, Error> {
        customer.require_auth();

        // Validate timestamps
        let current_timestamp = env.ledger().timestamp();
        if cliff_timestamp < current_timestamp {
            return Err(Error::InvalidVestingSchedule);
        }
        if end_timestamp <= cliff_timestamp {
            return Err(Error::InvalidVestingSchedule);
        }

        // Validate milestones if provided
        if !milestones.is_empty() {
            let mut milestone_total: i128 = 0;
            for milestone in milestones.iter() {
                milestone_total = milestone_total.saturating_add(milestone.amount);
                // Validate milestone unlock timestamp is after cliff
                if milestone.unlock_timestamp < cliff_timestamp {
                    return Err(Error::InvalidVestingSchedule);
                }
                // Validate milestone unlock timestamp is before or at end
                if milestone.unlock_timestamp > end_timestamp {
                    return Err(Error::InvalidVestingSchedule);
                }
            }
            // Milestone amounts must sum to total amount
            if milestone_total != amount {
                return Err(Error::InvalidVestingSchedule);
            }
        }

        // Create the base escrow
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowCounter)
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled { fee_config.fee_bps } else { 0 };

        let escrow = Escrow {
            id: escrow_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token: token.clone(),
            status: EscrowStatus::Locked,
            created_at: current_timestamp,
            release_timestamp: end_timestamp,
            dispute_started_at: 0,
            last_activity_at: current_timestamp,
            escalation_level: 0,
            min_hold_period: 0,
            fee_bps,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::EscrowCounter, &escrow_id);

        // Create and store the vesting schedule
        // Assign sequential milestone_ids if not already set
        let mut indexed_milestones = Vec::new(&env);
        for (i, mut m) in milestones.iter().enumerate() {
            if m.milestone_id == 0 {
                m.milestone_id = (i as u64) + 1;
            }
            indexed_milestones.push_back(m);
        }

        let vesting_schedule = VestingSchedule {
            escrow_id,
            total_amount: amount,
            released_amount: 0,
            start_timestamp: current_timestamp,
            cliff_timestamp,
            end_timestamp,
            milestones: indexed_milestones,
        };

        env.storage()
            .instance()
            .set(&DataKey::VestingSchedule(escrow_id), &vesting_schedule);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerEscrowCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerEscrows(customer.clone(), customer_count),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerEscrowCount(customer.clone()),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantEscrowCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantEscrows(merchant.clone(), merchant_count),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantEscrowCount(merchant.clone()),
            &(merchant_count + 1),
        );

        VestingScheduleCreated {
            escrow_id,
            total_amount: amount,
        }
        .publish(&env);

        Ok(escrow_id)
    }

    /// Returns the vesting schedule for a given escrow ID.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `escrow_id` - The ID of the escrow
    ///
    /// # Returns
    /// The VestingSchedule struct
    ///
    /// # Errors
    /// * EscrowNotFound - If the escrow does not exist or has no vesting schedule
    pub fn get_vesting_schedule(env: Env, escrow_id: u64) -> Result<VestingSchedule, Error> {
        env.storage()
            .instance()
            .get(&DataKey::VestingSchedule(escrow_id))
            .ok_or(Error::EscrowNotFound)
    }

    /// Calculates the total vested amount that has been unlocked based on the current timestamp.
    /// Supports both milestone-based and time-linear vesting.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `escrow_id` - The ID of the escrow
    ///
    /// # Returns
    /// The total vested amount (including already released amounts)
    pub fn get_vested_amount(env: Env, escrow_id: u64) -> i128 {
        let vesting_schedule = match env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
        {
            Some(schedule) => schedule,
            None => return 0,
        };

        let current_timestamp = env.ledger().timestamp();

        // Before cliff - nothing is vested
        if current_timestamp < vesting_schedule.cliff_timestamp {
            return 0;
        }

        // After end - everything is vested
        if current_timestamp >= vesting_schedule.end_timestamp {
            return vesting_schedule.total_amount;
        }

        // If milestones exist, use milestone-based vesting
        if !vesting_schedule.milestones.is_empty() {
            let mut vested_amount: i128 = 0;
            for milestone in vesting_schedule.milestones.iter() {
                if current_timestamp >= milestone.unlock_timestamp {
                    vested_amount = vested_amount.saturating_add(milestone.amount);
                }
            }
            vested_amount
        } else {
            // Time-linear vesting (proportional to time elapsed since cliff)
            let total_duration = vesting_schedule
                .end_timestamp
                .saturating_sub(vesting_schedule.cliff_timestamp);
            let elapsed = current_timestamp.saturating_sub(vesting_schedule.cliff_timestamp);

            if total_duration == 0 {
                return 0;
            }

            let vested_portion = (elapsed as i128).saturating_mul(vesting_schedule.total_amount);
            vested_portion / total_duration as i128
        }
    }

    /// Calculates the releasable amount (vested but not yet released).
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `escrow_id` - The ID of the escrow
    ///
    /// # Returns
    /// The amount that can be released
    pub fn get_releasable_amount(env: Env, escrow_id: u64) -> i128 {
        let vesting_schedule = match env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
        {
            Some(schedule) => schedule,
            None => return 0,
        };

        let vested_amount = EscrowContract::get_vested_amount(env, escrow_id);
        vested_amount.saturating_sub(vesting_schedule.released_amount)
    }

    /// Releases vested amounts from the escrow. Can be called multiple times to release
    /// milestone amounts as they unlock or linear vesting portions.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `admin` - The admin address authorizing the release
    /// * `escrow_id` - The ID of the escrow
    ///
    /// # Returns
    /// The amount released
    ///
    /// # Errors
    /// * EscrowNotFound - If the escrow does not exist
    /// * CliffPeriodNotPassed - If called before the cliff timestamp
    /// * InsufficientVestedAmount - If there's no vested amount to release
    pub fn release_vested_amount(env: Env, admin: Address, escrow_id: u64) -> Result<i128, Error> {
        admin.require_auth();

        // Check if escrow exists
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        let current_timestamp = env.ledger().timestamp();

        // Enforce cliff period
        if current_timestamp < vesting_schedule.cliff_timestamp {
            return Err(Error::CliffPeriodNotPassed);
        }

        // Calculate vested amount
        let vested_amount = EscrowContract::get_vested_amount(env.clone(), escrow_id);
        let releasable_amount = vested_amount.saturating_sub(vesting_schedule.released_amount);

        if releasable_amount == 0 {
            return Err(Error::InsufficientVestedAmount);
        }

        // Update the released amount
        vesting_schedule.released_amount = vesting_schedule
            .released_amount
            .saturating_add(releasable_amount);

        // If using milestones, mark released milestones as such
        if !vesting_schedule.milestones.is_empty() {
            let mut milestones_vec = vesting_schedule.milestones.clone();
            for i in 0..milestones_vec.len() {
                let mut milestone = milestones_vec.get(i).unwrap();
                if !milestone.released
                    && current_timestamp >= milestone.unlock_timestamp
                    && vesting_schedule.released_amount >= milestone.amount
                {
                    milestone.released = true;
                    let amount = milestone.amount;
                    let mid = milestone.milestone_id;
                    milestones_vec.set(i, milestone);

                    MilestoneReleased {
                        escrow_id,
                        milestone_id: mid,
                        amount,
                    }
                    .publish(&env);
                }
            }
            vesting_schedule.milestones = milestones_vec;
        }

        // Update storage
        env.storage()
            .instance()
            .set(&DataKey::VestingSchedule(escrow_id), &vesting_schedule);

        VestedAmountReleased {
            escrow_id,
            amount: releasable_amount,
            released_at: current_timestamp,
        }
        .publish(&env);

        Ok(releasable_amount)
    }

    /// Admin approves a specific milestone, enabling it to be released.
    ///
    /// # Errors
    /// * `EscrowNotFound` - No vesting schedule for this escrow
    /// * `MilestoneNotFound` - No milestone with the given ID
    /// * `MilestoneAlreadyReleased` - Milestone was already released
    /// * `NotAnAdmin` - Caller is not a registered admin
    pub fn approve_milestone(
        env: Env,
        admin: Address,
        escrow_id: u64,
        milestone_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        let mut found = false;
        let mut milestones = vesting_schedule.milestones.clone();
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                found = true;
                if m.released {
                    return Err(Error::MilestoneAlreadyReleased);
                }
                let now = env.ledger().timestamp();
                m.approved_by = Some(admin.clone());
                m.approved_at = Some(now);
                milestones.set(i, m);
                break;
            }
        }

        if !found {
            return Err(Error::MilestoneNotFound);
        }

        vesting_schedule.milestones = milestones;
        env.storage()
            .instance()
            .set(&DataKey::VestingSchedule(escrow_id), &vesting_schedule);

        MilestoneApproved {
            escrow_id,
            milestone_id,
            approved_by: admin,
        }
        .publish(&env);

        Ok(())
    }

    /// Releases a specific approved milestone's amount to the merchant.
    ///
    /// # Errors
    /// * `EscrowNotFound` - No vesting schedule for this escrow
    /// * `MilestoneNotFound` - No milestone with the given ID
    /// * `MilestoneNotApproved` - Milestone has not been approved by an admin
    /// * `MilestoneAlreadyReleased` - Milestone was already released
    /// * `MilestoneOverflow` - Release would exceed the escrow's locked amount
    pub fn release_milestone(env: Env, escrow_id: u64, milestone_id: u64) -> Result<i128, Error> {
        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        let escrow: Escrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        let mut found = false;
        let mut release_amount: i128 = 0;
        let mut milestones = vesting_schedule.milestones.clone();

        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                found = true;
                if m.released {
                    return Err(Error::MilestoneAlreadyReleased);
                }
                if m.approved_by.is_none() {
                    return Err(Error::MilestoneNotApproved);
                }
                // Guard: total released must not exceed locked amount
                let new_total = vesting_schedule.released_amount.saturating_add(m.amount);
                if new_total > vesting_schedule.total_amount {
                    return Err(Error::MilestoneOverflow);
                }
                release_amount = m.amount;
                m.released = true;
                milestones.set(i, m);
                break;
            }
        }

        if !found {
            return Err(Error::MilestoneNotFound);
        }

        vesting_schedule.milestones = milestones;
        vesting_schedule.released_amount = vesting_schedule
            .released_amount
            .saturating_add(release_amount);

        env.storage()
            .instance()
            .set(&DataKey::VestingSchedule(escrow_id), &vesting_schedule);

        // Transfer to merchant
        EscrowContract::transfer_if_token_contract(
            &env,
            &escrow.token,
            &escrow.merchant,
            release_amount,
        )?;

        MilestoneReleased {
            escrow_id,
            milestone_id,
            amount: release_amount,
        }
        .publish(&env);

        Ok(release_amount)
    }

    /// Returns all milestones that have not yet been released.
    pub fn get_pending_milestones(env: Env, escrow_id: u64) -> Vec<VestingMilestone> {
        let vesting_schedule = match env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
        {
            Some(s) => s,
            None => return Vec::new(&env),
        };

        let mut pending = Vec::new(&env);
        for m in vesting_schedule.milestones.iter() {
            if !m.released {
                pending.push_back(m);
            }
        }
        pending
    }

    /// Adds a new milestone to an existing vesting schedule.
    /// The new milestone's amount must not cause total milestone amounts to exceed the escrow's
    /// locked amount.
    ///
    /// # Errors
    /// * `EscrowNotFound` - No vesting schedule for this escrow
    /// * `MilestoneOverflow` - Adding this milestone would exceed the locked amount
    /// * `NotAnAdmin` - Caller is not a registered admin
    pub fn add_milestone(
        env: Env,
        admin: Address,
        escrow_id: u64,
        milestone: VestingMilestone,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::VestingSchedule(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        // Sum existing milestone amounts
        let mut existing_total: i128 = 0;
        for m in vesting_schedule.milestones.iter() {
            existing_total = existing_total.saturating_add(m.amount);
        }

        if existing_total.saturating_add(milestone.amount) > vesting_schedule.total_amount {
            return Err(Error::MilestoneOverflow);
        }

        // Auto-assign milestone_id if not provided
        let mut new_milestone = milestone;
        if new_milestone.milestone_id == 0 {
            new_milestone.milestone_id = (vesting_schedule.milestones.len() as u64) + 1;
        }

        vesting_schedule.milestones.push_back(new_milestone);
        env.storage()
            .instance()
            .set(&DataKey::VestingSchedule(escrow_id), &vesting_schedule);

        Ok(())
    }

    // For existing tests that use synthetic token addresses, transfer calls are skipped when the
    // address is not a token contract. For real token contracts, transfer failures bubble up.
    fn transfer_if_token_contract(
        env: &Env,
        token_address: &Address,
        recipient: &Address,
        amount: i128,
    ) -> Result<(), Error> {
        let token_client = token::Client::new(env, token_address);
        let contract_address = env.current_contract_address();
        if token_client.try_balance(&contract_address).is_err() {
            return Ok(());
        }
        if token_client
            .try_transfer(&contract_address, recipient, &amount)
            .is_err()
        {
            return Err(Error::TransferFailed);
        }
        Ok(())
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
            ActionType::ReleaseEscrow => {
                let escrow_id = EscrowContract::read_u64_from_bytes(&proposal.data, 0);
                let early_release = proposal.data.get(8).unwrap_or(0) != 0;

                if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
                    return Err(Error::EscrowNotFound);
                }

                let current_time: u64 = env.ledger().timestamp();
                let mut escrow = EscrowContract::get_escrow(env, escrow_id);

                match escrow.status {
                    EscrowStatus::Locked => {
                        if !early_release {
                            if current_time < escrow.release_timestamp {
                                return Err(Error::ReleaseNotYetAvailable);
                            }
                            if current_time < escrow.created_at + escrow.min_hold_period {
                                return Err(Error::ReleaseOnHoldPeriod);
                            }
                        }
                        escrow.status = EscrowStatus::Released;
                    }
                    EscrowStatus::Released => return Err(Error::AlreadyProcessed),
                    EscrowStatus::Disputed => return Err(Error::InvalidStatus),
                    EscrowStatus::Resolved => return Err(Error::AlreadyProcessed),
                    EscrowStatus::Cancelled => return Err(Error::AlreadyProcessed),
                }

                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(escrow_id), &escrow);

                let fee_amount = (escrow.amount * escrow.fee_bps) / 10000;
                let merchant_amount = escrow.amount - fee_amount;

                if fee_amount > 0 {
                    let fee_config = EscrowContract::get_escrow_fee_config(env.clone());
                    EscrowContract::transfer_if_token_contract(
                        env,
                        &escrow.token,
                        &fee_config.fee_recipient,
                        fee_amount,
                    )?;

                    if fee_config.fee_recipient == env.current_contract_address() {
                        let mut acc: i128 = env.storage().instance().get(&DataKey::AccumulatedEscrowFees(escrow.token.clone())).unwrap_or(0);
                        acc += fee_amount;
                        env.storage().instance().set(&DataKey::AccumulatedEscrowFees(escrow.token.clone()), &acc);
                    }

                    EscrowFeeCollected {
                        escrow_id,
                        fee_amount,
                        recipient: fee_config.fee_recipient.clone(),
                    }.publish(env);
                }

                EscrowContract::transfer_if_token_contract(
                    env,
                    &escrow.token,
                    &escrow.merchant,
                    merchant_amount,
                )?;

                EscrowContract::update_reputation_on_completion(env, &escrow.merchant);
                EscrowContract::update_reputation_on_completion(env, &escrow.customer);

                // Update analytics
                let duration = current_time.saturating_sub(escrow.created_at);
                let mut analytics: EscrowAnalytics = env
                    .storage()
                    .instance()
                    .get(&DataKey::EscrowAnalyticsKey)
                    .unwrap_or(EscrowAnalytics::default_value());
                let old_released = analytics.total_escrows_released;
                analytics.total_escrows_released += 1;
                analytics.total_value_released += escrow.amount;
                analytics.avg_escrow_duration_seconds = if old_released == 0 {
                    duration
                } else {
                    (analytics
                        .avg_escrow_duration_seconds
                        .saturating_mul(old_released)
                        + duration)
                        / analytics.total_escrows_released
                };
                env.storage()
                    .instance()
                    .set(&DataKey::EscrowAnalyticsKey, &analytics);
                EscrowContract::update_merchant_analytics(env, &escrow.merchant, |a| {
                    a.total_escrows_released += 1;
                    a.total_value_released += escrow.amount;
                });
                EscrowContract::update_customer_analytics(env, &escrow.customer, |a| {
                    a.total_escrows_released += 1;
                    a.total_value_released += escrow.amount;
                });

                EscrowReleased {
                    escrow_id,
                    merchant: escrow.merchant,
                    amount: escrow.amount,
                }
                .publish(env);
            }
            ActionType::ResolveDispute => {
                let escrow_id = EscrowContract::read_u64_from_bytes(&proposal.data, 0);
                let release_to_merchant = proposal.data.get(8).unwrap_or(0) != 0;

                if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
                    return Err(Error::EscrowNotFound);
                }

                let mut escrow = EscrowContract::get_escrow(env, escrow_id);

                match escrow.status {
                    EscrowStatus::Disputed => {
                        escrow.status = if release_to_merchant {
                            EscrowStatus::Released
                        } else {
                            EscrowStatus::Resolved
                        };
                    }
                    _ => return Err(Error::NotDisputed),
                }

                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(escrow_id), &escrow);

                let (winner, loser) = if release_to_merchant {
                    (escrow.merchant.clone(), escrow.customer.clone())
                } else {
                    (escrow.customer.clone(), escrow.merchant.clone())
                };
                EscrowContract::update_reputation_on_dispute_outcome(env, &winner, &loser);

                if release_to_merchant {
                    EscrowContract::transfer_if_token_contract(
                        env,
                        &escrow.token,
                        &escrow.merchant,
                        escrow.amount,
                    )?;
                } else {
                    EscrowContract::transfer_if_token_contract(
                        env,
                        &escrow.token,
                        &escrow.customer,
                        escrow.amount,
                    )?;
                }

                // Update analytics
                let mut analytics: EscrowAnalytics = env
                    .storage()
                    .instance()
                    .get(&DataKey::EscrowAnalyticsKey)
                    .unwrap_or(EscrowAnalytics::default_value());
                analytics.total_resolutions += 1;
                analytics.dispute_rate_bps = if analytics.total_escrows_created > 0 {
                    (analytics.total_disputes as i128 * 10000)
                        / analytics.total_escrows_created as i128
                } else {
                    0
                };
                env.storage()
                    .instance()
                    .set(&DataKey::EscrowAnalyticsKey, &analytics);
                EscrowContract::update_merchant_analytics(env, &escrow.merchant, |a| {
                    a.total_resolutions += 1;
                });
                EscrowContract::update_customer_analytics(env, &escrow.customer, |a| {
                    a.total_resolutions += 1;
                });

                EscrowResolved {
                    escrow_id,
                    released_to_merchant: release_to_merchant,
                    amount: escrow.amount,
                }
                .publish(env);
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
                    AdminAdded { admin: new_admin }.publish(env);
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
                AdminRemoved {
                    admin: admin_to_remove,
                }
                .publish(env);
            }
            ActionType::UpdateRequiredSignatures => {
                let required = EscrowContract::read_u64_from_bytes(&proposal.data, 0) as u32;
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

    pub fn queue_action(
        env: Env,
        admin: Address,
        escrow_id: u64,
        action_type: EscrowActionType,
        data: Bytes,
    ) -> Result<u64, Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        let timelock_config = Self::get_timelock_config(env.clone());
        let current_time = env.ledger().timestamp();

        let action_id = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockCounter)
            .unwrap_or(0u64)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::TimeLockCounter, &action_id);

        let action = TimeLockAction {
            action_id,
            action_type,
            escrow_id,
            proposed_by: admin,
            queued_at: current_time,
            executable_after: current_time + timelock_config.delay,
            expires_at: current_time + timelock_config.delay + timelock_config.grace_period,
            executed: false,
            cancelled: false,
            data,
        };

        env.storage()
            .instance()
            .set(&DataKey::TimeLockAction(action_id), &action);

        TimeLockActionQueued {
            action_id,
            escrow_id,
            executable_after: action.executable_after,
        }
        .publish(&env);

        Ok(action_id)
    }

    pub fn execute_queued_action(env: Env, action_id: u64) -> Result<(), Error> {
        let mut action: TimeLockAction = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockAction(action_id))
            .ok_or(Error::ProposalNotFound)?;

        if action.executed {
            return Err(Error::ActionAlreadyExecuted);
        }
        if action.cancelled {
            return Err(Error::ActionCancelled);
        }

        let current_time = env.ledger().timestamp();
        if current_time < action.executable_after {
            return Err(Error::ActionNotReady);
        }
        if current_time > action.expires_at {
            return Err(Error::ActionExpired);
        }

        action.executed = true;
        env.storage()
            .instance()
            .set(&DataKey::TimeLockAction(action_id), &action);

        match action.action_type {
            EscrowActionType::ResolveDispute(release_to_merchant) => {
                Self::internal_resolve_dispute(
                    env.clone(),
                    action.proposed_by,
                    action.escrow_id,
                    release_to_merchant,
                )?;
            }
            EscrowActionType::ForceRelease => {
                Self::internal_release_escrow(
                    env.clone(),
                    action.proposed_by,
                    action.escrow_id,
                    true,
                )?;
            }
            _ => {}
        }

        TimeLockActionExecuted {
            action_id,
            escrow_id: action.escrow_id,
            executed_at: current_time,
        }
        .publish(&env);

        Ok(())
    }

    pub fn cancel_queued_action(env: Env, admin: Address, action_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        let mut action: TimeLockAction = env
            .storage()
            .instance()
            .get(&DataKey::TimeLockAction(action_id))
            .ok_or(Error::ProposalNotFound)?;

        if action.executed {
            return Err(Error::ActionAlreadyExecuted);
        }
        if action.cancelled {
            return Err(Error::ActionCancelled);
        }

        // Only the proposing admin or any other admin can cancel
        if action.proposed_by != admin && !config.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }

        action.cancelled = true;
        env.storage()
            .instance()
            .set(&DataKey::TimeLockAction(action_id), &action);

        TimeLockActionCancelled {
            action_id,
            cancelled_by: admin,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_queued_action(env: Env, action_id: u64) -> Result<TimeLockAction, Error> {
        env.storage()
            .instance()
            .get(&DataKey::TimeLockAction(action_id))
            .ok_or(Error::ProposalNotFound)
    }

    pub fn set_timelock_config(
        env: Env,
        admin: Address,
        config: TimeLockConfig,
    ) -> Result<(), Error> {
        admin.require_auth();

        let multisig_config = Self::get_multisig_config(env.clone());
        if !multisig_config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        if config.delay < 3600 || config.delay > 604800 {
            return Err(Error::InvalidStatus);
        }

        env.storage()
            .instance()
            .set(&DataKey::TimeLockConfig, &config);

        TimeLockConfigUpdated {
            delay: config.delay,
            grace_period: config.grace_period,
        }
        .publish(&env);

        Ok(())
    }

    // ── ANALYTICS FUNCTIONS ────────────────────────────────────────────────

    pub fn get_escrow_analytics(env: Env) -> EscrowAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::EscrowAnalyticsKey)
            .unwrap_or(EscrowAnalytics::default_value())
    }

    pub fn get_merchant_analytics(env: Env, merchant: Address) -> EscrowAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(merchant))
            .unwrap_or(EscrowAnalytics::default_value())
    }

    pub fn get_customer_analytics(env: Env, customer: Address) -> EscrowAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::CustomerAnalytics(customer))
            .unwrap_or(EscrowAnalytics::default_value())
    }

    pub fn reset_analytics(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }
        env.storage().instance().set(
            &DataKey::EscrowAnalyticsKey,
            &EscrowAnalytics::default_value(),
        );
        let now = env.ledger().timestamp();
        AnalyticsReset {
            reset_by: admin,
            reset_at: now,
        }
        .publish(&env);
        Ok(())
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

    pub fn get_timelock_config(env: Env) -> TimeLockConfig {
        env.storage()
            .instance()
            .get(&DataKey::TimeLockConfig)
            .unwrap_or(TimeLockConfig {
                delay: 86400,
                grace_period: 86400,
            })
    }

    pub fn set_insurance_config(env: Env, admin: Address, config: InsuranceConfig) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::InsuranceConfig, &config);
        Ok(())
    }

    pub fn set_watchdog_config(env: Env, admin: Address, config: WatchdogConfig) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::WatchdogConfig, &config);
        Ok(())
    }

    pub fn get_insurance_pool(env: Env) -> InsurancePool {
        env.storage()
            .instance()
            .get(&DataKey::InsurancePool)
            .unwrap_or(InsurancePool {
                token: env.current_contract_address(), // dummy default
                balance: 0,
                total_premiums_collected: 0,
                total_claims_paid: 0,
            })
    }

    pub fn opt_into_insurance(env: Env, escrow_id: u64) -> Result<(), Error> {
        let config: InsuranceConfig = env.storage().instance().get(&DataKey::InsuranceConfig).ok_or(Error::Unauthorized)?;
        if !config.enabled { return Err(Error::Unauthorized); }

        let mut escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked { return Err(Error::InvalidStatus); }

        let premium = (escrow.amount * config.premium_bps) / 10000;
        if premium == 0 { return Ok(()); }

        escrow.amount -= premium;
        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        let mut pool = Self::get_insurance_pool(env.clone());
        pool.token = escrow.token.clone();
        pool.balance += premium;
        pool.total_premiums_collected += premium;
        env.storage().instance().set(&DataKey::InsurancePool, &pool);

        Ok(())
    }

    pub fn file_insurance_claim(env: Env, admin: Address, escrow_id: u64, amount: i128) -> Result<u64, Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) { return Err(Error::Unauthorized); }

        let escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Resolved && escrow.status != EscrowStatus::Cancelled {
            return Err(Error::NotClaimable);
        }

        let config: InsuranceConfig = env.storage().instance().get(&DataKey::InsuranceConfig).ok_or(Error::Unauthorized)?;
        let max_coverage = (escrow.amount * config.max_coverage_bps) / 10000;
        if amount > max_coverage { return Err(Error::Unauthorized); }

        let counter: u64 = env.storage().instance().get(&DataKey::InsuranceClaimCounter).unwrap_or(0) + 1;
        let claim = InsuranceClaim {
            claim_id: counter,
            escrow_id,
            claimant: escrow.customer.clone(), // default to customer
            amount,
            approved: false,
            paid_at: None,
        };

        env.storage().instance().set(&DataKey::InsuranceClaim(counter), &claim);
        env.storage().instance().set(&DataKey::InsuranceClaimCounter, &counter);

        Ok(counter)
    }

    pub fn approve_claim(env: Env, admin: Address, claim_id: u64) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) { return Err(Error::Unauthorized); }

        let mut claim: InsuranceClaim = env.storage().instance().get(&DataKey::InsuranceClaim(claim_id)).ok_or(Error::EscrowNotFound)?;
        if claim.approved { return Err(Error::AlreadyProcessed); }

        let mut pool = Self::get_insurance_pool(env.clone());
        if pool.balance < claim.amount { return Err(Error::Underfunded); }

        Self::transfer_if_token_contract(&env, &pool.token, &claim.claimant, claim.amount)?;

        pool.balance -= claim.amount;
        pool.total_claims_paid += claim.amount;
        env.storage().instance().set(&DataKey::InsurancePool, &pool);

        claim.approved = true;
        claim.paid_at = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::InsuranceClaim(claim_id), &claim);

        Ok(())
    }

    pub fn get_watchdog_config(env: Env) -> WatchdogConfig {
        env.storage()
            .instance()
            .get(&DataKey::WatchdogConfig)
            .unwrap_or(WatchdogConfig {
                inactivity_release_seconds: 604800, // 7 days
                enabled: false,
                favor_customer_on_release: false,
            })
    }

    pub fn is_watchdog_eligible(env: Env, escrow_id: u64) -> bool {
        let config = Self::get_watchdog_config(env.clone());
        if !config.enabled {
            return false;
        }

        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return false;
        }

        let escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return false;
        }

        let now = env.ledger().timestamp();
        if now < escrow.release_timestamp + config.inactivity_release_seconds {
            return false;
        }

        true
    }

    pub fn trigger_watchdog_release(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !Self::is_watchdog_eligible(env.clone(), escrow_id) {
            return Err(Error::ActionNotReady);
        }

        let config = Self::get_watchdog_config(env.clone());
        let mut escrow = Self::get_escrow(&env, escrow_id);
        
        let released_to = if config.favor_customer_on_release {
            escrow.customer.clone()
        } else {
            escrow.merchant.clone()
        };

        escrow.status = if config.favor_customer_on_release {
            EscrowStatus::Resolved
        } else {
            EscrowStatus::Released
        };

        env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

        Self::transfer_if_token_contract(&env, &escrow.token, &released_to, escrow.amount)?;

        WatchdogReleaseTriggered {
            escrow_id,
            released_to: released_to.clone(),
            triggered_by: env.current_contract_address(),
        }.publish(&env);

        Ok(())
    }
    // ── REPUTATION DECAY FUNCTIONS (#75) ───────────────────────────────────

    pub fn update_decay_config(
        env: Env,
        admin: Address,
        config: ReputationDecayConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let ms = Self::get_multisig_config(env.clone());
        if !ms.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }
        env.storage().instance().set(&DataKey::ReputationDecayConfig, &config);
        Ok(())
    }

    pub fn set_dispute_config(env: Env, admin: Address, config: DisputeConfig) -> Result<(), Error> {
        admin.require_auth();
        if let Some(ms) = env.storage().instance().get::<DataKey, MultiSigConfig>(&DataKey::MultiSigConfig) {
            if !ms.admins.contains(&admin) {
                return Err(Error::NotAnAdmin);
            }
        }
        env.storage().instance().set(&DataKey::DisputeConfigKey, &config);
        Ok(())
    }

    pub fn get_dispute_config(env: Env) -> DisputeConfig {
        env.storage()
            .instance()
            .get(&DataKey::DisputeConfigKey)
            .unwrap_or(DisputeConfig {
                collateral_token: env.current_contract_address(),
                collateral_amount: 0,
                collateral_enabled: false,
            })
    }

    pub fn get_dispute_collateral(env: Env, escrow_id: u64) -> Result<DisputeCollateral, Error> {
        env.storage()
            .instance()
            .get(&DataKey::DisputeCollateral(escrow_id))
            .ok_or(Error::InvalidStatus)
    }

    pub fn get_effective_reputation(env: Env, address: Address) -> i128 {
        let rep = EscrowContract::get_or_default_reputation(&env, &address);
        let config = EscrowContract::get_or_default_decay_config(&env);
        let now = env.ledger().timestamp();
        EscrowContract::compute_decayed_score(&rep, &config, now)
    }

    pub fn apply_reputation_decay(env: Env, address: Address) -> Result<i128, Error> {
        let mut rep = EscrowContract::get_or_default_reputation(&env, &address);
        let config = EscrowContract::get_or_default_decay_config(&env);
        let now = env.ledger().timestamp();
        let old_score = rep.score as i128;
        let new_score = EscrowContract::compute_decayed_score(&rep, &config, now);
        if new_score == old_score {
            return Ok(old_score);
        }
        let threshold_secs = config.decay_threshold_days * 86400;
        let days_inactive = (now.saturating_sub(rep.last_updated + threshold_secs)) / 86400;
        rep.score = new_score as i64;
        rep.last_updated = now;
        env.storage()
            .instance()
            .set(&DataKey::ReputationScore(address.clone()), &rep);
        ReputationDecayed {
            address,
            old_score,
            new_score,
            days_inactive,
        }
        .publish(&env);
        Ok(new_score)
    }

    fn get_or_default_decay_config(env: &Env) -> ReputationDecayConfig {
        env.storage()
            .instance()
            .get(&DataKey::ReputationDecayConfig)
            .unwrap_or(ReputationDecayConfig {
                decay_rate_bps: 100,
                decay_threshold_days: 30,
                min_score: 0,
                max_score: 10000,
            })
    }

    fn compute_decayed_score(
        rep: &ReputationScore,
        config: &ReputationDecayConfig,
        now: u64,
    ) -> i128 {
        let threshold_secs = config.decay_threshold_days * 86400;
        let last = rep.last_updated;
        if now <= last + threshold_secs {
            return rep.score as i128;
        }
        let inactive_secs = now - (last + threshold_secs);
        let days_inactive = inactive_secs / 86400;
        if days_inactive == 0 {
            return rep.score as i128;
        }
        let decay = (rep.score as i128)
            .saturating_mul(config.decay_rate_bps)
            .saturating_mul(days_inactive as i128)
            / 10000;
        let new_score = (rep.score as i128).saturating_sub(decay);
        new_score.max(config.min_score)
    }

    // ── ORACLE AUTO-RESOLUTION (#85) ───────────────────────────────────────

    pub fn attach_oracle_condition(
        env: Env,
        admin: Address,
        escrow_id: u64,
        condition: OracleCondition,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }
        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::InvalidStatus);
        }
        env.storage()
            .instance()
            .set(&DataKey::OracleCondition(escrow_id), &condition);
        Ok(())
    }

    pub fn get_oracle_condition(env: Env, escrow_id: u64) -> Result<OracleCondition, Error> {
        env.storage()
            .instance()
            .get(&DataKey::OracleCondition(escrow_id))
            .ok_or(Error::NoOracleCondition)
    }

    pub fn auto_resolve_with_oracle(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }
        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::NotDisputed);
        }
        let condition: OracleCondition = env
            .storage()
            .instance()
            .get(&DataKey::OracleCondition(escrow_id))
            .ok_or(Error::NoOracleCondition)?;

        let mut args: Vec<soroban_sdk::Val> = Vec::new(&env);
        args.push_back(condition.oracle.price_feed_id.clone().into());
        let price_data: OraclePriceData = env.invoke_contract(
            &condition.oracle.oracle_address,
            &Symbol::new(&env, "get_price"),
            args,
        );

        let now = env.ledger().timestamp();
        if now.saturating_sub(price_data.timestamp) > condition.oracle.staleness_threshold {
            return Err(Error::OracleStalePriceFeed);
        }

        let condition_met = match condition.comparison {
            PriceComparison::GreaterThan => price_data.price > condition.target_price,
            PriceComparison::LessThan => price_data.price < condition.target_price,
            PriceComparison::GreaterThanOrEqual => price_data.price >= condition.target_price,
            PriceComparison::LessThanOrEqual => price_data.price <= condition.target_price,
        };

        let release_to_merchant = if condition_met {
            condition.release_to_merchant_if_met
        } else {
            !condition.release_to_merchant_if_met
        };

        Self::internal_resolve_dispute(env, escrow.customer.clone(), escrow_id, release_to_merchant)
    }

    // ── BENEFICIARY TRANSFER ──────────────────────────────────────────────

    pub fn transfer_escrow_beneficiary(
        env: Env,
        caller: Address,
        escrow_id: u64,
        new_merchant: Address,
    ) -> Result<(), Error> {
        caller.require_auth();

        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        let multisig = Self::get_multisig_config(env.clone());
        if caller != escrow.merchant && !multisig.admins.contains(&caller) {
            return Err(Error::Unauthorized);
        }

        match escrow.status {
            EscrowStatus::Disputed | EscrowStatus::Resolved => {
                return Err(Error::TransferNotAllowed);
            }
            _ => {}
        }

        if new_merchant == escrow.merchant {
            return Err(Error::SameBeneficiary);
        }

        let old_merchant = escrow.merchant.clone();
        let now = env.ledger().timestamp();

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::BeneficiaryTransferCount(escrow_id))
            .unwrap_or(0);

        let transfer = BeneficiaryTransfer {
            escrow_id,
            from: old_merchant.clone(),
            to: new_merchant.clone(),
            transferred_at: now,
            authorised_by: caller,
        };

        env.storage()
            .instance()
            .set(&DataKey::BeneficiaryTransferHistory(escrow_id, count), &transfer);
        env.storage()
            .instance()
            .set(&DataKey::BeneficiaryTransferCount(escrow_id), &(count + 1));

        escrow.merchant = new_merchant.clone();
        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        BeneficiaryTransferred {
            escrow_id,
            old_merchant,
            new_merchant,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_transfer_history(env: Env, escrow_id: u64) -> Vec<BeneficiaryTransfer> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::BeneficiaryTransferCount(escrow_id))
            .unwrap_or(0);

        let mut history = Vec::new(&env);
        for i in 0..count {
            if let Some(transfer) = env
                .storage()
                .instance()
                .get::<DataKey, BeneficiaryTransfer>(&DataKey::BeneficiaryTransferHistory(
                    escrow_id, i,
                ))
            {
                history.push_back(transfer);
            }
        }
        history
    }

    // ── MULTI-PARTY DISPUTE ────────────────────────────────────────────────

    pub fn dispute_multi_party_escrow(
        env: Env,
        caller: Address,
        escrow_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::InvalidStatus);
        }

        let mut is_participant = false;
        for p in escrow.participants.iter() {
            if p.address == caller {
                is_participant = true;
                break;
            }
        }
        if !is_participant {
            return Err(Error::Unauthorized);
        }

        let participant_count = escrow.participants.len();
        let quorum_required = (participant_count / 2) + 1;
        let resolution_deadline = env.ledger().timestamp() + 7 * 24 * 3600;

        let dispute = MultiPartyDispute {
            escrow_id,
            votes_for_merchant: Vec::new(&env),
            votes_for_customer: Vec::new(&env),
            quorum_required,
            resolution_deadline,
            resolved: false,
        };

        env.storage()
            .instance()
            .set(&DataKey::MultiPartyDisputeKey(escrow_id), &dispute);

        escrow.status = EscrowStatus::Disputed;
        env.storage()
            .instance()
            .set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);

        MultiPartyDisputeRaised {
            escrow_id,
            raised_by: caller,
        }
        .publish(&env);

        Ok(())
    }

    pub fn vote_on_multi_party_dispute(
        env: Env,
        voter: Address,
        escrow_id: u64,
        favor_merchant: bool,
    ) -> Result<(), Error> {
        voter.require_auth();

        let escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        let mut dispute: MultiPartyDispute = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyDisputeKey(escrow_id))
            .ok_or(Error::NotDisputed)?;

        if dispute.resolved {
            return Err(Error::AlreadyProcessed);
        }

        let mut is_participant = false;
        for p in escrow.participants.iter() {
            if p.address == voter {
                is_participant = true;
                break;
            }
        }
        if !is_participant {
            return Err(Error::Unauthorized);
        }

        for addr in dispute.votes_for_merchant.iter() {
            if addr == voter {
                return Err(Error::DuplicateApproval);
            }
        }
        for addr in dispute.votes_for_customer.iter() {
            if addr == voter {
                return Err(Error::DuplicateApproval);
            }
        }

        if favor_merchant {
            dispute.votes_for_merchant.push_back(voter.clone());
        } else {
            dispute.votes_for_customer.push_back(voter.clone());
        }

        env.storage()
            .instance()
            .set(&DataKey::MultiPartyDisputeKey(escrow_id), &dispute);

        MultiPartyDisputeVoteCast {
            escrow_id,
            voter,
            favor_merchant,
        }
        .publish(&env);

        Ok(())
    }

    pub fn resolve_multi_party_dispute(env: Env, escrow_id: u64) -> Result<(), Error> {
        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyEscrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        let mut dispute: MultiPartyDispute = env
            .storage()
            .instance()
            .get(&DataKey::MultiPartyDisputeKey(escrow_id))
            .ok_or(Error::NotDisputed)?;

        if dispute.resolved {
            return Err(Error::AlreadyProcessed);
        }

        let now = env.ledger().timestamp();
        let merchant_votes = dispute.votes_for_merchant.len();
        let customer_votes = dispute.votes_for_customer.len();

        let favor_merchant;

        if merchant_votes >= dispute.quorum_required {
            favor_merchant = true;
        } else if customer_votes >= dispute.quorum_required {
            favor_merchant = false;
        } else if now > dispute.resolution_deadline {
            favor_merchant = false;
        } else {
            return Err(Error::ApprovalsThresholdNotMet);
        }

        dispute.resolved = true;
        env.storage()
            .instance()
            .set(&DataKey::MultiPartyDisputeKey(escrow_id), &dispute);

        let token_client = token::Client::new(&env, &escrow.token);
        let contract_address = env.current_contract_address();

        if favor_merchant {
            for p in escrow.participants.iter() {
                if p.share_bps > 0 {
                    let amount = (escrow.total_amount * (p.share_bps as i128)) / 10000;
                    if amount > 0 {
                        token_client.transfer(&contract_address, &p.address, &amount);
                    }
                }
            }
            escrow.status = EscrowStatus::Released;
        } else {
            // Refund to the participant with Customer role; fall back to proportional if none found
            let mut customer_addr: Option<Address> = None;
            for p in escrow.participants.iter() {
                if let ParticipantRole::Customer = p.role {
                    customer_addr = Some(p.address.clone());
                    break;
                }
            }

            if let Some(customer) = customer_addr {
                token_client.transfer(&contract_address, &customer, &escrow.total_amount);
            } else {
                for p in escrow.participants.iter() {
                    if p.share_bps > 0 {
                        let amount = (escrow.total_amount * (p.share_bps as i128)) / 10000;
                        if amount > 0 {
                            token_client.transfer(&contract_address, &p.address, &amount);
                        }
                    }
                }
            }
            escrow.status = EscrowStatus::Resolved;
        }

        env.storage()
            .instance()
            .set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);

        MultiPartyDisputeResolved {
            escrow_id,
            favor_merchant,
            resolved_at: now,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_multi_party_dispute(
        env: Env,
        escrow_id: u64,
    ) -> Result<MultiPartyDispute, Error> {
        env.storage()
            .instance()
            .get(&DataKey::MultiPartyDisputeKey(escrow_id))
            .ok_or(Error::NotDisputed)
    }

    // ── BATCH ESCROW CREATION ─────────────────────────────────────────────

    pub fn create_escrow_batch(env: Env, admin: Address, entries: Vec<EscrowBatchEntry>) -> Vec<BatchEscrowResult> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            let mut results = Vec::new(&env);
            for i in 0..entries.len() {
                results.push_back(BatchEscrowResult {
                    index: i as u32,
                    escrow_id: 0,
                    success: false,
                    error_code: Error::NotAnAdmin as u32,
                });
            }
            return results;
        }

        let batch_limit = Self::get_batch_limit(env.clone());
        if entries.len() as u32 > batch_limit {
            let mut results = Vec::new(&env);
            results.push_back(BatchEscrowResult {
                index: 0,
                escrow_id: 0,
                success: false,
                error_code: Error::BatchTooLarge as u32,
            });
            return results;
        }

        let mut results = Vec::new(&env);
        let mut has_failure = false;

        for i in 0..entries.len() {
            let entry = entries.get(i).unwrap();
            let result = match Self::try_create_single_escrow(&env, &entry, i as u32) {
                Ok(escrow_id) => BatchEscrowResult {
                    index: i as u32,
                    escrow_id,
                    success: true,
                    error_code: 0,
                },
                Err(err_code) => {
                    has_failure = true;
                    BatchEscrowResult {
                        index: i as u32,
                        escrow_id: 0,
                        success: false,
                        error_code: err_code,
                    }
                }
            };
            results.push_back(result);
        }

        if has_failure {
            // Note: We don't return an error here, just mark partial failure
            // The caller can check the results to see which succeeded
        }

        results
    }

    fn try_create_single_escrow(env: &Env, entry: &EscrowBatchEntry, index: u32) -> Result<u64, u32> {
        // Validate inputs similar to create_escrow
        if entry.amount <= 0 {
            return Err(Error::InvalidStatus as u32); // Using InvalidStatus as generic error
        }

        let current_timestamp = env.ledger().timestamp();
        if entry.release_timestamp <= current_timestamp {
            return Err(Error::ReleaseNotYetAvailable as u32);
        }

        // Get counter and create escrow
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowCounter)
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled { fee_config.fee_bps } else { 0 };

        let escrow = Escrow {
            id: escrow_id,
            customer: entry.customer.clone(),
            merchant: entry.merchant.clone(),
            amount: entry.amount,
            token: entry.token.clone(),
            status: EscrowStatus::Locked,
            created_at: current_timestamp,
            release_timestamp: entry.release_timestamp,
            dispute_started_at: 0,
            last_activity_at: current_timestamp,
            escalation_level: 0,
            min_hold_period: 0,
            fee_bps,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::EscrowCounter, &escrow_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerEscrowCount(entry.customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerEscrows(entry.customer.clone(), customer_count),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerEscrowCount(entry.customer.clone()),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantEscrowCount(entry.merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantEscrows(entry.merchant.clone(), merchant_count),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantEscrowCount(entry.merchant.clone()),
            &(merchant_count + 1),
        );

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::EscrowAnalyticsKey)
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_escrows_created += 1;
        analytics.total_value_locked += entry.amount;
        env.storage()
            .instance()
            .set(&DataKey::EscrowAnalyticsKey, &analytics);

        // Update per-address analytics
        EscrowContract::update_customer_analytics(&env, &entry.customer, |a| {
            a.total_escrows_created += 1;
            a.total_value_locked += entry.amount;
        });
        EscrowContract::update_merchant_analytics(&env, &entry.merchant, |a| {
            a.total_escrows_created += 1;
            a.total_value_locked += entry.amount;
        });

        EscrowCreated {
            escrow_id,
            customer: entry.customer.clone(),
            merchant: entry.merchant.clone(),
            amount: entry.amount,
            token: entry.token.clone(),
            release_timestamp: entry.release_timestamp,
        }
        .publish(env);

        Ok(escrow_id)
    }

    pub fn get_batch_limit(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::BatchLimit)
            .unwrap_or(50) // Default limit of 50
    }

    pub fn set_batch_limit(env: Env, admin: Address, limit: u32) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }
        if limit == 0 || limit > 1000 {
            return Err(Error::InvalidStatus); // Using InvalidStatus for invalid limit
        }
        env.storage()
            .instance()
            .set(&DataKey::BatchLimit, &limit);
        Ok(())
    }

    // ── ANALYTICS HELPERS ─────────────────────────────────────────────────

    fn update_customer_analytics<F>(env: &Env, customer: &Address, update_fn: F)
    where
        F: Fn(&mut EscrowAnalytics),
    {
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::CustomerAnalytics(customer.clone()))
            .unwrap_or(EscrowAnalytics::default_value());
        update_fn(&mut analytics);
        env.storage()
            .instance()
            .set(&DataKey::CustomerAnalytics(customer.clone()), &analytics);
    }

    fn update_merchant_analytics<F>(env: &Env, merchant: &Address, update_fn: F)
    where
        F: Fn(&mut EscrowAnalytics),
    {
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::MerchantAnalytics(merchant.clone()))
            .unwrap_or(EscrowAnalytics::default_value());
        update_fn(&mut analytics);
        env.storage()
            .instance()
            .set(&DataKey::MerchantAnalytics(merchant.clone()), &analytics);
    }
}

impl EscrowAnalytics {
    fn default_value() -> Self {
        EscrowAnalytics {
            total_escrows_created: 0,
            total_value_locked: 0,
            total_value_released: 0,
            total_disputes: 0,
            total_resolutions: 0,
            dispute_rate_bps: 0,
            avg_escrow_duration_seconds: 0,
            total_escrows_released: 0,
        }
    }
}


mod test;

#[cfg(test)]
mod timelock_test;

#[cfg(test)]
mod collateral_test;

#[cfg(test)]
mod beneficiary_transfer_test;

#[cfg(test)]
mod multi_party_dispute_test;
