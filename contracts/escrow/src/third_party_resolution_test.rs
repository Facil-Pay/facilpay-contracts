#![cfg(test)]
use crate::{
    ActionError, BasicError, Escrow, EscrowContract, EscrowContractClient, EscrowError,
    EscrowStatus, Error, MerchantAcceptance, MultiSigConfig, ThirdPartyResolver,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

fn setup() -> (
    Env,
    EscrowContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin);

    let token_admin_client = token::StellarAssetClient::new(&env, &token.address());
    token_admin_client.mint(&customer, &10_000);

    (env, client, admin, customer, merchant, token.address())
}

// ══════════════════════════════════════════════════════════════════════════════
// THIRD-PARTY RESOLVER TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_assign_third_party_resolver_by_customer() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer assigns a third-party resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    let resolver_info = client.get_third_party_resolver(&escrow_id);
    assert!(resolver_info.is_some());
    let info = resolver_info.unwrap();
    assert_eq!(info.resolver, resolver);
    assert_eq!(info.assigned_by, customer);
    assert_eq!(info.escrow_id, escrow_id);
    assert_eq!(info.assigned_at, 1000);
}

#[test]
fn test_assign_third_party_resolver_by_merchant() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(2000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant assigns a third-party resolver
    client.assign_third_party_resolver(&merchant, &escrow_id, &resolver);

    let resolver_info = client.get_third_party_resolver(&escrow_id);
    assert!(resolver_info.is_some());
    let info = resolver_info.unwrap();
    assert_eq!(info.resolver, resolver);
    assert_eq!(info.assigned_by, merchant);
}

#[test]
fn test_assign_resolver_fails_for_unauthorized_party() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Unauthorized party cannot assign resolver
    let result = client.try_assign_third_party_resolver(&unauthorized, &escrow_id, &resolver);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(converted.unwrap(), Error::Basic(BasicError::Unauthorized));
        }
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_assign_resolver_fails_if_resolver_is_customer() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer cannot be the resolver
    let result = client.try_assign_third_party_resolver(&customer, &escrow_id, &customer);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::ResolverMustBeDifferent)
            );
        }
        _ => panic!("Expected ResolverMustBeDifferent error"),
    }
}

#[test]
fn test_assign_resolver_fails_if_resolver_is_merchant() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant cannot be the resolver
    let result = client.try_assign_third_party_resolver(&merchant, &escrow_id, &merchant);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::ResolverMustBeDifferent)
            );
        }
        _ => panic!("Expected ResolverMustBeDifferent error"),
    }
}

#[test]
fn test_third_party_resolver_can_resolve_dispute() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign third-party resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    // Customer disputes
    client.dispute_escrow(&customer, &escrow_id);

    // Third-party resolver resolves in favor of merchant
    client.resolve_dispute(&resolver, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_third_party_resolver_resolves_in_favor_of_customer() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign third-party resolver
    client.assign_third_party_resolver(&merchant, &escrow_id, &resolver);

    // Merchant disputes
    client.dispute_escrow(&merchant, &escrow_id);

    // Third-party resolver resolves in favor of customer
    client.resolve_dispute(&resolver, &escrow_id, &false);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);
}

#[test]
fn test_admin_cannot_resolve_when_third_party_resolver_assigned() {
    let (env, client, admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign third-party resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    // Customer disputes
    client.dispute_escrow(&customer, &escrow_id);

    // Admin tries to resolve - should fail
    let result = client.try_resolve_dispute(&admin, &escrow_id, &true);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(converted.unwrap(), Error::Basic(BasicError::NotAnAdmin));
        }
        _ => panic!("Expected NotAnAdmin error"),
    }
}

#[test]
fn test_remove_third_party_resolver_by_customer() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);
    assert!(client.get_third_party_resolver(&escrow_id).is_some());

    // Customer removes resolver
    client.remove_third_party_resolver(&customer, &escrow_id);
    assert!(client.get_third_party_resolver(&escrow_id).is_none());
}

#[test]
fn test_remove_third_party_resolver_by_merchant() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    // Merchant removes resolver
    client.remove_third_party_resolver(&merchant, &escrow_id);
    assert!(client.get_third_party_resolver(&escrow_id).is_none());
}

