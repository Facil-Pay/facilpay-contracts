#[cfg(test)]
mod timelock_tests {
    use crate::*;
    use soroban_sdk::{testutils::Address as _, Address, Bytes, Env};

    #[test]
    fn test_queue_action() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        EscrowContract::initialize(env.clone(), admin.clone());

        let escrow_id = 1u64;
        let action_type = EscrowActionType::ResolveDispute(true);
        let data = Bytes::new(&env);

        let action_id = EscrowContract::queue_action(
            env.clone(),
            admin.clone(),
            escrow_id,
            action_type,
            data,
        ).unwrap();

        assert_eq!(action_id, 1);

        let queued_action = EscrowContract::get_queued_action(env.clone(), action_id).unwrap();
        assert_eq!(queued_action.escrow_id, escrow_id);
        assert!(!queued_action.executed);
        assert!(!queued_action.cancelled);
    }

    #[test]
    fn test_execute_action_too_early() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        EscrowContract::initialize(env.clone(), admin.clone());

        let escrow_id = 1u64;
        let action_type = EscrowActionType::ResolveDispute(true);
        let data = Bytes::new(&env);

        let action_id = EscrowContract::queue_action(
            env.clone(),
            admin.clone(),
            escrow_id,
            action_type,
            data,
        ).unwrap();

        // Try to execute immediately - should fail
        let result = EscrowContract::execute_queued_action(env.clone(), action_id);
        assert_eq!(result, Err(Error::ActionNotReady));
    }

    #[test]
    fn test_cancel_queued_action() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        EscrowContract::initialize(env.clone(), admin.clone());

        let escrow_id = 1u64;
        let action_type = EscrowActionType::ForceRelease;
        let data = Bytes::new(&env);

        let action_id = EscrowContract::queue_action(
            env.clone(),
            admin.clone(),
            escrow_id,
            action_type,
            data,
        ).unwrap();

        EscrowContract::cancel_queued_action(env.clone(), admin.clone(), action_id).unwrap();

        let queued_action = EscrowContract::get_queued_action(env.clone(), action_id).unwrap();
        assert!(queued_action.cancelled);
    }

    #[test]
    fn test_set_timelock_config() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        EscrowContract::initialize(env.clone(), admin.clone());

        let config = TimeLockConfig {
            delay: 7200,      // 2 hours
            grace_period: 3600, // 1 hour
        };

        EscrowContract::set_timelock_config(env.clone(), admin.clone(), config.clone()).unwrap();

        let stored_config = EscrowContract::get_timelock_config(env.clone());
        assert_eq!(stored_config.delay, config.delay);
        assert_eq!(stored_config.grace_period, config.grace_period);
    }

    #[test]
    fn test_invalid_timelock_delay() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        EscrowContract::initialize(env.clone(), admin.clone());

        // Test delay too short (less than 1 hour)
        let config = TimeLockConfig {
            delay: 1800,      // 30 minutes
            grace_period: 3600,
        };

        let result = EscrowContract::set_timelock_config(env.clone(), admin.clone(), config);
        assert_eq!(result, Err(Error::InvalidStatus));

        // Test delay too long (more than 7 days)
        let config = TimeLockConfig {
            delay: 700000,    // > 7 days
            grace_period: 3600,
        };

        let result = EscrowContract::set_timelock_config(env.clone(), admin.clone(), config);
        assert_eq!(result, Err(Error::InvalidStatus));
    }
}
