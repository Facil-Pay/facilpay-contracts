#![no_std]
use escrow::EscrowContractClient;
use payments::PaymentContractClient;
use refund::RefundContractClient;
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    BytesN, Env, String, Vec,
};

/// Schema version of the coordinator itself, reported by `get_platform_versions` (#817).
pub const ADMIN_SCHEMA_VERSION: u32 = 1;

/// How long a multisig proposal stays approvable/executable after creation (#816).
pub const PROPOSAL_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;

/// Upper bound on the size of the admin set, to bound approval-counting cost.
pub const MAX_ADMINS: u32 = 20;

/// Maximum number of entries `get_incident_log` returns per page (#818).
pub const MAX_INCIDENT_PAGE: u32 = 50;

/// Maximum number of proposals a listing page returns (#873).
pub const MAX_PROPOSAL_PAGE: u32 = 50;

/// Default delay, in seconds, before a queued child upgrade may run (#874).
pub const DEFAULT_CHILD_UPGRADE_DELAY: u64 = 24 * 60 * 60;

/// Default number of vetoes that cancel a queued child upgrade (#874).
pub const DEFAULT_CHILD_UPGRADE_VETOES: u32 = 2;

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidFunctionName = 4,
    // #816: multisig control of the coordinator admin
    InvalidAdminSet = 5,
    InvalidThreshold = 6,
    ProposalNotFound = 7,
    ProposalExpired = 8,
    ProposalAlreadyExecuted = 9,
    AlreadyApproved = 10,
    ThresholdNotMet = 11,
    MultisigRequired = 12,
    // #820: bounded emergency pause
    NoPauseExpiry = 13,
    PauseNotExpired = 14,
    PauseExpired = 15,
    InvalidPauseDuration = 16,
    // #872: cancelling proposals
    ProposalCancelled = 17,
    CancellationNotAllowed = 18,
    // #871: revoking an approval
    ApprovalNotFound = 19,
    // #873: bounded proposal pagination
    InvalidPagination = 20,
    // #874: veto window for child upgrades
    NoQueuedUpgrade = 21,
    UpgradeTooEarly = 22,
    UpgradeVetoed = 23,
    InvalidUpgradeConfig = 24,
    AlreadyVetoed = 25,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ContractAddresses {
    pub payment: Address,
    pub escrow: Address,
    pub refund: Address,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Pauser,
    PaymentContract,
    EscrowContract,
    RefundContract,
    // #816: admin set, signature threshold and proposals.
    Admins,
    Threshold,
    ProposalCount,
    Proposal(u64),
    // #820: timestamp at which the current emergency pause may be expired.
    EmergencyPauseExpiry,
    // #874: veto-window configuration and queued child upgrades.
    ChildUpgradeConfig,
    QueuedChildUpgrade(ChildKind),
    // #818: number of incident log entries and each entry by id (persistent).
    IncidentCount,
    Incident(u64),
    // #876: number of incident entries per structured reason code.
    IncidentReasonCount(u32),
}

/// Configuration for the child-upgrade veto window (#874).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildUpgradeConfig {
    /// Seconds that must elapse between queueing and execution.
    pub delay_seconds: u64,
    /// Number of distinct admin vetoes that cancel a queued upgrade.
    pub veto_threshold: u32,
}

/// A queued, not-yet-executed child upgrade (#874).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueuedChildUpgrade {
    pub child_kind: ChildKind,
    pub wasm_hash: BytesN<32>,
    pub queued_at: u64,
    pub execute_after: u64,
    pub vetoes: Vec<Address>,
    pub vetoed: bool,
}

/// A privileged coordinator action that goes through propose/approve/execute (#816).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminAction {
    SetPaymentContract(Address),
    SetEscrowContract(Address),
    SetRefundContract(Address),
    Upgrade(BytesN<32>),
    /// Replace the admin set and threshold.
    SetAdmins(Vec<Address>, u32),
    /// Upgrade the payment, escrow and refund contracts together (#819).
    UpgradeChildren(BytesN<32>, BytesN<32>, BytesN<32>),
}

/// A pending or executed multisig proposal (#816).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposal {
    pub id: u64,
    pub action: AdminAction,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed: bool,
    /// Set by `cancel_proposal` (#872); cancelled proposals stay readable.
    pub cancelled: bool,
    /// Optional hash of the off-chain rationale for this proposal (#875).
    pub description_hash: Option<BytesN<32>>,
    /// Set by `cancel_proposal` (#872); cancelled proposals stay readable.
    pub cancelled: bool,
}

/// Configuration for the child-upgrade veto window (#874).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildUpgradeConfig {
    /// Seconds that must elapse between queueing and execution.
    pub delay_seconds: u64,
    /// Number of distinct admin vetoes that cancel a queued upgrade.
    pub veto_threshold: u32,
}

/// A queued, not-yet-executed child upgrade (#874).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueuedChildUpgrade {
    pub child_kind: ChildKind,
    pub wasm_hash: BytesN<32>,
    pub queued_at: u64,
    pub execute_after: u64,
    pub vetoes: Vec<Address>,
    pub vetoed: bool,
}

/// Proposal shape used before #875, kept so existing stored proposals stay readable.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyAdminProposal {
    pub id: u64,
    pub action: AdminAction,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    pub executed: bool,
}

/// Configuration for the child-upgrade veto window (#874).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildUpgradeConfig {
    /// Seconds that must elapse between queueing and execution.
    pub delay_seconds: u64,
    /// Number of distinct admin vetoes that cancel a queued upgrade.
    pub veto_threshold: u32,
}

/// A queued, not-yet-executed child upgrade (#874).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueuedChildUpgrade {
    pub child_kind: ChildKind,
    pub wasm_hash: BytesN<32>,
    pub queued_at: u64,
    pub execute_after: u64,
    pub vetoes: Vec<Address>,
    pub vetoed: bool,
}

/// Schema versions across the platform (#817). A child is `None` when its
/// address is not set or it could not be queried.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformVersions {
    pub admin: u32,
    pub payment: Option<u32>,
    pub escrow: Option<u32>,
    pub refund: Option<u32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposalCreated {
    pub proposal_id: u64,
    pub proposer: Address,
    pub action: AdminAction,
    pub expires_at: u64,
    pub description_hash: Option<BytesN<32>>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposalApproved {
    pub proposal_id: u64,
    pub approver: Address,
    pub approvals: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminProposalExecuted {
    pub proposal_id: u64,
    pub executed_by: Address,
}

/// Emitted when an admin takes back an approval (#871).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRevoked {
    pub proposal_id: u64,
    pub admin: Address,
    pub approvals: u32,
}

/// Emitted when a pending proposal is cancelled (#872).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCancelled {
    pub proposal_id: u64,
    pub cancelled_by: Address,
}

/// Emitted when a child upgrade enters its veto window (#874).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildUpgradeQueued {
    pub child_kind: ChildKind,
    pub wasm_hash: BytesN<32>,
    pub execute_after: u64,
}

/// Emitted on every veto cast against a queued child upgrade (#874).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildUpgradeVetoed {
    pub child_kind: ChildKind,
    pub admin: Address,
    pub vetoes: u32,
    pub threshold: u32,
}

/// Emitted after a queued child upgrade is applied (#874).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildUpgradeExecuted {
    pub child_kind: ChildKind,
    pub wasm_hash: BytesN<32>,
    pub executed_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminSetUpdated {
    pub admins: Vec<Address>,
    pub threshold: u32,
}

/// Event emitted when the coordinator's WASM is replaced in place (#644).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminContractUpgraded {
    pub new_wasm_hash: BytesN<32>,
    pub upgraded_by: Address,
}

/// Identifies which child contract a targeted admin action applies to.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChildKind {
    Payment,
    Escrow,
    Refund,
}

/// Emitted when a child contract address is updated (#651).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildContractUpdated {
    pub child_kind: ChildKind,
    pub new_address: Address,
    pub updated_by: Address,
    pub updated_at: u64,
}

/// Emitted once per child contract replaced by `upgrade_children` (#819).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildContractUpgraded {
    pub child_kind: ChildKind,
    pub contract: Address,
    pub new_wasm_hash: BytesN<32>,
    pub upgraded_by: Address,
    pub upgraded_at: u64,
}

/// Emitted when the pauser pauses a single function on one child contract.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyFunctionPausedEvent {
    pub target: ChildKind,
    pub function_name: String,
    pub paused_by: Address,
    pub reason: String,
    pub paused_at: u64,
    pub reason_code: IncidentReason,
}

/// Emitted when the pauser unpauses a single function on one child contract.
#[contractevent(topics = ["emergency_fn_unpaused"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyFunctionUnpausedEvent {
    pub target: ChildKind,
    pub function_name: String,
    pub unpaused_by: Address,
    pub unpaused_at: u64,
}

/// Emitted when an admin pushes back the expiry of a bounded emergency pause (#820).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyPauseExtended {
    pub extended_by: Address,
    pub previous_expires_at: u64,
    pub new_expires_at: u64,
}

/// Emitted when a bounded emergency pause is lifted by `expire_emergency_pause`
/// rather than by the pauser (#820).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmergencyPauseAutoExpired {
    pub expires_at: u64,
    pub unpaused_at: u64,
}

/// Kind of emergency action recorded in the incident log (#818).
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncidentAction {
    PauseAll,
    UnpauseAll,
    ExtendPause,
    PauseAutoExpired,
    PauseFunction,
    UnpauseFunction,
}

/// Structured reason code for an emergency action, stored alongside the
/// free-form reason text (#876).
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncidentReason {
    SecurityExploit,
    OracleFailure,
    ChildUpgrade,
    Maintenance,
    Other,
}

/// Stable numeric code used to aggregate incident counts by reason (#876).
fn incident_reason_code(reason: &IncidentReason) -> u32 {
    match reason {
        IncidentReason::SecurityExploit => 0,
        IncidentReason::OracleFailure => 1,
        IncidentReason::ChildUpgrade => 2,
        IncidentReason::Maintenance => 3,
        IncidentReason::Other => 4,
    }
}

/// What an incident log entry acted on (#818).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncidentTarget {
    /// Every child contract (platform-wide pause / unpause / expiry).
    AllChildren,
    /// A single child contract.
    Child(ChildKind),
}

/// One entry in the on-chain incident log (#818).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncidentEntry {
    /// Sequential id, starting at 1.
    pub id: u64,
    pub action: IncidentAction,
    pub target: IncidentTarget,
    /// The child function acted on, for function-level actions.
    pub function: Option<String>,
    /// The reason supplied with the action; empty when the action takes none.
    pub reason: String,
    /// Who performed the action, or `None` for a permissionless auto-expiry.
    pub actor: Option<Address>,
    /// Ledger timestamp of the action.
    pub at: u64,
    /// Structured reason code (#876): 0 SecurityExploit, 1 OracleFailure,
    /// 2 ChildUpgrade, 3 Maintenance, 4 Other. `None` for pre-#876 entries.
    pub reason_code: Option<u32>,
}

/// Incident entry shape used before #876, kept for backwards-compatible reads.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyIncidentEntry {
    pub id: u64,
    pub action: IncidentAction,
    pub target: IncidentTarget,
    pub function: Option<String>,
    pub reason: String,
    pub actor: Option<Address>,
    pub at: u64,
}

/// Emitted whenever an entry is appended to the incident log (#818).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncidentLogged {
    pub incident_id: u64,
    pub action: IncidentAction,
    pub at: u64,
    pub reason_code: IncidentReason,
}