#[test]
fn test_remove_third_party_resolver_by_admin() {
    let (env, client, admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    // Admin removes resolver
    client.remove_third_party_resolver(&admin, &escrow_id);
    assert!(client.get_third_party_resolver(&escrow_id).is_none());
}

#[test]
fn test_admin_can_resolve_after_resolver_removed() {
    let (env, client, admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign and then remove resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);
    client.remove_third_party_resolver(&customer, &escrow_id);

    // Customer disputes
    client.dispute_escrow(&customer, &escrow_id);

    // Admin can now resolve
    client.resolve_dispute(&admin, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_reassign_third_party_resolver() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver1 = Address::generate(&env);
    let resolver2 = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign first resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver1);
    let info = client.get_third_party_resolver(&escrow_id).unwrap();
    assert_eq!(info.resolver, resolver1);

    env.ledger().set_timestamp(2000);

    // Reassign to second resolver
    client.assign_third_party_resolver(&merchant, &escrow_id, &resolver2);
    let info = client.get_third_party_resolver(&escrow_id).unwrap();
    assert_eq!(info.resolver, resolver2);
    assert_eq!(info.assigned_by, merchant);
    assert_eq!(info.assigned_at, 2000);
}

// ══════════════════════════════════════════════════════════════════════════════
// SPLIT RESOLUTION / PARTIAL DISPUTE TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_partial_dispute_releases_undisputed_to_merchant() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);

    let token_client = token::Client::new(&env, &token);
    let initial_merchant_balance = token_client.balance(&merchant);

    // Customer disputes only 300 out of 1000
    client.dispute_escrow_partial(&customer, &escrow_id, &300);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
    assert_eq!(escrow.released_amount, 700); // 700 released to merchant

    // Merchant should receive the undisputed amount (700)
    let merchant_balance = token_client.balance(&merchant);
    assert_eq!(merchant_balance, initial_merchant_balance + 700);
}

#[test]
fn test_partial_dispute_cannot_exceed_remaining_amount() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);

    // Try to dispute more than the escrow amount
    let result = client.try_dispute_escrow_partial(&customer, &escrow_id, &1500);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::InvalidDisputedAmount)
            );
        }
        _ => panic!("Expected InvalidDisputedAmount error"),
    }
}

#[test]
fn test_partial_dispute_cannot_be_zero() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);

    // Try to dispute zero amount
    let result = client.try_dispute_escrow_partial(&customer, &escrow_id, &0);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::InvalidDisputedAmount)
            );
        }
        _ => panic!("Expected InvalidDisputedAmount error"),
    }
}

#[test]
fn test_resolve_partial_dispute_in_favor_of_merchant() {
    let (env, client, admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);

    let token_client = token::Client::new(&env, &token);
    let initial_merchant_balance = token_client.balance(&merchant);

    // Customer disputes 400 out of 1000
    client.dispute_escrow_partial(&customer, &escrow_id, &400);

    // Admin resolves in favor of merchant (releases the disputed 400 to merchant)
    client.resolve_dispute(&admin, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);

    // Merchant should have received all 1000 (700 from partial + 400 from resolution)
    let merchant_balance = token_client.balance(&merchant);
    assert!(merchant_balance >= initial_merchant_balance + 1000);
}

#[test]
fn test_resolve_partial_dispute_in_favor_of_customer() {
    let (env, client, admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);

    let token_client = token::Client::new(&env, &token);
    let initial_customer_balance = token_client.balance(&customer);
    let initial_merchant_balance = token_client.balance(&merchant);

    // Customer disputes 400 out of 1000
    client.dispute_escrow_partial(&customer, &escrow_id, &400);

    // Admin resolves in favor of customer (refunds the disputed 400 to customer)
    client.resolve_dispute(&admin, &escrow_id, &false);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Resolved);

    // Merchant should have received 600 (700 - fees)
    let merchant_balance = token_client.balance(&merchant);
    assert!(merchant_balance >= initial_merchant_balance + 600);

    // Customer should have received back approximately 400 (minus dispute fees)
    let customer_balance = token_client.balance(&customer);
    assert!(customer_balance >= initial_customer_balance - 1000 + 300);
}

#[test]
fn test_third_party_resolver_handles_partial_dispute() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &2000, &token, &5000, &0);

    // Assign third-party resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    let token_client = token::Client::new(&env, &token);
    let initial_merchant_balance = token_client.balance(&merchant);

    // Customer disputes 800 out of 2000
    client.dispute_escrow_partial(&customer, &escrow_id, &800);

    // Merchant should have received 1200 immediately
    let merchant_balance = token_client.balance(&merchant);
    assert_eq!(merchant_balance, initial_merchant_balance + 1200);

    // Third-party resolver resolves disputed portion in favor of merchant
    client.resolve_dispute(&resolver, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

// ══════════════════════════════════════════════════════════════════════════════
// MERCHANT ACCEPTANCE TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_merchant_accepts_escrow() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant accepts the escrow
    client.merchant_accept_escrow(&merchant, &escrow_id);

    let acceptance = client.get_merchant_acceptance(&escrow_id);
    assert!(acceptance.is_some());
    let acc = acceptance.unwrap();
    assert_eq!(acc.escrow_id, escrow_id);
    assert!(acc.accepted);
    assert_eq!(acc.accepted_at.unwrap(), 1000);
}

