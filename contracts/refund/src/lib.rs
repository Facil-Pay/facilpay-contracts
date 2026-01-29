#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, String,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Refund(u64),
    RefundCounter,
    Admin,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum RefundStatus {
    Requested,
    Approved,
    Rejected,
    Processed,
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
    InvalidStatus = 5,
    AlreadyProcessed = 6,
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

#[derive(Clone)]
#[contracttype]
pub struct Refund {
    pub id: u64,
    pub payment_id: u64,
    pub merchant: Address,
    pub customer: Address,
    pub amount: i128,
    pub token: Address,
    pub status: RefundStatus,
    pub requested_at: u64,
    pub reason: String,
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
    }

    pub fn request_refund(
        env: Env,
        merchant: Address,
        payment_id: u64,
        customer: Address,
        amount: i128,
        token: Address,
        reason: String,
    ) -> Result<u64, Error> {
        // Require merchant authentication
        merchant.require_auth();

        // Validate amount is positive
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Validate payment_id is valid (greater than 0)
        if payment_id == 0 {
            return Err(Error::InvalidPaymentId);
        }

        // Get and increment refund counter
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);
        let refund_id = counter + 1;

        // Create Refund struct with Requested status
        let refund = Refund {
            id: refund_id,
            payment_id,
            merchant: merchant.clone(),
            customer: customer.clone(),
            amount,
            token: token.clone(),
            status: RefundStatus::Requested,
            requested_at: env.ledger().timestamp(),
            reason,
        };

        // Store refund in contract storage
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        env.storage()
            .instance()
            .set(&DataKey::RefundCounter, &refund_id);

        // Emit RefundRequested event
        RefundRequested {
            refund_id,
            payment_id,
            merchant,
            customer,
            amount,
            token,
        }
        .publish(&env);

        // Return the new refund ID
        Ok(refund_id)
    }

    pub fn approve_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        // Authenticate admin
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?; // Or appropriate error if not initialized
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Get refund
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Update status
        refund.status = RefundStatus::Approved;

        // Store updated refund
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);

        Ok(())
    }

    pub fn process_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        // Authenticate admin
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Get refund
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Validate status
        if refund.status != RefundStatus::Approved {
            return Err(Error::NotApproved);
        }

        // Transfer tokens from merchant to customer
        // Assuming merchant has approved this contract to spend tokens on their behalf
        let token_client = token::Client::new(&env, &refund.token);

        // We use transfer_from to move funds from merchant to customer
        // The merchant must have authorized this contract.
        // If the merchant revoked auth or lacks funds, this will fail.
        let transfer_result = token_client.try_transfer_from(
            &env.current_contract_address(),
            &refund.merchant,
            &refund.customer,
            &refund.amount,
        );

        match transfer_result {
            Ok(_) => {
                // Update status to Processed
                refund.status = RefundStatus::Processed;
                let processed_at = env.ledger().timestamp();

                // Store updated refund
                env.storage()
                    .instance()
                    .set(&DataKey::Refund(refund_id), &refund);

                // Emit RefundProcessed event
                RefundProcessed {
                    refund_id,
                    processed_by: admin,
                    customer: refund.customer,
                    amount: refund.amount,
                    token: refund.token,
                    processed_at,
                }
                .publish(&env);

                Ok(())
            }
            Err(_) => {
                // Return TransferFailed error, status remains Approved
                Err(Error::TransferFailed)
            }
        }
    }

    pub fn get_refund(env: &Env, refund_id: u64) -> Result<Refund, Error> {
        // Retrieve refund from storage by ID
        env.storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)
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

        // Update refund status to Approved
        refund.status = RefundStatus::Approved;

        // Store updated refund back to storage
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);

        // Emit RefundApproved event
        RefundApproved {
            refund_id,
            approved_by: admin,
            approved_at: env.ledger().timestamp(),
        }
        .publish(&env);

        Ok(())
    }

    pub fn reject_refund(
        env: Env,
        admin: Address,
        refund_id: u64,
        rejection_reason: String,
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

        // Update refund status to Rejected
        refund.status = RefundStatus::Rejected;

        // Store updated refund back to storage
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);

        // Emit RefundRejected event
        RefundRejected {
            refund_id,
            rejected_by: admin,
            rejected_at: env.ledger().timestamp(),
            rejection_reason,
        }
        .publish(&env);

        Ok(())
    }
}

mod test;
mod test_process;
