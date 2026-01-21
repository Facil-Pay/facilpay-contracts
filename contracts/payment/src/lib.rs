#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

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
    ) -> u64 {
        customer.require_auth();

        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentCounter)
            .unwrap_or(0);
        let payment_id = counter + 1;

        let payment = Payment {
            id: payment_id,
            customer: customer.clone(),
            merchant,
            amount,
            token,
            status: PaymentStatus::Pending,
            created_at: env.ledger().timestamp(),
        };

        env.storage()
            .instance()
            .set(&DataKey::Payment(payment_id), &payment);
        env.storage()
            .instance()
            .set(&DataKey::PaymentCounter, &payment_id);

        payment_id
    }

    pub fn get_payment(env: Env, payment_id: u64) -> Payment {
        env.storage()
            .instance()
            .get(&DataKey::Payment(payment_id))
            .expect("Payment not found")
    }
}

mod test;