/// Appends an entry to the persistent incident log and emits `IncidentLogged`.
fn log_incident(
    env: &Env,
    action: IncidentAction,
    target: IncidentTarget,
    function: Option<String>,
    reason: String,
    actor: Option<Address>,
    reason_code: IncidentReason,
) {
    let id = incident_count(env) + 1;
    let at = env.ledger().timestamp();
    let entry = IncidentEntry {
        id,
        action,
        target,
        function,
        reason,
        actor,
        at,
        reason_code: Some(incident_reason_code(&reason_code)),
    };
    env.storage()
        .persistent()
        .set(&DataKey::Incident(id), &entry);
    env.storage().persistent().set(&DataKey::IncidentCount, &id);
    let code = incident_reason_code(&reason_code);
    let reason_count: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::IncidentReasonCount(code))
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&DataKey::IncidentReasonCount(code), &(reason_count + 1));
    IncidentLogged {
        incident_id: id,
        action,
        at,
        reason_code,
    }
    .publish(env);
}

fn incident_count(env: &Env) -> u64 {
    env.storage()
        .persistent()
        .get(&DataKey::IncidentCount)
        .unwrap_or(0)
}

/// Verifies `pauser` is authenticated and matches the stored pauser.
fn require_pauser(env: &Env, pauser: &Address) -> Result<(), Error> {
    pauser.require_auth();

    let stored_pauser: Address = env
        .storage()
        .instance()
        .get(&DataKey::Pauser)
        .ok_or(Error::NotInitialized)?;
    if *pauser != stored_pauser {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

/// Returns the stored address of the child contract identified by `target`.
fn child_contract(env: &Env, target: ChildKind) -> Result<Address, Error> {
    let key = match target {
        ChildKind::Payment => DataKey::PaymentContract,
        ChildKind::Escrow => DataKey::EscrowContract,
        ChildKind::Refund => DataKey::RefundContract,
    };
    env.storage()
        .instance()
        .get(&key)
        .ok_or(Error::NotInitialized)
}

/// Returns the stored (payment, escrow, refund) child contract addresses.
fn child_contracts(env: &Env) -> Result<(Address, Address, Address), Error> {
    Ok((
        child_contract(env, ChildKind::Payment)?,
        child_contract(env, ChildKind::Escrow)?,
        child_contract(env, ChildKind::Refund)?,
    ))
}

/// Calls `unpause_contract(pauser)` on every child contract.
fn unpause_children(env: &Env, pauser: &Address) -> Result<(), Error> {
    let (payment, escrow, refund) = child_contracts(env)?;
    PaymentContractClient::new(env, &payment).unpause_contract(pauser);
    EscrowContractClient::new(env, &escrow).unpause_contract(pauser);
    RefundContractClient::new(env, &refund).unpause_contract(pauser);
    Ok(())
}

fn load_pause_expiry(env: &Env) -> Result<u64, Error> {
    env.storage()
        .instance()
        .get(&DataKey::EmergencyPauseExpiry)
        .ok_or(Error::NoPauseExpiry)
}

/// Returns the admin set. Contracts initialized before #816 only stored a
/// single `Admin`, which is treated as a one-member set.
fn load_admins(env: &Env) -> Result<Vec<Address>, Error> {
    if let Some(admins) = env.storage().instance().get(&DataKey::Admins) {
        return Ok(admins);
    }
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    Ok(Vec::from_array(env, [admin]))
}

fn load_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::Threshold)
        .unwrap_or(1)
}

/// Verifies `admin` is authenticated and a member of the admin set.
fn require_admin_member(env: &Env, admin: &Address) -> Result<(), Error> {
    admin.require_auth();
    if !load_admins(env)?.contains(admin) {
        return Err(Error::Unauthorized);
    }
    Ok(())
}

/// Direct (single-signature) privileged calls are only allowed while the
/// threshold is 1; otherwise the action must go through a proposal.
fn require_single_admin(env: &Env, admin: &Address) -> Result<(), Error> {
    require_admin_member(env, admin)?;
    if load_threshold(env) > 1 {
        return Err(Error::MultisigRequired);
    }
    Ok(())
}

fn validate_admin_set(admins: &Vec<Address>, threshold: u32) -> Result<(), Error> {
    if admins.is_empty() || admins.len() > MAX_ADMINS {
        return Err(Error::InvalidAdminSet);
    }
    for i in 0..admins.len() {
        for j in (i + 1)..admins.len() {
            if admins.get_unchecked(i) == admins.get_unchecked(j) {
                return Err(Error::InvalidAdminSet);
            }
        }
    }
    if threshold == 0 || threshold > admins.len() {
        return Err(Error::InvalidThreshold);
    }
    Ok(())
}

fn store_admin_set(env: &Env, admins: Vec<Address>, threshold: u32) {
    // `Admin` keeps the first member so `get_admin` stays meaningful.
    env.storage()
        .instance()
        .set(&DataKey::Admin, &admins.get_unchecked(0));
    env.storage().instance().set(&DataKey::Admins, &admins);
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
    AdminSetUpdated { admins, threshold }.publish(env);
}

fn upgrade_wasm(env: &Env, new_wasm_hash: BytesN<32>, upgraded_by: Address) {
    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());
    (AdminContractUpgraded {
        new_wasm_hash,
        upgraded_by,
    })
    .publish(env);
}

fn apply_action(env: &Env, action: AdminAction, executed_by: Address) {
    match action {
        AdminAction::SetPaymentContract(addr) => env
            .storage()
            .instance()
            .set(&DataKey::PaymentContract, &addr),
        AdminAction::SetEscrowContract(addr) => env
            .storage()
            .instance()
            .set(&DataKey::EscrowContract, &addr),
        AdminAction::SetRefundContract(addr) => env
            .storage()
            .instance()
            .set(&DataKey::RefundContract, &addr),
        AdminAction::Upgrade(hash) => upgrade_wasm(env, hash, executed_by),
        AdminAction::SetAdmins(admins, threshold) => store_admin_set(env, admins, threshold),
        AdminAction::UpgradeChildren(payment_hash, escrow_hash, refund_hash) => {
            if let Err(e) =
                upgrade_children_wasm(env, payment_hash, escrow_hash, refund_hash, executed_by)
            {
                panic_with_error!(env, e);
            }
        }
    }
}

/// Pauses every child, replaces each child's WASM and unpauses them again,
/// all inside the current invocation (#819). The coordinator acts on the
/// children as itself, so its own address must be an admin on each child
/// (a single-signature admin on payment and escrow). Any child call that fails
/// panics, which reverts the whole invocation, including the pauses. A child
/// that was already globally paused before the upgrade is left paused.
fn upgrade_children_wasm(
    env: &Env,
    payment_hash: BytesN<32>,
    escrow_hash: BytesN<32>,
    refund_hash: BytesN<32>,
    upgraded_by: Address,
) -> Result<(), Error> {
    let (payment, escrow, refund) = child_contracts(env)?;
    let coordinator = env.current_contract_address();
    let reason = String::from_str(env, "coordinated upgrade");

    let payment_client = PaymentContractClient::new(env, &payment);
    let escrow_client = EscrowContractClient::new(env, &escrow);
    let refund_client = RefundContractClient::new(env, &refund);

    let payment_was_paused = payment_client.get_pause_state().globally_paused;
    let escrow_was_paused = escrow_client.get_pause_state().globally_paused;
    let refund_was_paused = refund_client.get_pause_state().globally_paused;

    if !payment_was_paused {
        payment_client.pause_contract(&coordinator, &reason);
    }
    if !escrow_was_paused {
        escrow_client.pause_contract(&coordinator, &reason);
    }
    if !refund_was_paused {
        refund_client.pause_contract(&coordinator, &reason);
    }

    let now = env.ledger().timestamp();
    payment_client.upgrade(&coordinator, &payment_hash);
    ChildContractUpgraded {
        child_kind: ChildKind::Payment,
        contract: payment,
        new_wasm_hash: payment_hash,
        upgraded_by: upgraded_by.clone(),
        upgraded_at: now,
    }
    .publish(env);
    escrow_client.upgrade(&coordinator, &escrow_hash);
    ChildContractUpgraded {
        child_kind: ChildKind::Escrow,
        contract: escrow,
        new_wasm_hash: escrow_hash,
        upgraded_by: upgraded_by.clone(),
        upgraded_at: now,
    }
    .publish(env);
    refund_client.upgrade(&coordinator, &refund_hash);
    ChildContractUpgraded {
        child_kind: ChildKind::Refund,
        contract: refund,
        new_wasm_hash: refund_hash,
        upgraded_by,
        upgraded_at: now,
    }
    .publish(env);

    if !payment_was_paused {
        payment_client.unpause_contract(&coordinator);
    }
    if !escrow_was_paused {
        escrow_client.unpause_contract(&coordinator);
    }
    if !refund_was_paused {
        refund_client.unpause_contract(&coordinator);
    }
    Ok(())
}

/// Pauses, upgrades and unpauses a single child contract (#874). A child that
/// was already paused is left paused, matching `upgrade_children_wasm`.
fn upgrade_single_child_wasm(
    env: &Env,
    target: ChildKind,
    wasm_hash: BytesN<32>,
    upgraded_by: Address,
) -> Result<(), Error> {
    let child = child_contract(env, target)?;
    let coordinator = env.current_contract_address();
    let reason = String::from_str(env, "veto-window upgrade");
    let now = env.ledger().timestamp();

    let was_paused = match target {
        ChildKind::Payment => {
            PaymentContractClient::new(env, &child)
                .get_pause_state()
                .globally_paused
        }
        ChildKind::Escrow => {
            EscrowContractClient::new(env, &child)
                .get_pause_state()
                .globally_paused
        }
        ChildKind::Refund => {
            RefundContractClient::new(env, &child)
                .get_pause_state()
                .globally_paused
        }
    };
    if !was_paused {
        match target {
            ChildKind::Payment => {
                PaymentContractClient::new(env, &child).pause_contract(&coordinator, &reason)
            }
            ChildKind::Escrow => {
                EscrowContractClient::new(env, &child).pause_contract(&coordinator, &reason)
            }
            ChildKind::Refund => {
                RefundContractClient::new(env, &child).pause_contract(&coordinator, &reason)
            }
        }
    }

    match target {
        ChildKind::Payment => {
            PaymentContractClient::new(env, &child).upgrade(&coordinator, &wasm_hash)
        }
        ChildKind::Escrow => {
            EscrowContractClient::new(env, &child).upgrade(&coordinator, &wasm_hash)
        }
        ChildKind::Refund => {
            RefundContractClient::new(env, &child).upgrade(&coordinator, &wasm_hash)
        }
    }
    ChildContractUpgraded {
        child_kind: target,
        contract: child.clone(),
        new_wasm_hash: wasm_hash.clone(),
        upgraded_by: upgraded_by.clone(),
        upgraded_at: now,
    }
    .publish(env);

    if !was_paused {
        match target {
            ChildKind::Payment => {
                PaymentContractClient::new(env, &child).unpause_contract(&coordinator)
            }
            ChildKind::Escrow => {
                EscrowContractClient::new(env, &child).unpause_contract(&coordinator)
            }
            ChildKind::Refund => {
                RefundContractClient::new(env, &child).unpause_contract(&coordinator)
            }
        }
    }
    Ok(())
}

fn load_child_upgrade_config(env: &Env) -> ChildUpgradeConfig {
    env.storage()
        .instance()
        .get(&DataKey::ChildUpgradeConfig)
        .unwrap_or(ChildUpgradeConfig {
            delay_seconds: DEFAULT_CHILD_UPGRADE_DELAY,
            veto_threshold: DEFAULT_CHILD_UPGRADE_VETOES,
        })
}

/// Validates and bounds a proposal listing page (#873).
fn checked_page(offset: u64, limit: u32) -> Result<(u64, u32), Error> {
    if limit == 0 {
        return Err(Error::InvalidPagination);
    }
    Ok((offset, limit.min(MAX_PROPOSAL_PAGE)))
}

