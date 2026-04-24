#[cfg(test)]
mod timelock_tests {
    use crate::*;
    use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

    fn setup_test(env: &Env) -> (EscrowContractClient, Address) {
        env.mock_all_auths();
        let contract_id = env.register(EscrowContract, ());
        let client = EscrowContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin);
        (client, admin)
    }

    #[test]
    fn test_queue_action() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let escrow_id = 1u64;
        let action_type = EscrowActionType::ResolveDispute(true);
        let data = Bytes::new(&env);

        // We need to create an escrow first so it exists in storage
        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

        let action_id = client.queue_action(
            &admin,
            &escrow_id,
            &action_type,
            &data,
        );

        assert_eq!(action_id, 1);

        let queued_action = client.get_queued_action(&action_id);
        assert_eq!(queued_action.escrow_id, escrow_id);
        assert!(!queued_action.executed);
        assert!(!queued_action.cancelled);
    }

    #[test]
    fn test_execute_action_too_early() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let escrow_id = 1u64;
        let action_type = EscrowActionType::ResolveDispute(true);
        let data = Bytes::new(&env);

        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);
        client.dispute_escrow(&customer, &escrow_id);

        let action_id = client.queue_action(
            &admin,
            &escrow_id,
            &action_type,
            &data,
        );

        // Try to execute immediately - should fail
        let result = client.try_execute_queued_action(&action_id);
        assert_eq!(result, Err(Ok(Error::ActionNotReady)));
    }

    #[test]
    fn test_cancel_queued_action() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let escrow_id = 1u64;
        let action_type = EscrowActionType::ForceRelease;
        let data = Bytes::new(&env);

        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);
        client.create_escrow(&customer, &merchant, &1000_i128, &token, &5000_u64, &0_u64);

        let action_id = client.queue_action(
            &admin,
            &escrow_id,
            &action_type,
            &data,
        );

        client.cancel_queued_action(&admin, &action_id);

        let queued_action = client.get_queued_action(&action_id);
        assert!(queued_action.cancelled);
    }

    #[test]
    fn test_set_timelock_config() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let config = TimeLockConfig {
            delay: 7200,      // 2 hours
            grace_period: 3600, // 1 hour
        };

        client.set_timelock_config(&admin, &config);

        let stored_config = client.get_timelock_config();
        assert_eq!(stored_config.delay, 7200);
        assert_eq!(stored_config.grace_period, 3600);
    }

    #[test]
    fn test_invalid_timelock_delay() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin) = setup_test(&env);

        let config = TimeLockConfig {
            delay: 0,
            grace_period: 3600,
        };

        let result = client.try_set_timelock_config(&admin, &config);
        assert_eq!(result, Err(Ok(Error::InvalidStatus)));
    }
}
