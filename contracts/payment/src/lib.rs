#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, String, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Payment(u64),
    PaymentCounter,
    CustomerPayments(Address, u64),
    MerchantPayments(Address, u64),
    CustomerPaymentCount(Address),
    MerchantPaymentCount(Address),
    PaymentNotes(u64),
    ConversionRate(Currency),
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum Currency {
    XLM,
    USDC,
    USDT,
    BTC,
    ETH,
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
    TransferFailed = 8,
    MetadataTooLarge = 9,
    NotesTooLarge = 10,
    InvalidCurrency = 11,
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
    pub currency: Currency,
    pub status: PaymentStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub metadata: String,
    pub notes: String,
}

#[contract]
pub struct PaymentContract;

// Constants for size limits
const MAX_METADATA_SIZE: u32 = 512;
const MAX_NOTES_SIZE: u32 = 1024;

#[contractimpl]
impl PaymentContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn create_payment(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        expiration_duration: u64,
        metadata: String,
    ) -> Result<u64, Error> {
        customer.require_auth();

        // Validate currency
        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::InvalidCurrency);
        }

        // Validate metadata size
        if metadata.len() > MAX_METADATA_SIZE {
            return Err(Error::MetadataTooLarge);
        }

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
            merchant: merchant.clone(),
            amount,
            token,
            currency,
            status: PaymentStatus::Pending,
            created_at: current_timestamp,
            expires_at,
            metadata,
            notes: String::from_str(&env, ""),
        };

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);
        env.storage()
            .instance()
            .set(&DataKey::PaymentCounter, &payment_id);

        // Index by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerPaymentCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerPayments(customer.clone(), customer_count),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerPaymentCount(customer),
            &(customer_count + 1),
        );

        // Index by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantPaymentCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantPayments(merchant.clone(), merchant_count),
            &payment_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantPaymentCount(merchant),
            &(merchant_count + 1),
        );

        Ok(payment_id)
    }

    pub fn get_payment(env: &Env, payment_id: u64) -> Payment {
        env.storage()
            .instance()
            .get(&DataKey::Payment(payment_id))
            .expect("Payment not found")
    }

    pub fn update_payment_notes(
        env: Env,
        merchant: Address,
        payment_id: u64,
        notes: String,
    ) -> Result<(), Error> {
        merchant.require_auth();

        // Validate notes size
        if notes.len() > MAX_NOTES_SIZE {
            return Err(Error::NotesTooLarge);
        }

        // Check if payment exists
        if !env.storage().instance().has(&DataKey::Payment(payment_id)) {
            return Err(Error::PaymentNotFound);
        }

        let mut payment = PaymentContract::get_payment(&env, payment_id);

        // Verify caller is the merchant
        if payment.merchant != merchant {
            return Err(Error::Unauthorized);
        }

        // Update notes
        payment.notes = notes;

        // Save updated payment
        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        Ok(())
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

        // Verify caller is the legitimate admin
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

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

        // token transfer from customer to merchant
        let token_client = token::Client::new(&env, &payment.token);
        let contract_address = env.current_contract_address();

        token_client.transfer_from(
            &contract_address,
            &payment.customer,
            &payment.merchant,
            &payment.amount,
        );

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);

        PaymentCompleted {
            payment_id,
            merchant: payment.merchant,
            amount: payment.amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn refund_payment(env: Env, admin: Address, payment_id: u64) -> Result<(), Error> {
        admin.require_auth();

        // Verify caller is the legitimate admin
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

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
        }
        .publish(&env);

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
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_payments_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Payment> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerPaymentCount(customer.clone()))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::CustomerPayments(customer.clone(), i))
            {
                if let Some(payment) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Payment>(&DataKey::Payment(payment_id))
                {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    pub fn get_payment_count_by_customer(env: Env, customer: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::CustomerPaymentCount(customer))
            .unwrap_or(0)
    }

    pub fn get_payments_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Payment> {
        let total_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantPaymentCount(merchant.clone()))
            .unwrap_or(0);

        let mut payments = Vec::new(&env);
        let start = offset;
        let end = (offset + limit).min(total_count);

        for i in start..end {
            if let Some(payment_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::MerchantPayments(merchant.clone(), i))
            {
                if let Some(payment) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Payment>(&DataKey::Payment(payment_id))
                {
                    payments.push_back(payment);
                }
            }
        }

        payments
    }

    pub fn get_payment_count_by_merchant(env: Env, merchant: Address) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MerchantPaymentCount(merchant))
            .unwrap_or(0)
    }

    fn is_valid_currency(currency: &Currency) -> bool {
        matches!(currency, Currency::XLM | Currency::USDC | Currency::USDT | Currency::BTC | Currency::ETH)
    }

    pub fn set_conversion_rate(env: Env, admin: Address, currency: Currency, rate: i128) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::InvalidCurrency);
        }

        env.storage()
            .instance()
            .set(&DataKey::ConversionRate(currency), &rate);

        Ok(())
    }

    pub fn get_conversion_rate(env: Env, currency: Currency) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::ConversionRate(currency))
            .unwrap_or(1_0000000)
    }
}

mod test;
