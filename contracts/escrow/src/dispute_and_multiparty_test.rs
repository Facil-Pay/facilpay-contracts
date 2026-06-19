#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env, Vec};

fn setup_client(env: &Env) -> (EscrowContractClient<'static>, Address) {
    env.mock_all_auths();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    (client, admin)
}

fn participant(
    address: Address,
    role: ParticipantRole,
    share_bps: u32,
    weight_bps: u32,
) -> Participant {
    Participant {
        address,
        role,
        share_bps,
        weight_bps,
        approved: false,
        approved_at: None,
    }
}

fn three_party_participants(
    env: &Env,
    customer: &Address,
    merchant: &Address,
    service_provider: &Address,
    merchant_share_bps: u32,
    service_provider_share_bps: u32,
) -> Vec<Participant> {
    Vec::from_array(
        env,
        [
            participant(customer.clone(), ParticipantRole::Customer, 0, 0),
            participant(
                merchant.clone(),
                ParticipantRole::Merchant,
                merchant_share_bps,
                6000,
            ),
            participant(
                service_provider.clone(),
                ParticipantRole::ServiceProvider,
                service_provider_share_bps,
                4000,
            ),
        ],
    )
}

#[test]
fn resolve_dispute_rejects_non_admin_and_allows_admin() {
    let env = Env::default();
    let (client, admin) = setup_client(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let token = Address::generate(&env);

    let escrow_id = client.create_escrow(
        &customer, &merchant, &1000_i128, &token, &1000_u64, &0_u64, &0_u64, &false,
    );
    client.dispute_escrow(&customer, &escrow_id);

    let non_admin_result = client.try_resolve_dispute(&non_admin, &escrow_id, &true);
    assert_eq!(non_admin_result, Err(Ok(Error::NotAnAdmin)));

    client.resolve_dispute(&admin, &escrow_id, &true);
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn shares_sum_to_9500_is_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_client(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let service_provider = Address::generate(&env);
    let token = Address::generate(&env);

    let participants =
        three_party_participants(&env, &customer, &merchant, &service_provider, 5000, 4500);

    let result =
        client.try_create_multi_party_escrow(&customer, &participants, &1000_i128, &token, &1000);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn shares_sum_to_10500_is_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_client(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let service_provider = Address::generate(&env);
    let token = Address::generate(&env);

    let participants =
        three_party_participants(&env, &customer, &merchant, &service_provider, 6000, 4500);

    let result =
        client.try_create_multi_party_escrow(&customer, &participants, &1000_i128, &token, &1000);
    assert_eq!(result, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn shares_sum_to_10000_releases_correctly() {
    let env = Env::default();
    let (client, admin) = setup_client(&env);
    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let service_provider = Address::generate(&env);

    let token_id = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);
    token_admin.mint(&customer, &1000_i128);

    let participants =
        three_party_participants(&env, &customer, &merchant, &service_provider, 6000, 4000);

    let escrow_id = client.create_multi_party_escrow(
        &customer,
        &participants,
        &1000_i128,
        &token_id,
        &1000_u64,
    );

    client.approve_release(&merchant, &escrow_id);
    client.approve_release(&service_provider, &escrow_id);
    env.ledger().set_timestamp(1000);
    client.release_multi_party_escrow(&escrow_id);

    let token_client = token::Client::new(&env, &token_id);
    assert_eq!(token_client.balance(&merchant), 600_i128);
    assert_eq!(token_client.balance(&service_provider), 400_i128);

    let escrow = client.get_multi_party_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}