/// Collects open proposals newest-first, optionally hiding ones `awaiting` has
/// already approved (#873).
fn collect_open_proposals(
    env: &Env,
    offset: u64,
    limit: u32,
    awaiting: Option<&Address>,
) -> Vec<AdminProposal> {
    let mut page = Vec::new(env);
    let total: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ProposalCount)
        .unwrap_or(0);
    let now = env.ledger().timestamp();
    let mut skipped = 0u64;
    let mut id = total;
    while id > 0 {
        if let Some(proposal) = env
            .storage()
            .persistent()
            .get::<_, AdminProposal>(&DataKey::Proposal(id))
        {
            let open = !proposal.executed && !proposal.cancelled && now <= proposal.expires_at;
            let not_approved = match awaiting {
                Some(admin) => !proposal.approvals.contains(admin),
                None => true,
            };
            if open && not_approved {
                if skipped < offset {
                    skipped += 1;
                } else if page.len() < limit {
                    page.push_back(proposal);
                }
            }
        }
        id -= 1;
    }
    page
}

fn load_proposal(env: &Env, proposal_id: u64) -> Result<AdminProposal, Error> {
    if let Some(proposal) = env
        .storage()
        .persistent()
        .get(&DataKey::Proposal(proposal_id))
    {
        return Ok(proposal);
    }
    // Proposals stored before #875 have no description hash.
    let legacy: LegacyAdminProposal = env
        .storage()
        .persistent()
        .get(&DataKey::Proposal(proposal_id))
        .ok_or(Error::ProposalNotFound)?;
    Ok(AdminProposal {
        id: legacy.id,
        action: legacy.action,
        proposer: legacy.proposer,
        approvals: legacy.approvals,
        created_at: legacy.created_at,
        expires_at: legacy.expires_at,
        executed: legacy.executed,
        cancelled: false,
        description_hash: None,
        cancelled: false,
    })
}

/// Rejects proposals that already ran or whose approval window has closed.
fn require_open(env: &Env, proposal: &AdminProposal) -> Result<(), Error> {
    if proposal.executed {
        return Err(Error::ProposalAlreadyExecuted);
    }
    if proposal.cancelled {
        return Err(Error::ProposalCancelled);
    }
    if env.ledger().timestamp() > proposal.expires_at {
        return Err(Error::ProposalExpired);
    }
    Ok(())
}

/// Returns the schema version of a child, or `None` if its address is unset
/// or the call fails.
fn child_version(env: &Env, target: ChildKind) -> Option<u32> {
    let child = child_contract(env, target).ok()?;
    match target {
        ChildKind::Payment => PaymentContractClient::new(env, &child)
            .try_get_schema_version()
            .ok()?
            .ok(),
        ChildKind::Escrow => EscrowContractClient::new(env, &child)
            .try_get_schema_version()
            .ok()?
            .ok(),
        ChildKind::Refund => RefundContractClient::new(env, &child)
            .try_get_schema_version()
            .ok()?
            .ok(),
    }
}

#[contract]
pub struct AdminContract;

#[contractimpl]
impl AdminContract {
    /// Replaces the coordinator's code with an already-uploaded WASM (#644).
    ///
    /// The contract address and all stored data (admin, pauser, child contract
    /// addresses) are kept; the new code runs from the next invocation.
    ///
    /// # Parameters
    /// - `admin`: the stored admin address, which must authorize the call.
    /// - `new_wasm_hash`: hash of the WASM previously uploaded to the network.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized,
    /// `Error::Unauthorized` if `admin` is not in the admin set, and
    /// `Error::MultisigRequired` if the threshold is above 1 (use
    /// `propose_action` with `AdminAction::Upgrade` instead).
    pub fn upgrade(env: Env, admin: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        require_single_admin(&env, &admin)?;
        upgrade_wasm(&env, new_wasm_hash, admin);
        Ok(())
    }

    /// Upgrades the payment, escrow and refund contracts in one call (#819).
    ///
    /// Pauses all three children, replaces each child's WASM, then unpauses
    /// them, so the platform is never left half-upgraded. If any step fails
    /// the whole call reverts and no child is upgraded or left paused. A child
    /// that was already globally paused (e.g. by an emergency pause) is not
    /// unpaused.
    ///
    /// The coordinator calls the children as itself, so its own contract
    /// address must be authorized on each child: an admin of the payment and
    /// escrow multisig configs (with `required_signatures == 1`, which their
    /// `upgrade` requires) and the refund contract's admin.
    ///
    /// # Parameters
    /// - `admin`: an admin-set member, who must authorize the call.
    /// - `payment_hash`: hash of the new payment WASM, already uploaded.
    /// - `escrow_hash`: hash of the new escrow WASM, already uploaded.
    /// - `refund_hash`: hash of the new refund WASM, already uploaded.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized,
    /// `Error::Unauthorized` if `admin` is not in the admin set, and
    /// `Error::MultisigRequired` if the threshold is above 1 (use
    /// `propose_action` with `AdminAction::UpgradeChildren` instead). A child
    /// rejecting the coordinator, or a WASM hash that is not uploaded, aborts
    /// the call with the child's error.
    ///
    /// # Events
    /// Emits one `ChildContractUpgraded` per child, plus the children's own
    /// pause, unpause and upgrade events.
    pub fn upgrade_children(
        env: Env,
        admin: Address,
        payment_hash: BytesN<32>,
        escrow_hash: BytesN<32>,
        refund_hash: BytesN<32>,
    ) -> Result<(), Error> {
        require_single_admin(&env, &admin)?;
        upgrade_children_wasm(&env, payment_hash, escrow_hash, refund_hash, admin)
    }

    /// Initializes the admin contract with the addresses of the payment, escrow,
    /// and refund contracts.
    ///
    /// # Parameters
    /// - `admin`: the address authorized to manage the contract.
    /// - `pauser`: the address authorized to pause/unpause the platform.
    /// - `payment_contract`: the deployed payment contract address.
    /// - `escrow_contract`: the deployed escrow contract address.
    /// - `refund_contract`: the deployed refund contract address.
    ///
    /// # Returns
    /// Returns `Ok(())` when initialization succeeds.
    ///
    /// # Errors
    /// Returns `Error::AlreadyInitialized` if the contract has already been set up.
    pub fn initialize(
        env: Env,
        admin: Address,
        pauser: Address,
        payment_contract: Address,
        escrow_contract: Address,
        refund_contract: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Admins, &Vec::from_array(&env, [admin.clone()]));
        env.storage().instance().set(&DataKey::Threshold, &1u32);
        env.storage().instance().set(&DataKey::Pauser, &pauser);
        env.storage()
            .instance()
            .set(&DataKey::PaymentContract, &payment_contract);
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .instance()
            .set(&DataKey::RefundContract, &refund_contract);

