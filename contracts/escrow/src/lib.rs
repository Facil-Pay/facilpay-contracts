#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, String, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Escrow(u64),
    EscrowCounter,
    CustomerEscrows(Address, u64),
    MerchantEscrows(Address, u64),
    CustomerEscrowCount(Address),
    MerchantEscrowCount(Address),
    EscrowEvidence(u64, u64),
    EscrowEvidenceCount(u64),
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
pub struct EscrowReleased {
    pub escrow_id: u64,
    pub merchant: Address,
    pub amount: i128,
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

    pub fn get_escrow(env: &Env, escrow_id: u64) -> Escrow {
        env.storage()
            .instance()
            .get(&DataKey::Escrow(escrow_id))
            .expect("Escrow not found")
    }

    pub fn release_escrow(env: Env, admin: Address, escrow_id: u64) -> Result<(), Error> {
        admin.require_auth();

        // Check if escrow exists
        if !env.storage().instance().has(&DataKey::Escrow(escrow_id)) {
            return Err(Error::EscrowNotFound);
        }

        let mut escrow = EscrowContract::get_escrow(&env, escrow_id);

        match escrow.status {
            EscrowStatus::Locked => {
                // Check current time has passed release_timestamp
                if env.ledger().timestamp() < escrow.release_timestamp {
                    return Err(Error::ReleaseNotYetAvailable);
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

        EscrowReleased {
            escrow_id,
            merchant: escrow.merchant,
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
        let total = EscrowContract::get_evidence_count(&env, escrow_id);
        let mut cust: u64 = 0;
        let mut merch: u64 = 0;
        let mut i: u64 = 0;
        while i < total {
            if let Some(ev) = env
                .storage()
                .instance()
                .get::<DataKey, Evidence>(&DataKey::EscrowEvidence(escrow_id, i))
            {
                if ev.submitter == escrow.customer {
                    cust = cust.saturating_add(1);
                } else if ev.submitter == escrow.merchant {
                    merch = merch.saturating_add(1);
                }
            }
            i += 1;
        }
        let release_to_merchant = merch > cust;
        escrow.status = if release_to_merchant {
            EscrowStatus::Released
        } else {
            EscrowStatus::Resolved
        };
        env.storage()
            .instance()
            .set(&DataKey::Escrow(escrow_id), &escrow);
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
}

mod test;
