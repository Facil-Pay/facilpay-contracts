#![cfg(test)]
use super::*;
use soroban_sdk::{Address, Env, Vec};

// Mock payment contract to avoid complex dependencies in policy tests
#[contract]
pub struct MockPaymentContract;

#[contractimpl]
impl MockPaymentContract {
    pub fn get_payment(env: Env, id: u64) -> ExternalPayment {
        let created_at = env.storage().instance().get(&id).unwrap_or(0);
        ExternalPayment {
            id,
            customer: Address::generate(&env),
            merchant: Address::generate(&env),
            amount: 1000,
            token: Address::generate(&env),
            currency: ExternalCurrency::USDC,
            status: ExternalPaymentStatus::Completed,
            created_at,
            expires_at: created_at + 86400,
            metadata: String::from_str(&env, ""),
            notes: String::from_str(&env, ""),
            refunded_amount: 0,
        }
    }
}

fn setup_test(env: &Env) -> (RefundContractClient, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(env, &contract_id);
    client.initialize(&admin);

    // Setup mock payment contract
    let payment_contract_id = env.register(MockPaymentContract, ());
    let payment_contract_addr = env.get_contract_address(&payment_contract_id);
    env.storage().instance().set(&DataKey::PaymentContractAddress, &payment_contract_addr);

    (client, admin)
}

#[test]
fn test_tier_selection_logic() {
    let env = Env::default();
    let (client, _) = setup_test(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();

    // 7 days (100%), 30 days (50%), 60 days (25%)
    let tiers = Vec::from_array(&env, &[
        RefundTier { days_from_purchase: 7, max_refund_bps: 10000 },
        RefundTier { days_from_purchase: 30, max_refund_bps: 5000 },
        RefundTier { days_from_purchase: 60, max_refund_bps: 2500 },
    ]);
    client.set_refund_policy(&merchant, &tiers);

    let payment_id = 1u64;

    // Case 1: Day 6 (should be 100% - Tier 1)
    env.ledger().set_timestamp(1000 * 24 * 60 * 60); // current time
    env.storage().instance().set(&payment_id, &(1000 * 24 * 60 * 60 - 6 * 24 * 60 * 60)); // 6 days ago
    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 10000);

    // Case 2: Day 8 (should be 50% - Tier 2)
    env.storage().instance().set(&payment_id, &(1000 * 24 * 60 * 60 - 8 * 24 * 60 * 60)); // 8 days ago
    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 5000);

    // Case 3: Day 31 (should be 25% - Tier 3)
    env.storage().instance().set(&payment_id, &(1000 * 24 * 60 * 60 - 31 * 24 * 60 * 60)); // 31 days ago
    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 2500);

    // Case 4: Day 61 (should be 0% - No match)
    env.storage().instance().set(&payment_id, &(1000 * 24 * 60 * 60 - 61 * 24 * 60 * 60)); // 61 days ago
    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 0);
}

#[test]
fn test_zero_tiers_rejection() {
    let env = Env::default();
    let (client, _) = setup_test(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();

    let tiers = Vec::new(&env);
    client.set_refund_policy(&merchant, &tiers);

    let payment_id = 1u64;
    env.storage().instance().set(&payment_id, &0u64);

    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 0);
}

#[test]
fn test_tier_update_immediate_effect() {
    let env = Env::default();
    let (client, _) = setup_test(&env);
    let merchant = Address::generate(&env);
    env.mock_all_auths();

    let payment_id = 1u64;
    env.storage().instance().set(&payment_id, &(1000 * 24 * 60 * 60 - 10 * 24 * 60 * 60)); // 10 days ago

    // Policy 1: 100% up to 7 days, 50% up to 30 days
    let tiers1 = Vec::from_array(&env, &[
        RefundTier { days_from_purchase: 7, max_refund_bps: 10000 },
        RefundTier { days_from_purchase: 30, max_refund_bps: 5000 },
    ]);
    client.set_refund_policy(&merchant, &tiers1);
    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 5000);

    // Update Policy 2: Change 30-day tier to 20%
    let tiers2 = Vec::from_array(&env, &[
        RefundTier { days_from_purchase: 7, max_refund_bps: 10000 },
        RefundTier { days_from_purchase: 30, max_refund_bps: 2000 },
    ]);
    client.set_refund_policy(&merchant, &tiers2);
    assert_eq!(client.get_applicable_refund_bps(&merchant, &payment_id), 2000);
}
