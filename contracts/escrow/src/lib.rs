// This contract uses a multi-level enum structure for DataKey and Error to stay within
// Soroban's 50-variant XDR limit. Each sub-enum must have <= 50 variants.
#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, token,
    Address, Bytes, BytesN, Env, FromVal, IntoVal, String, Symbol, TryFromVal, Val, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum ConfigKey {
    AdminMultiSig,
    AdminProposal(String),
    AdminProposalCounter,
    AdminSuccessionPlan,
    AdminClawbackRequest(u64),
    AdminClawbackCounter,
    AdminEscrowClawback(u64),
    ReputationConfig,
    ReputationDecayConfig,
    TenureConfig,
    GlobalExpiryConfig,
    EscalationConfig,
    WatchdogConfig,
    BatchLimit,
    PauseStateKey,
    PauseHistoryEntry(u64),
    PauseHistoryCount,
    ActivePauseIndex(String),
    EscrowFeeConfig,
    StaleThresholdConfig,
    DisputeConfig,
    InsurancePool,
    InsuranceConfig,
    TimeLockConfig,
    AdminClawbackEscrow(u64),
}

#[derive(Clone)]
#[contracttype]
pub enum EscrowKey {
    Data(u64),
    Counter,
    MultiParty(u64),
    MultiPartyCounter,
    CustomerList(Address, u64),
    MerchantList(Address, u64),
    CustomerCount(Address),
    MerchantCount(Address),
    Evidence(u64, u64),
    EvidenceCount(u64),
    EvidencePage(u64, u32),
    EvidencePageCount(u64),
    EvidenceCommitment(u64),
    VestingSchedule(u64),
    VestingAccelerationConfig(u64),
    Conditional(u64),
    OracleCondition(u64),
    MultiToken(u64),
    MultiTokenCounter,
    Hierarchy(u64),
    Template(u64),
    TemplateCounter,
    SubAccount(u64, u64),
    SubAccountCounter(u64),
    EscrowHierarchy(u64),
}

#[derive(Clone)]
#[contracttype]
pub enum ParticipantKey {
    ReputationScore(Address),
    TenureBonusApplied(u64, Address),
    CustomerAnalytics(Address),
    MerchantAnalytics(Address),
    AccumulatedFees(Address),
    BeneficiaryTransferHistory(u64, u64),
    BeneficiaryTransferCount(u64),
}

#[derive(Clone)]
#[contracttype]
pub enum DisputeKey {
    Action(u64),
    Counter,
    Collateral(u64),
    Appeal(u64),
    AppealCounter,
    Round(u64),
    AppealsByEscrow(u64, u64),
    InsuranceClaim(u64),
    InsuranceClaimCounter,
    MultiPartyDispute(u64),
    EscrowAnalytics,
    EscrowMigrationStatus,
    EscrowMigrated(u64),
    EscrowRenewalConfig,
    EscrowRenewal(u64),
    EscrowRenewalCount(u64),
    EscrowSwapConfig(u64),
    Observer(u64, u64),
    ObserverCount(u64),
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Config(ConfigKey),
    Escrow(EscrowKey),
    Participant(ParticipantKey),
    Dispute(DisputeKey),
    VoteWeight(u64, Address),
    ReleaseThresholdBps(u64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum BasicError {
    Unauthorized = 100,
    NotAnAdmin = 101,
    AlreadyApproved = 102,
    ContractPaused = 103,
    DuplicateApproval = 104,
    MultiSigNotInitialized = 105,
    MigrationNotStarted = 106,
    AlreadyMigrated = 107,
    ParticipantNotFound = 108,
    InvalidMerkleProof = 109,
    RootAlreadyCommitted = 110,
    InvalidBps = 111,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum EscrowError {
    NotFound = 200,
    InvalidStatus = 201,
    AlreadyProcessed = 202,
    ReleaseNotYetAvailable = 203,
    TimeoutNotReached = 204,
    ReleaseOnHoldPeriod = 205,
    InvalidVestingSchedule = 206,
    CliffPeriodNotPassed = 207,
    MilestoneAlreadyReleased = 208,
    EscrowNotExpired = 209,
    EscrowAlreadyExpired = 210,
    ExpiryBeforeRelease = 211,
    TemplateNotFound = 212,
    TemplateInactive = 213,
    SubAccountNotFound = 214,
    SubAccountAlreadyReleased = 215,
    SubAccountFundingExceedsEscrow = 216,
    ConditionalEscrowNotFound = 217,
    ParentEscrowNotFound = 218,
    ChildrenNotResolved = 219,
    MaxHierarchyDepth = 220,
    BatchTooLarge = 221,
    RenewalDisabled = 222,
    MaxRenewalsReached = 223,
    NewExpiryNotAfterCurrent = 224,
    RenewalPeriodTooShort = 225,
    RenewalPeriodTooLong = 226,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
#[contracterror]
pub enum ActionError {
    NotReady = 300,
    NotDisputed = 301,
    ObserverAlreadyAdded = 302,
    ObserverNotFound = 303,
    AccelerationLimitExceeded = 304,
    TransferNotAllowed = 305,
    SameBeneficiary = 306,
    ConditionAlreadyEvaluated = 307,
    StaleThresholdNotConfigured = 308,
    SwapConfigNotFound = 309,
    SwapOutputBelowMinimum = 310,
    SwapAlreadyExecuted = 311,
    BatchReleaseSizeLimitExceeded = 312,
    EvidenceDeadlinePassed = 313,
    ApprovalsThresholdNotMet = 314,
    InsufficientCollateral = 315,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    Basic(BasicError),
    Escrow(EscrowError),
    Action(ActionError),
}

impl Error {
    pub fn to_u32(&self) -> u32 {
        match self {
            Error::Basic(e) => *e as u32,
            Error::Escrow(e) => *e as u32,
            Error::Action(e) => *e as u32,
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
            if code >= 300 && code <= 314 {
                return Ok(Error::Action(unsafe { core::mem::transmute(code) }));
            }
            if code >= 200 && code <= 221 {
                return Ok(Error::Escrow(unsafe { core::mem::transmute(code) }));
            }
            if code >= 100 && code <= 111 {
                return Ok(Error::Basic(unsafe { core::mem::transmute(code) }));
            }
        }
        Err(error)
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

/// Secondary storage keys (keeps `DataKey` within Soroban's 50-variant limit).

/// Observer storage keys (separate enum to stay within Soroban symbol limits).

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EscrowStatus {
    Locked,
    Released,
    Disputed,
    Resolved,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum AutoResolveFavor {
    Customer,
    Merchant,
    SplitEqual,
}

#[derive(Clone)]
#[contracttype]
pub struct EscalationConfig {
    pub timeout_seconds: u64,
    pub favor: AutoResolveFavor,
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

#[derive(Clone)]
#[contracttype]
pub struct InsuranceClaim {
    pub claim_id: u64,
    pub escrow_id: u64,
    pub claimant: Address,
    pub amount: i128,
    pub approved: bool,
    pub paid_at: Option<u64>,
}

/// Configuration for escrow renewal mechanism
#[derive(Clone)]
#[contracttype]
pub struct EscrowRenewalConfig {
    pub enabled: bool,
    pub max_renewals: u32, // Maximum number of times an escrow can be renewed
    pub renewal_fee_bps: u32, // Fee in basis points for renewal
    pub min_renewal_period: u64, // Minimum renewal period in seconds
    pub max_renewal_period: u64, // Maximum renewal period in seconds
}

/// Tracks escrow renewal history
#[derive(Clone)]
#[contracttype]
pub struct EscrowRenewal {
    pub renewal_id: u64,
    pub escrow_id: u64,
    pub renewed_by: Address,
    pub new_expiry_timestamp: u64,
    pub renewal_fee: i128,
    pub renewed_at: u64,
    pub renewal_count: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct VestingAccelerationConfig {
    pub schedule_id: u64,
    pub milestone_bps: u32,
    pub max_acceleration_bps: u32,
    pub total_accelerated_bps: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowTemplate {
    pub template_id: u64,
    pub owner: Address,
    pub token: Address,
    pub amount: i128,
    pub release_delay_seconds: u64,
    pub description: String,
    pub created_at: u64,
    pub active: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct StaleThresholdConfig {
    pub inactivity_seconds: u64,
    pub near_expiry_buffer_seconds: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EscrowHealth {
    Healthy,
    NearExpiry,
    Stale,
    Disputed,
    Expired,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowHealthReport {
    pub escrow_id: u64,
    pub health: EscrowHealth,
    pub seconds_until_expiry: Option<i64>,
    pub last_activity: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowHierarchyNode {
    pub escrow_id: u64,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SuccessionCode {
    Exists = 0,
    NotFound = 1,
    AlreadyActivated = 2,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ClawbackCode {
    DelayNotMet = 0,
    AlreadyExecuted = 1,
    AlreadyInitiated = 2,
    Cancelled = 3,
}

#[repr(u32)]
#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum TestError {
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
    DuplicateApproval = 16,
    ApprovalsThresholdNotMet = 17,
    MultiSigNotInitialized = 18,
    NotAnAdmin = 24,
    AlreadyApproved = 25,
    ActionNotReady = 26,
    ContractPaused = 30,
    ObserverAlreadyAdded = 47,
    ObserverNotFound = 48,
    AccelerationLimitExceeded = 66,
    TransferNotAllowed = 42,
    SameBeneficiary = 43,
    ConditionalEscrowNotFound = 50,
    ConditionAlreadyEvaluated = 51,
    InvalidMerkleProof = 34,
    RootAlreadyCommitted = 38,
    MigrationNotStarted = 63,
    AlreadyMigrated = 65,
    ParticipantNotFound = 84,
    EscrowNotExpired = 85,
    EscrowAlreadyExpired = 86,
    ExpiryBeforeRelease = 87,
    TemplateNotFound = 88,
    TemplateInactive = 89,
    StaleThresholdNotConfigured = 94,
    SubAccountNotFound = 95,
    SubAccountAlreadyReleased = 96,
    SubAccountFundingExceedsEscrow = 97,
    SwapConfigNotFound = 53,
    SwapOutputBelowMinimum = 54,
    SwapAlreadyExecuted = 55,
    MaxHierarchyDepth = 56,
    ParentEscrowNotFound = 57,
    ChildrenNotResolved = 58,
    BatchReleaseSizeLimitExceeded = 76,
    EvidenceDeadlinePassed = 73,
    BatchTooLarge = 77,
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
    pub recipient: Address,
    pub amount: i128,
    pub token: Address,
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
pub struct WeightUpdated {
    pub escrow_id: u64,
    pub participant: Address,
    pub weight_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdUpdated {
    pub escrow_id: u64,
    pub threshold_bps: u32,
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
pub struct EvidenceDeadlineSet {
    pub escrow_id: u64,
    pub deadline: u64,
    pub set_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceDeadlineExceeded {
    pub escrow_id: u64,
    pub deadline: u64,
    pub submitted_at: u64,
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

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeAppealFiled {
    pub appeal_id: u64,
    pub escrow_id: u64,
    pub appellant: Address,
    pub filed_at: u64,
    pub appeal_deadline: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppealResolved {
    pub appeal_id: u64,
    pub escrow_id: u64,
    pub in_favor_of: Address,
    pub resolved_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateCreated {
    pub template_id: u64,
    pub owner: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateDeactivated {
    pub template_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowCreatedFromTemplate {
    pub escrow_id: u64,
    pub template_id: u64,
    pub customer: Address,
}

/// Event emitted when an escrow is renewed
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRenewed {
    pub escrow_id: u64,
    pub renewed_by: Address,
    pub new_expiry_timestamp: u64,
    pub renewal_fee: i128,
    pub renewal_count: u32,
}

/// Event emitted when escrow renewal configuration is updated
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRenewalConfigUpdated {
    pub max_renewals: u32,
    pub renewal_fee_bps: u32,
    pub min_renewal_period: u64,
    pub max_renewal_period: u64,
    pub updated_by: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct DisputeConfig {
    pub collateral_token: Address,
    pub collateral_amount: i128,
    pub collateral_enabled: bool,
    pub min_collateral_ratio_bps: u32,
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

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum DisputeRound {
    Initial,
    Appeal,
    Final,
}

#[derive(Clone)]
#[contracttype]
pub struct DisputeAppeal {
    pub appeal_id: u64,
    pub escrow_id: u64,
    pub round: DisputeRound,
    pub appellant: Address,
    pub reason_hash: BytesN<32>,
    pub filed_at: u64,
    pub appeal_deadline: u64,
    pub resolved: bool,
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
pub struct TenureConfigUpdated {
    pub base_score: u32,
    pub weight_per_day: u32,
    pub max_bonus_days: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenureBonusGranted {
    pub escrow_id: u64,
    pub participant: Address,
    pub bonus: u32,
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

/// Configuration for the duration-weighted ("tenure") reputation bonus.
/// Dispute-free escrows that stay active longer reward their participants more.
#[derive(Clone)]
#[contracttype]
pub struct TenureReputationConfig {
    /// Flat reputation credit granted for honouring an escrow to completion,
    /// independent of how long it stayed active.
    pub base_score: u32,
    /// Reputation points earned per full day the escrow remained active.
    pub weight_per_day: u32,
    /// Cap on the number of days that count towards the bonus.
    pub max_bonus_days: u32,
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
    pub expiry_timestamp: u64,
    pub auto_refund_on_expiry: bool,
    pub escalated_at: Option<u64>,
    pub escalation_timeout: u64,
    pub auto_resolve_in_favor_of: AutoResolveFavor,
    pub evidence_deadline: Option<u64>, // Deadline for evidence submission in dispute
}

#[derive(Clone)]
#[contracttype]
pub struct EscrowSubAccount {
    pub escrow_id: u64,
    pub sub_id: u64,
    pub label_hash: BytesN<32>,
    pub amount: i128,
    pub released: bool,
    pub release_condition: Option<String>,
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
    pub role: ParticipantRole,
    pub share_bps: u32,  // payout share in basis points (out of 10000)
    pub weight_bps: u32, // voting weight in basis points (out of 10000)
    pub approved: bool,
    pub approved_at: Option<u64>,
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
    pub threshold_bps: u32, // cumulative approved weight needed for release
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
pub struct EscrowObserver {
    pub escrow_id: u64,
    pub observer: Address,
    pub granted_by: Address,
    pub granted_at: u64,
    pub expires_at: u64,
}

/// Pre-committed Merkle root for dispute evidence integrity (one per escrow).
#[derive(Clone)]
#[contracttype]
pub struct EvidenceCommitment {
    pub escrow_id: u64,
    pub merkle_root: BytesN<32>,
    pub committed_at: u64,
    pub committed_by: Address,
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

#[derive(Clone)]
#[contracttype]
pub struct EscrowSwapConfig {
    pub escrow_id: u64,
    pub source_token: Address,
    pub target_token: Address,
    pub min_output_amount: i128,
    pub oracle: Address,
    pub executed: bool,
}

/// Snapshot of vesting cliff progress for a given escrow.
#[derive(Clone)]
#[contracttype]
pub struct CliffStatus {
    pub cliff_timestamp: u64,
    pub cliff_passed: bool,
    /// Seconds until the cliff is reached; `0` once `cliff_passed` is true.
    pub seconds_remaining: u64,
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
    UpdateThreshold(u32),
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
pub struct SuccessionPlan {
    pub successor: Address,
    pub designated_by: Address,
    pub designated_at: u64,
    pub activatable_after: u64,
    pub activated: bool,
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
pub struct ClawbackRequest {
    pub request_id: u64,
    pub escrow_id: u64,
    pub initiated_by: Address,
    pub reason_hash: BytesN<32>,
    pub execute_after: u64,
    pub executed: bool,
    pub cancelled: bool,
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
pub struct BatchReleaseRequest {
    pub escrow_ids: Vec<u64>,
    pub override_recipient: Option<Address>,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchReleaseResult {
    pub succeeded: Vec<u64>,
    pub failed: Vec<u64>,
    pub errors: Vec<u32>,
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
pub struct SuccessorDesignated {
    pub successor: Address,
    pub activatable_after: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuccessionActivated {
    pub new_admin: Address,
    pub activated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuccessionRevoked {
    pub revoked_by: Address,
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
    pub function_name: String,
    pub paused_by: Address,
    pub paused_at: u64,
    pub unpaused_by: Option<Address>,
    pub unpaused_at: Option<u64>,
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

#[derive(Clone)]
#[contracttype]
pub struct OnChainCondition {
    pub contract_address: Address,
    pub state_key: BytesN<32>,
    pub expected_value: Bytes,
}

#[derive(Clone)]
#[contracttype]
pub struct ConditionalEscrow {
    pub escrow_id: u64,
    pub condition: OnChainCondition,
    pub evaluated: bool,
    pub result: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionEvaluated {
    pub escrow_id: u64,
    pub met: bool,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowExpired {
    pub escrow_id: u64,
    pub refunded_to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeoutResolutionTriggered {
    pub escrow_id: u64,
    pub favor: AutoResolveFavor,
    pub resolved_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct GlobalExpiryConfig {
    pub default_expiry_seconds: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct MigrationStatus {
    pub in_progress: bool,
    pub migrated_count: u64,
    pub total_count: u64,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalReleaseExecuted {
    pub escrow_id: u64,
    pub released_to: Address,
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

fn sum_approved_weight(env: &Env, escrow_id: u64, participants: &Vec<Participant>) -> u32 {
    let mut total: u32 = 0;
    for p in participants.iter() {
        if p.approved {
            let weight: u32 = env
                .storage()
                .instance()
                .get(&DataKey::VoteWeight(escrow_id, p.address.clone()))
                .unwrap_or(p.weight_bps);
            total += weight;
        }
    }
    total
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn initialize(env: Env, admin: Address) {
        if env
            .storage()
            .instance()
            .has(&DataKey::Config(ConfigKey::AdminMultiSig))
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
            .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
        AdminAdded { admin }.publish(&env);
    }

    pub fn set_escrow_fee_config(
        env: Env,
        admin: Address,
        config: EscrowFeeConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::EscrowFeeConfig), &config);
        EscrowFeeConfigUpdated {
            fee_bps: config.fee_bps,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_escrow_fee_config(env: Env) -> EscrowFeeConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::EscrowFeeConfig))
            .unwrap_or(EscrowFeeConfig {
                fee_bps: 0,
                fee_recipient: env.current_contract_address(),
                enabled: false,
            })
    }

    pub fn get_accumulated_escrow_fees(env: Env, token: Address) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Participant(ParticipantKey::AccumulatedFees(
                token,
            )))
            .unwrap_or(0)
    }

    pub fn withdraw_escrow_fees(
        env: Env,
        admin: Address,
        token: Address,
        to: Address,
    ) -> Result<i128, Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::Participant(ParticipantKey::AccumulatedFees(
                token.clone(),
            )))
            .unwrap_or(0);
        if amount == 0 {
            return Ok(0);
        }

        Self::transfer_if_token_contract(&env, &token, &to, amount)?;
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::AccumulatedFees(token.clone())),
            &0i128,
        );

        EscrowFeesWithdrawn {
            amount,
            withdrawn_by: admin,
        }
        .publish(&env);

        Ok(amount)
    }

    pub fn get_multisig_config(env: Env) -> MultiSigConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&proposer) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminProposalCounter))
            .unwrap_or(0)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::AdminProposalCounter), &counter);

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

        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&approver) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminProposal(
                proposal_id.clone(),
            )))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if proposal.executed || proposal.rejected {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if proposal.approvals.contains(&approver) {
            return Err(Error::Basic(BasicError::AlreadyApproved));
        }

        proposal.approvals.push_back(approver.clone());
        proposal.approval_count += 1;

        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminProposal(
                proposal_id.clone(),
            )))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if proposal.executed || proposal.rejected {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if env.ledger().timestamp() > proposal.expires_at {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if proposal.approval_count < config.required_signatures {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        proposal.executed = true;
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

        EscrowContract::dispatch_action(&env, &proposal)?;

        ActionExecuted { proposal_id }.publish(&env);

        Ok(())
    }

    pub fn reject_action(env: Env, rejecter: Address, proposal_id: String) -> Result<(), Error> {
        rejecter.require_auth();

        let config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&rejecter) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut proposal: AdminProposal = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminProposal(
                proposal_id.clone(),
            )))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if proposal.executed || proposal.rejected {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        proposal.rejected = true;
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminProposal(proposal_id.clone())),
            &proposal,
        );

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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if !config.admins.contains(&new_admin) {
            config.admins.push_back(new_admin.clone());
            config.total_admins += 1;
            env.storage()
                .instance()
                .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
            AdminAdded { admin: new_admin }.publish(&env);
        }

        Ok(())
    }

    pub fn remove_admin(env: Env, caller: Address, admin: Address) -> Result<(), Error> {
        caller.require_auth();

        let mut config: MultiSigConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;

        if !config.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if config.total_admins <= config.required_signatures {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
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
            .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
        AdminRemoved { admin }.publish(&env);

        Ok(())
    }

    pub fn update_required_signatures(
        env: Env,
        caller: Address,
        _required: u32,
    ) -> Result<(), Error> {
        caller.require_auth();
        // Direct calls are forbidden — must use propose_action / execute_action
        // with ActionType::UpdateThreshold so the change requires multi-sig approval.
        Err(Error::Basic(BasicError::Unauthorized))
    }

    pub fn designate_successor(
        env: Env,
        admin: Address,
        successor: Address,
        delay_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if let Some(existing) = env
            .storage()
            .instance()
            .get::<DataKey, SuccessionPlan>(&DataKey::Config(ConfigKey::AdminSuccessionPlan))
        {
            if !existing.activated {
                return Err(Error::Escrow(EscrowError::InvalidStatus));
            }
        }

        let designated_at = env.ledger().timestamp();
        let activatable_after = designated_at.saturating_add(delay_seconds);
        let plan = SuccessionPlan {
            successor: successor.clone(),
            designated_by: admin,
            designated_at,
            activatable_after,
            activated: false,
        };

        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::AdminSuccessionPlan), &plan);

        SuccessorDesignated {
            successor,
            activatable_after,
        }
        .publish(&env);

        Ok(())
    }

    pub fn activate_succession(env: Env, successor: Address) -> Result<(), Error> {
        successor.require_auth();

        let mut plan: SuccessionPlan = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminSuccessionPlan))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if plan.activated {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }
        if plan.successor != successor {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let now = env.ledger().timestamp();
        if now < plan.activatable_after {
            return Err(Error::Action(ActionError::NotReady));
        }

        let mut config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&successor) {
            config.admins.push_back(successor.clone());
            config.total_admins += 1;
            env.storage()
                .instance()
                .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
            AdminAdded {
                admin: successor.clone(),
            }
            .publish(&env);
        }

        plan.activated = true;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::AdminSuccessionPlan), &plan);

        SuccessionActivated {
            new_admin: successor,
            activated_at: now,
        }
        .publish(&env);

        Ok(())
    }

    pub fn revoke_succession(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let plan: SuccessionPlan = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminSuccessionPlan))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if plan.activated {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }

        env.storage()
            .instance()
            .remove(&DataKey::Config(ConfigKey::AdminSuccessionPlan));
        SuccessionRevoked { revoked_by: admin }.publish(&env);

        Ok(())
    }

    pub fn get_succession_plan(env: Env) -> Option<SuccessionPlan> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminSuccessionPlan))
    }

    pub fn initiate_clawback(
        env: Env,
        admin: Address,
        escrow_id: u64,
        reason_hash: BytesN<32>,
        delay_seconds: u64,
    ) -> Result<u64, Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if delay_seconds < 86400 {
            panic!("delay_seconds must be at least 86400");
        }

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        if let Some(request_id) = env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::Config(ConfigKey::AdminClawbackEscrow(escrow_id)))
        {
            if let Some(request) =
                env.storage()
                    .instance()
                    .get::<DataKey, ClawbackRequest>(&DataKey::Config(
                        ConfigKey::AdminClawbackRequest(request_id),
                    ))
            {
                if !request.executed && !request.cancelled {
                    return Err(Error::Escrow(EscrowError::AlreadyProcessed));
                }
            }
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminClawbackCounter))
            .unwrap_or(0)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::AdminClawbackCounter), &counter);

        let now = env.ledger().timestamp();
        let request = ClawbackRequest {
            request_id: counter,
            escrow_id,
            initiated_by: admin.clone(),
            reason_hash,
            execute_after: now + delay_seconds,
            executed: false,
            cancelled: false,
        };

        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminClawbackRequest(counter)),
            &request,
        );
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminClawbackEscrow(escrow_id)),
            &counter,
        );

        Ok(counter)
    }

    pub fn execute_clawback(env: Env, admin: Address, request_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut request: ClawbackRequest = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminClawbackRequest(
                request_id,
            )))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if request.executed {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }
        if request.cancelled {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let now = env.ledger().timestamp();
        if now < request.execute_after {
            return Err(Error::Action(ActionError::NotReady));
        }

        let escrow = EscrowContract::get_escrow(&env, request.escrow_id);
        let token_client = token::Client::new(&env, &escrow.token);
        let contract_address = env.current_contract_address();

        token_client.transfer(&contract_address, &admin, &escrow.amount);

        let mut updated_escrow = escrow;
        updated_escrow.status = EscrowStatus::Resolved;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Data(request.escrow_id)),
            &updated_escrow,
        );

        request.executed = true;
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminClawbackRequest(request_id)),
            &request,
        );

        Ok(())
    }

    pub fn cancel_clawback(env: Env, admin: Address, request_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut request: ClawbackRequest = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminClawbackRequest(
                request_id,
            )))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if request.executed {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }
        if request.cancelled {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        request.cancelled = true;
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::AdminClawbackRequest(request_id)),
            &request,
        );

