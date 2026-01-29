#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Payment(u64),
    PaymentCounter,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PaymentStatus {
    Pending,
    Completed,
    Refunded,
    Cancelled,
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

#[derive(Clone)]
#[contracttype]
pub struct Payment {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub status: PaymentStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[contract]
pub struct PaymentContract;

#[contractimpl]
impl PaymentContract {
    pub fn create_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        expiration_duration: u64,
    ) -> u64 {
        customer.require_auth();

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
            merchant,
            amount,
            token,
            status: PaymentStatus::Pending,
            created_at: current_timestamp,
            expires_at,
        };

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);
        env.storage()
            .instance()
            .set(&DataKey::PaymentCounter, &payment_id);

        payment_id
    }

    pub fn get_payment(env: &Env, payment_id: u64) -> Payment {
        env.storage()
            .instance()
            .get(&DataKey::Payment(payment_id))
            .expect("Payment not found")
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
        PaymentExpired {
            payment_id,
            expiration_timestamp: payment.expires_at,
        }
        .publish(&env);

        Ok(())
    }

    pub fn complete_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        admin.require_auth();

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Before updating status, check if payment is expired
        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Completed;
            }
            PaymentStatus::Completed => return Err(Error::AlreadyProcessed),
            PaymentStatus::Refunded => return Err(Error::InvalidStatus),
            PaymentStatus::Cancelled => return Err(Error::InvalidStatus),
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        PaymentCompleted {
            payment_id,
            merchant: payment.merchant,
            amount: payment.amount,
        };

        Ok(())
    }

    pub fn refund_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        admin.require_auth();

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Before updating status, check if payment is expired
        if PaymentContract::is_payment_expired(&env, payment_id) {
            return Err(Error::PaymentExpired);
        }

        match payment.status {
            PaymentStatus::Pending => {
                payment.status = PaymentStatus::Refunded;
            }
            PaymentStatus::Completed => return Err(Error::InvalidStatus),
            PaymentStatus::Refunded => return Err(Error::AlreadyProcessed),
            PaymentStatus::Cancelled => return Err(Error::InvalidStatus),
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        PaymentRefunded {
            payment_id,
            customer: payment.customer,
            amount: payment.amount,
        };

        Ok(())
    }

    pub fn cancel_payment(env: Env, caller: Address, payment_id: u64) -> Result<(), Error> {
        caller.require_auth();

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

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
            PaymentStatus::Completed => return Err(Error::InvalidStatus),
            PaymentStatus::Refunded => return Err(Error::InvalidStatus),
            PaymentStatus::Cancelled => return Err(Error::InvalidStatus),
        }

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        let timestamp = env.ledger().timestamp();
        PaymentCancelled {
            payment_id,
            cancelled_by: caller,
            timestamp,
        };

        Ok(())
    }
}

mod test;