        Ok(())
    }

    /// Pauses the payment, escrow, and refund contracts in one Soroban call.
    ///
    /// # Parameters
    /// - `pauser`: the pauser address that must be authorized.
    /// - `reason`: a human-readable explanation for the emergency pause.
    /// - `max_pause_seconds`: if `Some(n)`, the pause may be lifted by anyone via
    ///   `expire_emergency_pause` once `n` seconds have passed (#820). `None`
    ///   pauses with no expiry and clears any expiry left by an earlier pause.
    ///
    /// # Returns
    /// Returns `Ok(())` when all child contracts are paused successfully.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if the provided pauser address does not match the
    /// stored pauser, and `Error::InvalidPauseDuration` if `max_pause_seconds` is
    /// `Some(0)` or overflows the ledger timestamp.
    pub fn emergency_pause_all(
        env: Env,
        pauser: Address,
        reason: String,
        max_pause_seconds: Option<u64>,
        reason_code: IncidentReason,
    ) -> Result<(), Error> {
        require_pauser(&env, &pauser)?;

        let expires_at = match max_pause_seconds {
            Some(0) => return Err(Error::InvalidPauseDuration),
            Some(seconds) => Some(
                env.ledger()
                    .timestamp()
                    .checked_add(seconds)
                    .ok_or(Error::InvalidPauseDuration)?,
            ),
            None => None,
        };

        let (payment_contract, escrow_contract, refund_contract) = child_contracts(&env)?;

        PaymentContractClient::new(&env, &payment_contract).pause_contract(&pauser, &reason);
        EscrowContractClient::new(&env, &escrow_contract).pause_contract(&pauser, &reason);
        RefundContractClient::new(&env, &refund_contract).pause_contract(&pauser, &reason);

        match expires_at {
            Some(expires_at) => env
                .storage()
                .instance()
                .set(&DataKey::EmergencyPauseExpiry, &expires_at),
            None => env
                .storage()
                .instance()
                .remove(&DataKey::EmergencyPauseExpiry),
        }

        log_incident(
            &env,
            IncidentAction::PauseAll,
            IncidentTarget::AllChildren,
            None,
            reason,
            Some(pauser),
            reason_code,
        );

        Ok(())
    }

    /// Unpauses the payment, escrow, and refund contracts in one Soroban call.
    ///
    /// # Parameters
    /// - `pauser`: the pauser address that must be authorized.
    ///
    /// # Returns
    /// Returns `Ok(())` when all child contracts are unpaused successfully.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// and `Error::Unauthorized` if the provided pauser address does not match the
    /// stored pauser.
    pub fn emergency_unpause_all(env: Env, pauser: Address) -> Result<(), Error> {
        require_pauser(&env, &pauser)?;
        unpause_children(&env, &pauser)?;
        env.storage()
            .instance()
            .remove(&DataKey::EmergencyPauseExpiry);
        log_incident(
            &env,
            IncidentAction::UnpauseAll,
            IncidentTarget::AllChildren,
            None,
            String::from_str(&env, ""),
            Some(pauser),
            IncidentReason::Other,
        );
        Ok(())
    }

    /// Pushes back the expiry of a bounded emergency pause (#820).
    ///
    /// # Parameters
    /// - `admin`: an admin-set member, who must authorize the call.
    /// - `additional_seconds`: how far to move the expiry forward; must be non-zero.
    ///
    /// # Returns
    /// The new expiry timestamp.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if `admin` is not in the admin set,
    /// `Error::NoPauseExpiry` if there is no bounded pause in effect,
    /// `Error::PauseExpired` if the expiry has already been reached, and
    /// `Error::InvalidPauseDuration` if `additional_seconds` is 0 or overflows.
    ///
    /// # Events
    /// Emits `EmergencyPauseExtended`.
    pub fn extend_emergency_pause(
        env: Env,
        admin: Address,
        additional_seconds: u64,
    ) -> Result<u64, Error> {
        require_admin_member(&env, &admin)?;
        let expires_at = load_pause_expiry(&env)?;
        if env.ledger().timestamp() >= expires_at {
            return Err(Error::PauseExpired);
        }
        if additional_seconds == 0 {
            return Err(Error::InvalidPauseDuration);
        }
        let new_expires_at = expires_at
            .checked_add(additional_seconds)
            .ok_or(Error::InvalidPauseDuration)?;

        env.storage()
            .instance()
            .set(&DataKey::EmergencyPauseExpiry, &new_expires_at);

        EmergencyPauseExtended {
            extended_by: admin.clone(),
            previous_expires_at: expires_at,
            new_expires_at,
        }
        .publish(&env);
        log_incident(
            &env,
            IncidentAction::ExtendPause,
            IncidentTarget::AllChildren,
            None,
            String::from_str(&env, ""),
            Some(admin),
            IncidentReason::Other,
        );

        Ok(new_expires_at)
    }

    /// Lifts a bounded emergency pause once its expiry has been reached (#820).
    ///
    /// Anyone may call this: the coordinator does not check the caller. It
    /// calls `unpause_contract(pauser)` on each child with the stored pauser,
    /// so each child still verifies the pauser's authorization for that call.
    ///
    /// # Errors
    /// Returns `Error::NoPauseExpiry` if there is no bounded pause in effect,
    /// `Error::PauseNotExpired` if the expiry has not been reached yet, and
    /// `Error::NotInitialized` if the pauser or a child address is missing.
    ///
    /// # Events
    /// Emits `EmergencyPauseAutoExpired`.
    pub fn expire_emergency_pause(env: Env) -> Result<(), Error> {
        let expires_at = load_pause_expiry(&env)?;
        let now = env.ledger().timestamp();
        if now < expires_at {
            return Err(Error::PauseNotExpired);
        }

        let pauser: Address = env
            .storage()
            .instance()
            .get(&DataKey::Pauser)
            .ok_or(Error::NotInitialized)?;
        unpause_children(&env, &pauser)?;
        env.storage()
            .instance()
            .remove(&DataKey::EmergencyPauseExpiry);

        EmergencyPauseAutoExpired {
            expires_at,
            unpaused_at: now,
        }
        .publish(&env);
        log_incident(
            &env,
            IncidentAction::PauseAutoExpired,
            IncidentTarget::AllChildren,
            None,
            String::from_str(&env, ""),
            None,
            IncidentReason::Other,
        );

        Ok(())
    }

    /// Returns when the current emergency pause may be expired, or `None` if
    /// no bounded pause is in effect (#820).
    pub fn get_emergency_pause_expiry(env: Env) -> Option<u64> {
        env.storage().instance().get(&DataKey::EmergencyPauseExpiry)
    }

    /// Returns a page of the incident log, oldest entry first (#818).
    ///
    /// # Parameters
    /// - `offset`: number of entries to skip from the start of the log.
    /// - `limit`: maximum number of entries to return, capped at `MAX_INCIDENT_PAGE`.
    ///
    /// # Returns
    /// Up to `limit` entries; empty when `offset` is past the end or `limit` is 0.
    pub fn get_incident_log(env: Env, offset: u64, limit: u32) -> Vec<IncidentEntry> {
        let mut page = Vec::new(&env);
        let total = incident_count(&env);
        let limit = u64::from(limit.min(MAX_INCIDENT_PAGE));
        let end = offset.saturating_add(limit).min(total);
        let mut id = offset.saturating_add(1);
        while id <= end {
            if let Some(entry) = env.storage().persistent().get(&DataKey::Incident(id)) {
                page.push_back(entry);
            } else if let Some(legacy) = env.storage().persistent().get(&DataKey::Incident(id)).map(
                |entry: LegacyIncidentEntry| IncidentEntry {
                    id: entry.id,
                    action: entry.action,
                    target: entry.target,
                    function: entry.function,
                    reason: entry.reason,
                    actor: entry.actor,
                    at: entry.at,
                    reason_code: None,
                },
            ) {
                page.push_back(legacy);
            }
            id += 1;
        }
        page
    }

    /// Returns the total number of entries in the incident log (#818).
    pub fn get_incident_count(env: Env) -> u64 {
        incident_count(&env)
    }

    /// Returns the number of incident log entries with the given reason code (#876).
    pub fn get_incident_count_by_reason(env: Env, reason: IncidentReason) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::IncidentReasonCount(incident_reason_code(&reason)))
            .unwrap_or(0)
    }

    /// Pauses a single function on one child contract, leaving every other
    /// function and every other child contract untouched.
    ///
    /// # Parameters
    /// - `pauser`: the pauser address that must be authorized.
    /// - `target`: which child contract (payment, escrow or refund) to act on.
    /// - `function_name`: the child function to pause (e.g. `"process_refund"`).
    /// - `reason`: a human-readable explanation for the pause.
    ///
    /// # Returns
    /// Returns `Ok(())` when the targeted child contract has paused the function.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if the provided pauser address does not match the stored
    /// pauser, and `Error::InvalidFunctionName` if `function_name` is empty.
    ///
    /// # Events
    /// Emits `EmergencyFunctionPausedEvent`.
    pub fn emergency_pause_function(
        env: Env,
        pauser: Address,
        target: ChildKind,
        function_name: String,
        reason: String,
        reason_code: IncidentReason,
    ) -> Result<(), Error> {
        require_pauser(&env, &pauser)?;
        if function_name.is_empty() {
            return Err(Error::InvalidFunctionName);
        }

        let child = child_contract(&env, target)?;
        match target {
            ChildKind::Payment => PaymentContractClient::new(&env, &child).pause_function(
                &pauser,
                &function_name,
                &reason,
            ),
            ChildKind::Escrow => EscrowContractClient::new(&env, &child).pause_function(
                &pauser,
                &function_name,
                &reason,
            ),
            ChildKind::Refund => RefundContractClient::new(&env, &child).pause_function(
                &pauser,
                &function_name,
                &reason,
            ),
        }

        EmergencyFunctionPausedEvent {
            target,
            function_name: function_name.clone(),
            paused_by: pauser.clone(),
            reason: reason.clone(),
            paused_at: env.ledger().timestamp(),
            reason_code,
        }
        .publish(&env);
        log_incident(
            &env,
            IncidentAction::PauseFunction,
            IncidentTarget::Child(target),
            Some(function_name),
            reason,
            Some(pauser),
            reason_code,
        );

        Ok(())
    }

    /// Unpauses a single function on one child contract.
    ///
    /// # Parameters
    /// - `pauser`: the pauser address that must be authorized.
    /// - `target`: which child contract (payment, escrow or refund) to act on.
    /// - `function_name`: the child function to unpause.
    ///
    /// # Returns
    /// Returns `Ok(())` when the targeted child contract has unpaused the function.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if the provided pauser address does not match the stored
    /// pauser, and `Error::InvalidFunctionName` if `function_name` is empty.
    ///
    /// # Events
    /// Emits `EmergencyFunctionUnpausedEvent`.
    pub fn emergency_unpause_function(
        env: Env,
        pauser: Address,
        target: ChildKind,
        function_name: String,
    ) -> Result<(), Error> {
        require_pauser(&env, &pauser)?;
        if function_name.is_empty() {
            return Err(Error::InvalidFunctionName);
        }

        let child = child_contract(&env, target)?;
        match target {
            ChildKind::Payment => {
                PaymentContractClient::new(&env, &child).unpause_function(&pauser, &function_name)
            }
            ChildKind::Escrow => {
                EscrowContractClient::new(&env, &child).unpause_function(&pauser, &function_name)
            }
            ChildKind::Refund => {
                RefundContractClient::new(&env, &child).unpause_function(&pauser, &function_name)
            }
        }

        EmergencyFunctionUnpausedEvent {
            target,
            function_name: function_name.clone(),
            unpaused_by: pauser.clone(),
            unpaused_at: env.ledger().timestamp(),
        }
        .publish(&env);
        log_incident(
            &env,
            IncidentAction::UnpauseFunction,
            IncidentTarget::Child(target),
            Some(function_name),
            String::from_str(&env, ""),
            Some(pauser),
            IncidentReason::Other,
        );

        Ok(())
    }

    /// Updates the stored payment contract address.
    ///
    /// # Parameters
    /// - `admin`: the admin address that must be authorized.
    /// - `payment_contract`: the new payment contract address.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if the provided admin address is not in the admin set,
    /// and `Error::MultisigRequired` if the threshold is above 1.
    pub fn set_payment_contract(
        env: Env,
        admin: Address,
        payment_contract: Address,
    ) -> Result<(), Error> {
        require_single_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&DataKey::PaymentContract, &payment_contract.clone());

        ChildContractUpdated {
            child_kind: ChildKind::Payment,
            new_address: payment_contract,
            updated_by: admin,
            updated_at: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    /// Updates the stored escrow contract address.
    ///
    /// # Parameters
    /// - `admin`: the admin address that must be authorized.
    /// - `escrow_contract`: the new escrow contract address.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if the provided admin address is not in the admin set,
    /// and `Error::MultisigRequired` if the threshold is above 1.
    pub fn set_escrow_contract(
        env: Env,
        admin: Address,
        escrow_contract: Address,
    ) -> Result<(), Error> {
        require_single_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract.clone());

        ChildContractUpdated {
            child_kind: ChildKind::Escrow,
            new_address: escrow_contract,
            updated_by: admin,
            updated_at: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    /// Updates the stored refund contract address.
    ///
    /// # Parameters
    /// - `admin`: the admin address that must be authorized.
    /// - `refund_contract`: the new refund contract address.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// `Error::Unauthorized` if the provided admin address is not in the admin set,
    /// and `Error::MultisigRequired` if the threshold is above 1.
    pub fn set_refund_contract(
        env: Env,
        admin: Address,
        refund_contract: Address,
    ) -> Result<(), Error> {
        require_single_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&DataKey::RefundContract, &refund_contract.clone());

        ChildContractUpdated {
            child_kind: ChildKind::Refund,
            new_address: refund_contract,
            updated_by: admin,
            updated_at: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    /// Replaces the admin set and signature threshold directly (#816).
    ///
    /// Only allowed while the current threshold is 1, so a single admin can
    /// bootstrap a multisig; afterwards use `AdminAction::SetAdmins`.
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `MultisigRequired`, `InvalidAdminSet`
    /// (empty, duplicated, or more than `MAX_ADMINS` members) and
    /// `InvalidThreshold` (zero or above the number of admins).
    ///
    /// # Events
    /// Emits `AdminSetUpdated`.
    pub fn set_admins(
        env: Env,
        admin: Address,
        admins: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        require_single_admin(&env, &admin)?;
        validate_admin_set(&admins, threshold)?;
        store_admin_set(&env, admins, threshold);
        Ok(())
    }

    /// Proposes a privileged action (#816). The proposer's approval is
    /// recorded immediately; the proposal expires after `PROPOSAL_TTL_SECONDS`.
    ///
    /// # Returns
    /// The new proposal id.
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized` (proposer not in the admin set), and for
    /// `SetAdmins`, `InvalidAdminSet` / `InvalidThreshold`.
    ///
    /// # Events
    /// Emits `AdminProposalCreated`.
    pub fn propose_action(
        env: Env,
        proposer: Address,
        action: AdminAction,
        description_hash: Option<BytesN<32>>,
    ) -> Result<u64, Error> {
        require_admin_member(&env, &proposer)?;
        if let AdminAction::SetAdmins(admins, threshold) = &action {
            validate_admin_set(admins, *threshold)?;
        }

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0u64)
            + 1;
        let created_at = env.ledger().timestamp();
        let expires_at = created_at.saturating_add(PROPOSAL_TTL_SECONDS);
        let proposal = AdminProposal {
            id,
            action: action.clone(),
            proposer: proposer.clone(),
            approvals: Vec::from_array(&env, [proposer.clone()]),
            created_at,
            expires_at,
            executed: false,
            cancelled: false,
            description_hash: description_hash.clone(),
            cancelled: false,
        };
        env.storage().instance().set(&DataKey::ProposalCount, &id);
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(id), &proposal);

        AdminProposalCreated {
            proposal_id: id,
            proposer,
            action,
            expires_at,
            description_hash: description_hash.clone(),
        }
        .publish(&env);

        Ok(id)
    }

    /// Adds `approver`'s approval to an open proposal (#816).
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `ProposalNotFound`,
    /// `ProposalAlreadyExecuted`, `ProposalExpired` and `AlreadyApproved`.
    ///
    /// # Events
    /// Emits `AdminProposalApproved`.
    pub fn approve_action(env: Env, approver: Address, proposal_id: u64) -> Result<(), Error> {
        require_admin_member(&env, &approver)?;
        let mut proposal = load_proposal(&env, proposal_id)?;
        require_open(&env, &proposal)?;
        if proposal.approvals.contains(&approver) {
            return Err(Error::AlreadyApproved);
        }

        proposal.approvals.push_back(approver.clone());
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        AdminProposalApproved {
            proposal_id,
            approver,
            approvals: proposal.approvals.len(),
        }
        .publish(&env);

        Ok(())
    }

    /// Executes a proposal once approvals from current admins reach the
    /// threshold (#816). Approvals from addresses removed from the admin set
    /// since they approved do not count.
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `ProposalNotFound`,
    /// `ProposalAlreadyExecuted`, `ProposalExpired` and `ThresholdNotMet`.
    ///
    /// # Events
    /// Emits `AdminProposalExecuted`, plus the action's own event
    /// (`AdminContractUpgraded` or `AdminSetUpdated`) where it has one.
    pub fn execute_action(env: Env, executor: Address, proposal_id: u64) -> Result<(), Error> {
        require_admin_member(&env, &executor)?;
        let mut proposal = load_proposal(&env, proposal_id)?;
        require_open(&env, &proposal)?;

        let admins = load_admins(&env)?;
        let valid_approvals = proposal
            .approvals
            .iter()
            .filter(|a| admins.contains(a))
            .count() as u32;
        if valid_approvals < load_threshold(&env) {
            return Err(Error::ThresholdNotMet);
        }

        proposal.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        apply_action(&env, proposal.action, executor.clone());

        AdminProposalExecuted {
            proposal_id,
            executed_by: executor,
        }
        .publish(&env);

        Ok(())
    }

    /// Returns a proposal by id (#816).
    ///
    /// # Errors
    /// `ProposalNotFound` if no proposal has that id.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<AdminProposal, Error> {
        load_proposal(&env, proposal_id)
    }

    /// Removes the caller's approval from an open proposal (#871).
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `ProposalNotFound`, `ProposalCancelled`,
    /// `ProposalAlreadyExecuted`, `ProposalExpired`, `ApprovalNotFound`.
    ///
    /// # Events
    /// Emits `ApprovalRevoked` with the remaining approval count.
    pub fn revoke_approval(env: Env, admin: Address, proposal_id: u64) -> Result<(), Error> {
        require_admin_member(&env, &admin)?;
        let mut proposal = load_proposal(&env, proposal_id)?;
        require_open(&env, &proposal)?;
        let index = proposal
            .approvals
            .iter()
            .position(|a| a == admin)
            .ok_or(Error::ApprovalNotFound)?;
        proposal.approvals.remove(index as u32);
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        ApprovalRevoked {
            proposal_id,
            admin: admin.clone(),
            approvals: proposal.approvals.len(),
        }
        .publish(&env);
        Ok(())
    }

    /// Cancels an open proposal (#872).
    ///
    /// The proposer may cancel directly. Any other current admin may cancel
    /// once approvals from current admins reach the threshold.
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `ProposalNotFound`, `ProposalCancelled`,
    /// `ProposalAlreadyExecuted`, `ProposalExpired`, `CancellationNotAllowed`.
    ///
    /// # Events
    /// Emits `ProposalCancelled`.
    pub fn cancel_proposal(env: Env, admin: Address, proposal_id: u64) -> Result<(), Error> {
        require_admin_member(&env, &admin)?;
        let mut proposal = load_proposal(&env, proposal_id)?;
        require_open(&env, &proposal)?;

        let mut allowed = proposal.proposer == admin;
        if !allowed {
            let admins = load_admins(&env)?;
            let valid = proposal
                .approvals
                .iter()
                .filter(|a| admins.contains(a))
                .count() as u32;
            allowed = valid >= load_threshold(&env);
        }
        if !allowed {
            return Err(Error::CancellationNotAllowed);
        }

        proposal.cancelled = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        ProposalCancelled {
            proposal_id,
            cancelled_by: admin,
        }
        .publish(&env);
        Ok(())
    }

    /// Returns open proposals, newest first (#873).
    ///
    /// `limit` is capped at `MAX_PROPOSAL_PAGE` (50); `offset` skips that many
    /// open proposals. Expired, executed and cancelled proposals are excluded.
    pub fn get_pending_proposals(
        env: Env,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<AdminProposal>, Error> {
        load_admins(&env)?;
        let (offset, limit) = checked_page(offset, limit)?;
        Ok(collect_open_proposals(&env, offset, limit, None))
    }

    /// Returns open proposals the given admin has not approved, newest first (#873).
    ///
    /// `limit` is capped at `MAX_PROPOSAL_PAGE` (50). Expired, executed and
    /// cancelled proposals are excluded.
    pub fn get_proposals_awaiting(
        env: Env,
        admin: Address,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<AdminProposal>, Error> {
        load_admins(&env)?;
        let (offset, limit) = checked_page(offset, limit)?;
        Ok(collect_open_proposals(&env, offset, limit, Some(&admin)))
    }

    /// Configures the child-upgrade veto window (#874).
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `InvalidUpgradeConfig` when
    /// `veto_threshold` is 0 or above `MAX_ADMINS`.
    pub fn configure_child_upgrade(
        env: Env,
        admin: Address,
        delay_seconds: u64,
        veto_threshold: u32,
    ) -> Result<(), Error> {
        require_admin_member(&env, &admin)?;
        if veto_threshold == 0 || veto_threshold > MAX_ADMINS {
            return Err(Error::InvalidUpgradeConfig);
        }
        env.storage().instance().set(
            &DataKey::ChildUpgradeConfig,
            &ChildUpgradeConfig {
                delay_seconds,
                veto_threshold,
            },
        );
        Ok(())
    }

    /// Returns the current child-upgrade veto-window configuration (#874).
    pub fn get_child_upgrade_config(env: Env) -> ChildUpgradeConfig {
        load_child_upgrade_config(&env)
    }

    /// Queues a single child WASM upgrade behind a veto window (#874).
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `InvalidUpgradeConfig` on overflow.
    ///
    /// # Events
    /// Emits `ChildUpgradeQueued` with the target and WASM hash.
    pub fn queue_child_upgrade(
        env: Env,
        admin: Address,
        child_kind: ChildKind,
        wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        require_admin_member(&env, &admin)?;
        child_contract(&env, child_kind)?;
        let config = load_child_upgrade_config(&env);
        let queued_at = env.ledger().timestamp();
        let execute_after = queued_at
            .checked_add(config.delay_seconds)
            .ok_or(Error::InvalidUpgradeConfig)?;
        let queued = QueuedChildUpgrade {
            child_kind,
            wasm_hash: wasm_hash.clone(),
            queued_at,
            execute_after,
            vetoes: Vec::new(&env),
            vetoed: false,
        };
        env.storage()
            .instance()
            .set(&DataKey::QueuedChildUpgrade(child_kind), &queued);
        ChildUpgradeQueued {
            child_kind,
            wasm_hash,
            execute_after,
        }
        .publish(&env);
        Ok(())
    }

    /// Returns the queued upgrade for a child, if any (#874).
    pub fn get_queued_child_upgrade(env: Env, child_kind: ChildKind) -> Option<QueuedChildUpgrade> {
        env.storage()
            .instance()
            .get(&DataKey::QueuedChildUpgrade(child_kind))
    }

    /// Casts a veto against a queued child upgrade (#874).
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `NoQueuedUpgrade`, `UpgradeVetoed`,
    /// `AlreadyVetoed`.
    ///
    /// # Events
    /// Emits `ChildUpgradeVetoed` with the current veto count and threshold.
    pub fn veto_child_upgrade(
        env: Env,
        admin: Address,
        child_kind: ChildKind,
    ) -> Result<(), Error> {
        require_admin_member(&env, &admin)?;
        let key = DataKey::QueuedChildUpgrade(child_kind);
        let mut queued: QueuedChildUpgrade = env
            .storage()
            .instance()
            .get(&key)
            .ok_or(Error::NoQueuedUpgrade)?;
        if queued.vetoed {
            return Err(Error::UpgradeVetoed);
        }
        if queued.vetoes.contains(&admin) {
            return Err(Error::AlreadyVetoed);
        }
        queued.vetoes.push_back(admin.clone());
        let threshold = load_child_upgrade_config(&env).veto_threshold;
        if queued.vetoes.len() >= threshold {
            queued.vetoed = true;
        }
        env.storage().instance().set(&key, &queued);
        ChildUpgradeVetoed {
            child_kind,
            admin,
            vetoes: queued.vetoes.len(),
            threshold,
        }
        .publish(&env);
        Ok(())
    }

    /// Executes a queued child upgrade after its veto window (#874).
    ///
    /// # Errors
    /// `NotInitialized`, `Unauthorized`, `NoQueuedUpgrade`, `UpgradeVetoed`,
    /// `UpgradeTooEarly`; child errors abort the call.
    ///
    /// # Events
    /// Emits `ChildUpgradeExecuted` with the WASM hash, plus the child's
    /// `ChildContractUpgraded`, pause and unpause events.
    pub fn execute_child_upgrade(
        env: Env,
        admin: Address,
        child_kind: ChildKind,
    ) -> Result<(), Error> {
        require_admin_member(&env, &admin)?;
        let key = DataKey::QueuedChildUpgrade(child_kind);
        let queued: QueuedChildUpgrade = env
            .storage()
            .instance()
            .get(&key)
            .ok_or(Error::NoQueuedUpgrade)?;
        if queued.vetoed {
            return Err(Error::UpgradeVetoed);
        }
        if env.ledger().timestamp() < queued.execute_after {
            return Err(Error::UpgradeTooEarly);
        }
        upgrade_single_child_wasm(&env, child_kind, queued.wasm_hash.clone(), admin.clone())?;
        env.storage().instance().remove(&key);
        ChildUpgradeExecuted {
            child_kind,
            wasm_hash: queued.wasm_hash,
            executed_by: admin,
        }
        .publish(&env);
        Ok(())
    }

    /// Returns the admin set (#816).
    ///
    /// # Errors
    /// `NotInitialized` if the contract has not been initialized.
    pub fn get_admins(env: Env) -> Result<Vec<Address>, Error> {
        load_admins(&env)
    }

    /// Returns the signature threshold (#816).
    ///
    /// # Errors
    /// `NotInitialized` if the contract has not been initialized.
    pub fn get_threshold(env: Env) -> Result<u32, Error> {
        load_admins(&env)?;
        Ok(load_threshold(&env))
    }

    /// Returns the coordinator's own schema version (#817).
    pub fn get_schema_version(_env: Env) -> u32 {
        ADMIN_SCHEMA_VERSION
    }

    /// Returns the coordinator's version and each child's `get_schema_version` (#817).
    ///
    /// Never fails: a child whose address is not set, or whose call fails,
    /// is reported as `None`.
    pub fn get_platform_versions(env: Env) -> PlatformVersions {
        PlatformVersions {
            admin: ADMIN_SCHEMA_VERSION,
            payment: child_version(&env, ChildKind::Payment),
            escrow: child_version(&env, ChildKind::Escrow),
            refund: child_version(&env, ChildKind::Refund),
        }
    }

    /// Returns the current admin address (the first member of the admin set).
    ///
    /// # Returns
    /// The `Address` of the current admin.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    /// Returns the current pauser address.
    ///
    /// # Returns
    /// The `Address` of the current pauser.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized.
    pub fn get_pauser(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Pauser)
            .ok_or(Error::NotInitialized)
    }

    /// Returns all child contract addresses.
    ///
    /// # Returns
    /// A `ContractAddresses` struct containing the payment, escrow, and refund contract addresses.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized.
    pub fn get_contracts(env: Env) -> Result<ContractAddresses, Error> {
        let payment = env
            .storage()
            .instance()
            .get(&DataKey::PaymentContract)
            .ok_or(Error::NotInitialized)?;
        let escrow = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .ok_or(Error::NotInitialized)?;
        let refund = env
            .storage()
            .instance()
            .get(&DataKey::RefundContract)
            .ok_or(Error::NotInitialized)?;

        Ok(ContractAddresses {
            payment,
            escrow,
            refund,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup_payment(env: &Env, admin: &Address) -> Address {
        let contract_id = env.register(payments::PaymentContract, ());
        let client = PaymentContractClient::new(env, &contract_id);
        client.initialize(admin);
        contract_id
    }

    fn setup_escrow(env: &Env, admin: &Address) -> Address {
        let contract_id = env.register(escrow::EscrowContract, ());
        let client = EscrowContractClient::new(env, &contract_id);
        client.initialize(admin);
        contract_id
    }

    fn setup_refund(env: &Env, admin: &Address) -> Address {
        let contract_id = env.register(refund::RefundContract, ());
        let client = RefundContractClient::new(env, &contract_id);
        client.initialize(admin);
        contract_id
    }

    #[test]
    fn test_initialize_and_pause_all() {
        let env = Env::default();
        env.mock_all_auths();

        let admin_contract_id = env.register(AdminContract, ());
        let client = AdminContractClient::new(&env, &admin_contract_id);

        let admin = Address::generate(&env);
        let pauser = Address::generate(&env);
        let payment_contract = setup_payment(&env, &pauser);
        let escrow_contract = setup_escrow(&env, &pauser);
        let refund_contract = setup_refund(&env, &pauser);

        client.initialize(
            &admin,
            &pauser,
            &payment_contract,
            &escrow_contract,
            &refund_contract,
        );

        let reason = String::from_str(&env, "security incident");
        client.emergency_pause_all(&pauser, &reason, &None, &IncidentReason::Other);
        client.emergency_unpause_all(&pauser);
    }

    /// Registers the admin contract and three child contracts (all administered by
    /// `pauser`), then initializes the admin contract.
    /// Returns (client, admin, pauser, payment, escrow, refund).
    fn setup_initialized(
        env: &Env,
    ) -> (
        AdminContractClient<'_>,
        Address,
        Address,
        Address,
        Address,
        Address,
    ) {
        let admin_contract_id = env.register(AdminContract, ());
        let client = AdminContractClient::new(env, &admin_contract_id);

        let admin = Address::generate(env);
        let pauser = Address::generate(env);
        let payment_contract = setup_payment(env, &pauser);
        let escrow_contract = setup_escrow(env, &pauser);
        let refund_contract = setup_refund(env, &pauser);

        client.initialize(
            &admin,
            &pauser,
            &payment_contract,
            &escrow_contract,
            &refund_contract,
        );

        (
            client,
            admin,
            pauser,
            payment_contract,
            escrow_contract,
            refund_contract,
        )
    }

    #[test]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, pauser, payment_contract, escrow_contract, refund_contract) =
            setup_initialized(&env);

        let result = client.try_initialize(
            &admin,
            &pauser,
            &payment_contract,
            &escrow_contract,
            &refund_contract,
        );
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }

    #[test]
    fn test_pause_and_unpause_all_updates_child_contracts() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, pauser, payment_contract, escrow_contract, refund_contract) =
            setup_initialized(&env);
        let payment = PaymentContractClient::new(&env, &payment_contract);
        let escrow = EscrowContractClient::new(&env, &escrow_contract);
        let refund = RefundContractClient::new(&env, &refund_contract);

        let reason = String::from_str(&env, "security incident");
        client.emergency_pause_all(&pauser, &reason, &None, &IncidentReason::Other);

        assert!(payment.get_pause_state().globally_paused);
        assert!(escrow.get_pause_state().globally_paused);
        assert!(refund.get_pause_state().globally_paused);
        assert_eq!(payment.get_pause_state().pause_reason, reason);

        client.emergency_unpause_all(&pauser);

        assert!(!payment.get_pause_state().globally_paused);
        assert!(!escrow.get_pause_state().globally_paused);
        assert!(!refund.get_pause_state().globally_paused);
    }

    #[test]
    fn test_pause_all_rejects_non_pauser_and_uninitialized() {
        let env = Env::default();
        env.mock_all_auths();

        // Uninitialized contract returns NotInitialized.
        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);
        let someone = Address::generate(&env);
        let reason = String::from_str(&env, "incident");
        assert_eq!(
            fresh.try_emergency_pause_all(&someone, &reason, &None, &IncidentReason::Other),
            Err(Ok(Error::NotInitialized))
        );
        assert_eq!(
            fresh.try_emergency_unpause_all(&someone),
            Err(Ok(Error::NotInitialized))
        );

        // Initialized contract rejects anyone but the stored pauser, including the admin.
        let (client, admin, _pauser, payment_contract, _, _) = setup_initialized(&env);
        assert_eq!(
            client.try_emergency_pause_all(&admin, &reason, &None, &IncidentReason::Other),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_emergency_unpause_all(&someone),
            Err(Ok(Error::Unauthorized))
        );
        assert!(
            !PaymentContractClient::new(&env, &payment_contract)
                .get_pause_state()
                .globally_paused
        );
    }

    #[test]
    fn test_contract_setters_admin_only_and_take_effect() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, pauser, old_payment, old_escrow, old_refund) = setup_initialized(&env);
        let new_payment = setup_payment(&env, &pauser);
        let new_escrow = setup_escrow(&env, &pauser);
        let new_refund = setup_refund(&env, &pauser);

        // Non-admin callers (including the pauser) are rejected.
        let stranger = Address::generate(&env);
        assert_eq!(
            client.try_set_payment_contract(&stranger, &new_payment),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_set_escrow_contract(&pauser, &new_escrow),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_set_refund_contract(&stranger, &new_refund),
            Err(Ok(Error::Unauthorized))
        );

        // Setters on an uninitialized contract return NotInitialized.
        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);
        assert_eq!(
            fresh.try_set_payment_contract(&admin, &new_payment),
            Err(Ok(Error::NotInitialized))
        );

        client.set_payment_contract(&admin, &new_payment);
        client.set_escrow_contract(&admin, &new_escrow);
        client.set_refund_contract(&admin, &new_refund);

        // Pausing now targets the new contracts and leaves the old ones untouched.
        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "rotation"),
            &None,
            &IncidentReason::Other,
        );

        assert!(
            PaymentContractClient::new(&env, &new_payment)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            EscrowContractClient::new(&env, &new_escrow)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            RefundContractClient::new(&env, &new_refund)
                .get_pause_state()
                .globally_paused
        );

        assert!(
            !PaymentContractClient::new(&env, &old_payment)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            !EscrowContractClient::new(&env, &old_escrow)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            !RefundContractClient::new(&env, &old_refund)
                .get_pause_state()
                .globally_paused
        );
    }

    #[test]
    fn test_get_admin_returns_stored_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, _, _, _, _) = setup_initialized(&env);

        assert_eq!(client.get_admin(), admin.clone());
    }

    #[test]
    fn test_get_admin_returns_not_initialized_before_init() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);

        assert_eq!(fresh.try_get_admin(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_get_pauser_returns_stored_pauser() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, pauser, _, _, _) = setup_initialized(&env);

        assert_eq!(client.get_pauser(), pauser.clone());
    }

    #[test]
    fn test_get_pauser_returns_not_initialized_before_init() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);

        assert_eq!(fresh.try_get_pauser(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_get_contracts_returns_all_addresses() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, _, payment, escrow, refund) = setup_initialized(&env);

        let contracts = client.get_contracts();
        assert_eq!(contracts.payment, payment);
        assert_eq!(contracts.escrow, escrow);
        assert_eq!(contracts.refund, refund);
    }

    #[test]
    fn test_get_contracts_returns_not_initialized_before_init() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);

        assert_eq!(fresh.try_get_contracts(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_get_contracts_reflects_updated_addresses() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, pauser, _, _, _) = setup_initialized(&env);
        let new_payment = setup_payment(&env, &pauser);
        let new_escrow = setup_escrow(&env, &pauser);
        let new_refund = setup_refund(&env, &pauser);

        client.set_payment_contract(&admin, &new_payment);
        client.set_escrow_contract(&admin, &new_escrow);
        client.set_refund_contract(&admin, &new_refund);

        let contracts = client.get_contracts();
        assert_eq!(contracts.payment, new_payment);
        assert_eq!(contracts.escrow, new_escrow);
        assert_eq!(contracts.refund, new_refund);
    }

    /// Turns the single-admin setup into a 2-of-3 multisig.
    fn setup_multisig(env: &Env) -> (AdminContractClient<'_>, Address, Address, Address, Address) {
        let (client, admin, pauser, _, _, _) = setup_initialized(env);
        let a2 = Address::generate(env);
        let a3 = Address::generate(env);
        client.set_admins(
            &admin,
            &Vec::from_array(env, [admin.clone(), a2.clone(), a3.clone()]),
            &2,
        );
        (client, admin, a2, a3, pauser)
    }

    #[test]
    fn test_initialize_sets_single_member_admin_set() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _, _, _, _) = setup_initialized(&env);
        assert_eq!(client.get_admins(), Vec::from_array(&env, [admin]));
        assert_eq!(client.get_threshold(), 1);
    }

    #[test]
    fn test_set_admins_validates_set_and_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _, _, _, _) = setup_initialized(&env);
        let a2 = Address::generate(&env);

        assert_eq!(
            client.try_set_admins(&admin, &Vec::new(&env), &1),
            Err(Ok(Error::InvalidAdminSet))
        );
        assert_eq!(
            client.try_set_admins(
                &admin,
                &Vec::from_array(&env, [admin.clone(), admin.clone()]),
                &1
            ),
            Err(Ok(Error::InvalidAdminSet))
        );
        let set = Vec::from_array(&env, [admin.clone(), a2.clone()]);
        assert_eq!(
            client.try_set_admins(&admin, &set, &0),
            Err(Ok(Error::InvalidThreshold))
        );
        assert_eq!(
            client.try_set_admins(&admin, &set, &3),
            Err(Ok(Error::InvalidThreshold))
        );
        assert_eq!(
            client.try_set_admins(&a2, &set, &2),
            Err(Ok(Error::Unauthorized))
        );
    }

    #[test]
    fn test_direct_privileged_calls_blocked_above_threshold_one() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, _, pauser) = setup_multisig(&env);
        let new_payment = setup_payment(&env, &pauser);

        assert_eq!(
            client.try_set_payment_contract(&admin, &new_payment),
            Err(Ok(Error::MultisigRequired))
        );
        assert_eq!(
            client.try_set_escrow_contract(&a2, &new_payment),
            Err(Ok(Error::MultisigRequired))
        );
        assert_eq!(
            client.try_set_refund_contract(&admin, &new_payment),
            Err(Ok(Error::MultisigRequired))
        );
        assert_eq!(
            client.try_upgrade(&admin, &BytesN::from_array(&env, &[0u8; 32])),
            Err(Ok(Error::MultisigRequired))
        );
        assert_eq!(
            client.try_set_admins(&admin, &Vec::from_array(&env, [admin.clone()]), &1),
            Err(Ok(Error::MultisigRequired))
        );
    }

    #[test]
    fn test_proposal_below_threshold_cannot_execute() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, _, pauser) = setup_multisig(&env);
        let old_payment = client.get_contracts().payment;
        let new_payment = setup_payment(&env, &pauser);

        let id = client.propose_action(
            &admin,
            &AdminAction::SetPaymentContract(new_payment.clone()),
            &None,
        );
        assert_eq!(
            client.try_execute_action(&admin, &id),
            Err(Ok(Error::ThresholdNotMet))
        );
        assert_eq!(client.get_contracts().payment, old_payment);

        assert_eq!(
            client.try_approve_action(&admin, &id),
            Err(Ok(Error::AlreadyApproved))
        );
        client.approve_action(&a2, &id);
        client.execute_action(&a2, &id);
        assert_eq!(client.get_contracts().payment, new_payment);
        assert!(client.get_proposal(&id).executed);

        assert_eq!(
            client.try_execute_action(&admin, &id),
            Err(Ok(Error::ProposalAlreadyExecuted))
        );
    }

    #[test]
    fn test_non_admin_cannot_propose_approve_or_execute() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _, _, pauser) = setup_multisig(&env);
        let stranger = Address::generate(&env);
        let action = AdminAction::SetRefundContract(stranger.clone());

        assert_eq!(
            client.try_propose_action(&pauser, &action, &None),
            Err(Ok(Error::Unauthorized))
        );
        let id = client.propose_action(&admin, &action, &None);
        assert_eq!(
            client.try_approve_action(&stranger, &id),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_execute_action(&stranger, &id),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_approve_action(&admin, &99),
            Err(Ok(Error::ProposalNotFound))
        );
    }

    #[test]
    fn test_proposals_expire() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, a3, pauser) = setup_multisig(&env);
        let new_escrow = setup_escrow(&env, &pauser);

        let id = client.propose_action(&admin, &AdminAction::SetEscrowContract(new_escrow), &None);
        client.approve_action(&a2, &id);

        let expires_at = client.get_proposal(&id).expires_at;
        env.ledger().with_mut(|l| l.timestamp = expires_at + 1);

        assert_eq!(
            client.try_approve_action(&a3, &id),
            Err(Ok(Error::ProposalExpired))
        );
        assert_eq!(
            client.try_execute_action(&admin, &id),
            Err(Ok(Error::ProposalExpired))
        );
    }

    #[test]
    fn test_set_admins_via_proposal_and_removed_approvals_do_not_count() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, a3, pauser) = setup_multisig(&env);
        let new_refund = setup_refund(&env, &pauser);

        // a3 approves a pending action, then is removed from the set.
        let pending =
            client.propose_action(&a3, &AdminAction::SetRefundContract(new_refund), &None);

        let new_set = Vec::from_array(&env, [admin.clone(), a2.clone()]);
        let id = client.propose_action(&admin, &AdminAction::SetAdmins(new_set.clone(), 2), &None);
        client.approve_action(&a2, &id);
        client.execute_action(&admin, &id);
        assert_eq!(client.get_admins(), new_set);
        assert_eq!(client.get_threshold(), 2);

        // Only a3's (now stale) approval exists, so the threshold is not met.
        assert_eq!(
            client.try_execute_action(&admin, &pending),
            Err(Ok(Error::ThresholdNotMet))
        );

        // Invalid SetAdmins proposals are rejected up front.
        assert_eq!(
            client.try_propose_action(&admin, &AdminAction::SetAdmins(new_set, 5), &None),
            Err(Ok(Error::InvalidThreshold))
        );
    }

    #[test]
    fn test_pauser_stays_single_key_under_multisig() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, _, pauser) = setup_multisig(&env);
        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &None,
            &IncidentReason::Other,
        );
        let payment = PaymentContractClient::new(&env, &client.get_contracts().payment);
        assert!(payment.get_pause_state().globally_paused);
    }

    #[test]
    fn test_get_platform_versions_aggregates_children() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _, payment, escrow, refund) = setup_initialized(&env);

        let versions = client.get_platform_versions();
        assert_eq!(versions.admin, ADMIN_SCHEMA_VERSION);
        assert_eq!(versions.admin, client.get_schema_version());
        assert_eq!(
            versions.payment,
            Some(PaymentContractClient::new(&env, &payment).get_schema_version())
        );
        assert_eq!(
            versions.escrow,
            Some(EscrowContractClient::new(&env, &escrow).get_schema_version())
        );
        assert_eq!(
            versions.refund,
            Some(RefundContractClient::new(&env, &refund).get_schema_version())
        );
    }

    #[test]
    fn test_get_platform_versions_handles_unset_children() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);
        assert_eq!(
            fresh.get_platform_versions(),
            PlatformVersions {
                admin: ADMIN_SCHEMA_VERSION,
                payment: None,
                escrow: None,
                refund: None,
            }
        );
    }

    // ── #820: bounded emergency pause ──────────────────────────────────────

    const PAUSE_SECONDS: u64 = 3_600;

    fn has_event(env: &Env, name: &str) -> bool {
        use soroban_sdk::testutils::Events;
        use soroban_sdk::{Symbol, TryFromVal};
        let expected = Symbol::new(env, name);
        env.events().all().iter().any(|(_, topics, _)| {
            topics
                .get(0)
                .and_then(|t| Symbol::try_from_val(env, &t).ok())
                .is_some_and(|s| s == expected)
        })
    }

    fn all_paused(env: &Env, client: &AdminContractClient<'_>) -> [bool; 3] {
        let contracts = client.get_contracts();
        [
            PaymentContractClient::new(env, &contracts.payment)
                .get_pause_state()
                .globally_paused,
            EscrowContractClient::new(env, &contracts.escrow)
                .get_pause_state()
                .globally_paused,
            RefundContractClient::new(env, &contracts.refund)
                .get_pause_state()
                .globally_paused,
        ]
    }

    fn set_time(env: &Env, timestamp: u64) {
        env.ledger().with_mut(|l| l.timestamp = timestamp);
    }

    #[test]
    fn test_bounded_pause_cannot_expire_early_and_auto_unpauses_after() {
        let env = Env::default();
        // The children check the pauser's auth on a non-root call.
        env.mock_all_auths_allowing_non_root_auth();
        set_time(&env, 1_000);
        let (client, _, pauser, _, _, _) = setup_initialized(&env);

        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        let expires_at = 1_000 + PAUSE_SECONDS;
        assert_eq!(client.get_emergency_pause_expiry(), Some(expires_at));
        assert_eq!(all_paused(&env, &client), [true; 3]);

        set_time(&env, expires_at - 1);
        assert_eq!(
            client.try_expire_emergency_pause(),
            Err(Ok(Error::PauseNotExpired))
        );
        assert_eq!(all_paused(&env, &client), [true; 3]);

        set_time(&env, expires_at);
        client.expire_emergency_pause();
        assert!(has_event(&env, "emergency_pause_auto_expired"));
        assert_eq!(all_paused(&env, &client), [false; 3]);
        assert_eq!(client.get_emergency_pause_expiry(), None);

        // The expiry is consumed; a second call has nothing to expire.
        assert_eq!(
            client.try_expire_emergency_pause(),
            Err(Ok(Error::NoPauseExpiry))
        );
    }

    #[test]
    fn test_expire_needs_no_caller_auth_only_the_childrens_pauser_auth() {
        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
        use soroban_sdk::IntoVal;

        let env = Env::default();
        env.mock_all_auths();
        let (client, _, pauser, payment, escrow, refund) = setup_initialized(&env);
        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        set_time(&env, env.ledger().timestamp() + PAUSE_SECONDS);

        // Only the children's own `unpause_contract(pauser)` checks are
        // authorized; nothing authorizes the top-level call itself.
        let args: Vec<soroban_sdk::Val> = (pauser.clone(),).into_val(&env);
        let invoke = |contract| MockAuthInvoke {
            contract,
            fn_name: "unpause_contract",
            args: args.clone(),
            sub_invokes: &[],
        };
        let (payment_invoke, escrow_invoke, refund_invoke) =
            (invoke(&payment), invoke(&escrow), invoke(&refund));
        env.mock_auths(&[
            MockAuth {
                address: &pauser,
                invoke: &payment_invoke,
            },
            MockAuth {
                address: &pauser,
                invoke: &escrow_invoke,
            },
            MockAuth {
                address: &pauser,
                invoke: &refund_invoke,
            },
        ]);

        client.expire_emergency_pause();
        assert_eq!(all_paused(&env, &client), [false; 3]);
    }

    #[test]
    fn test_unbounded_pause_has_no_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, pauser, _, _, _) = setup_initialized(&env);

        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &None,
            &IncidentReason::Other,
        );
        assert_eq!(client.get_emergency_pause_expiry(), None);

        set_time(&env, env.ledger().timestamp() + 365 * 24 * 60 * 60);
        assert_eq!(
            client.try_expire_emergency_pause(),
            Err(Ok(Error::NoPauseExpiry))
        );
        assert_eq!(
            client.try_extend_emergency_pause(&admin, &PAUSE_SECONDS),
            Err(Ok(Error::NoPauseExpiry))
        );
        assert_eq!(all_paused(&env, &client), [true; 3]);
    }

    #[test]
    fn test_zero_max_pause_seconds_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, pauser, _, _, _) = setup_initialized(&env);

        assert_eq!(
            client.try_emergency_pause_all(
                &pauser,
                &String::from_str(&env, "incident"),
                &Some(0),
                &IncidentReason::Other
            ),
            Err(Ok(Error::InvalidPauseDuration))
        );
        assert_eq!(all_paused(&env, &client), [false; 3]);
        assert_eq!(client.get_emergency_pause_expiry(), None);
    }

    #[test]
    fn test_admin_extends_pause_before_expiry() {
        let env = Env::default();
        // The children check the pauser's auth on a non-root call.
        env.mock_all_auths_allowing_non_root_auth();
        set_time(&env, 1_000);
        let (client, admin, pauser, _, _, _) = setup_initialized(&env);
        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        let expires_at = 1_000 + PAUSE_SECONDS;

        set_time(&env, expires_at - 1);
        let new_expires_at = client.extend_emergency_pause(&admin, &PAUSE_SECONDS);
        assert_eq!(new_expires_at, expires_at + PAUSE_SECONDS);
        assert!(has_event(&env, "emergency_pause_extended"));
        assert!(!has_event(&env, "emergency_pause_auto_expired"));
        assert_eq!(client.get_emergency_pause_expiry(), Some(new_expires_at));

        // The original expiry no longer lifts the pause.
        set_time(&env, expires_at);
        assert_eq!(
            client.try_expire_emergency_pause(),
            Err(Ok(Error::PauseNotExpired))
        );
        assert_eq!(all_paused(&env, &client), [true; 3]);

        set_time(&env, new_expires_at);
        client.expire_emergency_pause();
        assert_eq!(all_paused(&env, &client), [false; 3]);
    }

    #[test]
    fn test_extend_rejects_non_admin_zero_and_expired_pause() {
        let env = Env::default();
        env.mock_all_auths();
        set_time(&env, 1_000);
        let (client, admin, pauser, _, _, _) = setup_initialized(&env);
        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        let expires_at = 1_000 + PAUSE_SECONDS;

        // Only admin-set members may extend; the pauser is not one.
        let stranger = Address::generate(&env);
        assert_eq!(
            client.try_extend_emergency_pause(&pauser, &PAUSE_SECONDS),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_extend_emergency_pause(&stranger, &PAUSE_SECONDS),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_extend_emergency_pause(&admin, &0),
            Err(Ok(Error::InvalidPauseDuration))
        );
        assert_eq!(client.get_emergency_pause_expiry(), Some(expires_at));

        // Once the expiry is reached it can no longer be extended.
        set_time(&env, expires_at);
        assert_eq!(
            client.try_extend_emergency_pause(&admin, &PAUSE_SECONDS),
            Err(Ok(Error::PauseExpired))
        );
    }

    #[test]
    fn test_any_multisig_member_can_extend() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, a2, _, pauser) = setup_multisig(&env);
        client.emergency_pause_all(
            &pauser,
            &String::from_str(&env, "incident"),
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        let expires_at = client.get_emergency_pause_expiry().unwrap();

        assert_eq!(
            client.extend_emergency_pause(&a2, &PAUSE_SECONDS),
            expires_at + PAUSE_SECONDS
        );
    }

    #[test]
    fn test_manual_unpause_and_unbounded_repause_clear_expiry() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, pauser, _, _, _) = setup_initialized(&env);
        let reason = String::from_str(&env, "incident");

        client.emergency_pause_all(
            &pauser,
            &reason,
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        client.emergency_unpause_all(&pauser);
        assert_eq!(client.get_emergency_pause_expiry(), None);
        assert_eq!(
            client.try_expire_emergency_pause(),
            Err(Ok(Error::NoPauseExpiry))
        );

        client.emergency_pause_all(
            &pauser,
            &reason,
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        client.emergency_unpause_all(&pauser);
        client.emergency_pause_all(&pauser, &reason, &None, &IncidentReason::Other);
        assert_eq!(client.get_emergency_pause_expiry(), None);
    }

    // ── #818: incident log ─────────────────────────────────────────────────

    #[test]
    fn test_incident_log_records_every_emergency_action() {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();
        set_time(&env, 1_000);
        let (client, admin, pauser, _, _, _) = setup_initialized(&env);
        let reason = String::from_str(&env, "incident");
        let empty = String::from_str(&env, "");
        let function = String::from_str(&env, "process_refund");

        assert_eq!(client.get_incident_count(), 0);
        assert_eq!(client.get_incident_log(&0, &10).len(), 0);

        client.emergency_pause_function(
            &pauser,
            &ChildKind::Refund,
            &function,
            &reason,
            &IncidentReason::Other,
        );
        client.emergency_unpause_function(&pauser, &ChildKind::Refund, &function);
        client.emergency_pause_all(
            &pauser,
            &reason,
            &Some(PAUSE_SECONDS),
            &IncidentReason::Other,
        );
        set_time(&env, 1_100);
        client.extend_emergency_pause(&admin, &PAUSE_SECONDS);
        set_time(&env, 1_000 + 2 * PAUSE_SECONDS);
        client.expire_emergency_pause();
        client.emergency_pause_all(&pauser, &reason, &None, &IncidentReason::Other);
        client.emergency_unpause_all(&pauser);
        assert!(has_event(&env, "incident_logged"));

        assert_eq!(client.get_incident_count(), 7);
        let log = client.get_incident_log(&0, &10);
        assert_eq!(log.len(), 7);

        assert_eq!(
            log.get(0).unwrap(),
            IncidentEntry {
                id: 1,
                action: IncidentAction::PauseFunction,
                target: IncidentTarget::Child(ChildKind::Refund),
                function: Some(function.clone()),
                reason: reason.clone(),
                actor: Some(pauser.clone()),
                at: 1_000,
                reason_code: Some(4),
            }
        );
        let unpause_fn = log.get(1).unwrap();
        assert_eq!(unpause_fn.action, IncidentAction::UnpauseFunction);
        assert_eq!(unpause_fn.function, Some(function));
        assert_eq!(unpause_fn.reason, empty);

        let pause_all = log.get(2).unwrap();
        assert_eq!(pause_all.action, IncidentAction::PauseAll);
        assert_eq!(pause_all.target, IncidentTarget::AllChildren);
        assert_eq!(pause_all.function, None);
        assert_eq!(pause_all.reason, reason);

        let extend = log.get(3).unwrap();
        assert_eq!(extend.action, IncidentAction::ExtendPause);
        assert_eq!(extend.actor, Some(admin));
        assert_eq!(extend.at, 1_100);

        let expired = log.get(4).unwrap();
        assert_eq!(expired.action, IncidentAction::PauseAutoExpired);
        assert_eq!(expired.actor, None);
        assert_eq!(expired.at, 1_000 + 2 * PAUSE_SECONDS);

        assert_eq!(log.get(5).unwrap().action, IncidentAction::PauseAll);
        assert_eq!(log.get(6).unwrap().action, IncidentAction::UnpauseAll);
        for (i, entry) in log.iter().enumerate() {
            assert_eq!(entry.id, i as u64 + 1);
        }
    }

    #[test]
    fn test_incident_log_is_paginated_and_persistent() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, pauser, _, _, _) = setup_initialized(&env);
        let reason = String::from_str(&env, "incident");

        for _ in 0..30 {
            client.emergency_pause_all(&pauser, &reason, &None, &IncidentReason::Other);
            client.emergency_unpause_all(&pauser);
        }
        assert_eq!(client.get_incident_count(), 60);

        let first = client.get_incident_log(&0, &5);
        assert_eq!(first.len(), 5);
        assert_eq!(first.get(0).unwrap().id, 1);
        assert_eq!(first.get(4).unwrap().id, 5);

        let middle = client.get_incident_log(&5, &3);
        assert_eq!(middle.len(), 3);
        assert_eq!(middle.get(0).unwrap().id, 6);

        // The tail page is truncated at the end of the log.
        let tail = client.get_incident_log(&58, &10);
        assert_eq!(tail.len(), 2);
        assert_eq!(tail.get(1).unwrap().id, 60);

        // Past the end, zero limit and oversized limits.
        assert_eq!(client.get_incident_log(&60, &10).len(), 0);
        assert_eq!(client.get_incident_log(&u64::MAX, &10).len(), 0);
        assert_eq!(client.get_incident_log(&0, &0).len(), 0);
        assert_eq!(
            client.get_incident_log(&0, &u32::MAX).len(),
            MAX_INCIDENT_PAGE
        );

        // Entries live in persistent storage, not instance storage.
        env.as_contract(&client.address, || {
            assert!(env.storage().persistent().has(&DataKey::Incident(1)));
            assert!(env.storage().persistent().has(&DataKey::IncidentCount));
            assert!(!env.storage().instance().has(&DataKey::Incident(1)));
        });
    }

    #[test]
    fn test_rejected_emergency_actions_are_not_logged() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, pauser, _, _, _) = setup_initialized(&env);
        let reason = String::from_str(&env, "incident");

        assert!(client
            .try_emergency_pause_all(&admin, &reason, &None, &IncidentReason::Other)
            .is_err());
        assert!(client
            .try_emergency_pause_all(&pauser, &reason, &Some(0), &IncidentReason::Other)
            .is_err());
        assert!(client
            .try_emergency_pause_function(
                &pauser,
                &ChildKind::Payment,
                &String::from_str(&env, ""),
                &reason,
                &IncidentReason::Other
            )
            .is_err());
        assert!(client.try_expire_emergency_pause().is_err());
        assert_eq!(client.get_incident_count(), 0);
        assert_eq!(client.get_incident_log(&0, &10).len(), 0);
    }

    #[test]
    fn test_revoke_approval_reduces_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, a3, pauser) = setup_multisig(&env);
        let new_payment = setup_payment(&env, &pauser);
        let id = client.propose_action(
            &admin,
            &AdminAction::SetPaymentContract(new_payment.clone()),
            &None,
        );
        client.approve_action(&a2, &id);

        assert_eq!(
            client.try_revoke_approval(&a3, &id),
            Err(Ok(Error::ApprovalNotFound))
        );
        client.revoke_approval(&a2, &id);
        assert_eq!(client.get_proposal(&id).approvals.len(), 1);
        assert_eq!(
            client.try_execute_action(&admin, &id),
            Err(Ok(Error::ThresholdNotMet))
        );

        client.approve_action(&a2, &id);
        client.execute_action(&admin, &id);
        assert_eq!(client.get_contracts().payment, new_payment);
        assert_eq!(
            client.try_revoke_approval(&a2, &id),
            Err(Ok(Error::ProposalAlreadyExecuted))
        );
    }

    #[test]
    fn test_cancel_proposal_blocks_approval_and_execution() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, a3, pauser) = setup_multisig(&env);
        let new_payment = setup_payment(&env, &pauser);
        let id = client.propose_action(
            &admin,
            &AdminAction::SetPaymentContract(new_payment.clone()),
            &None,
        );

        // A third admin cannot cancel while only one approval is in place.
        assert_eq!(
            client.try_cancel_proposal(&a3, &id),
            Err(Ok(Error::CancellationNotAllowed))
        );
        // Two current admins have approved, so any admin may cancel.
        client.approve_action(&a2, &id);
        client.cancel_proposal(&a3, &id);
        let proposal = client.get_proposal(&id);
        assert!(proposal.cancelled);
        assert_eq!(
            client.try_approve_action(&a3, &id),
            Err(Ok(Error::ProposalCancelled))
        );
        assert_eq!(
            client.try_execute_action(&admin, &id),
            Err(Ok(Error::ProposalCancelled))
        );
        assert_eq!(
            client.try_cancel_proposal(&admin, &id),
            Err(Ok(Error::ProposalCancelled))
        );
        // The proposer may always cancel their own proposal.
        let id2 =
            client.propose_action(&admin, &AdminAction::SetPaymentContract(new_payment), &None);
        client.cancel_proposal(&admin, &id2);
        assert!(client.get_proposal(&id2).cancelled);
    }

    #[test]
    fn test_proposal_listings_are_bounded_and_filtered() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, _, pauser) = setup_multisig(&env);
        let p1 = setup_payment(&env, &pauser);
        let p2 = setup_payment(&env, &pauser);
        let p3 = setup_payment(&env, &pauser);

        let id1 = client.propose_action(&admin, &AdminAction::SetPaymentContract(p1), &None);
        let id2 = client.propose_action(&admin, &AdminAction::SetPaymentContract(p2), &None);
        let id3 = client.propose_action(&admin, &AdminAction::SetPaymentContract(p3), &None);
        client.approve_action(&a2, &id2);
        client.cancel_proposal(&admin, &id1);

        let pending = client.get_pending_proposals(&0, &10);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending.get(0).unwrap().id, id3);
        assert_eq!(pending.get(1).unwrap().id, id2);

        let awaiting = client.get_proposals_awaiting(&a2, &0, &10);
        assert_eq!(awaiting.len(), 1);
        assert_eq!(awaiting.get(0).unwrap().id, id3);
        assert_eq!(client.get_pending_proposals(&1, &10).len(), 1);
        assert_eq!(client.get_pending_proposals(&0, &1000).len(), 2);
        assert_eq!(
            client.try_get_pending_proposals(&0, &0),
            Err(Ok(Error::InvalidPagination))
        );

        client.approve_action(&a2, &id3);
        client.execute_action(&admin, &id3);
        assert_eq!(client.get_pending_proposals(&0, &10).len(), 1);

        set_time(&env, env.ledger().timestamp() + PROPOSAL_TTL_SECONDS + 1);
        assert_eq!(client.get_pending_proposals(&0, &10).len(), 0);
    }

    #[test]
    fn test_child_upgrade_veto_window() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, a2, a3, _) = setup_multisig(&env);
        let hash = BytesN::from_array(&env, &[7u8; 32]);

        client.configure_child_upgrade(&admin, &100, &2);
        let config = client.get_child_upgrade_config();
        assert_eq!(config.delay_seconds, 100);
        assert_eq!(config.veto_threshold, 2);
        assert_eq!(
            client.try_configure_child_upgrade(&admin, &100, &0),
            Err(Ok(Error::InvalidUpgradeConfig))
        );

        client.queue_child_upgrade(&admin, &ChildKind::Payment, &hash);
        let queued = client
            .get_queued_child_upgrade(&ChildKind::Payment)
            .unwrap();
        assert_eq!(queued.wasm_hash, hash);
        assert_eq!(queued.execute_after, queued.queued_at + 100);
        assert_eq!(
            client.try_execute_child_upgrade(&admin, &ChildKind::Payment),
            Err(Ok(Error::UpgradeTooEarly))
        );
        assert_eq!(
            client.try_execute_child_upgrade(&admin, &ChildKind::Escrow),
            Err(Ok(Error::NoQueuedUpgrade))
        );

        client.veto_child_upgrade(&a2, &ChildKind::Payment);
        assert_eq!(
            client.try_veto_child_upgrade(&a2, &ChildKind::Payment),
            Err(Ok(Error::AlreadyVetoed))
        );
        client.veto_child_upgrade(&a3, &ChildKind::Payment);
        assert!(
            client
                .get_queued_child_upgrade(&ChildKind::Payment)
                .unwrap()
                .vetoed
        );
        set_time(&env, queued.execute_after);
        assert_eq!(
            client.try_execute_child_upgrade(&admin, &ChildKind::Payment),
            Err(Ok(Error::UpgradeVetoed))
        );
    }
}