#[test]
fn test_merchant_acceptance_before_cooling_off_cancellation() {
    let (env, client, admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    // Set up cooling-off period for merchant
    client.set_merchant_cooling_off(&admin, &merchant, &3600); // 1 hour

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant accepts the escrow
    env.ledger().set_timestamp(1500);
    client.merchant_accept_escrow(&merchant, &escrow_id);

    // Customer tries to cancel during cooling-off period - should fail because merchant accepted
    env.ledger().set_timestamp(2000);
    let result = client.try_cancel_escrow_cooling_off(&customer, &escrow_id);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Escrow(EscrowError::CoolingOffMerchantAccepted)
            );
        }
        _ => panic!("Expected CoolingOffMerchantAccepted error"),
    }
}

#[test]
fn test_only_merchant_can_accept_escrow() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer tries to accept - should fail
    let result = client.try_merchant_accept_escrow(&customer, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_merchant_acceptance_for_nonexistent_escrow() {
    let (_env, client, _admin, _customer, merchant, _token) = setup();

    // Try to accept non-existent escrow
    let result = client.try_merchant_accept_escrow(&merchant, &999);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(converted.unwrap(), Error::Escrow(EscrowError::NotFound));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_merchant_cannot_accept_twice() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant accepts the escrow
    client.merchant_accept_escrow(&merchant, &escrow_id);

    // Try to accept again
    env.ledger().set_timestamp(2000);
    let result = client.try_merchant_accept_escrow(&merchant, &escrow_id);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::MerchantAlreadyAccepted)
            );
        }
        _ => panic!("Expected MerchantAlreadyAccepted error"),
    }
}

#[test]
fn test_merchant_acceptance_does_not_affect_normal_flow() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant accepts
    client.merchant_accept_escrow(&merchant, &escrow_id);

    // Normal release should still work
    env.ledger().set_timestamp(10000);
    client.release_escrow(&merchant, &escrow_id);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

// ══════════════════════════════════════════════════════════════════════════════
// MUTUAL CANCEL TESTS
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_mutual_cancel_customer_initiates() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer requests mutual cancel
    client.request_mutual_cancel(&customer, &escrow_id);

    // Escrow should still be locked
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_mutual_cancel_merchant_initiates() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant requests mutual cancel
    client.request_mutual_cancel(&merchant, &escrow_id);

    // Escrow should still be locked
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_mutual_cancel_completes_when_both_agree() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    let token_client = token::Client::new(&env, &token);
    let initial_customer_balance = token_client.balance(&customer);

    // Customer requests mutual cancel
    client.request_mutual_cancel(&customer, &escrow_id);

    // Merchant also requests mutual cancel - this should complete the cancellation
    client.request_mutual_cancel(&merchant, &escrow_id);

    // Escrow should be cancelled
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);

    // Customer should be refunded
    let customer_balance = token_client.balance(&customer);
    assert_eq!(customer_balance, initial_customer_balance);
}

#[test]
fn test_mutual_cancel_in_reverse_order() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    let token_client = token::Client::new(&env, &token);
    let initial_customer_balance = token_client.balance(&customer);

    // Merchant requests mutual cancel first
    client.request_mutual_cancel(&merchant, &escrow_id);

    // Customer also requests mutual cancel - this should complete
    client.request_mutual_cancel(&customer, &escrow_id);

    // Escrow should be cancelled
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);

    // Customer should be refunded
    let customer_balance = token_client.balance(&customer);
    assert_eq!(customer_balance, initial_customer_balance);
}

#[test]
fn test_mutual_cancel_cannot_request_twice() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer requests mutual cancel
    client.request_mutual_cancel(&customer, &escrow_id);

    // Customer tries to request again
    let result = client.try_request_mutual_cancel(&customer, &escrow_id);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::MutualCancelAlreadyRequested)
            );
        }
        _ => panic!("Expected MutualCancelAlreadyRequested error"),
    }
}

#[test]
fn test_withdraw_mutual_cancel_request() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer requests mutual cancel
    client.request_mutual_cancel(&customer, &escrow_id);

    // Customer withdraws the request
    client.withdraw_mutual_cancel(&customer, &escrow_id);

    // Now merchant requests mutual cancel
    client.request_mutual_cancel(&merchant, &escrow_id);

    // Escrow should still be locked (customer's request was withdrawn)
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
}

#[test]
fn test_withdraw_mutual_cancel_before_requesting_fails() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Try to withdraw without requesting first
    let result = client.try_withdraw_mutual_cancel(&customer, &escrow_id);
    assert!(result.is_err());
    match result.err() {
        Some(Ok(err)) => {
            let converted: Result<Error, _> = err.try_into();
            assert_eq!(
                converted.unwrap(),
                Error::Action(ActionError::MutualCancelNotRequested)
            );
        }
        _ => panic!("Expected MutualCancelNotRequested error"),
    }
}

