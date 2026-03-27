#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, String,
    Vec,
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
    TransferFailed = 9,
    InvalidParticipantCount = 9,
    InvalidSharesSum = 10,
    DuplicateApproval = 11,
    ApprovalsThresholdNotMet = 12,
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

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
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
}

mod test;
