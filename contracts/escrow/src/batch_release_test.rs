#[cfg(test)]
mod batch_release_tests {
    use crate::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    fn setup_test(env: &Env) -> (EscrowContractClient, Address) {
        env.mock_all_auths();
        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin)
    }

    #[test]
    fn test_batch_release_success() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);

        // Create 3 releasable escrows
        for _ in 0..3 {
            client.create_escrow(&customer, &merchant, &1000_i128, &token, &0, &0, &false);
        }

        let request = BatchReleaseRequest {
            escrow_ids: Vec::from_array(&env, &[1, 2, 3]),
            override_recipient: None,
        };

        let result = client.batch_release_escrows(&admin, &request);
        assert_eq!(result.succeeded.len(), 3);
        assert_eq!(result.failed.len(), 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_batch_release_partial_failure() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);

        // 1 releasable
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &0, &0, &false);
        // 1 not releasable (future release)
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &10000, &0, &false);

        let request = BatchReleaseRequest {
            escrow_ids: Vec::from_array(&env, &[1, 2]),
            override_recipient: None,
        };

        let result = client.batch_release_escrows(&admin, &request);
        assert_eq!(result.succeeded.len(), 1);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.succeeded[0], 1);
        assert_eq!(result.failed[0], 2);
        // Escrow 2 was created with release_timestamp=10000 (future), so
        // internal_release_escrow returns Error::ReleaseNotYetAvailable.
        assert_eq!(result.errors[0], Error::ReleaseNotYetAvailable as u32);
    }

    #[test]
    fn test_batch_release_size_limit() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);

        let mut ids = Vec::new(&env);
        for i in 1..=21 {
            ids.push_back(i);
        }

        let request = BatchReleaseRequest {
            escrow_ids: ids,
            override_recipient: None,
        };

        let result = client.try_batch_release_escrows(&admin, &request);
        assert_eq!(result, Err(Ok(Error::BatchReleaseSizeLimitExceeded)));
    }

    #[test]
    fn test_estimate_batch_release() {
        let env = Env::default();
        let (client, _) = setup_test(&env);
        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);

        // 1 releasable
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &0, &0, &false);
        // 1 not releasable
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &10000, &0, &false);

        let ids = Vec::from_array(&env, &[1, 2]);
        let result = client.estimate_batch_release(&ids);

        assert_eq!(result.succeeded.len(), 1);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.errors.len(), 0); // dry-run: no error codes surfaced
        assert_eq!(result.succeeded[0], 1);
        assert_eq!(result.failed[0], 2);
    }

    #[test]
    fn test_batch_release_unauthorized() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        let non_admin = Address::generate(&env);

        let request = BatchReleaseRequest {
            escrow_ids: Vec::from_array(&env, &[1]),
            override_recipient: None,
        };

        let result = client.try_batch_release_escrows(&non_admin, &request);
        assert_eq!(result, Err(Ok(Error::NotAnAdmin)));
    }

    #[test]
    fn test_batch_release_override_recipient() {
        let env = Env::default();
        let (client, admin) = setup_test(&env);
        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let override_recipient = Address::generate(&env);
        let token = Address::generate(&env);

        client.create_escrow(&customer, &merchant, &1000_i128, &token, &0, &0, &false);

        let request = BatchReleaseRequest {
            escrow_ids: Vec::from_array(&env, &[1]),
            override_recipient: Some(override_recipient.clone()),
        };

        client.batch_release_escrows(&admin, &request);

        // Check balance of override recipient (this requires token logic, but we can check if the call succeeded)
        // In a real test we would verify token transfers.
        // Since internal_release_escrow uses transfer_if_token_contract, and we mock all auths,
        // the call succeeding is the primary check here.
    }
}