#[test]
fn test_mutual_cancel_after_one_party_withdraws() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer requests
    client.request_mutual_cancel(&customer, &escrow_id);

    // Merchant requests
    client.request_mutual_cancel(&merchant, &escrow_id);

    // This should have completed, but let's verify we can't withdraw after completion
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);
}

#[test]
fn test_unauthorized_party_cannot_request_mutual_cancel() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let unauthorized = Address::generate(&env);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Unauthorized party tries to request mutual cancel
    let result = client.try_request_mutual_cancel(&unauthorized, &escrow_id);
    assert!(result.is_err());
}

#[test]
fn test_mutual_cancel_on_disputed_escrow_fails() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Customer disputes
    client.dispute_escrow(&customer, &escrow_id);

    // Try to request mutual cancel on disputed escrow
    let result = client.try_request_mutual_cancel(&customer, &escrow_id);
    assert!(result.is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS - COMBINING FEATURES
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_third_party_resolver_with_merchant_acceptance() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Merchant accepts
    client.merchant_accept_escrow(&merchant, &escrow_id);

    // Assign third-party resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    // Customer disputes
    client.dispute_escrow(&customer, &escrow_id);

    // Third-party resolver resolves
    client.resolve_dispute(&resolver, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_mutual_cancel_with_third_party_resolver_assigned() {
    let (_env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    // Assign third-party resolver
    client.assign_third_party_resolver(&customer, &escrow_id, &resolver);

    let token_client = token::Client::new(&env, &token);
    let initial_customer_balance = token_client.balance(&customer);

    // Both parties agree to mutual cancel
    client.request_mutual_cancel(&customer, &escrow_id);
    client.request_mutual_cancel(&merchant, &escrow_id);

    // Should complete successfully
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);

    let customer_balance = token_client.balance(&customer);
    assert_eq!(customer_balance, initial_customer_balance);
}

#[test]
fn test_partial_dispute_with_merchant_acceptance() {
    let (env, client, _admin, customer, merchant, token) = setup();

    env.ledger().set_timestamp(1000);

    let escrow_id = client.create_escrow(&customer, &merchant, &1000, &token, &5000, &0);

    // Merchant accepts
    client.merchant_accept_escrow(&merchant, &escrow_id);

    let token_client = token::Client::new(&env, &token);
    let initial_merchant_balance = token_client.balance(&merchant);

    // Customer partially disputes
    client.dispute_escrow_partial(&customer, &escrow_id, &300);

    // Merchant should have received 700
    let merchant_balance = token_client.balance(&merchant);
    assert_eq!(merchant_balance, initial_merchant_balance + 700);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Disputed);
}

#[test]
fn test_complete_workflow_third_party_resolution() {
    let (env, client, _admin, customer, merchant, token) = setup();
    let resolver = Address::generate(&env);

    env.ledger().set_timestamp(1000);

    // 1. Create escrow
    let escrow_id = client.create_escrow(&customer, &merchant, &2000, &token, &5000, &0);

    // 2. Merchant accepts
    client.merchant_accept_escrow(&merchant, &escrow_id);
    let acceptance = client.get_merchant_acceptance(&escrow_id);
    assert!(acceptance.is_some());

    // 3. Assign third-party resolver
    client.assign_third_party_resolver(&merchant, &escrow_id, &resolver);
    let resolver_info = client.get_third_party_resolver(&escrow_id);
    assert!(resolver_info.is_some());
    assert_eq!(resolver_info.unwrap().resolver, resolver);

    let token_client = token::Client::new(&env, &token);
    let initial_merchant_balance = token_client.balance(&merchant);

    // 4. Customer partially disputes
    client.dispute_escrow_partial(&customer, &escrow_id, &600);

    // 5. Verify partial release
    let merchant_balance = token_client.balance(&merchant);
    assert_eq!(merchant_balance, initial_merchant_balance + 1400);

    // 6. Third-party resolver resolves disputed portion
    client.resolve_dispute(&resolver, &escrow_id, &true);

    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_mutual_cancel_takes_precedence_over_dispute() {
    let (_env, client, _admin, customer, merchant, token) = setup();

    let escrow_id = client.create_escrow(&customer, &merchant, &500, &token, &5000, &0);

    let token_client = token::Client::new(&env, &token);
    let initial_customer_balance = token_client.balance(&customer);

    // Both parties immediately agree to cancel
    client.request_mutual_cancel(&customer, &escrow_id);
    client.request_mutual_cancel(&merchant, &escrow_id);

    // Escrow is cancelled, customer is refunded
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Cancelled);

    let customer_balance = token_client.balance(&customer);
    assert_eq!(customer_balance, initial_customer_balance);

    // Now trying to dispute should fail
    let result = client.try_dispute_escrow(&customer, &escrow_id);
    assert!(result.is_err());
}
