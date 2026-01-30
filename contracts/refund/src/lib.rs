#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, String, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Refund(u64),
    RefundCounter,
    RefundsByStatus(RefundStatus, u64),
    RefundStatusCount(RefundStatus),
    RefundStatusIndex(u64),
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
    RefundExceedsPayment = 7,
    TotalRefundsExceedPayment = 8,
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
    pub original_payment_amount: i128,
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
        original_payment_amount: i128,
        token: Address,
        reason: String,
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

        Self::can_refund_payment(&env, payment_id, amount, original_payment_amount)?;

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
            original_payment_amount,
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
        Self::add_to_status_index(&env, RefundStatus::Requested, refund_id);

        // Index refund by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRefundCount(merchant.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefunds(merchant.clone(), merchant_count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefundCount(merchant.clone()), &(merchant_count + 1));

        // Index refund by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundCount(customer.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::CustomerRefunds(customer.clone(), customer_count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::CustomerRefundCount(customer.clone()), &(customer_count + 1));

        // Index refund by payment
        let payment_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundCount(payment_id))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PaymentRefunds(payment_id, payment_count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::PaymentRefundCount(payment_id), &(payment_count + 1));

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

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;

        // Update refund status to Approved
        refund.status = RefundStatus::Approved;

        // Store updated refund back to storage
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Approved, refund_id);

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

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;

        // Update refund status to Rejected
        refund.status = RefundStatus::Rejected;

        // Store updated refund back to storage
        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);

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
            refund.original_payment_amount,
        )?;

        Self::remove_from_status_index(&env, RefundStatus::Approved, refund_id)?;
        refund.status = RefundStatus::Processed;

        env.storage()
            .instance()
            .set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Processed, refund_id);

        Ok(())
    }

    pub fn get_refunds_by_status(
        env: &Env,
        status: RefundStatus,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(env);
        let total = Self::get_refund_count_by_status(env, status.clone());

        if limit == 0 || offset >= total {
            return results;
        }

        let end = core::cmp::min(total, offset.saturating_add(limit));
        let mut index = offset;
        while index < end {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get::<_, u64>(&DataKey::RefundsByStatus(status.clone(), index))
            {
                if let Some(refund) = env
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

    pub fn get_refund_count_by_status(env: &Env, status: RefundStatus) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::RefundStatusCount(status))
            .unwrap_or(0)
    }

    pub fn get_total_refunded_amount(env: &Env, payment_id: u64) -> i128 {
        let total_refunds: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundCounter)
            .unwrap_or(0);
        let mut total: i128 = 0;

        let mut id: u64 = 1;
        while id <= total_refunds {
            if let Some(refund) = env.storage().instance().get::<_, Refund>(&DataKey::Refund(id)) {
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
        original_amount: i128,
    ) -> Result<bool, Error> {
        let total_refunded = Self::get_total_refunded_amount(env, payment_id);
        if requested_amount.saturating_add(total_refunded) > original_amount {
            return Err(Error::TotalRefundsExceedPayment);
        }

        Ok(true)
    }

    fn add_to_status_index(env: &Env, status: RefundStatus, refund_id: u64) {
        let count = Self::get_refund_count_by_status(env, status.clone());
        env.storage()
            .instance()
            .set(&DataKey::RefundsByStatus(status.clone(), count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusCount(status.clone()), &(count + 1));
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusIndex(refund_id), &count);
    }

    fn remove_from_status_index(
        env: &Env,
        status: RefundStatus,
        refund_id: u64,
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
            env.storage()
                .instance()
                .set(&DataKey::RefundsByStatus(status.clone(), index), &last_refund_id);
            env.storage()
                .instance()
                .set(&DataKey::RefundStatusIndex(last_refund_id), &index);
        }

        env.storage()
            .instance()
            .remove(&DataKey::RefundsByStatus(status.clone(), last_index));
        env.storage()
            .instance()
            .remove(&DataKey::RefundStatusIndex(refund_id));
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusCount(status), &last_index);

        Ok(())
    }
}

mod test;
mod test_process;
