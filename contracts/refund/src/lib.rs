#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, String, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Refund(u64),
    RefundCounter,
    MerchantRefunds(Address, u64),
    CustomerRefunds(Address, u64),
    PaymentRefunds(u64, u64),
    MerchantRefundCount(Address),
    CustomerRefundCount(Address),
    PaymentRefundCount(u64),
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

    pub fn get_refunds_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut refunds = Vec::new(&env);
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRefundCount(merchant.clone()))
            .unwrap_or(0);

        if offset >= total_count {
            return refunds;
        }

        let end = if offset + limit > total_count {
            total_count
        } else {
            offset + limit
        };

        for i in offset..end {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get(&DataKey::MerchantRefunds(merchant.clone(), i))
            {
                if let Some(refund) = env.storage().instance().get(&DataKey::Refund(refund_id)) {
                    refunds.push_back(refund);
                }
            }
        }

        refunds
    }

    pub fn get_refund_count_by_merchant(env: Env, merchant: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MerchantRefundCount(merchant))
            .unwrap_or(0)
    }

    pub fn get_refunds_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut refunds = Vec::new(&env);
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundCount(customer.clone()))
            .unwrap_or(0);

        if offset >= total_count {
            return refunds;
        }

        let end = if offset + limit > total_count {
            total_count
        } else {
            offset + limit
        };

        for i in offset..end {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get(&DataKey::CustomerRefunds(customer.clone(), i))
            {
                if let Some(refund) = env.storage().instance().get(&DataKey::Refund(refund_id)) {
                    refunds.push_back(refund);
                }
            }
        }

        refunds
    }

    pub fn get_refund_count_by_customer(env: Env, customer: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::CustomerRefundCount(customer))
            .unwrap_or(0)
    }

    pub fn get_refunds_by_payment(
        env: Env,
        payment_id: u64,
        limit: u64,
        offset: u64,
    ) -> Vec<Refund> {
        let mut refunds = Vec::new(&env);
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundCount(payment_id))
            .unwrap_or(0);

        if offset >= total_count {
            return refunds;
        }

        let end = if offset + limit > total_count {
            total_count
        } else {
            offset + limit
        };

        for i in offset..end {
            if let Some(refund_id) = env
                .storage()
                .instance()
                .get(&DataKey::PaymentRefunds(payment_id, i))
            {
                if let Some(refund) = env.storage().instance().get(&DataKey::Refund(refund_id)) {
                    refunds.push_back(refund);
                }
            }
        }

        refunds
    }

    pub fn get_refund_count_by_payment(env: Env, payment_id: u64) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::PaymentRefundCount(payment_id))
            .unwrap_or(0)
    }
}

mod test;