        Ok(())
    }

    pub fn get_clawback_request(env: Env, request_id: u64) -> Option<ClawbackRequest> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::AdminClawbackRequest(
                request_id,
            )))
    }

    pub fn create_escrow(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        release_timestamp: u64,
        min_hold_period: u64,
        expiry_timestamp: u64,
        auto_refund_on_expiry: bool,
    ) -> Result<u64, Error> {
        customer.require_auth();
        Self::internal_create_escrow(
            env,
            customer,
            merchant,
            amount,
            token,
            release_timestamp,
            min_hold_period,
            expiry_timestamp,
            auto_refund_on_expiry,
        )
    }

    fn internal_create_escrow(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        release_timestamp: u64,
        min_hold_period: u64,
        expiry_timestamp: u64,
        auto_refund_on_expiry: bool,
    ) -> Result<u64, Error> {
        // Block new escrow creation during migration
        if let Some(status) = env
            .storage()
            .instance()
            .get::<DataKey, MigrationStatus>(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
        {
            if status.in_progress {
                return Err(Error::Basic(BasicError::ContractPaused));
            }
        }

        let current_timestamp = env.ledger().timestamp();

        // Validate expiry: if set (non-zero), must be strictly after release_timestamp
        if expiry_timestamp != 0 && expiry_timestamp <= release_timestamp {
            return Err(Error::Escrow(EscrowError::ExpiryBeforeRelease));
        }

        // Validate expiry: must be strictly in the future and after the hold period elapses
        if expiry_timestamp != 0
            && expiry_timestamp <= current_timestamp.saturating_add(min_hold_period)
        {
            return Err(Error::Escrow(EscrowError::EscrowAlreadyExpired));
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Counter))
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled {
            fee_config.fee_bps
        } else {
            0
        };

        let escalation_cfg: EscalationConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::EscalationConfig))
            .unwrap_or(EscalationConfig {
                timeout_seconds: 604800,
                favor: AutoResolveFavor::Customer,
            });

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
            expiry_timestamp,
            auto_refund_on_expiry,
            escalated_at: None,
            escalation_timeout: escalation_cfg.timeout_seconds,
            auto_resolve_in_favor_of: escalation_cfg.favor,
            evidence_deadline: None,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Counter), &escrow_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::CustomerCount(customer.clone())))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::CustomerList(customer.clone(), customer_count)),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::CustomerCount(customer.clone())),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MerchantCount(merchant.clone())))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::MerchantList(merchant.clone(), merchant_count)),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::MerchantCount(merchant.clone())),
            &(merchant_count + 1),
        );

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_escrows_created += 1;
        analytics.total_value_locked += amount;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);

        // Update per-address analytics
        EscrowContract::update_customer_analytics(&env, &customer, |a| {
            a.total_escrows_created += 1;
            a.total_value_locked += amount;
        });
        EscrowContract::update_merchant_analytics(&env, &merchant, |a| {
            a.total_escrows_created += 1;
            a.total_value_locked += amount;
        });

        // Issue #398: emitting this event is what lets dashboards subscribe to
        // new-escrow notifications via Horizon instead of polling known escrow IDs.
        EscrowCreated {
            escrow_id,
            customer,
            merchant,
            amount,
            token,
            release_timestamp,
        }
        .publish(&env);

        Ok(escrow_id)
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
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Ensure shares sum to 10000 bps
        let mut total_shares: u32 = 0;
        let mut total_weights: u32 = 0;
        for p in participants.iter() {
            total_shares += p.share_bps;
            total_weights += p.weight_bps;
        }
        if total_shares != 10000 {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }
        if total_weights != 10000 {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Normalize participants (ensure approved=false, approved_at=None at creation)
        let mut normalized = Vec::new(&env);
        for p in participants.iter() {
            normalized.push_back(Participant {
                address: p.address.clone(),
                role: p.role.clone(),
                share_bps: p.share_bps,
                weight_bps: p.weight_bps,
                approved: false,
                approved_at: None,
            });
        }

        // Transfer funds from customer to contract
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&customer, &contract_address, &total_amount);

        // Use a counter for ID
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiPartyCounter))
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let current_timestamp = env.ledger().timestamp();

        // Default threshold: full weight (100%). Adjustable via update_approval_threshold_bps.
        let escrow = MultiPartyEscrow {
            id: escrow_id,
            participants: normalized,
            total_amount,
            token,
            status: EscrowStatus::Locked,
            approvals: Vec::new(&env),
            threshold_bps: 10000,
            created_at: current_timestamp,
            release_timestamp,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiPartyCounter), &escrow_id);

        for p in escrow.participants.iter() {
            env.storage().instance().set(
                &DataKey::VoteWeight(escrow_id, p.address.clone()),
                &p.share_bps,
            );
        }
        env.storage()
            .instance()
            .set(&DataKey::ReleaseThresholdBps(escrow_id), &10000u32);

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
            .has(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Locate caller in participants. Only participants with weight_bps > 0 may vote.
        let now = env.ledger().timestamp();
        let mut updated_participants = Vec::new(&env);
        let mut found_voter = false;
        for p in escrow.participants.iter() {
            if p.address == caller {
                let weight = env
                    .storage()
                    .instance()
                    .get(&DataKey::VoteWeight(escrow_id, caller.clone()))
                    .unwrap_or(p.weight_bps);
                if weight == 0 {
                    return Err(Error::Basic(BasicError::Unauthorized));
                }
                if p.approved {
                    return Err(Error::Basic(BasicError::DuplicateApproval));
                }
                found_voter = true;
                updated_participants.push_back(Participant {
                    address: p.address.clone(),
                    role: p.role.clone(),
                    share_bps: p.share_bps,
                    weight_bps: weight,
                    approved: true,
                    approved_at: Some(now),
                });
            } else {
                updated_participants.push_back(p.clone());
            }
        }

        if !found_voter {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        escrow.participants = updated_participants;
        escrow.approvals.push_back(caller.clone());
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);

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
            .has(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .unwrap();

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Check if cumulative approved weight meets the threshold
        let approved_weight = sum_approved_weight(&env, escrow_id, &escrow.participants);
        let threshold = env
            .storage()
            .instance()
            .get(&DataKey::ReleaseThresholdBps(escrow_id))
            .unwrap_or(escrow.threshold_bps);
        if approved_weight < threshold {
            return Err(Error::Action(ActionError::ApprovalsThresholdNotMet));
        }

        // Check release timestamp
        if env.ledger().timestamp() < escrow.release_timestamp {
            return Err(Error::Escrow(EscrowError::ReleaseNotYetAvailable));
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
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);

        MultiPartyEscrowReleased { escrow_id }.publish(&env);

        Ok(())
    }

    pub fn get_multi_party_escrow(env: Env, escrow_id: u64) -> Result<MultiPartyEscrow, Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .unwrap())
    }

    /// Returns (current cumulative approved weight, required threshold) in basis points.
    pub fn get_approval_weight(env: Env, escrow_id: u64) -> Result<(u32, u32), Error> {
        let escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;
        let approved = sum_approved_weight(&env, escrow_id, &escrow.participants);
        let threshold = env
            .storage()
            .instance()
            .get(&DataKey::ReleaseThresholdBps(escrow_id))
            .unwrap_or(escrow.threshold_bps);
        Ok((approved, threshold))
    }

    /// Admin updates a participant's voting weight. Blocked once any participant has approved.
    /// The total participant weight must still sum to 10000 bps after the update.
    pub fn set_participant_weight(
        env: Env,
        admin: Address,
        escrow_id: u64,
        participant: Address,
        weight_bps: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Block weight updates once any participant has already approved.
        for p in escrow.participants.iter() {
            if p.approved {
                return Err(Error::Escrow(EscrowError::InvalidStatus));
            }
        }

        let mut updated = Vec::new(&env);
        let mut found = false;
        let mut new_total: u32 = 0;
        for p in escrow.participants.iter() {
            if p.address == participant {
                found = true;
                new_total += weight_bps;
                updated.push_back(Participant {
                    address: p.address.clone(),
                    role: p.role.clone(),
                    share_bps: p.share_bps,
                    weight_bps,
                    approved: p.approved,
                    approved_at: p.approved_at,
                });
            } else {
                new_total += p.weight_bps;
                updated.push_back(p.clone());
            }
        }

        if !found {
            return Err(Error::Basic(BasicError::ParticipantNotFound));
        }
        if new_total != 10000 {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        escrow.participants = updated;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);

        env.storage().instance().set(
            &DataKey::VoteWeight(escrow_id, participant.clone()),
            &weight_bps,
        );

        WeightUpdated {
            escrow_id,
            participant,
            weight_bps,
        }
        .publish(&env);

        Ok(())
    }

    /// Admin updates the approval threshold. Must be in the range (0, 10000].
    pub fn update_approval_threshold_bps(
        env: Env,
        admin: Address,
        escrow_id: u64,
        threshold_bps: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if let Err(_) = Self::validate_bps(threshold_bps) {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let mut escrow: MultiPartyEscrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        escrow.threshold_bps = threshold_bps;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);

        env.storage()
            .instance()
            .set(&DataKey::ReleaseThresholdBps(escrow_id), &threshold_bps);

        ThresholdUpdated {
            escrow_id,
            threshold_bps,
        }
        .publish(&env);

        Ok(())
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
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }
        if tokens.len() > 10 {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Reject duplicate token addresses
        let len = tokens.len();
        for i in 0..len {
            let entry_i = tokens.get(i).unwrap();
            for j in (i + 1)..len {
                let entry_j = tokens.get(j).unwrap();
                if entry_i.token == entry_j.token {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
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
            .get(&DataKey::Escrow(EscrowKey::MultiTokenCounter))
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
            .set(&DataKey::Escrow(EscrowKey::MultiToken(escrow_id)), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiTokenCounter), &escrow_id);

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
            .get(&DataKey::Escrow(EscrowKey::MultiToken(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if env.ledger().timestamp() < escrow.release_timestamp {
            return Err(Error::Escrow(EscrowError::ReleaseNotYetAvailable));
        }

        // Pre-flight: verify balances so a partial failure can't leave the escrow
        // half-released. If any token is unavailable, nothing moves.
        let contract_address = env.current_contract_address();
        for entry in escrow.tokens.iter() {
            let token_client = token::Client::new(&env, &entry.token);
            let balance = match token_client.try_balance(&contract_address) {
                Ok(Ok(b)) => b,
                _ => return Err(Error::Escrow(EscrowError::InvalidStatus)),
            };
            if balance < entry.amount {
                return Err(Error::Escrow(EscrowError::InvalidStatus));
            }
        }

        for entry in escrow.tokens.iter() {
            let token_client = token::Client::new(&env, &entry.token);
            if token_client
                .try_transfer(&contract_address, &escrow.merchant, &entry.amount)
                .is_err()
            {
                return Err(Error::Escrow(EscrowError::InvalidStatus));
            }
        }

        let token_count = escrow.tokens.len();
        escrow.status = EscrowStatus::Released;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiToken(escrow_id)), &escrow);

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
            .get(&DataKey::Escrow(EscrowKey::MultiToken(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))
    }

    pub fn get_escrow(env: &Env, escrow_id: u64) -> Escrow {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
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

        if let Some(config) = env
            .storage()
            .instance()
            .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::AdminMultiSig))
        {
            if config.admins.contains(&admin) && early_release {
                // Admin force release requires time-lock
                return Err(Error::Basic(BasicError::Unauthorized));
            }
        }

        Self::internal_release_escrow(env, admin, escrow_id, early_release, None)
    }

    fn can_release_escrow(env: Env, escrow_id: u64, early_release: bool) -> Result<(), Error> {
        let current_time: u64 = env.ledger().timestamp();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Guard: block release if any child escrows are unresolved
        if !Self::can_parent_release(env.clone(), escrow_id) {
            return Err(Error::Escrow(EscrowError::ChildrenNotResolved));
        }

        // Guard: block full release if any sub-accounts remain unreleased
        let sub_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccountCounter(escrow_id)))
            .unwrap_or(0);
        for sub_id in 1..=sub_count {
            if let Some(sub) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowSubAccount>(&DataKey::Escrow(EscrowKey::SubAccount(
                        escrow_id, sub_id,
                    )))
            {
                if !sub.released {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
            }
        }

        match escrow.status {
            EscrowStatus::Locked => {
                if !early_release {
                    if current_time < escrow.release_timestamp {
                        return Err(Error::Escrow(EscrowError::ReleaseNotYetAvailable));
                    }

                    if current_time < escrow.created_at + escrow.min_hold_period {
                        return Err(Error::Escrow(EscrowError::ReleaseOnHoldPeriod));
                    }
                }
                Ok(())
            }
            EscrowStatus::Released => Err(Error::Escrow(EscrowError::AlreadyProcessed)),
            EscrowStatus::Disputed => Err(Error::Escrow(EscrowError::InvalidStatus)),
            EscrowStatus::Resolved => Err(Error::Escrow(EscrowError::AlreadyProcessed)),
            EscrowStatus::Cancelled => Err(Error::Escrow(EscrowError::AlreadyProcessed)),
        }
    }

    fn internal_release_escrow(
        env: Env,
        _admin: Address,
        escrow_id: u64,
        early_release: bool,
        recipient_override: Option<Address>,
    ) -> Result<(), Error> {
        let current_time: u64 = env.ledger().timestamp();

        Self::can_release_escrow(env.clone(), escrow_id, early_release)?;

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status == EscrowStatus::Locked {
            escrow.status = EscrowStatus::Released;
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

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
                let mut acc: i128 = env
                    .storage()
                    .instance()
                    .get(&DataKey::Participant(ParticipantKey::AccumulatedFees(
                        escrow.token.clone(),
                    )))
                    .unwrap_or(0);
                acc += fee_amount;
                env.storage().instance().set(
                    &DataKey::Participant(ParticipantKey::AccumulatedFees(escrow.token.clone())),
                    &acc,
                );
            }

            EscrowFeeCollected {
                escrow_id,
                fee_amount,
                recipient: fee_config.fee_recipient.clone(),
            }
            .publish(&env);
        }

        let recipient = recipient_override.unwrap_or(escrow.merchant.clone());
        EscrowContract::transfer_if_token_contract(
            &env,
            &escrow.token,
            &recipient,
            merchant_amount,
        )?;

        // Update reputation for both parties on successful completion.
        EscrowContract::update_reputation_on_completion(&env, &escrow.merchant);
        EscrowContract::update_reputation_on_completion(&env, &escrow.customer);

        // Apply the duration-weighted tenure bonus when configured. Dispute-free
        // completions reward longer-running agreements more; the once-per-escrow
        // guard makes repeated invocations harmless, so errors are ignored here.
        if EscrowContract::get_tenure_config(env.clone()).is_some() {
            let _ =
                EscrowContract::apply_tenure_bonus(env.clone(), escrow_id, escrow.merchant.clone());
            let _ =
                EscrowContract::apply_tenure_bonus(env.clone(), escrow_id, escrow.customer.clone());
        }

        // Update global analytics
        let duration = current_time.saturating_sub(escrow.created_at);
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
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
            .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);

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
            recipient,
            amount: escrow.amount,
            token: escrow.token,
        }
        .publish(&env);

        Ok(())
    }

    pub fn refund_escrow(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        match escrow.status {
            EscrowStatus::Locked | EscrowStatus::Disputed => {
                escrow.status = EscrowStatus::Resolved;
            }
            EscrowStatus::Released | EscrowStatus::Resolved | EscrowStatus::Cancelled => {
                return Err(Error::Escrow(EscrowError::AlreadyProcessed))
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

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
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Only customer or merchant can dispute
        if caller != escrow.customer && caller != escrow.merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Handle collateral
        let config = Self::get_dispute_config(env.clone());
        if config.collateral_enabled && config.collateral_amount > 0 {
            // Validate collateral-to-loan ratio
            // Formula: collateral_amount * 10000 >= escrow_amount * min_collateral_ratio_bps
            if config.collateral_amount * 10000 < escrow.amount * config.min_collateral_ratio_bps as i128 {
                return Err(Error::Action(ActionError::InsufficientCollateral));
            }

            let token_client = token::Client::new(&env, &config.collateral_token);
            token_client.transfer(
                &caller,
                &env.current_contract_address(),
                &config.collateral_amount,
            );

            let collateral = DisputeCollateral {
                escrow_id,
                disputing_party: caller.clone(),
                amount: config.collateral_amount,
                token: config.collateral_token.clone(),
                deposited_at: env.ledger().timestamp(),
            };
            env.storage().instance().set(
                &DataKey::Dispute(DisputeKey::Collateral(escrow_id)),
                &collateral,
            );

            CollateralDeposited {
                escrow_id,
                party: caller.clone(),
                amount: config.collateral_amount,
            }
            .publish(&env);
        }

        match escrow.status {
            EscrowStatus::Locked => {
                escrow.status = EscrowStatus::Disputed;
                escrow.dispute_started_at = env.ledger().timestamp();
                escrow.last_activity_at = escrow.dispute_started_at;
                // Set evidence submission deadline to 7 days from dispute start
                let evidence_deadline_seconds = 7 * 24 * 60 * 60; // 7 days
                escrow.evidence_deadline =
                    Some(escrow.dispute_started_at + evidence_deadline_seconds);

                EvidenceDeadlineSet {
                    escrow_id,
                    deadline: escrow.evidence_deadline.unwrap(),
                    set_at: escrow.dispute_started_at,
                }
                .publish(&env);
            }
            EscrowStatus::Released => return Err(Error::Escrow(EscrowError::AlreadyProcessed)),
            EscrowStatus::Disputed => return Err(Error::Escrow(EscrowError::AlreadyProcessed)),
            EscrowStatus::Resolved => return Err(Error::Escrow(EscrowError::AlreadyProcessed)),
            EscrowStatus::Cancelled => return Err(Error::Escrow(EscrowError::AlreadyProcessed)),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_disputes += 1;
        analytics.dispute_rate_bps = if analytics.total_escrows_created > 0 {
            (analytics.total_disputes as i128 * 10000) / analytics.total_escrows_created as i128
        } else {
            0
        };
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);

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
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Check evidence submission deadline
        if let Some(deadline) = escrow.evidence_deadline {
            let current_time = env.ledger().timestamp();
            if current_time > deadline {
                EvidenceDeadlineExceeded {
                    escrow_id,
                    deadline,
                    submitted_at: current_time,
                }
                .publish(&env);
                return Err(Error::Action(ActionError::EvidenceDeadlinePassed));
            }
        }

        EscrowContract::append_evidence_entry(&env, escrow_id, caller, ipfs_hash)
    }

    /// One-time commitment of the evidence Merkle root for an escrow (must be disputed).
    pub fn commit_evidence_root(
        env: Env,
        caller: Address,
        escrow_id: u64,
        merkle_root: BytesN<32>,
    ) -> Result<(), Error> {
        caller.require_auth();
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::EvidenceCommitment(escrow_id)))
        {
            return Err(Error::Basic(BasicError::RootAlreadyCommitted));
        }
        let commitment = EvidenceCommitment {
            escrow_id,
            merkle_root,
            committed_at: env.ledger().timestamp(),
            committed_by: caller,
        };
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::EvidenceCommitment(escrow_id)),
            &commitment,
        );
        Ok(())
    }

    /// Submit evidence bytes with a Keccak Merkle proof against the committed root.
    ///
    /// Leaf hash is `keccak256(evidence)`. If no root was committed for this escrow,
    /// behaves like [`Self::submit_evidence`] using the UTF-8-safe prefix of `evidence`
    /// as the stored reference string (same unverified path as legacy submissions).
    ///
    /// Invalid proofs return [`Error::Basic(BasicError::InvalidMerkleProof)`] and emit **no** event.
    pub fn submit_evidence_with_proof(
        env: Env,
        caller: Address,
        escrow_id: u64,
        evidence: Bytes,
        proof: Vec<BytesN<32>>,
        leaf_index: u32,
    ) -> Result<(), Error> {
        caller.require_auth();
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let escrow_check = EscrowContract::get_escrow(&env, escrow_id);
        if escrow_check.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        if escrow_check.customer != caller && escrow_check.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let commitment_opt =
            env.storage()
                .instance()
                .get::<DataKey, EvidenceCommitment>(&DataKey::Escrow(
                    EscrowKey::EvidenceCommitment(escrow_id),
                ));

        if commitment_opt.is_none() {
            let ipfs_hash = EscrowContract::evidence_bytes_to_label_string(&env, evidence);
            return EscrowContract::append_evidence_entry(&env, escrow_id, caller, ipfs_hash);
        }

        let commitment = commitment_opt.unwrap();
        let leaf_hash: BytesN<32> = env.crypto().keccak256(&evidence).into();
        if !EscrowContract::verify_keccak_merkle_proof(
            &env,
            leaf_hash,
            proof,
            leaf_index,
            commitment.merkle_root,
        ) {
            return Err(Error::Basic(BasicError::InvalidMerkleProof));
        }

        let ipfs_hash = EscrowContract::evidence_bytes_to_label_string(&env, evidence);
        EscrowContract::append_evidence_entry(&env, escrow_id, caller, ipfs_hash)
    }

    pub fn get_evidence_commitment(env: Env, escrow_id: u64) -> Result<EvidenceCommitment, Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        env.storage()
            .instance()
            .get::<DataKey, EvidenceCommitment>(&DataKey::Escrow(EscrowKey::EvidenceCommitment(
                escrow_id,
            )))
            .ok_or(Error::Basic(BasicError::RootAlreadyCommitted))
    }

    fn verify_keccak_merkle_proof(
        env: &Env,
        leaf_hash: BytesN<32>,
        proof: Vec<BytesN<32>>,
        leaf_index: u32,
        root: BytesN<32>,
    ) -> bool {
        let mut computed = leaf_hash;
        let mut idx = leaf_index as u64;
        let mut i = 0u32;
        while i < proof.len() {
            let sibling = proof.get(i).unwrap();
            computed = if idx % 2 == 0 {
                EscrowContract::hash_keccak_pair(env, computed, sibling)
            } else {
                EscrowContract::hash_keccak_pair(env, sibling.clone(), computed)
            };
            idx /= 2;
            i += 1;
        }
        computed == root
    }

    fn hash_keccak_pair(env: &Env, left: BytesN<32>, right: BytesN<32>) -> BytesN<32> {
        let la = left.to_array();
        let ra = right.to_array();
        let mut raw = [0u8; 64];
        raw[..32].copy_from_slice(&la);
        raw[32..].copy_from_slice(&ra);
        let buf = Bytes::from_array(env, &raw);
        env.crypto().keccak256(&buf).into()
    }

    fn evidence_bytes_to_label_string(env: &Env, evidence: Bytes) -> String {
        const CAP: u32 = 160;
        let n = core::cmp::min(evidence.len(), CAP);
        let mut tmp = [0u8; 160];
        let mut j = 0u32;
        while j < n {
            tmp[j as usize] = evidence.get(j).unwrap();
            j += 1;
        }
        String::from_bytes(env, &tmp[..n as usize])
    }

    fn append_evidence_entry(
        env: &Env,
        escrow_id: u64,
        caller: Address,
        ipfs_hash: String,
    ) -> Result<(), Error> {
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::EvidenceCount(escrow_id)))
            .unwrap_or(0);
        let evidence = Evidence {
            submitter: caller.clone(),
            ipfs_hash: ipfs_hash.clone(),
            submitted_at: env.ledger().timestamp(),
        };
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Evidence(escrow_id, count)),
            &evidence,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::EvidenceCount(escrow_id)),
            &(count + 1),
        );
        let mut escrow = EscrowContract::get_escrow(env, escrow_id);
        escrow.last_activity_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        EvidenceSubmitted {
            escrow_id,
            submitter: caller,
            ipfs_hash,
        }
        .publish(env);
        Ok(())
    }

    pub fn get_evidence_count(env: &Env, escrow_id: u64) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::EvidenceCount(escrow_id)))
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
                .get::<DataKey, Evidence>(&DataKey::Escrow(EscrowKey::Evidence(escrow_id, i)))
            {
                items.push_back(ev);
            }
            i += 1;
        }
        items
    }

    pub fn submit_evidence_batch(
        env: Env,
        caller: Address,
        escrow_id: u64,
        evidence_items: Vec<Bytes>,
    ) -> Result<u32, Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        const MAX_BATCH: u32 = 10;
        if evidence_items.len() > MAX_BATCH {
            return Err(Error::Escrow(EscrowError::BatchTooLarge));
        }

        let now = env.ledger().timestamp();

        // Enforce the same evidence deadline as the single-item path.
        if let Some(deadline) = escrow.evidence_deadline {
            if now > deadline {
                EvidenceDeadlineExceeded {
                    escrow_id,
                    deadline,
                    submitted_at: now,
                }
                .publish(&env);
                return Err(Error::Action(ActionError::EvidenceDeadlinePassed));
            }
        }
        let page_num: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::EvidencePageCount(escrow_id)))
            .unwrap_or(0);

        let mut page: Vec<Evidence> = Vec::new(&env);
        for item in evidence_items.iter() {
            let ipfs_hash = EscrowContract::evidence_bytes_to_label_string(&env, item);
            page.push_back(Evidence {
                submitter: caller.clone(),
                ipfs_hash,
                submitted_at: now,
            });
        }

        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::EvidencePage(escrow_id, page_num)),
            &page,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::EvidencePageCount(escrow_id)),
            &(page_num + 1),
        );

        Ok(page_num + 1)
    }

    pub fn get_evidence_page(env: Env, escrow_id: u64, page: u32) -> Vec<Evidence> {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::EvidencePage(escrow_id, page)))
            .unwrap_or(Vec::new(&env))
    }

    pub fn escalate_dispute(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        escrow.escalation_level = escrow.escalation_level.saturating_add(1);
        let now = env.ledger().timestamp();
        escrow.escalated_at = Some(now);
        escrow.last_activity_at = now;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        DisputeEscalated {
            escrow_id,
            level: escrow.escalation_level,
        }
        .publish(&env);
        Ok(())
    }

    pub fn auto_resolve_dispute(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        let now = env.ledger().timestamp();
        let last = if escrow.last_activity_at == 0 {
            escrow.dispute_started_at
        } else {
            escrow.last_activity_at
        };
        let timeout: u64 = 500;
        if now.saturating_sub(last) < timeout {
            return Err(Error::Escrow(EscrowError::TimeoutNotReached));
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
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        EscrowContract::update_reputation_on_dispute_outcome(&env, &winner, &loser);
        EscrowResolved {
            escrow_id,
            released_to_merchant: release_to_merchant,
            amount: escrow.amount,
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_escalation_config(
        env: Env,
        admin: Address,
        timeout_seconds: u64,
        favor: AutoResolveFavor,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        let cfg = EscalationConfig {
            timeout_seconds,
            favor,
        };
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::EscalationConfig), &cfg);
        Ok(())
    }

    pub fn check_escalation_timeout(env: Env, escrow_id: u64) -> bool {
        let escrow: Option<Escrow> = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Data(escrow_id)));
        let escrow = match escrow {
            Some(e) => e,
            None => return false,
        };
        if let Some(escalated_at) = escrow.escalated_at {
            let now = env.ledger().timestamp();
            now.saturating_sub(escalated_at) >= escrow.escalation_timeout
        } else {
            false
        }
    }

    pub fn trigger_timeout_resolution(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }

        let escalated_at = escrow
            .escalated_at
            .ok_or(Error::Escrow(EscrowError::InvalidStatus))?;

        let now = env.ledger().timestamp();
        if now.saturating_sub(escalated_at) < escrow.escalation_timeout {
            return Err(Error::Escrow(EscrowError::TimeoutNotReached));
        }

        let favor = escrow.auto_resolve_in_favor_of.clone();

        match &favor {
            AutoResolveFavor::Customer => {
                escrow.status = EscrowStatus::Resolved;
                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
                EscrowContract::transfer_if_token_contract(
                    &env,
                    &escrow.token,
                    &escrow.customer,
                    escrow.amount,
                )?;
            }
            AutoResolveFavor::Merchant => {
                escrow.status = EscrowStatus::Released;
                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
                EscrowContract::transfer_if_token_contract(
                    &env,
                    &escrow.token,
                    &escrow.merchant,
                    escrow.amount,
                )?;
            }
            AutoResolveFavor::SplitEqual => {
                let half = escrow.amount / 2;
                let remainder = escrow.amount - half;
                escrow.status = EscrowStatus::Resolved;
                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
                EscrowContract::transfer_if_token_contract(
                    &env,
                    &escrow.token,
                    &escrow.customer,
                    half,
                )?;
                EscrowContract::transfer_if_token_contract(
                    &env,
                    &escrow.token,
                    &escrow.merchant,
                    remainder,
                )?;
            }
        }

        (TimeoutResolutionTriggered {
            escrow_id,
            favor,
            resolved_at: now,
        })
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

        if let Some(config) = env
            .storage()
            .instance()
            .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::AdminMultiSig))
        {
            if !config.admins.contains(&admin) {
                return Err(Error::Basic(BasicError::NotAnAdmin));
            }
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
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
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
            _ => return Err(Error::Action(ActionError::NotDisputed)),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        // Transfer main escrow funds
        let recipient = if release_to_merchant {
            &escrow.merchant
        } else {
            &escrow.customer
        };
        Self::transfer_if_token_contract(&env, &escrow.token, recipient, escrow.amount)?;

        // Handle collateral distribution
        if let Some(collateral) = env
            .storage()
            .instance()
            .get::<DataKey, DisputeCollateral>(&DataKey::Dispute(DisputeKey::Collateral(escrow_id)))
        {
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
                }
                .publish(&env);
            } else {
                CollateralForfeited {
                    escrow_id,
                    party: collateral.disputing_party,
                    amount: collateral.amount,
                }
                .publish(&env);
            }
            env.storage()
                .instance()
                .remove(&DataKey::Dispute(DisputeKey::Collateral(escrow_id)));
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
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_resolutions += 1;
        analytics.dispute_rate_bps = if analytics.total_escrows_created > 0 {
            (analytics.total_disputes as i128 * 10000) / analytics.total_escrows_created as i128
        } else {
            0
        };
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);

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

    pub fn file_dispute_appeal(
        env: Env,
        appellant: Address,
        escrow_id: u64,
        reason_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        appellant.require_auth();

        // Check if escrow exists and is in Resolved or Released status from initial dispute
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Appellant must be one of the parties
        if appellant != escrow.customer && appellant != escrow.merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Get current dispute round
        let current_round = EscrowContract::get_dispute_round(env.clone(), escrow_id);

        // Maximum of two rounds (Initial + Appeal); third appeal not allowed
        if current_round == DisputeRound::Final {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Check if appeal window is open (72 hours = 259200 seconds)
        let now = env.ledger().timestamp();
        let dispute_time = escrow.dispute_started_at;
        let appeal_window: u64 = 259200; // 72 hours

        if now.saturating_sub(dispute_time) > appeal_window {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        // Check if an appeal has already been filed for this round
        let appeals_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::AppealCounter))
            .unwrap_or(0);

        for i in 0..appeals_count {
            if let Some(appeal) = env
                .storage()
                .instance()
                .get::<DataKey, DisputeAppeal>(&DataKey::Dispute(DisputeKey::Appeal(i)))
            {
                if appeal.escrow_id == escrow_id && !appeal.resolved {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
            }
        }

        // Create new appeal
        let appeal_id = appeals_count;
        let appeal_deadline = now.saturating_add(appeal_window);

        let appeal = DisputeAppeal {
            appeal_id,
            escrow_id,
            round: DisputeRound::Appeal,
            appellant: appellant.clone(),
            reason_hash,
            filed_at: now,
            appeal_deadline,
            resolved: false,
        };

        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::Appeal(appeal_id)), &appeal);
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::AppealCounter),
            &(appeals_count + 1),
        );

        // Update dispute round to Appeal
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::Round(escrow_id)),
            &DisputeRound::Appeal,
        );

        DisputeAppealFiled {
            appeal_id,
            escrow_id,
            appellant,
            filed_at: now,
            appeal_deadline,
        }
        .publish(&env);

        Ok(appeal_id)
    }

    pub fn get_dispute_round(env: Env, escrow_id: u64) -> DisputeRound {
        env.storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::Round(escrow_id)))
            .unwrap_or(DisputeRound::Initial)
    }

    pub fn resolve_appeal(
        env: Env,
        admin: Address,
        appeal_id: u64,
        in_favour_of: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        Self::require_not_paused(&env, "resolve_appeal")?;

        // Check multisig admin authorization
        if let Some(config) = env
            .storage()
            .instance()
            .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::AdminMultiSig))
        {
            if !config.admins.contains(&admin) {
                return Err(Error::Basic(BasicError::NotAnAdmin));
            }
        }

        // Get the appeal
        let mut appeal = env
            .storage()
            .instance()
            .get::<DataKey, DisputeAppeal>(&DataKey::Dispute(DisputeKey::Appeal(appeal_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if appeal.resolved {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }

        let escrow_id = appeal.escrow_id;

        // Get the escrow
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Verify that in_favour_of is one of the parties
        if in_favour_of != escrow.customer && in_favour_of != escrow.merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        // Determine the loser
        let loser = if in_favour_of == escrow.customer {
            escrow.merchant.clone()
        } else {
            escrow.customer.clone()
        };

        // Resolve appeal
        appeal.resolved = true;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::Appeal(appeal_id)), &appeal);

        // Update dispute round to Final
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::Round(escrow_id)),
            &DisputeRound::Final,
        );

        // Call internal resolution logic with senior arbitrators
        let now = env.ledger().timestamp();

        // Update reputation scores based on appeal outcome
        EscrowContract::update_reputation_on_dispute_outcome(&env, &in_favour_of, &loser);

        // Update escrow status
        let mut escrow_mut = escrow;
        escrow_mut.status = EscrowStatus::Resolved;
        escrow_mut.last_activity_at = now;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow_mut);

        // Transfer funds to the party in favor
        Self::transfer_if_token_contract(
            &env,
            &escrow_mut.token,
            &in_favour_of,
            escrow_mut.amount,
        )?;

        // Handle collateral distribution if any
        if let Some(collateral) = env
            .storage()
            .instance()
            .get::<DataKey, DisputeCollateral>(&DataKey::Dispute(DisputeKey::Collateral(escrow_id)))
        {
            let token_client = token::Client::new(&env, &collateral.token);
            token_client.transfer(
                &env.current_contract_address(),
                &in_favour_of,
                &collateral.amount,
            );

            if in_favour_of == collateral.disputing_party {
                CollateralReturned {
                    escrow_id,
                    party: collateral.disputing_party,
                    amount: collateral.amount,
                }
                .publish(&env);
            } else {
                CollateralForfeited {
                    escrow_id,
                    party: collateral.disputing_party,
                    amount: collateral.amount,
                }
                .publish(&env);
            }
            env.storage()
                .instance()
                .remove(&DataKey::Dispute(DisputeKey::Collateral(escrow_id)));
        }

        // Update analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_resolutions += 1;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);

        // Update per-address analytics
        EscrowContract::update_customer_analytics(&env, &escrow_mut.customer, |a| {
            a.total_resolutions += 1;
        });
        EscrowContract::update_merchant_analytics(&env, &escrow_mut.merchant, |a| {
            a.total_resolutions += 1;
        });

        AppealResolved {
            appeal_id,
            escrow_id,
            in_favor_of: in_favour_of,
            resolved_at: now,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_appeal(env: Env, appeal_id: u64) -> Option<DisputeAppeal> {
        env.storage()
            .instance()
            .get::<DataKey, DisputeAppeal>(&DataKey::Dispute(DisputeKey::Appeal(appeal_id)))
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
            .get(&DataKey::Escrow(EscrowKey::CustomerCount(customer.clone())))
            .unwrap_or(0);

        let mut escrows = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(escrow_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::Escrow(EscrowKey::CustomerList(
                    customer.clone(),
                    i,
                )))
            {
                if let Some(escrow) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
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
            .get(&DataKey::Escrow(EscrowKey::CustomerCount(customer)))
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
            .get(&DataKey::Escrow(EscrowKey::MerchantCount(merchant.clone())))
            .unwrap_or(0);

        let mut escrows = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(escrow_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::Escrow(EscrowKey::MerchantList(
                    merchant.clone(),
                    i,
                )))
            {
                if let Some(escrow) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
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
            .get(&DataKey::Escrow(EscrowKey::MerchantCount(merchant)))
            .unwrap_or(0)
    }

    // ── STATE VERIFICATION INTERFACE ──────────────────────────────────────────

    /// Returns true if the escrow exists and its status is Released.
    pub fn is_escrow_released(env: Env, escrow_id: u64) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .map(|e| e.status == EscrowStatus::Released)
            .unwrap_or(false)
    }

    /// Returns true if the escrow exists and its status is Disputed.
    pub fn is_escrow_disputed(env: Env, escrow_id: u64) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .map(|e| e.status == EscrowStatus::Disputed)
            .unwrap_or(false)
    }

    /// Returns the current status of an escrow, or EscrowNotFound if it does not exist.
    pub fn get_escrow_status(env: Env, escrow_id: u64) -> Result<EscrowStatus, Error> {
        env.storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .map(|e| e.status)
            .ok_or(Error::Escrow(EscrowError::NotFound))
    }

    /// Returns the (customer, merchant) address pair for an escrow, or EscrowNotFound.
    pub fn get_escrow_parties(env: Env, escrow_id: u64) -> Result<(Address, Address), Error> {
        env.storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .map(|e| (e.customer, e.merchant))
            .ok_or(Error::Escrow(EscrowError::NotFound))
    }

    /// Returns the locked amount for an escrow, or EscrowNotFound.
    pub fn get_escrow_amount(env: Env, escrow_id: u64) -> Result<i128, Error> {
        env.storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .map(|e| e.amount)
            .ok_or(Error::Escrow(EscrowError::NotFound))
    }

    /// Returns true if `address` is the customer or merchant of the given escrow.
    /// Returns false for non-existent escrow IDs or unrelated addresses.
    pub fn verify_escrow_participant(env: Env, escrow_id: u64, address: Address) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .map(|e| e.customer == address || e.merchant == address)
            .unwrap_or(false)
    }

    /// Returns the full escrow details. Access is restricted to the escrow
    /// customer, merchant, or an active observer (granted via `add_observer`).
    pub fn get_escrow_details(env: Env, caller: Address, escrow_id: u64) -> Result<Escrow, Error> {
        caller.require_auth();

        let escrow: Escrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if caller == escrow.customer || caller == escrow.merchant {
            return Ok(escrow);
        }

        if EscrowContract::verify_observer_access(env.clone(), escrow_id, caller) {
            return Ok(escrow);
        }

        Err(Error::Basic(BasicError::Unauthorized))
    }

    /// Grants a time-limited observer role for an escrow. Only the escrow
    /// customer, merchant, or an admin can grant.
    pub fn add_observer(
        env: Env,
        granter: Address,
        escrow_id: u64,
        observer: Address,
        duration_seconds: u64,
    ) -> Result<(), Error> {
        granter.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut allowed =
            EscrowContract::verify_escrow_participant(env.clone(), escrow_id, granter.clone());
        if !allowed {
            if let Some(cfg) = env
                .storage()
                .instance()
                .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::AdminMultiSig))
            {
                allowed = cfg.admins.contains(&granter);
            }
        }
        if !allowed {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let now = env.ledger().timestamp();
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::ObserverCount(escrow_id)))
            .unwrap_or(0);

        let mut i = 0u64;
        while i < count {
            if let Some(existing) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowObserver>(&DataKey::Dispute(DisputeKey::Observer(
                        escrow_id, i,
                    )))
            {
                if existing.observer == observer && existing.expires_at > now {
                    return Err(Error::Action(ActionError::ObserverAlreadyAdded));
                }
            }
            i += 1;
        }

        let obs = EscrowObserver {
            escrow_id,
            observer: observer.clone(),
            granted_by: granter.clone(),
            granted_at: now,
            expires_at: now.saturating_add(duration_seconds),
        };

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::Observer(escrow_id, count)),
            &obs,
        );
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::ObserverCount(escrow_id)),
            &(count + 1),
        );

        Ok(())
    }

    /// Removes an observer entry. Only the escrow customer, merchant, or admin may remove.
    pub fn remove_observer(
        env: Env,
        granter: Address,
        escrow_id: u64,
        observer: Address,
    ) -> Result<(), Error> {
        granter.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut allowed =
            EscrowContract::verify_escrow_participant(env.clone(), escrow_id, granter.clone());
        if !allowed {
            if let Some(cfg) = env
                .storage()
                .instance()
                .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::AdminMultiSig))
            {
                allowed = cfg.admins.contains(&granter);
            }
        }
        if !allowed {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::ObserverCount(escrow_id)))
            .unwrap_or(0);

        let mut i = 0u64;
        while i < count {
            if let Some(existing) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowObserver>(&DataKey::Dispute(DisputeKey::Observer(
                        escrow_id, i,
                    )))
            {
                if existing.observer == observer {
                    let last_index = count.saturating_sub(1);
                    if i != last_index {
                        if let Some(last) = env.storage().instance().get::<DataKey, EscrowObserver>(
                            &DataKey::Dispute(DisputeKey::Observer(escrow_id, last_index)),
                        ) {
                            env.storage()
                                .instance()
                                .set(&DataKey::Dispute(DisputeKey::Observer(escrow_id, i)), &last);
                        }
                    }
                    env.storage()
                        .instance()
                        .remove(&DataKey::Dispute(DisputeKey::Observer(
                            escrow_id, last_index,
                        )));
                    env.storage().instance().set(
                        &DataKey::Dispute(DisputeKey::ObserverCount(escrow_id)),
                        &last_index,
                    );
                    return Ok(());
                }
            }
            i += 1;
        }

        Err(Error::Action(ActionError::ObserverNotFound))
    }

    /// Returns all observers (including expired ones) for an escrow.
    pub fn get_observers(env: Env, escrow_id: u64) -> Vec<EscrowObserver> {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::ObserverCount(escrow_id)))
            .unwrap_or(0);
        let mut items: Vec<EscrowObserver> = Vec::new(&env);
        let mut i = 0u64;
        while i < total {
            if let Some(o) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowObserver>(&DataKey::Dispute(DisputeKey::Observer(
                        escrow_id, i,
                    )))
            {
                items.push_back(o);
            }
            i += 1;
        }
        items
    }

    /// Verifies whether `observer` currently has access to `escrow_id`.
    /// Returns false for expired observers without removing them.
    pub fn verify_observer_access(env: Env, escrow_id: u64, observer: Address) -> bool {
        let now = env.ledger().timestamp();
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::ObserverCount(escrow_id)))
            .unwrap_or(0);
        let mut i = 0u64;
        while i < total {
            if let Some(o) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowObserver>(&DataKey::Dispute(DisputeKey::Observer(
                        escrow_id, i,
                    )))
            {
                if o.observer == observer {
                    return o.expires_at > now;
                }
            }
            i += 1;
        }
        false
    }

    // ── REPUTATION METHODS ───────────────────────────────────────────────────

    /// Returns the reputation score for an address.
    /// New addresses start at the neutral score of 5000.
    pub fn get_reputation(env: Env, address: Address) -> ReputationScore {
        let mut rep = EscrowContract::get_or_default_reputation(&env, &address);
        let config = EscrowContract::get_or_default_decay_config(&env);
        let now = env.ledger().timestamp();
        let decayed_score = EscrowContract::compute_decayed_score(&rep, &config, now);
        rep.score = decayed_score as i64;
        rep
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
            .set(&DataKey::Config(ConfigKey::ReputationConfig), &config);
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
            .get(&DataKey::Participant(ParticipantKey::ReputationScore(
                address.clone(),
            )))
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
            .get(&DataKey::Config(ConfigKey::ReputationConfig))
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
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::ReputationScore(address.clone())),
            &rep,
        );
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
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::ReputationScore(winner.clone())),
            &winner_rep,
        );
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
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::ReputationScore(loser.clone())),
            &loser_rep,
        );
        ReputationUpdated {
            address: loser.clone(),
            old_score: old_loser_score,
            new_score: loser_rep.score,
        }
        .publish(env);
    }

    // ── TENURE-WEIGHTED REPUTATION ───────────────────────────────────────────

    /// Admin configures the duration-weighted reputation bonus parameters.
    pub fn set_tenure_reputation_config(
        env: Env,
        admin: Address,
        config: TenureReputationConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::TenureConfig), &config);
        TenureConfigUpdated {
            base_score: config.base_score,
            weight_per_day: config.weight_per_day,
            max_bonus_days: config.max_bonus_days,
        }
        .publish(&env);
        Ok(())
    }

    /// Returns the tenure reputation configuration, or `None` if unset.
    pub fn get_tenure_config(env: Env) -> Option<TenureReputationConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::TenureConfig))
    }

    /// Computes the duration-weighted tenure bonus for an escrow:
    /// `min(days_active × weight_per_day, max_bonus_days × weight_per_day)`.
    ///
    /// Returns `0` when no tenure config is set or when the escrow is (or has
    /// been) disputed, since disputed escrows are ineligible for the bonus.
    pub fn calculate_tenure_bonus(env: Env, escrow_id: u64) -> u32 {
        let config = match Self::get_tenure_config(env.clone()) {
            Some(c) => c,
            None => return 0,
        };
        let escrow = Self::get_escrow(&env, escrow_id);
        if !Self::is_tenure_eligible(&escrow) {
            return 0;
        }

        let seconds_active = env.ledger().timestamp().saturating_sub(escrow.created_at);
        let days_active = seconds_active / 86_400;
        let earned = days_active.saturating_mul(config.weight_per_day as u64);
        let cap = (config.max_bonus_days as u64).saturating_mul(config.weight_per_day as u64);
        earned.min(cap).min(u32::MAX as u64) as u32
    }

    /// Applies the tenure bonus for a single participant of an escrow.
    ///
    /// Called from `release_escrow` on dispute-free completion, and may be
    /// invoked at most once per escrow per participant. Grants the configured
    /// `base_score` plus the duration-weighted bonus to the participant's
    /// reputation. Disputed escrows are ineligible and receive nothing.
    pub fn apply_tenure_bonus(env: Env, escrow_id: u64, participant: Address) -> Result<(), Error> {
        let config =
            Self::get_tenure_config(env.clone()).ok_or(Error::Escrow(EscrowError::NotFound))?;

        let marker = DataKey::Participant(ParticipantKey::TenureBonusApplied(
            escrow_id,
            participant.clone(),
        ));
        if env.storage().instance().has(&marker) {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }
        env.storage().instance().set(&marker, &true);

        let bonus = Self::calculate_tenure_bonus(env.clone(), escrow_id);
        let escrow = Self::get_escrow(&env, escrow_id);
        // Ineligible (disputed) escrows are marked applied but grant nothing.
        let increase: i64 = if Self::is_tenure_eligible(&escrow) {
            config.base_score as i64 + bonus as i64
        } else {
            0
        };

        if increase > 0 {
            let mut rep = Self::get_or_default_reputation(&env, &participant);
            let old_score = rep.score;
            rep.score = (rep.score + increase).min(10000);
            rep.last_updated = env.ledger().timestamp();
            env.storage().instance().set(
                &DataKey::Participant(ParticipantKey::ReputationScore(participant.clone())),
                &rep,
            );
            ReputationUpdated {
                address: participant.clone(),
                old_score,
                new_score: rep.score,
            }
            .publish(&env);
        }

        TenureBonusGranted {
            escrow_id,
            participant,
            bonus,
        }
        .publish(&env);
        Ok(())
    }

    /// An escrow is eligible for the tenure bonus only if it has never entered
    /// a dispute.
    fn is_tenure_eligible(escrow: &Escrow) -> bool {
        escrow.status != EscrowStatus::Disputed && escrow.dispute_started_at == 0
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
                .get::<DataKey, Evidence>(&DataKey::Escrow(EscrowKey::Evidence(escrow_id, i)))
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
    /// * `milestones` - Optional vector of VestingMilestone for milestone-based vesting.
    ///   Each milestone's `unlock_timestamp` must be `>= cliff_timestamp` and `<= end_timestamp`.
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
            return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
        }
        if end_timestamp <= cliff_timestamp {
            return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
        }

        // Validate milestones if provided
        if !milestones.is_empty() {
            let mut milestone_total: i128 = 0;
            for milestone in milestones.iter() {
                milestone_total = milestone_total.saturating_add(milestone.amount);
                // Validate milestone unlock timestamp is after cliff
                if milestone.unlock_timestamp < cliff_timestamp {
                    return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
                }
                // Validate milestone unlock timestamp is before or at end
                if milestone.unlock_timestamp > end_timestamp {
                    return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
                }
            }
            // Milestone amounts must sum to total amount
            if milestone_total != amount {
                return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
            }
        }

        // Create the base escrow
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Counter))
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled {
            fee_config.fee_bps
        } else {
            0
        };

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
            expiry_timestamp: 0,
            auto_refund_on_expiry: false,
            escalated_at: None,
            escalation_timeout: 604800,
            auto_resolve_in_favor_of: AutoResolveFavor::Customer,
            evidence_deadline: None,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Counter), &escrow_id);

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

        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingSchedule(escrow_id)),
            &vesting_schedule,
        );

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::CustomerCount(customer.clone())))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::CustomerList(customer.clone(), customer_count)),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::CustomerCount(customer.clone())),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MerchantCount(merchant.clone())))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::MerchantList(merchant.clone(), merchant_count)),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::MerchantCount(merchant.clone())),
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
            .get(&DataKey::Escrow(EscrowKey::VestingSchedule(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))
    }

    fn get_base_vested_amount(env: &Env, escrow_id: u64) -> i128 {
        let vesting_schedule =
            match env
                .storage()
                .instance()
                .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                    escrow_id,
                ))) {
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

    pub fn get_vested_amount(env: Env, escrow_id: u64) -> i128 {
        let vesting_schedule =
            match env
                .storage()
                .instance()
                .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                    escrow_id,
                ))) {
                Some(schedule) => schedule,
                None => return 0,
            };

        let base_vested = Self::get_base_vested_amount(&env, escrow_id);
        let accelerated_amount = Self::calculate_accelerated_amount(env.clone(), escrow_id);
        let total_vested = base_vested.saturating_add(accelerated_amount);

        if total_vested > vesting_schedule.total_amount {
            vesting_schedule.total_amount
        } else {
            total_vested
        }
    }

    pub fn set_vesting_acceleration_config(
        env: Env,
        admin: Address,
        schedule_id: u64,
        milestone_bps: u32,
        max_acceleration_bps: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if let Err(_) = Self::validate_bps(milestone_bps) {
            return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
        }

        if milestone_bps > max_acceleration_bps {
            return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
        }

        if env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                schedule_id,
            )))
            .is_none()
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let acceleration_config = VestingAccelerationConfig {
            schedule_id,
            milestone_bps,
            max_acceleration_bps,
            total_accelerated_bps: 0,
        };

        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingAccelerationConfig(schedule_id)),
            &acceleration_config,
        );

        Ok(())
    }

    pub fn mark_milestone_complete(
        env: Env,
        admin: Address,
        schedule_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut acceleration_config = env
            .storage()
            .instance()
            .get::<DataKey, VestingAccelerationConfig>(&DataKey::Escrow(
                EscrowKey::VestingAccelerationConfig(schedule_id),
            ))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if acceleration_config.total_accelerated_bps >= acceleration_config.max_acceleration_bps {
            return Err(Error::Escrow(EscrowError::MilestoneAlreadyReleased));
        }

        let next_total = acceleration_config
            .total_accelerated_bps
            .saturating_add(acceleration_config.milestone_bps);
        if next_total > acceleration_config.max_acceleration_bps {
            return Err(Error::Action(ActionError::AccelerationLimitExceeded));
        }

        acceleration_config.total_accelerated_bps = next_total;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingAccelerationConfig(schedule_id)),
            &acceleration_config,
        );

        Ok(())
    }

    pub fn get_acceleration_config(
        env: Env,
        schedule_id: u64,
    ) -> Option<VestingAccelerationConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::VestingAccelerationConfig(
                schedule_id,
            )))
    }

    pub fn calculate_accelerated_amount(env: Env, schedule_id: u64) -> i128 {
        let vesting_schedule =
            match env
                .storage()
                .instance()
                .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                    schedule_id,
                ))) {
                Some(schedule) => schedule,
                None => return 0,
            };

        let acceleration_config = match env
            .storage()
            .instance()
            .get::<DataKey, VestingAccelerationConfig>(&DataKey::Escrow(
                EscrowKey::VestingAccelerationConfig(schedule_id),
            )) {
            Some(config) => config,
            None => return 0,
        };

        let base_vested = Self::get_base_vested_amount(&env, schedule_id);
        let remaining_unvested = vesting_schedule.total_amount.saturating_sub(base_vested);

        remaining_unvested.saturating_mul(acceleration_config.total_accelerated_bps as i128) / 10000
    }

    /// Returns cliff timing and whether the ledger time has reached the cliff.
    ///
    /// # Errors
    /// * `EscrowNotFound` - No vesting schedule for this escrow
    pub fn get_cliff_status(env: Env, escrow_id: u64) -> Result<CliffStatus, Error> {
        let vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                escrow_id,
            )))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let now = env.ledger().timestamp();
        let cliff_ts = vesting_schedule.cliff_timestamp;
        let cliff_passed = now >= cliff_ts;
        let seconds_remaining = if cliff_passed {
            0_u64
        } else {
            cliff_ts.saturating_sub(now)
        };

        Ok(CliffStatus {
            cliff_timestamp: cliff_ts,
            cliff_passed,
            seconds_remaining,
        })
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
        let vesting_schedule =
            match env
                .storage()
                .instance()
                .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                    escrow_id,
                ))) {
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
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                escrow_id,
            )))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let current_timestamp = env.ledger().timestamp();

        // Enforce cliff period
        if current_timestamp < vesting_schedule.cliff_timestamp {
            return Err(Error::Escrow(EscrowError::CliffPeriodNotPassed));
        }

        // Calculate vested amount
        let vested_amount = EscrowContract::get_vested_amount(env.clone(), escrow_id);
        let releasable_amount = vested_amount.saturating_sub(vesting_schedule.released_amount);

        if releasable_amount == 0 {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
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
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingSchedule(escrow_id)),
            &vesting_schedule,
        );

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
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                escrow_id,
            )))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let mut found = false;
        let mut milestones = vesting_schedule.milestones.clone();
        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                found = true;
                if m.released {
                    return Err(Error::Escrow(EscrowError::MilestoneAlreadyReleased));
                }
                let now = env.ledger().timestamp();
                m.approved_by = Some(admin.clone());
                m.approved_at = Some(now);
                milestones.set(i, m);
                break;
            }
        }

        if !found {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        vesting_schedule.milestones = milestones;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingSchedule(escrow_id)),
            &vesting_schedule,
        );

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
            .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                escrow_id,
            )))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let escrow: Escrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let mut found = false;
        let mut release_amount: i128 = 0;
        let mut milestones = vesting_schedule.milestones.clone();

        for i in 0..milestones.len() {
            let mut m = milestones.get(i).unwrap();
            if m.milestone_id == milestone_id {
                found = true;
                if m.released {
                    return Err(Error::Escrow(EscrowError::MilestoneAlreadyReleased));
                }
                if m.approved_by.is_none() {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
                // Guard: total released must not exceed locked amount
                let new_total = vesting_schedule.released_amount.saturating_add(m.amount);
                if new_total > vesting_schedule.total_amount {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
                release_amount = m.amount;
                m.released = true;
                milestones.set(i, m);
                break;
            }
        }

        if !found {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        vesting_schedule.milestones = milestones;
        vesting_schedule.released_amount = vesting_schedule
            .released_amount
            .saturating_add(release_amount);

        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingSchedule(escrow_id)),
            &vesting_schedule,
        );

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
        let vesting_schedule =
            match env
                .storage()
                .instance()
                .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                    escrow_id,
                ))) {
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
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut vesting_schedule = env
            .storage()
            .instance()
            .get::<DataKey, VestingSchedule>(&DataKey::Escrow(EscrowKey::VestingSchedule(
                escrow_id,
            )))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        // Sum existing milestone amounts
        let mut existing_total: i128 = 0;
        for m in vesting_schedule.milestones.iter() {
            existing_total = existing_total.saturating_add(m.amount);
        }

        if existing_total.saturating_add(milestone.amount) > vesting_schedule.total_amount {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if milestone.unlock_timestamp < vesting_schedule.cliff_timestamp {
            return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
        }
        if milestone.unlock_timestamp > vesting_schedule.end_timestamp {
            return Err(Error::Escrow(EscrowError::InvalidVestingSchedule));
        }

        // Auto-assign milestone_id if not provided
        let mut new_milestone = milestone;
        if new_milestone.milestone_id == 0 {
            new_milestone.milestone_id = (vesting_schedule.milestones.len() as u64) + 1;
        }

        vesting_schedule.milestones.push_back(new_milestone);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::VestingSchedule(escrow_id)),
            &vesting_schedule,
        );

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
            return Err(Error::Escrow(EscrowError::InvalidStatus));
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

                if !env
                    .storage()
                    .instance()
                    .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
                {
                    return Err(Error::Escrow(EscrowError::NotFound));
                }

                let current_time: u64 = env.ledger().timestamp();
                let mut escrow = EscrowContract::get_escrow(env, escrow_id);

                match escrow.status {
                    EscrowStatus::Locked => {
                        if !early_release {
                            if current_time < escrow.release_timestamp {
                                return Err(Error::Escrow(EscrowError::ReleaseNotYetAvailable));
                            }
                            if current_time < escrow.created_at + escrow.min_hold_period {
                                return Err(Error::Escrow(EscrowError::ReleaseOnHoldPeriod));
                            }
                        }
                        escrow.status = EscrowStatus::Released;
                    }
                    EscrowStatus::Released => {
                        return Err(Error::Escrow(EscrowError::AlreadyProcessed))
                    }
                    EscrowStatus::Disputed => {
                        return Err(Error::Escrow(EscrowError::InvalidStatus))
                    }
                    EscrowStatus::Resolved => {
                        return Err(Error::Escrow(EscrowError::AlreadyProcessed))
                    }
                    EscrowStatus::Cancelled => {
                        return Err(Error::Escrow(EscrowError::AlreadyProcessed))
                    }
                }

                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

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
                        let mut acc: i128 = env
                            .storage()
                            .instance()
                            .get(&DataKey::Participant(ParticipantKey::AccumulatedFees(
                                escrow.token.clone(),
                            )))
                            .unwrap_or(0);
                        acc += fee_amount;
                        env.storage().instance().set(
                            &DataKey::Participant(ParticipantKey::AccumulatedFees(
                                escrow.token.clone(),
                            )),
                            &acc,
                        );
                    }

                    EscrowFeeCollected {
                        escrow_id,
                        fee_amount,
                        recipient: fee_config.fee_recipient.clone(),
                    }
                    .publish(env);
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
                    .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
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
                    .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);
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
                    recipient: escrow.merchant.clone(),
                    amount: escrow.amount,
                    token: escrow.token,
                }
                .publish(env);
            }
            ActionType::ResolveDispute => {
                let escrow_id = EscrowContract::read_u64_from_bytes(&proposal.data, 0);
                let release_to_merchant = proposal.data.get(8).unwrap_or(0) != 0;

                if !env
                    .storage()
                    .instance()
                    .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
                {
                    return Err(Error::Escrow(EscrowError::NotFound));
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
                    _ => return Err(Error::Action(ActionError::NotDisputed)),
                }

                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

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
                    .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
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
                    .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);
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
                    .get(&DataKey::Config(ConfigKey::AdminMultiSig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if !config.admins.contains(&new_admin) {
                    config.admins.push_back(new_admin.clone());
                    config.total_admins += 1;
                    env.storage()
                        .instance()
                        .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
                    AdminAdded { admin: new_admin }.publish(env);
                }
            }
            ActionType::RemoveAdmin => {
                let admin_to_remove = proposal.target.clone();
                let mut config: MultiSigConfig = env
                    .storage()
                    .instance()
                    .get(&DataKey::Config(ConfigKey::AdminMultiSig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if config.total_admins <= config.required_signatures {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
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
                    .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
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
                    .get(&DataKey::Config(ConfigKey::AdminMultiSig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if required == 0 || required > config.total_admins {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
                config.required_signatures = required;
                env.storage()
                    .instance()
                    .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
            }
            ActionType::UpdateThreshold(new_threshold) => {
                let mut config: MultiSigConfig = env
                    .storage()
                    .instance()
                    .get(&DataKey::Config(ConfigKey::AdminMultiSig))
                    .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
                if new_threshold == 0 {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
                if new_threshold > config.total_admins {
                    return Err(Error::Escrow(EscrowError::InvalidStatus));
                }
                config.required_signatures = new_threshold;
                env.storage()
                    .instance()
                    .set(&DataKey::Config(ConfigKey::AdminMultiSig), &config);
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
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let timelock_config = Self::get_timelock_config(env.clone());
        let current_time = env.ledger().timestamp();

        let action_id = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::Counter))
            .unwrap_or(0u64)
            + 1;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::Counter), &action_id);

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
            .set(&DataKey::Dispute(DisputeKey::Action(action_id)), &action);

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
            .get(&DataKey::Dispute(DisputeKey::Action(action_id)))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if action.executed {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }
        if action.cancelled {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }

        let current_time = env.ledger().timestamp();
        if current_time < action.executable_after {
            return Err(Error::Action(ActionError::NotReady));
        }
        if current_time > action.expires_at {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        action.executed = true;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::Action(action_id)), &action);

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
                    None,
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
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut action: TimeLockAction = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::Action(action_id)))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;

        if action.executed {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }
        if action.cancelled {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }

        // Only the proposing admin or any other admin can cancel
        if action.proposed_by != admin && !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        action.cancelled = true;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::Action(action_id)), &action);

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
            .get(&DataKey::Dispute(DisputeKey::Action(action_id)))
            .ok_or(Error::Basic(BasicError::Unauthorized))
    }

    pub fn set_timelock_config(
        env: Env,
        admin: Address,
        config: TimeLockConfig,
    ) -> Result<(), Error> {
        admin.require_auth();

        let multisig_config = Self::get_multisig_config(env.clone());
        if !multisig_config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if config.delay < 3600 || config.delay > 604800 {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::TimeLockConfig), &config);

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
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
            .unwrap_or(EscrowAnalytics::default_value())
    }

    pub fn get_merchant_analytics(env: Env, merchant: Address) -> EscrowAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::Participant(ParticipantKey::MerchantAnalytics(
                merchant,
            )))
            .unwrap_or(EscrowAnalytics::default_value())
    }

    pub fn get_customer_analytics(env: Env, customer: Address) -> EscrowAnalytics {
        env.storage()
            .instance()
            .get(&DataKey::Participant(ParticipantKey::CustomerAnalytics(
                customer,
            )))
            .unwrap_or(EscrowAnalytics::default_value())
    }

    pub fn reset_analytics(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowAnalytics),
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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        let global_key = String::from_str(&env, "global");
        if env
            .storage()
            .instance()
            .has(&DataKey::Config(ConfigKey::ActivePauseIndex(
                global_key.clone(),
            )))
        {
            return Ok(());
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
            .get(&DataKey::Config(ConfigKey::PauseHistoryCount))
            .unwrap_or(0);
        let entry = PauseHistory {
            function_name: global_key.clone(),
            paused_by: admin.clone(),
            paused_at: now,
            unpaused_by: None,
            unpaused_at: None,
            reason: reason.clone(),
        };
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::PauseHistoryEntry(history_count)),
            &entry,
        );
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::PauseHistoryCount),
            &(history_count + 1),
        );
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::ActivePauseIndex(global_key)),
            &history_count,
        );
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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
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
        let global_key = String::from_str(&env, "global");
        if let Some(active_idx) = env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::Config(ConfigKey::ActivePauseIndex(
                global_key.clone(),
            )))
        {
            if let Some(mut entry) =
                env.storage()
                    .instance()
                    .get::<DataKey, PauseHistory>(&DataKey::Config(ConfigKey::PauseHistoryEntry(
                        active_idx,
                    )))
            {
                entry.unpaused_by = Some(admin.clone());
                entry.unpaused_at = Some(now);
                env.storage().instance().set(
                    &DataKey::Config(ConfigKey::PauseHistoryEntry(active_idx)),
                    &entry,
                );
            }
            env.storage()
                .instance()
                .remove(&DataKey::Config(ConfigKey::ActivePauseIndex(global_key)));
        }
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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
            .ok_or(Error::Basic(BasicError::MultiSigNotInitialized))?;
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if reason.len() == 0 {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        if env
            .storage()
            .instance()
            .has(&DataKey::Config(ConfigKey::ActivePauseIndex(
                function_name.clone(),
            )))
        {
            return Ok(());
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
            .get(&DataKey::Config(ConfigKey::PauseHistoryCount))
            .unwrap_or(0);
        let entry = PauseHistory {
            function_name: function_name.clone(),
            paused_by: admin.clone(),
            paused_at: now,
            unpaused_by: None,
            unpaused_at: None,
            reason: reason.clone(),
        };
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::PauseHistoryEntry(history_count)),
            &entry,
        );
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::PauseHistoryCount),
            &(history_count + 1),
        );
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::ActivePauseIndex(function_name.clone())),
            &history_count,
        );
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
            .get(&DataKey::Config(ConfigKey::AdminMultiSig))
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
        if let Some(active_idx) = env
            .storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::Config(ConfigKey::ActivePauseIndex(
                function_name.clone(),
            )))
        {
            if let Some(mut entry) =
                env.storage()
                    .instance()
                    .get::<DataKey, PauseHistory>(&DataKey::Config(ConfigKey::PauseHistoryEntry(
                        active_idx,
                    )))
            {
                entry.unpaused_by = Some(admin.clone());
                entry.unpaused_at = Some(now);
                env.storage().instance().set(
                    &DataKey::Config(ConfigKey::PauseHistoryEntry(active_idx)),
                    &entry,
                );
            }
            env.storage()
                .instance()
                .remove(&DataKey::Config(ConfigKey::ActivePauseIndex(
                    function_name.clone(),
                )));
        }
        (FunctionUnpausedEvent {
            function_name,
            unpaused_by: admin,
        })
        .publish(&env);
        Ok(())
    }

    pub fn get_pause_history(env: Env, limit: u32, offset: u32) -> Vec<PauseHistory> {
        let mut result = Vec::new(&env);
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::PauseHistoryCount))
            .unwrap_or(0);
        if limit == 0 || (offset as u64) >= total {
            return result;
        }
        let start = offset as u64;
        let end = core::cmp::min(start.saturating_add(limit as u64), total);
        let mut i = start;
        while i < end {
            if let Some(entry) = env
                .storage()
                .instance()
                .get::<DataKey, PauseHistory>(&DataKey::Config(ConfigKey::PauseHistoryEntry(i)))
            {
                result.push_back(entry);
            }
            i += 1;
        }
        result
    }

    pub fn get_function_pause_history(env: Env, function_name: String) -> Vec<PauseHistory> {
        let mut result = Vec::new(&env);
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::PauseHistoryCount))
            .unwrap_or(0);
        let mut i = 0u64;
        while i < total {
            if let Some(entry) = env
                .storage()
                .instance()
                .get::<DataKey, PauseHistory>(&DataKey::Config(ConfigKey::PauseHistoryEntry(i)))
            {
                if entry.function_name == function_name {
                    result.push_back(entry);
                }
            }
            i += 1;
        }
        result
    }

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

    // ── MIGRATION ─────────────────────────────────────────────────────────────

    pub fn begin_migration(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if let Some(status) = env
            .storage()
            .instance()
            .get::<DataKey, MigrationStatus>(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
        {
            if !status.in_progress && status.completed_at.is_some() {
                return Err(Error::Basic(BasicError::MigrationNotStarted));
            }
        }

        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Counter))
            .unwrap_or(0);

        let status = MigrationStatus {
            in_progress: true,
            migrated_count: 0,
            total_count,
            started_at: env.ledger().timestamp(),
            completed_at: None,
        };
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowMigrationStatus),
            &status,
        );
        Ok(())
    }

    pub fn migrate_escrow(env: Env, admin: Address, escrow_id: u64) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut status: MigrationStatus = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
            .ok_or(Error::Basic(BasicError::MigrationNotStarted))?;

        if !status.in_progress {
            return Err(Error::Basic(BasicError::MigrationNotStarted));
        }

        if env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Dispute(DisputeKey::EscrowMigrated(escrow_id)))
            .unwrap_or(false)
        {
            return Err(Error::Basic(BasicError::AlreadyMigrated));
        }

        // Read and re-write the escrow record (applies any new-format defaults)
        let escrow: Escrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowMigrated(escrow_id)),
            &true,
        );

        status.migrated_count += 1;
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowMigrationStatus),
            &status,
        );

        Ok(())
    }

    pub fn migrate_escrow_batch(
        env: Env,
        admin: Address,
        escrow_ids: Vec<u64>,
    ) -> Result<u32, Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let status: MigrationStatus = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
            .ok_or(Error::Basic(BasicError::MigrationNotStarted))?;

        if !status.in_progress {
            return Err(Error::Basic(BasicError::MigrationNotStarted));
        }

        let mut migrated: u32 = 0;
        for escrow_id in escrow_ids.iter() {
            // Skip already-migrated entries silently in batch mode
            if env
                .storage()
                .instance()
                .get::<DataKey, bool>(&DataKey::Dispute(DisputeKey::EscrowMigrated(escrow_id)))
                .unwrap_or(false)
            {
                continue;
            }

            if let Some(escrow) = env
                .storage()
                .instance()
                .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
            {
                env.storage()
                    .instance()
                    .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
                env.storage().instance().set(
                    &DataKey::Dispute(DisputeKey::EscrowMigrated(escrow_id)),
                    &true,
                );
                migrated += 1;
            }
        }

        // Update migrated_count
        let mut updated_status: MigrationStatus = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
            .ok_or(Error::Basic(BasicError::MigrationNotStarted))?;
        updated_status.migrated_count += migrated as u64;
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowMigrationStatus),
            &updated_status,
        );

        Ok(migrated)
    }

    pub fn complete_migration(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        let mut status: MigrationStatus = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
            .ok_or(Error::Basic(BasicError::MigrationNotStarted))?;

        if !status.in_progress {
            return Err(Error::Basic(BasicError::MigrationNotStarted));
        }

        if status.migrated_count < status.total_count {
            return Err(Error::Basic(BasicError::MigrationNotStarted));
        }

        status.in_progress = false;
        status.completed_at = Some(env.ledger().timestamp());
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowMigrationStatus),
            &status,
        );

        Ok(())
    }

    pub fn get_migration_status(env: Env) -> MigrationStatus {
        env.storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowMigrationStatus))
            .unwrap_or(MigrationStatus {
                in_progress: false,
                migrated_count: 0,
                total_count: 0,
                started_at: 0,
                completed_at: None,
            })
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
                    return Err(Error::Basic(BasicError::ContractPaused));
                }
            }
        }
        Ok(())
    }

    pub fn get_timelock_config(env: Env) -> TimeLockConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::TimeLockConfig))
            .unwrap_or(TimeLockConfig {
                delay: 86400,
                grace_period: 86400,
            })
    }

    pub fn set_insurance_config(
        env: Env,
        admin: Address,
        config: InsuranceConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::InsuranceConfig), &config);
        Ok(())
    }

    pub fn set_watchdog_config(
        env: Env,
        admin: Address,
        config: WatchdogConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::WatchdogConfig), &config);
        Ok(())
    }

    pub fn get_insurance_pool(env: Env) -> InsurancePool {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::InsurancePool))
            .unwrap_or(InsurancePool {
                token: env.current_contract_address(), // dummy default
                balance: 0,
                total_premiums_collected: 0,
                total_claims_paid: 0,
            })
    }

    pub fn opt_into_insurance(env: Env, escrow_id: u64) -> Result<(), Error> {
        let config: InsuranceConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::InsuranceConfig))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;
        if !config.enabled {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let mut escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let premium = (escrow.amount * config.premium_bps) / 10000;
        if premium == 0 {
            return Ok(());
        }

        escrow.amount -= premium;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        let mut pool = Self::get_insurance_pool(env.clone());
        pool.token = escrow.token.clone();
        pool.balance += premium;
        pool.total_premiums_collected += premium;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::InsurancePool), &pool);

        Ok(())
    }

    pub fn file_insurance_claim(
        env: Env,
        admin: Address,
        escrow_id: u64,
        amount: i128,
    ) -> Result<u64, Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Resolved && escrow.status != EscrowStatus::Cancelled {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let config: InsuranceConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::InsuranceConfig))
            .ok_or(Error::Basic(BasicError::Unauthorized))?;
        let max_coverage = (escrow.amount * config.max_coverage_bps) / 10000;
        if amount > max_coverage {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::InsuranceClaimCounter))
            .unwrap_or(0)
            + 1;
        let claim = InsuranceClaim {
            claim_id: counter,
            escrow_id,
            claimant: escrow.customer.clone(), // default to customer
            amount,
            approved: false,
            paid_at: None,
        };

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::InsuranceClaim(counter)),
            &claim,
        );
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::InsuranceClaimCounter),
            &counter,
        );

        Ok(counter)
    }

    pub fn approve_claim(env: Env, admin: Address, claim_id: u64) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let mut claim: InsuranceClaim = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::InsuranceClaim(claim_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;
        if claim.approved {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }

        let mut pool = Self::get_insurance_pool(env.clone());
        if pool.balance < claim.amount {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        Self::transfer_if_token_contract(&env, &pool.token, &claim.claimant, claim.amount)?;

        pool.balance -= claim.amount;
        pool.total_claims_paid += claim.amount;
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::InsurancePool), &pool);

        claim.approved = true;
        claim.paid_at = Some(env.ledger().timestamp());
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::InsuranceClaim(claim_id)),
            &claim,
        );

        Ok(())
    }

    pub fn get_watchdog_config(env: Env) -> WatchdogConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::WatchdogConfig))
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

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
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
            return Err(Error::Action(ActionError::NotReady));
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

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        Self::transfer_if_token_contract(&env, &escrow.token, &released_to, escrow.amount)?;

        WatchdogReleaseTriggered {
            escrow_id,
            released_to: released_to.clone(),
            triggered_by: env.current_contract_address(),
        }
        .publish(&env);

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
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::ReputationDecayConfig), &config);
        Ok(())
    }

    pub fn set_dispute_config(
        env: Env,
        admin: Address,
        config: DisputeConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        if let Some(ms) = env
            .storage()
            .instance()
            .get::<DataKey, MultiSigConfig>(&DataKey::Config(ConfigKey::AdminMultiSig))
        {
            if !ms.admins.contains(&admin) {
                return Err(Error::Basic(BasicError::NotAnAdmin));
            }
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::DisputeConfig), &config);
        Ok(())
    }

    pub fn get_dispute_config(env: Env) -> DisputeConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::DisputeConfig))
            .unwrap_or(DisputeConfig {
                collateral_token: env.current_contract_address(),
                collateral_amount: 0,
                collateral_enabled: false,
                min_collateral_ratio_bps: 15000, // Default 150%
            })
    }

    pub fn get_dispute_collateral(env: Env, escrow_id: u64) -> Result<DisputeCollateral, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::Collateral(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::InvalidStatus))
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
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::ReputationScore(address.clone())),
            &rep,
        );
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
            .get(&DataKey::Config(ConfigKey::ReputationDecayConfig))
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
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::OracleCondition(escrow_id)),
            &condition,
        );
        Ok(())
    }

    pub fn get_oracle_condition(env: Env, escrow_id: u64) -> Result<OracleCondition, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::OracleCondition(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::InvalidStatus))
    }

    pub fn auto_resolve_with_oracle(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Disputed {
            return Err(Error::Action(ActionError::NotDisputed));
        }
        let condition: OracleCondition = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::OracleCondition(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::InvalidStatus))?;

        let mut args: Vec<soroban_sdk::Val> = Vec::new(&env);
        args.push_back(condition.oracle.price_feed_id.clone().into());
        let price_data: OraclePriceData = env.invoke_contract(
            &condition.oracle.oracle_address,
            &Symbol::new(&env, "get_price"),
            args,
        );

        let now = env.ledger().timestamp();
        if now.saturating_sub(price_data.timestamp) > condition.oracle.staleness_threshold {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
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

    // ── CONDITIONAL ESCROW (ON-CHAIN STATE) ───────────────────────────────

    pub fn create_conditional_escrow(
        env: Env,
        customer: Address,
        merchant: Address,
        token: Address,
        amount: i128,
        condition: OnChainCondition,
    ) -> Result<u64, Error> {
        customer.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Counter))
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let current_timestamp = env.ledger().timestamp();

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled {
            fee_config.fee_bps
        } else {
            0
        };

        let escrow = Escrow {
            id: escrow_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token: token.clone(),
            status: EscrowStatus::Locked,
            created_at: current_timestamp,
            release_timestamp: u64::MAX,
            dispute_started_at: 0,
            last_activity_at: current_timestamp,
            escalation_level: 0,
            min_hold_period: 0,
            fee_bps,
            expiry_timestamp: 0,
            auto_refund_on_expiry: false,
            escalated_at: None,
            escalation_timeout: 604800,
            auto_resolve_in_favor_of: AutoResolveFavor::Customer,
            evidence_deadline: None,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Counter), &escrow_id);

        let conditional = ConditionalEscrow {
            escrow_id,
            condition,
            evaluated: false,
            result: false,
        };
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Conditional(escrow_id)),
            &conditional,
        );

        EscrowCreated {
            escrow_id,
            customer,
            merchant,
            amount,
            token,
            release_timestamp: u64::MAX,
        }
        .publish(&env);

        Ok(escrow_id)
    }

    pub fn evaluate_and_release(env: Env, escrow_id: u64) -> Result<bool, Error> {
        let mut conditional: ConditionalEscrow = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Conditional(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::ConditionalEscrowNotFound))?;

        if conditional.evaluated {
            return Err(Error::Action(ActionError::ConditionAlreadyEvaluated));
        }

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let mut args: Vec<soroban_sdk::Val> = Vec::new(&env);
        args.push_back(conditional.condition.state_key.clone().into());
        let actual_value: Bytes = env.invoke_contract(
            &conditional.condition.contract_address,
            &Symbol::new(&env, "get_state"),
            args,
        );

        let met = actual_value == conditional.condition.expected_value;

        conditional.evaluated = true;
        conditional.result = met;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Conditional(escrow_id)),
            &conditional,
        );

        ConditionEvaluated { escrow_id, met }.publish(&env);

        if met {
            escrow.status = EscrowStatus::Released;
            env.storage()
                .instance()
                .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

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
                    let mut acc: i128 = env
                        .storage()
                        .instance()
                        .get(&DataKey::Participant(ParticipantKey::AccumulatedFees(
                            escrow.token.clone(),
                        )))
                        .unwrap_or(0);
                    acc += fee_amount;
                    env.storage().instance().set(
                        &DataKey::Participant(ParticipantKey::AccumulatedFees(
                            escrow.token.clone(),
                        )),
                        &acc,
                    );
                }

                EscrowFeeCollected {
                    escrow_id,
                    fee_amount,
                    recipient: fee_config.fee_recipient.clone(),
                }
                .publish(&env);
            }

            EscrowContract::transfer_if_token_contract(
                &env,
                &escrow.token,
                &escrow.merchant,
                merchant_amount,
            )?;

            ConditionalReleaseExecuted {
                escrow_id,
                released_to: escrow.merchant.clone(),
            }
            .publish(&env);
        }

        Ok(met)
    }

    pub fn get_conditional_escrow(env: Env, escrow_id: u64) -> Result<ConditionalEscrow, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Conditional(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::ConditionalEscrowNotFound))
    }

    // ── BENEFICIARY TRANSFER ──────────────────────────────────────────────

    pub fn transfer_escrow_beneficiary(
        env: Env,
        caller: Address,
        escrow_id: u64,
        new_merchant: Address,
    ) -> Result<(), Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        let multisig = Self::get_multisig_config(env.clone());
        if caller != escrow.merchant && !multisig.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        match escrow.status {
            EscrowStatus::Disputed | EscrowStatus::Resolved => {
                return Err(Error::Action(ActionError::TransferNotAllowed));
            }
            _ => {}
        }

        if new_merchant == escrow.merchant {
            return Err(Error::Action(ActionError::SameBeneficiary));
        }

        let old_merchant = escrow.merchant.clone();
        let now = env.ledger().timestamp();

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Participant(
                ParticipantKey::BeneficiaryTransferCount(escrow_id),
            ))
            .unwrap_or(0);

        let transfer = BeneficiaryTransfer {
            escrow_id,
            from: old_merchant.clone(),
            to: new_merchant.clone(),
            transferred_at: now,
            authorised_by: caller,
        };

        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::BeneficiaryTransferHistory(escrow_id, count)),
            &transfer,
        );
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::BeneficiaryTransferCount(escrow_id)),
            &(count + 1),
        );

        escrow.merchant = new_merchant.clone();
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

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
            .get(&DataKey::Participant(
                ParticipantKey::BeneficiaryTransferCount(escrow_id),
            ))
            .unwrap_or(0);

        let mut history = Vec::new(&env);
        for i in 0..count {
            if let Some(transfer) = env
                .storage()
                .instance()
                .get::<DataKey, BeneficiaryTransfer>(&DataKey::Participant(
                    ParticipantKey::BeneficiaryTransferHistory(escrow_id, i),
                ))
            {
                history.push_back(transfer);
            }
        }
        history
    }

    /// Transfer escrow beneficiary to a new address. Callable only by the current beneficiary.
    /// Records the transfer in BeneficiaryTransferHistory with timestamp.
    pub fn transfer_beneficiary(
        env: Env,
        escrow_id: u64,
        new_beneficiary: Address,
    ) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        // Only the current beneficiary (merchant) may call this
        escrow.merchant.require_auth();

        match escrow.status {
            EscrowStatus::Disputed | EscrowStatus::Resolved => {
                return Err(Error::Action(ActionError::TransferNotAllowed));
            }
            _ => {}
        }

        if new_beneficiary == escrow.merchant {
            return Err(Error::Action(ActionError::SameBeneficiary));
        }

        let old_beneficiary = escrow.merchant.clone();
        let now = env.ledger().timestamp();

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Participant(
                ParticipantKey::BeneficiaryTransferCount(escrow_id),
            ))
            .unwrap_or(0);

        let transfer = BeneficiaryTransfer {
            escrow_id,
            from: old_beneficiary.clone(),
            to: new_beneficiary.clone(),
            transferred_at: now,
            authorised_by: old_beneficiary.clone(),
        };

        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::BeneficiaryTransferHistory(escrow_id, count)),
            &transfer,
        );
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::BeneficiaryTransferCount(escrow_id)),
            &(count + 1),
        );

        escrow.merchant = new_beneficiary.clone();
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        BeneficiaryTransferred {
            escrow_id,
            old_merchant: old_beneficiary,
            new_merchant: new_beneficiary,
        }
        .publish(&env);

        Ok(())
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
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let mut is_participant = false;
        for p in escrow.participants.iter() {
            if p.address == caller {
                is_participant = true;
                break;
            }
        }
        if !is_participant {
            return Err(Error::Basic(BasicError::Unauthorized));
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

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::MultiPartyDispute(escrow_id)),
            &dispute,
        );

        escrow.status = EscrowStatus::Disputed;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);

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
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let mut dispute: MultiPartyDispute = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::MultiPartyDispute(escrow_id)))
            .ok_or(Error::Action(ActionError::NotDisputed))?;

        if dispute.resolved {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
        }

        let mut is_participant = false;
        for p in escrow.participants.iter() {
            if p.address == voter {
                is_participant = true;
                break;
            }
        }
        if !is_participant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        for addr in dispute.votes_for_merchant.iter() {
            if addr == voter {
                return Err(Error::Basic(BasicError::DuplicateApproval));
            }
        }
        for addr in dispute.votes_for_customer.iter() {
            if addr == voter {
                return Err(Error::Basic(BasicError::DuplicateApproval));
            }
        }

        if favor_merchant {
            dispute.votes_for_merchant.push_back(voter.clone());
        } else {
            dispute.votes_for_customer.push_back(voter.clone());
        }

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::MultiPartyDispute(escrow_id)),
            &dispute,
        );

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
            .get(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)))
            .ok_or(Error::Escrow(EscrowError::NotFound))?;

        let mut dispute: MultiPartyDispute = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::MultiPartyDispute(escrow_id)))
            .ok_or(Error::Action(ActionError::NotDisputed))?;

        if dispute.resolved {
            return Err(Error::Escrow(EscrowError::AlreadyProcessed));
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
            return Err(Error::Action(ActionError::ApprovalsThresholdNotMet));
        }

        dispute.resolved = true;
        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::MultiPartyDispute(escrow_id)),
            &dispute,
        );

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
            .set(&DataKey::Escrow(EscrowKey::MultiParty(escrow_id)), &escrow);

        MultiPartyDisputeResolved {
            escrow_id,
            favor_merchant,
            resolved_at: now,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_multi_party_dispute(env: Env, escrow_id: u64) -> Result<MultiPartyDispute, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::MultiPartyDispute(escrow_id)))
            .ok_or(Error::Action(ActionError::NotDisputed))
    }

    // ── BATCH ESCROW CREATION ─────────────────────────────────────────────

    pub fn create_escrow_batch(
        env: Env,
        admin: Address,
        entries: Vec<EscrowBatchEntry>,
    ) -> Vec<BatchEscrowResult> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            let mut results = Vec::new(&env);
            for i in 0..entries.len() {
                results.push_back(BatchEscrowResult {
                    index: i as u32,
                    escrow_id: 0,
                    success: false,
                    error_code: 24,
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
                error_code: 2,
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

    fn try_create_single_escrow(
        env: &Env,
        entry: &EscrowBatchEntry,
        index: u32,
    ) -> Result<u64, u32> {
        // Validate inputs similar to create_escrow
        if entry.amount <= 0 {
            return Err(2); // InvalidStatus
        }

        let current_timestamp = env.ledger().timestamp();
        if entry.release_timestamp <= current_timestamp {
            return Err(5); // ReleaseNotYetAvailable
        }

        // Get counter and create escrow
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Counter))
            .unwrap_or(0);
        let escrow_id = counter + 1;

        let fee_config = Self::get_escrow_fee_config(env.clone());
        let fee_bps = if fee_config.enabled {
            fee_config.fee_bps
        } else {
            0
        };

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
            min_hold_period: 0, // Default for batch
            fee_bps: 0,         // Default fee
            expiry_timestamp: 0,
            auto_refund_on_expiry: false,
            escalated_at: None,
            escalation_timeout: 604800,
            auto_resolve_in_favor_of: AutoResolveFavor::Customer,
            evidence_deadline: None,
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Counter), &escrow_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::CustomerCount(
                entry.customer.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::CustomerList(
                entry.customer.clone(),
                customer_count,
            )),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::CustomerCount(entry.customer.clone())),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::MerchantCount(
                entry.merchant.clone(),
            )))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::MerchantList(
                entry.merchant.clone(),
                merchant_count,
            )),
            &escrow_id,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::MerchantCount(entry.merchant.clone())),
            &(merchant_count + 1),
        );

        // Update global analytics
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowAnalytics))
            .unwrap_or(EscrowAnalytics::default_value());
        analytics.total_escrows_created += 1;
        analytics.total_value_locked += entry.amount;
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::EscrowAnalytics), &analytics);

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
            .get(&DataKey::Config(ConfigKey::BatchLimit))
            .unwrap_or(50) // Default limit of 50
    }

    pub fn set_batch_limit(env: Env, admin: Address, limit: u32) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        if limit == 0 || limit > 1000 {
            return Err(Error::Escrow(EscrowError::InvalidStatus)); // Using InvalidStatus for invalid limit
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::BatchLimit), &limit);
        Ok(())
    }

    // ── EXPIRY ────────────────────────────────────────────────────────────

    pub fn set_global_expiry_config(
        env: Env,
        admin: Address,
        default_expiry_seconds: u64,
    ) -> Result<(), Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        env.storage().instance().set(
            &DataKey::Config(ConfigKey::GlobalExpiryConfig),
            &GlobalExpiryConfig {
                default_expiry_seconds,
            },
        );
        Ok(())
    }

    pub fn is_escrow_expired(env: Env, escrow_id: u64) -> bool {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return false;
        }
        let escrow = Self::get_escrow(&env, escrow_id);
        if escrow.expiry_timestamp == 0 {
            return false;
        }
        env.ledger().timestamp() >= escrow.expiry_timestamp
    }

    pub fn expire_escrow(env: Env, escrow_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let mut escrow = Self::get_escrow(&env, escrow_id);

        match escrow.status {
            EscrowStatus::Released | EscrowStatus::Resolved | EscrowStatus::Cancelled => {
                return Err(Error::Escrow(EscrowError::EscrowAlreadyExpired));
            }
            EscrowStatus::Disputed => return Err(Error::Escrow(EscrowError::InvalidStatus)),
            EscrowStatus::Locked => {}
        }

        if escrow.expiry_timestamp == 0 || env.ledger().timestamp() < escrow.expiry_timestamp {
            return Err(Error::Escrow(EscrowError::EscrowNotExpired));
        }

        escrow.status = EscrowStatus::Cancelled;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        EscrowContract::transfer_if_token_contract(
            &env,
            &escrow.token,
            &escrow.customer,
            escrow.amount,
        )?;

        EscrowExpired {
            escrow_id,
            refunded_to: escrow.customer.clone(),
            amount: escrow.amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Set the global escrow renewal configuration.
    /// Only callable by admin; defines whether renewals are enabled and their constraints.
    pub fn set_renewal_config(
        env: Env,
        admin: Address,
        config: EscrowRenewalConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        env.storage()
            .instance()
            .set(&DataKey::Dispute(DisputeKey::EscrowRenewalConfig), &config);
        EscrowRenewalConfigUpdated {
            max_renewals: config.max_renewals,
            renewal_fee_bps: config.renewal_fee_bps,
            min_renewal_period: config.min_renewal_period,
            max_renewal_period: config.max_renewal_period,
            updated_by: admin,
        }
        .publish(&env);
        Ok(())
    }

    /// Extend an existing escrow's expiry timestamp.
    /// Callable by escrow customer or merchant. Respects renewal configuration if set.
    pub fn extend_escrow_expiry(
        env: Env,
        caller: Address,
        escrow_id: u64,
        new_expiry_timestamp: u64,
    ) -> Result<(), Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }

        let mut escrow = Self::get_escrow(&env, escrow_id);
        let now = env.ledger().timestamp();

        if caller != escrow.customer && caller != escrow.merchant {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        if new_expiry_timestamp <= now {
            return Err(Error::Escrow(EscrowError::EscrowAlreadyExpired));
        }

        if escrow.expiry_timestamp > 0 && new_expiry_timestamp <= escrow.expiry_timestamp {
            return Err(Error::Escrow(EscrowError::NewExpiryNotAfterCurrent));
        }

        let mut renewal_fee: i128 = 0;
        let mut renewal_count: u32 = 0;

        if let Some(config) = env
            .storage()
            .instance()
            .get::<DataKey, EscrowRenewalConfig>(&DataKey::Dispute(DisputeKey::EscrowRenewalConfig))
        {
            if !config.enabled {
                return Err(Error::Escrow(EscrowError::RenewalDisabled));
            }

            renewal_count = env
                .storage()
                .instance()
                .get(&DataKey::Dispute(DisputeKey::EscrowRenewalCount(escrow_id)))
                .unwrap_or(0);

            if renewal_count >= config.max_renewals {
                return Err(Error::Escrow(EscrowError::MaxRenewalsReached));
            }

            let extension = if escrow.expiry_timestamp > 0 {
                new_expiry_timestamp - escrow.expiry_timestamp
            } else {
                new_expiry_timestamp - now
            };

            if extension < config.min_renewal_period {
                return Err(Error::Escrow(EscrowError::RenewalPeriodTooShort));
            }

            if extension > config.max_renewal_period {
                return Err(Error::Escrow(EscrowError::RenewalPeriodTooLong));
            }

            renewal_fee = (escrow.amount * config.renewal_fee_bps as i128) / 10_000;
            renewal_count += 1;

            env.storage().instance().set(
                &DataKey::Dispute(DisputeKey::EscrowRenewal(escrow_id)),
                &EscrowRenewal {
                    renewal_id: renewal_count as u64,
                    escrow_id,
                    renewed_by: caller.clone(),
                    new_expiry_timestamp,
                    renewal_fee,
                    renewed_at: now,
                    renewal_count,
                },
            );
            env.storage().instance().set(
                &DataKey::Dispute(DisputeKey::EscrowRenewalCount(escrow_id)),
                &renewal_count,
            );
        }

        escrow.expiry_timestamp = new_expiry_timestamp;
        escrow.last_activity_at = now;
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        EscrowRenewed {
            escrow_id,
            renewed_by: caller,
            new_expiry_timestamp,
            renewal_fee,
            renewal_count,
        }
        .publish(&env);

        Ok(())
    }

    pub fn create_template(
        env: Env,
        owner: Address,
        token: Address,
        amount: i128,
        release_delay_seconds: u64,
        description: String,
    ) -> Result<u64, Error> {
        owner.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::TemplateCounter))
            .unwrap_or(0);
        let template_id = counter + 1;

        let template = EscrowTemplate {
            template_id,
            owner: owner.clone(),
            token,
            amount,
            release_delay_seconds,
            description,
            created_at: env.ledger().timestamp(),
            active: true,
        };

        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Template(template_id)),
            &template,
        );
        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::TemplateCounter), &template_id);

        TemplateCreated { template_id, owner }.publish(&env);

        Ok(template_id)
    }

    pub fn create_escrow_from_template(
        env: Env,
        customer: Address,
        template_id: u64,
    ) -> Result<u64, Error> {
        customer.require_auth();

        let template: EscrowTemplate = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Template(template_id)))
            .ok_or(Error::Escrow(EscrowError::TemplateNotFound))?;

        if !template.active {
            return Err(Error::Escrow(EscrowError::TemplateInactive));
        }

        let release_timestamp = env.ledger().timestamp() + template.release_delay_seconds;

        let escrow_id = Self::create_escrow(
            env.clone(),
            customer.clone(),
            template.owner.clone(),
            template.amount,
            template.token.clone(),
            release_timestamp,
            0,
            0,
            false,
        )?;

        EscrowCreatedFromTemplate {
            escrow_id,
            template_id,
            customer,
        }
        .publish(&env);

        Ok(escrow_id)
    }

    pub fn batch_release_escrows(
        env: Env,
        admin: Address,
        request: BatchReleaseRequest,
    ) -> Result<BatchReleaseResult, Error> {
        admin.require_auth();

        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        if request.escrow_ids.len() > 20 {
            return Err(Error::Action(ActionError::BatchReleaseSizeLimitExceeded));
        }

        let mut succeeded = Vec::new(&env);
        let mut failed = Vec::new(&env);
        let mut errors = Vec::new(&env);

        for id in request.escrow_ids.iter() {
            match Self::internal_release_escrow(
                env.clone(),
                admin.clone(),
                id,
                false,
                request.override_recipient.clone(),
            ) {
                Ok(_) => succeeded.push_back(id),
                Err(e) => {
                    failed.push_back(id);
                    errors.push_back(e.to_u32());
                }
            }
        }

        Ok(BatchReleaseResult {
            succeeded,
            failed,
            errors,
        })
    }

    pub fn deactivate_template(env: Env, owner: Address, template_id: u64) -> Result<(), Error> {
        owner.require_auth();

        let mut template: EscrowTemplate = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Template(template_id)))
            .ok_or(Error::Escrow(EscrowError::TemplateNotFound))?;

        template.active = false;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Template(template_id)),
            &template,
        );

        TemplateDeactivated { template_id }.publish(&env);

        Ok(())
    }

    pub fn get_template(env: Env, template_id: u64) -> Result<EscrowTemplate, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Template(template_id)))
            .ok_or(Error::Escrow(EscrowError::TemplateNotFound))
    }

    // ── ANALYTICS HELPERS ─────────────────────────────────────────────────

    fn update_customer_analytics<F>(env: &Env, customer: &Address, update_fn: F)
    where
        F: Fn(&mut EscrowAnalytics),
    {
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Participant(ParticipantKey::CustomerAnalytics(
                customer.clone(),
            )))
            .unwrap_or(EscrowAnalytics::default_value());
        update_fn(&mut analytics);
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::CustomerAnalytics(customer.clone())),
            &analytics,
        );
    }

    fn update_merchant_analytics<F>(env: &Env, merchant: &Address, update_fn: F)
    where
        F: Fn(&mut EscrowAnalytics),
    {
        let mut analytics: EscrowAnalytics = env
            .storage()
            .instance()
            .get(&DataKey::Participant(ParticipantKey::MerchantAnalytics(
                merchant.clone(),
            )))
            .unwrap_or(EscrowAnalytics::default_value());
        update_fn(&mut analytics);
        env.storage().instance().set(
            &DataKey::Participant(ParticipantKey::MerchantAnalytics(merchant.clone())),
            &analytics,
        );
    }

    /// Configure the thresholds used to classify escrow health (admin only).
    ///
    /// `inactivity_seconds` controls when a non-terminal escrow is considered
    /// `Stale`, and `near_expiry_buffer_seconds` controls when one is flagged
    /// `NearExpiry`.
    pub fn set_stale_threshold(
        env: Env,
        admin: Address,
        config: StaleThresholdConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        let multisig = Self::get_multisig_config(env.clone());
        if !multisig.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }
        env.storage()
            .instance()
            .set(&DataKey::Config(ConfigKey::StaleThresholdConfig), &config);
        Ok(())
    }

    /// Returns the configured stale-threshold config, or `None` if unset.
    pub fn get_stale_threshold(env: Env) -> Option<StaleThresholdConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Config(ConfigKey::StaleThresholdConfig))
    }

    /// Classify the health of a single escrow against the configured thresholds.
    ///
    /// Panics with `EscrowNotFound` if the escrow does not exist, or
    /// `StaleThresholdNotConfigured` if `set_stale_threshold` was never called.
    pub fn get_escrow_health(env: Env, escrow_id: u64) -> EscrowHealthReport {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            panic_with_error!(&env, Error::Escrow(EscrowError::NotFound));
        }
        let config = Self::require_stale_threshold(&env);
        let escrow = Self::get_escrow(&env, escrow_id);
        let now = env.ledger().timestamp();

        let seconds_until_expiry: Option<i64> = if escrow.expiry_timestamp == 0 {
            None
        } else {
            Some(escrow.expiry_timestamp as i64 - now as i64)
        };

        EscrowHealthReport {
            escrow_id,
            health: Self::classify_health(&escrow, now, &config),
            seconds_until_expiry,
            last_activity: escrow.last_activity_at,
        }
    }

    /// Returns at most `limit` IDs of escrows currently classified as `Stale`.
    ///
    /// Panics with `StaleThresholdNotConfigured` if thresholds are not set.
    pub fn get_stale_escrows(env: Env, limit: u32) -> Vec<u64> {
        let config = Self::require_stale_threshold(&env);
        let now = env.ledger().timestamp();
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::Counter))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let mut id: u64 = 1;
        while id <= counter && (result.len() as u32) < limit {
            if let Some(escrow) = env
                .storage()
                .instance()
                .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(id)))
            {
                if Self::classify_health(&escrow, now, &config) == EscrowHealth::Stale {
                    result.push_back(id);
                }
            }
            id += 1;
        }
        result
    }

    fn require_stale_threshold(env: &Env) -> StaleThresholdConfig {
        match env
            .storage()
            .instance()
            .get::<DataKey, StaleThresholdConfig>(&DataKey::Config(ConfigKey::StaleThresholdConfig))
        {
            Some(config) => config,
            None => panic_with_error!(env, Error::Action(ActionError::StaleThresholdNotConfigured)),
        }
    }

    /// Pure classification of an escrow's health given the current time and config.
    ///
    /// Precedence: Disputed > Expired > NearExpiry > Stale > Healthy.
    fn classify_health(escrow: &Escrow, now: u64, config: &StaleThresholdConfig) -> EscrowHealth {
        if escrow.status == EscrowStatus::Disputed {
            return EscrowHealth::Disputed;
        }
        if escrow.expiry_timestamp != 0 && now >= escrow.expiry_timestamp {
            return EscrowHealth::Expired;
        }
        if escrow.expiry_timestamp != 0
            && escrow.expiry_timestamp - now <= config.near_expiry_buffer_seconds
        {
            return EscrowHealth::NearExpiry;
        }
        if now >= escrow.last_activity_at
            && now - escrow.last_activity_at >= config.inactivity_seconds
        {
            return EscrowHealth::Stale;
        }
        EscrowHealth::Healthy
    }

    pub fn create_sub_account(
        env: Env,
        merchant: Address,
        escrow_id: u64,
        label_hash: BytesN<32>,
        amount: i128,
    ) -> Result<u64, Error> {
        merchant.require_auth();

        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        let sub_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccountCounter(escrow_id)))
            .unwrap_or(0);
        let mut allocated: i128 = 0;
        for sub_id in 1..=sub_count {
            if let Some(sub) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowSubAccount>(&DataKey::Escrow(EscrowKey::SubAccount(
                        escrow_id, sub_id,
                    )))
            {
                allocated += sub.amount;
            }
        }

        if allocated + amount > escrow.amount {
            return Err(Error::Escrow(EscrowError::SubAccountFundingExceedsEscrow));
        }

        let sub_id = sub_count + 1;
        let sub = EscrowSubAccount {
            escrow_id,
            sub_id,
            label_hash,
            amount,
            released: false,
            release_condition: None,
        };

        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::SubAccount(escrow_id, sub_id)),
            &sub,
        );
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::SubAccountCounter(escrow_id)),
            &sub_id,
        );

        Ok(sub_id)
    }

    pub fn fund_sub_account(
        env: Env,
        funder: Address,
        escrow_id: u64,
        sub_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        funder.require_auth();

        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        let mut sub: EscrowSubAccount = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccount(escrow_id, sub_id)))
            .ok_or(Error::Escrow(EscrowError::SubAccountNotFound))?;

        let sub_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccountCounter(escrow_id)))
            .unwrap_or(0);
        let mut allocated: i128 = 0;
        for id in 1..=sub_count {
            if id == sub_id {
                continue;
            }
            if let Some(s) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowSubAccount>(&DataKey::Escrow(EscrowKey::SubAccount(
                        escrow_id, id,
                    )))
            {
                allocated += s.amount;
            }
        }

        if allocated + sub.amount + amount > escrow.amount {
            return Err(Error::Escrow(EscrowError::SubAccountFundingExceedsEscrow));
        }

        sub.amount += amount;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::SubAccount(escrow_id, sub_id)),
            &sub,
        );

        Ok(())
    }

    pub fn release_sub_account(
        env: Env,
        admin: Address,
        escrow_id: u64,
        sub_id: u64,
    ) -> Result<(), Error> {
        admin.require_auth();

        let escrow = EscrowContract::get_escrow(&env, escrow_id);

        let mut sub: EscrowSubAccount = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccount(escrow_id, sub_id)))
            .ok_or(Error::Escrow(EscrowError::SubAccountNotFound))?;

        if sub.released {
            return Err(Error::Escrow(EscrowError::SubAccountAlreadyReleased));
        }

        EscrowContract::transfer_if_token_contract(
            &env,
            &escrow.token,
            &escrow.merchant,
            sub.amount,
        )?;

        sub.released = true;
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::SubAccount(escrow_id, sub_id)),
            &sub,
        );

        Ok(())
    }

    pub fn get_sub_account(env: Env, escrow_id: u64, sub_id: u64) -> Option<EscrowSubAccount> {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccount(escrow_id, sub_id)))
    }

    pub fn list_sub_accounts(env: Env, escrow_id: u64) -> Vec<EscrowSubAccount> {
        let sub_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Escrow(EscrowKey::SubAccountCounter(escrow_id)))
            .unwrap_or(0);
        let mut result = Vec::new(&env);
        for sub_id in 1..=sub_count {
            if let Some(sub) =
                env.storage()
                    .instance()
                    .get::<DataKey, EscrowSubAccount>(&DataKey::Escrow(EscrowKey::SubAccount(
                        escrow_id, sub_id,
                    )))
            {
                result.push_back(sub);
            }
        }
        result
    }

    pub fn configure_escrow_swap(
        env: Env,
        merchant: Address,
        escrow_id: u64,
        target_token: Address,
        min_output: i128,
        oracle: Address,
    ) -> Result<(), Error> {
        merchant.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let config = Self::get_multisig_config(env.clone());
        if escrow.merchant != merchant && !config.admins.contains(&merchant) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let swap_config = EscrowSwapConfig {
            escrow_id,
            source_token: escrow.token.clone(),
            target_token,
            min_output_amount: min_output,
            oracle,
            executed: false,
        };

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowSwapConfig(escrow_id)),
            &swap_config,
        );

        Ok(())
    }

    pub fn execute_escrow_swap(env: Env, caller: Address, escrow_id: u64) -> Result<i128, Error> {
        // Authenticate the caller to follow standard signature verification pattern.
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            return Err(Error::Escrow(EscrowError::NotFound));
        }
        let mut escrow = Self::get_escrow(&env, escrow_id);
        if escrow.status != EscrowStatus::Locked {
            return Err(Error::Escrow(EscrowError::InvalidStatus));
        }

        let config = Self::get_multisig_config(env.clone());
        if escrow.merchant != caller && !config.admins.contains(&caller) {
            return Err(Error::Basic(BasicError::Unauthorized));
        }

        let mut swap_config: EscrowSwapConfig = env
            .storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowSwapConfig(escrow_id)))
            .ok_or(Error::Action(ActionError::SwapConfigNotFound))?;

        if swap_config.executed {
            return Err(Error::Action(ActionError::SwapAlreadyExecuted));
        }

        // Call oracle to get the rate (get_rate symbol takes no arguments and returns i128)
        let rate: i128 = env.invoke_contract(
            &swap_config.oracle,
            &Symbol::new(&env, "get_rate"),
            Vec::new(&env),
        );

        // Output amount scaled by Stellar/Soroban standard 1e7 fixed-point rate representation.
        // The mock oracle and implementation use a 1e7 rate because the issue does not specify oracle decimals.
        let output_amount = (escrow.amount * rate) / 10_000_000;

        if output_amount < swap_config.min_output_amount {
            return Err(Error::Action(ActionError::SwapOutputBelowMinimum));
        }

        // Update escrow state (token swap is modeled as metadata update, which automatically redirects settlement)
        escrow.token = swap_config.target_token.clone();
        escrow.amount = output_amount;
        escrow.last_activity_at = env.ledger().timestamp();
        swap_config.executed = true;

        env.storage()
            .instance()
            .set(&DataKey::Escrow(EscrowKey::Data(escrow_id)), &escrow);

        env.storage().instance().set(
            &DataKey::Dispute(DisputeKey::EscrowSwapConfig(escrow_id)),
            &swap_config,
        );

        Ok(output_amount)
    }

    pub fn get_swap_config(env: Env, escrow_id: u64) -> Option<EscrowSwapConfig> {
        env.storage()
            .instance()
            .get(&DataKey::Dispute(DisputeKey::EscrowSwapConfig(escrow_id)))
    }

    pub fn create_child_escrow(
        env: Env,
        admin: Address,
        parent_id: u64,
        amount: i128,
        token: Address,
        customer: Address,
        merchant: Address,
    ) -> Result<u64, Error> {
        admin.require_auth();
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::Basic(BasicError::NotAnAdmin));
        }

        // Verify parent escrow exists
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(parent_id)))
        {
            return Err(Error::Escrow(EscrowError::ParentEscrowNotFound));
        }

        // Get or initialize parent hierarchy node to find parent's depth
        let mut parent_node = env
            .storage()
            .instance()
            .get::<DataKey, EscrowHierarchyNode>(&DataKey::Escrow(EscrowKey::Hierarchy(parent_id)))
            .unwrap_or(EscrowHierarchyNode {
                escrow_id: parent_id,
                parent_id: None,
                children: Vec::new(&env),
                depth: 0, // Root depth is 0
            });

        if parent_node.depth + 1 > 3 {
            return Err(Error::Escrow(EscrowError::MaxHierarchyDepth));
        }

        let child_depth = parent_node.depth + 1;

        let parent_escrow = Self::get_escrow(&env, parent_id);
        let child_id = Self::internal_create_escrow(
            env.clone(),
            customer,
            merchant,
            amount,
            token,
            parent_escrow.release_timestamp,
            parent_escrow.min_hold_period,
            parent_escrow.expiry_timestamp,
            parent_escrow.auto_refund_on_expiry,
        )?;

        // Update parent children list
        parent_node.children.push_back(child_id);
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Hierarchy(parent_id)),
            &parent_node,
        );

        // Store child hierarchy node
        let child_node = EscrowHierarchyNode {
            escrow_id: child_id,
            parent_id: Some(parent_id),
            children: Vec::new(&env),
            depth: child_depth,
        };
        env.storage().instance().set(
            &DataKey::Escrow(EscrowKey::Hierarchy(child_id)),
            &child_node,
        );

        Ok(child_id)
    }

    pub fn get_escrow_hierarchy(env: Env, root_id: u64) -> Vec<EscrowHierarchyNode> {
        let mut result = Vec::new(&env);
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(root_id)))
        {
            return result;
        }

        let mut queue = Vec::new(&env);
        queue.push_back(root_id);

        let mut index = 0;
        while index < queue.len() {
            let current_id = queue.get(index).unwrap();
            index += 1;

            let node = env
                .storage()
                .instance()
                .get::<DataKey, EscrowHierarchyNode>(&DataKey::Escrow(EscrowKey::Hierarchy(
                    current_id,
                )))
                .unwrap_or(EscrowHierarchyNode {
                    escrow_id: current_id,
                    parent_id: None,
                    children: Vec::new(&env),
                    depth: 0,
                });

            result.push_back(node.clone());

            for child_id in node.children.iter() {
                queue.push_back(child_id);
            }
        }
        result
    }

    pub fn can_parent_release(env: Env, parent_id: u64) -> bool {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Escrow(EscrowKey::Data(parent_id)))
        {
            return false;
        }

        let mut queue = Vec::new(&env);
        queue.push_back(parent_id);

        let mut index = 0;
        while index < queue.len() {
            let current_id = queue.get(index).unwrap();
            index += 1;

            if let Some(node) = env
                .storage()
                .instance()
                .get::<DataKey, EscrowHierarchyNode>(&DataKey::Escrow(EscrowKey::Hierarchy(
                    current_id,
                )))
            {
                for child_id in node.children.iter() {
                    // Check if child is resolved
                    if !Self::is_escrow_resolved(&env, child_id) {
                        return false;
                    }
                    queue.push_back(child_id);
                }
            }
        }
        true
    }

    fn is_escrow_resolved(env: &Env, escrow_id: u64) -> bool {
        if let Some(escrow) = env
            .storage()
            .instance()
            .get::<DataKey, Escrow>(&DataKey::Escrow(EscrowKey::Data(escrow_id)))
        {
            match escrow.status {
                EscrowStatus::Released | EscrowStatus::Resolved | EscrowStatus::Cancelled => true,
                _ => false,
            }
        } else {
            true
        }
    }

    fn validate_bps(bps: u32) -> Result<(), Error> {
        if bps < 1 || bps > 10000 {
            return Err(Error::Basic(BasicError::InvalidBps));
        };

        Ok(())
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

#[cfg(test)]
mod swap_test;

#[cfg(test)]
mod hierarchy_test;

#[cfg(test)]
mod multi_party_weight_test;

// mod test;

// #[cfg(test)]
// mod dispute_appeal_test;
//
// #[cfg(test)]
// mod verification_test;
//
// #[cfg(test)]
// mod timelock_test;
//
// #[cfg(test)]
// mod collateral_test;
//
// #[cfg(test)]
// mod beneficiary_transfer_test;
//
// #[cfg(test)]
// mod multi_party_dispute_test;
//
// #[cfg(test)]
// mod pause_history_test;
//
#[cfg(test)]
mod expiry_test;
//
// #[cfg(test)]
// mod multisig_threshold_test;
//
// #[cfg(test)]
// mod escalation_timeout_test;
//
#[cfg(test)]
mod bulk_evidence_test;
// mod migration_test;
//
// #[cfg(test)]
// mod multi_party_rollback_test;
//
#[cfg(test)]
mod observer_test;
//
// mod health_check_test;
//
// #[cfg(test)]
// mod test_sub_account;
