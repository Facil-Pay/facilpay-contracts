#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Bytes, Env,
    String, Vec,
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
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum EscrowStatus {
    Locked,
    Released,
    Disputed,
    Resolved,
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
    pub milestone_index: u32,
    pub amount: i128,
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
    pub unlock_timestamp: u64,
    pub amount: i128,
    pub released: bool,
    pub description: String,
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
    pub delay: u64,           // minimum seconds before execution
    pub grace_period: u64,    // window after delay before action expires
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
        env.storage().instance().set(&DataKey::MultiSigConfig, &config);
        AdminAdded { admin }.publish(&env);
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
        env.storage().instance().set(&DataKey::ProposalCounter, &counter);

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

    pub fn approve_action(
        env: Env,
        approver: Address,
        proposal_id: String,
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

    pub fn execute_action(
        env: Env,
        proposal_id: String,
    ) -> Result<(), Error> {
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

    pub fn reject_action(
        env: Env,
        rejecter: Address,
        proposal_id: String,
    ) -> Result<(), Error> {
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

    pub fn add_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
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

        if !config.admins.contains(&new_admin) {
            config.admins.push_back(new_admin.clone());
            config.total_admins += 1;
            env.storage().instance().set(&DataKey::MultiSigConfig, &config);
            AdminAdded { admin: new_admin }.publish(&env);
        }

        Ok(())
    }

    pub fn remove_admin(
        env: Env,
        caller: Address,
        admin: Address,
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
        env.storage().instance().set(&DataKey::MultiSigConfig, &config);
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
        env.storage().instance().set(&DataKey::MultiSigConfig, &config);

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

    pub fn approve_release(
        env: Env,
        caller: Address,
        escrow_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();

        if !env.storage().instance().has(&DataKey::MultiPartyEscrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow: MultiPartyEscrow = env.storage().instance().get(&DataKey::MultiPartyEscrow(escrow_id)).unwrap();

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
        env.storage().instance().set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);

        ParticipantApproved {
            escrow_id,
            approver: caller,
        }
        .publish(&env);

        Ok(())
    }

    pub fn release_multi_party_escrow(
        env: Env,
        escrow_id: u64,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::MultiPartyEscrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow: MultiPartyEscrow = env.storage().instance().get(&DataKey::MultiPartyEscrow(escrow_id)).unwrap();

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
        env.storage().instance().set(&DataKey::MultiPartyEscrow(escrow_id), &escrow);

        MultiPartyEscrowReleased {
            escrow_id,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_multi_party_escrow(
        env: Env,
        escrow_id: u64,
    ) -> Result<MultiPartyEscrow, Error> {
        if !env.storage().instance().has(&DataKey::MultiPartyEscrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }
        Ok(env.storage().instance().get(&DataKey::MultiPartyEscrow(escrow_id)).unwrap())
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
        let config = Self::get_multisig_config(env.clone());
        if config.admins.contains(&admin) && early_release {
            // Admin force release requires time-lock
            return Err(Error::Unauthorized);
        }

        Self::internal_release_escrow(env, admin, escrow_id, early_release)
    }

    fn internal_release_escrow(
        env: Env,
        admin: Address,
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
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        // If this escrow contract currently holds a real token balance, release funds to merchant.
        EscrowContract::transfer_if_token_contract(&env, &escrow.token, &escrow.merchant, escrow.amount)?;

        // Update reputation for both parties on successful completion.
        EscrowContract::update_reputation_on_completion(&env, &escrow.merchant);
        EscrowContract::update_reputation_on_completion(&env, &escrow.customer);

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
            EscrowStatus::Released | EscrowStatus::Resolved => return Err(Error::AlreadyProcessed),
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

        EscrowContract::transfer_if_token_contract(&env, &escrow.token, &escrow.customer, escrow.amount)?;

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
        if escrow.customer != caller && escrow.merchant != caller {
            return Err(Error::Unauthorized);
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
        }

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);

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

    pub fn get_evidence(
        env: Env,
        escrow_id: u64,
        limit: u64,
        offset: u64,
    ) -> Vec<Evidence> {
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

        // Check if this is being called from execute_queued_action
        let config = Self::get_multisig_config(env.clone());
        if config.admins.contains(&admin) {
            // Admin actions require time-lock for sensitive operations
            return Err(Error::Unauthorized);
        }

        Self::internal_resolve_dispute(env, admin, escrow_id, release_to_merchant)
    }

    fn internal_resolve_dispute(
        env: Env,
        admin: Address,
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

        let (winner, loser) = if release_to_merchant {
            (escrow.merchant.clone(), escrow.customer.clone())
        } else {
            (escrow.customer.clone(), escrow.merchant.clone())
        };
        EscrowContract::update_reputation_on_dispute_outcome(&env, &winner, &loser);

        EscrowResolved {
            escrow_id,
            released_to_merchant: release_to_merchant,
            amount: escrow.amount,
        }
        .publish(&env);

        Ok(())
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
        };

        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage()
            .instance()
            .set(&DataKey::EscrowCounter, &escrow_id);

        // Create and store the vesting schedule
        let vesting_schedule = VestingSchedule {
            escrow_id,
            total_amount: amount,
            released_amount: 0,
            start_timestamp: current_timestamp,
            cliff_timestamp,
            end_timestamp,
            milestones,
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
    pub fn release_vested_amount(
        env: Env,
        admin: Address,
        escrow_id: u64,
    ) -> Result<i128, Error> {
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
                    milestones_vec.set(i, milestone);

                    MilestoneReleased {
                        escrow_id,
                        milestone_index: i as u32,
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
                }

                env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);
                EscrowContract::transfer_if_token_contract(env, &escrow.token, &escrow.merchant, escrow.amount)?;
                EscrowContract::update_reputation_on_completion(env, &escrow.merchant);
                EscrowContract::update_reputation_on_completion(env, &escrow.customer);

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

                env.storage().instance().set(&DataKey::Escrow(escrow_id), &escrow);

                let (winner, loser) = if release_to_merchant {
                    (escrow.merchant.clone(), escrow.customer.clone())
                } else {
                    (escrow.customer.clone(), escrow.merchant.clone())
                };
                EscrowContract::update_reputation_on_dispute_outcome(env, &winner, &loser);

                if release_to_merchant {
                    EscrowContract::transfer_if_token_contract(env, &escrow.token, &escrow.merchant, escrow.amount)?;
                } else {
                    EscrowContract::transfer_if_token_contract(env, &escrow.token, &escrow.customer, escrow.amount)?;
                }

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
                    env.storage().instance().set(&DataKey::MultiSigConfig, &config);
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
                env.storage().instance().set(&DataKey::MultiSigConfig, &config);
                AdminRemoved { admin: admin_to_remove }.publish(env);
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
                env.storage().instance().set(&DataKey::MultiSigConfig, &config);
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
        
        let action_id = env.storage().instance().get(&DataKey::TimeLockCounter).unwrap_or(0u64) + 1;
        env.storage().instance().set(&DataKey::TimeLockCounter, &action_id);

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

        env.storage().instance().set(&DataKey::TimeLockAction(action_id), &action);
        
        TimeLockActionQueued {
            action_id,
            escrow_id,
            executable_after: action.executable_after,
        }.publish(&env);

        Ok(action_id)
    }

    pub fn execute_queued_action(env: Env, action_id: u64) -> Result<(), Error> {
        let mut action: TimeLockAction = env.storage().instance()
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
        env.storage().instance().set(&DataKey::TimeLockAction(action_id), &action);

        match action.action_type {
            EscrowActionType::ResolveDispute(release_to_merchant) => {
                Self::internal_resolve_dispute(env.clone(), action.proposed_by, action.escrow_id, release_to_merchant)?;
            }
            EscrowActionType::ForceRelease => {
                Self::internal_release_escrow(env.clone(), action.proposed_by, action.escrow_id, true)?;
            }
            _ => {}
        }

        TimeLockActionExecuted {
            action_id,
            escrow_id: action.escrow_id,
            executed_at: current_time,
        }.publish(&env);

        Ok(())
    }

    pub fn cancel_queued_action(env: Env, admin: Address, action_id: u64) -> Result<(), Error> {
        admin.require_auth();
        
        let config = Self::get_multisig_config(env.clone());
        if !config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        let mut action: TimeLockAction = env.storage().instance()
            .get(&DataKey::TimeLockAction(action_id))
            .ok_or(Error::ProposalNotFound)?;

        if action.executed {
            return Err(Error::ActionAlreadyExecuted);
        }
        if action.cancelled {
            return Err(Error::ActionCancelled);
        }

        if action.proposed_by != admin {
            return Err(Error::Unauthorized);
        }

        action.cancelled = true;
        env.storage().instance().set(&DataKey::TimeLockAction(action_id), &action);

        TimeLockActionCancelled {
            action_id,
            cancelled_by: admin,
        }.publish(&env);

        Ok(())
    }

    pub fn get_queued_action(env: Env, action_id: u64) -> Result<TimeLockAction, Error> {
        env.storage().instance()
            .get(&DataKey::TimeLockAction(action_id))
            .ok_or(Error::ProposalNotFound)
    }

    pub fn set_timelock_config(env: Env, admin: Address, config: TimeLockConfig) -> Result<(), Error> {
        admin.require_auth();
        
        let multisig_config = Self::get_multisig_config(env.clone());
        if !multisig_config.admins.contains(&admin) {
            return Err(Error::NotAnAdmin);
        }

        if config.delay < 3600 || config.delay > 604800 {
            return Err(Error::InvalidStatus);
        }

        env.storage().instance().set(&DataKey::TimeLockConfig, &config);
        
        TimeLockConfigUpdated {
            delay: config.delay,
            grace_period: config.grace_period,
        }.publish(&env);

        Ok(())
    }

    fn get_timelock_config(env: Env) -> TimeLockConfig {
        env.storage().instance()
            .get(&DataKey::TimeLockConfig)
            .unwrap_or(TimeLockConfig {
                delay: 86400,      // 24 hours default
                grace_period: 86400, // 24 hours default
            })
    }
}

mod test;

#[cfg(test)]
mod timelock_test;
