#![no_std]
// NOTE (CHANGED BY automated refactor step 1):
// This file contains many contract types and a large `Error` enum used as
// the contract's public error type. Soroban's XDR encoding currently has a
// practical limit of 50 enum variants for contract error / contracttype
// derivations. To address this safely we are performing a multi-step
// refactor on the `datakey-and-error-enums` branch:
//  1) Add this file-level note and prepare the repository for a split of
//     oversized enums (this commit).
//  2) Split very large enums (Error, DataKey if required) into several
//     smaller #[contracttype]/#[contracterror]-compatible pieces and add
//     a compact wrapper that preserves the original numeric discriminants
//     and public API. Associated helper constructors/consts will preserve
//     existing call sites (so most Err(Error::X) usages remain unchanged).
//  3) Update pattern matches and trait impls where necessary and run the
//     test-suite. Keep changes small and in separate commits to make review
//     easier.
// If you prefer a different approach or want me to pause before step 2,
// tell me and I'll stop after this documentation commit.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, token,
    Address, Bytes, BytesN, Env, String, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum AdminKey {
    MultiSigConfig,
    AdminProposal(String),
    ProposalCounter,
    SuccessionPlan,
    ClawbackRequest(u64),
    ClawbackCounter,
    EscrowClawback(u64),
}

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
    EvidenceCommitment(u64),
    ReputationScore(Address),
    ReputationConfig,
    VestingSchedule(u64),
    VestingAccelerationConfig(u64),
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
    ActivePauseIndex(String),
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
    // Conditional escrow (on-chain state)
    ConditionalEscrow(u64),
    GlobalExpiryConfig,
    EscalationConfig,
    EscrowEvidencePage(u64, u32),
    EscrowEvidencePageCount(u64),
    // Tenure-weighted reputation
    TenureConfig,
    TenureBonusApplied(u64, Address),
}

/// Secondary storage keys (keeps `DataKey` within Soroban's 50-variant limit).
#[derive(Clone)]
#[contracttype]
pub enum EscrowAuxKey {
    MigrationStatusKey,
    EscrowMigrated(u64),
    EscrowTemplate(u64),
    EscrowTemplateCounter,
    StaleThresholdConfigKey,
    DisputeAppeal(u64),
    DisputeAppealCounter,
    DisputeRoundKey(u64),
    AppealsByEscrow(u64, u64),
    // Escrow renewal mechanism
    EscrowRenewalConfig,
    EscrowRenewal(u64),
    EscrowRenewalCount(u64),
    // Sub-account milestones
    SubAccount(u64, u64),
    SubAccountCounter(u64),
    // Escrow swap configuration
    EscrowSwapConfig(u64),
    // Escrow hierarchy configuration
    EscrowHierarchy(u64),
}

/// Observer storage keys (separate enum to stay within Soroban symbol limits).
#[derive(Clone)]
#[contracttype]
pub enum ObserverKey {
    Entry(u64, u64),
    Count(u64),
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

[TRUNCATED FOR BREVITY]
