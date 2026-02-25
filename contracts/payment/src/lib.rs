#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env,
    String, Vec,
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
    SubscriptionCounter,
    Subscription(u64),
    CustomerSubscriptions(Address, u64),
    CustomerSubscriptionCount(Address),
    MerchantSubscriptions(Address, u64),
    MerchantSubscriptionCount(Address),
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

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SubscriptionStatus {
    Active,
    Paused,
    Cancelled,
    Expired,
}

#[derive(Clone)]
#[contracttype]
pub struct Subscription {
    pub id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub token: Address,
    pub currency: Currency,
    pub interval: u64, // seconds between payments
    pub duration: u64, // total seconds the subscription lives (0 = indefinite)
    pub status: SubscriptionStatus,
    pub created_at: u64,
    pub next_payment_at: u64,
    pub ends_at: u64,       // 0 = no hard end
    pub payment_count: u64, // successful executions so far
    pub retry_count: u64,   // consecutive failed attempts on current cycle
    pub max_retries: u64,   // max retries before marking failed cycle skipped
    pub metadata: String,
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
    SubscriptionNotFound = 12,
    SubscriptionNotActive = 13,
    PaymentNotDue = 14,
    MaxRetriesExceeded = 15,
    SubscriptionEnded = 16,
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

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionCreated {
    pub subscription_id: u64,
    pub customer: Address,
    pub merchant: Address,
    pub amount: i128,
    pub interval: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecurringPaymentExecuted {
    pub subscription_id: u64,
    pub payment_count: u64,
    pub amount: i128,
    pub next_payment_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecurringPaymentFailed {
    pub subscription_id: u64,
    pub retry_count: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionCancelled {
    pub subscription_id: u64,
    pub cancelled_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionPaused {
    pub subscription_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionResumed {
    pub subscription_id: u64,
    pub next_payment_at: u64,
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
const DEFAULT_MAX_RETRIES: u64 = 3;

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
        matches!(
            currency,
            Currency::XLM | Currency::USDC | Currency::USDT | Currency::BTC | Currency::ETH
        )
    }

    pub fn set_conversion_rate(
        env: Env,
        admin: Address,
        currency: Currency,
        rate: i128,
    ) -> Result<(), Error> {
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

    // ── RECURRING / SUBSCRIPTION METHODS ────────────────────────────────────

    /// Create a new subscription. The customer authorises the creation.
    /// `interval`          – seconds between each automatic payment
    /// `duration`          – total lifetime in seconds (0 = indefinite)
    /// `max_retries`       – how many times to retry a failed cycle (0 uses DEFAULT)
    pub fn create_subscription(
        env: Env,
        customer: Address,
        merchant: Address,
        amount: i128,
        token: Address,
        currency: Currency,
        interval: u64,
        duration: u64,
        max_retries: u64,
        metadata: String,
    ) -> Result<u64, Error> {
        customer.require_auth();

        if !PaymentContract::is_valid_currency(&currency) {
            return Err(Error::InvalidCurrency);
        }
        if metadata.len() > MAX_METADATA_SIZE {
            return Err(Error::MetadataTooLarge);
        }

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SubscriptionCounter)
            .unwrap_or(0);
        let sub_id = counter + 1;

        let now = env.ledger().timestamp();
        let ends_at = if duration > 0 { now + duration } else { 0 };
        let retries = if max_retries == 0 {
            DEFAULT_MAX_RETRIES
        } else {
            max_retries
        };

        let sub = Subscription {
            id: sub_id,
            customer: customer.clone(),
            merchant: merchant.clone(),
            amount,
            token,
            currency,
            interval,
            duration,
            status: SubscriptionStatus::Active,
            created_at: now,
            next_payment_at: now + interval,
            ends_at,
            payment_count: 0,
            retry_count: 0,
            max_retries: retries,
            metadata,
        };

        env.storage()
            .instance()
            .set(&DataKey::Subscription(sub_id), &sub);
        env.storage()
            .instance()
            .set(&DataKey::SubscriptionCounter, &sub_id);

        // Index by customer
        let c_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerSubscriptionCount(customer.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::CustomerSubscriptions(customer.clone(), c_count),
            &sub_id,
        );
        env.storage().instance().set(
            &DataKey::CustomerSubscriptionCount(customer),
            &(c_count + 1),
        );

        // Index by merchant
        let m_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantSubscriptionCount(merchant.clone()))
            .unwrap_or(0);
        env.storage().instance().set(
            &DataKey::MerchantSubscriptions(merchant.clone(), m_count),
            &sub_id,
        );
        env.storage().instance().set(
            &DataKey::MerchantSubscriptionCount(merchant.clone()),
            &(m_count + 1),
        );

        SubscriptionCreated {
            subscription_id: sub_id,
            customer: sub.customer.clone(),
            merchant: sub.merchant.clone(),
            amount: sub.amount,
            interval: sub.interval,
        }
        .publish(&env);

        Ok(sub_id)
    }

    /// Execute the next recurring payment for a subscription.
    /// Anyone (typically an off-chain keeper / cron) may call this once the
    /// payment is due. It handles retry logic internally.
    pub fn execute_recurring_payment(env: Env, subscription_id: u64) -> Result<(), Error> {
        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        // Must be Active
        if sub.status != SubscriptionStatus::Active {
            return Err(Error::SubscriptionNotActive);
        }

        let now = env.ledger().timestamp();

        // Check subscription has not ended
        if sub.ends_at > 0 && now >= sub.ends_at {
            sub.status = SubscriptionStatus::Expired;
            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
            return Err(Error::SubscriptionEnded);
        }

        // Check payment is due
        if now < sub.next_payment_at {
            return Err(Error::PaymentNotDue);
        }

        // Attempt token transfer
        let token_client = token::Client::new(&env, &sub.token);
        let contract_address = env.current_contract_address();

        let transfer_ok = token_client
            .try_transfer_from(&contract_address, &sub.customer, &sub.merchant, &sub.amount)
            .is_ok();

        if transfer_ok {
            sub.payment_count += 1;
            sub.retry_count = 0;
            sub.next_payment_at = now + sub.interval;

            // Auto-expire when duration is reached
            if sub.ends_at > 0 && sub.next_payment_at >= sub.ends_at {
                sub.status = SubscriptionStatus::Expired;
            }

            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);

            RecurringPaymentExecuted {
                subscription_id,
                payment_count: sub.payment_count,
                amount: sub.amount,
                next_payment_at: sub.next_payment_at,
            }
            .publish(&env);
        } else {
            // Failed payment — apply retry logic
            sub.retry_count += 1;

            RecurringPaymentFailed {
                subscription_id,
                retry_count: sub.retry_count,
            }
            .publish(&env);

            if sub.retry_count >= sub.max_retries {
                // Exhausted retries: cancel subscription
                sub.status = SubscriptionStatus::Cancelled;
                env.storage()
                    .instance()
                    .set(&DataKey::Subscription(subscription_id), &sub);
                return Err(Error::MaxRetriesExceeded);
            }

            env.storage()
                .instance()
                .set(&DataKey::Subscription(subscription_id), &sub);
            return Err(Error::TransferFailed);
        }

        Ok(())
    }

    /// Cancel a subscription. Only the customer, merchant, or admin may call this.
    pub fn cancel_subscription(
        env: Env,
        caller: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        caller.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        let stored_admin: Option<Address> = env.storage().instance().get(&DataKey::Admin);

        let is_authorized = sub.customer == caller
            || sub.merchant == caller
            || stored_admin.map_or(false, |a| a == caller);

        if !is_authorized {
            return Err(Error::Unauthorized);
        }

        if sub.status == SubscriptionStatus::Cancelled || sub.status == SubscriptionStatus::Expired
        {
            return Err(Error::InvalidStatus);
        }

        sub.status = SubscriptionStatus::Cancelled;
        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        SubscriptionCancelled {
            subscription_id,
            cancelled_by: caller,
        }
        .publish(&env);

        Ok(())
    }

    /// Pause an active subscription. Only the customer may pause.
    pub fn pause_subscription(
        env: Env,
        customer: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        customer.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Unauthorized);
        }

        if sub.status != SubscriptionStatus::Active {
            return Err(Error::SubscriptionNotActive);
        }

        sub.status = SubscriptionStatus::Paused;
        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        SubscriptionPaused { subscription_id }.publish(&env);

        Ok(())
    }

    /// Resume a paused subscription. Resets `next_payment_at` from now.
    pub fn resume_subscription(
        env: Env,
        customer: Address,
        subscription_id: u64,
    ) -> Result<(), Error> {
        customer.require_auth();

        if !env
            .storage()
            .instance()
            .has(&DataKey::Subscription(subscription_id))
        {
            return Err(Error::SubscriptionNotFound);
        }

        let mut sub: Subscription = env
            .storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .unwrap();

        if sub.customer != customer {
            return Err(Error::Unauthorized);
        }

        if sub.status != SubscriptionStatus::Paused {
            return Err(Error::InvalidStatus);
        }

        let now = env.ledger().timestamp();
        sub.next_payment_at = now + sub.interval;
        sub.status = SubscriptionStatus::Active;

        env.storage()
            .instance()
            .set(&DataKey::Subscription(subscription_id), &sub);

        SubscriptionResumed {
            subscription_id,
            next_payment_at: sub.next_payment_at,
        }
        .publish(&env);

        Ok(())
    }

    /// Read a single subscription.
    pub fn get_subscription(env: Env, subscription_id: u64) -> Subscription {
        env.storage()
            .instance()
            .get(&DataKey::Subscription(subscription_id))
            .expect("Subscription not found")
    }

    /// Paginated list of subscriptions for a customer.
    pub fn get_subscriptions_by_customer(
        env: Env,
        customer: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Subscription> {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerSubscriptionCount(customer.clone()))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let end = (offset + limit).min(total);

        for i in offset..end {
            if let Some(sub_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::CustomerSubscriptions(customer.clone(), i))
            {
                if let Some(sub) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Subscription>(&DataKey::Subscription(sub_id))
                {
                    result.push_back(sub);
                }
            }
        }

        result
    }

    /// Paginated list of subscriptions for a merchant.
    pub fn get_subscriptions_by_merchant(
        env: Env,
        merchant: Address,
        limit: u64,
        offset: u64,
    ) -> Vec<Subscription> {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantSubscriptionCount(merchant.clone()))
            .unwrap_or(0);

        let mut result = Vec::new(&env);
        let end = (offset + limit).min(total);

        for i in offset..end {
            if let Some(sub_id) = env
                .storage()
                .instance()
                .get::<DataKey, u64>(&DataKey::MerchantSubscriptions(merchant.clone(), i))
            {
                if let Some(sub) = env
                    .storage()
                    .instance()
                    .get::<DataKey, Subscription>(&DataKey::Subscription(sub_id))
                {
                    result.push_back(sub);
                }
            }
        }

        result
    }
}

mod test;
