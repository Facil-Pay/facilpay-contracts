#![cfg(test)]
mod tests {
    use crate::{
        Currency, Error, FeatureError, FeeConfig, PaymentContract, PaymentContractClient,
        PaymentStatus,
    };
    use soroban_sdk::{testutils::Address as AddressTestUtils, token, Address, Env, String};

    fn setup_env() -> (
        Env,
        PaymentContractClient<'static>,
        Address,
        Address,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        let contract_id = env.register(PaymentContract, ());
        let client = PaymentContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let customer = Address::generate(&env);
        let merchant = Address::generate(&env);
        let forward_to = Address::generate(&env);

        client.initialize(&admin);

        let token_admin = Address::generate(&env);
        let token = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();

        let fee_config = FeeConfig {
            fee_bps: 100,
            min_fee: 0,
            max_fee: i128::MAX,
            treasury: admin.clone(),
            fee_token: token.clone(),
            active: true,
        };
        client.set_fee_config(&admin, &fee_config);

        (env, client, admin, customer, merchant, forward_to, token)
    }

    fn create_and_complete_payment(
        env: &Env,
        client: &PaymentContractClient,
        admin: &Address,
        customer: &Address,
        merchant: &Address,
        amount: i128,
        token: &Address,
    ) -> u64 {
        let contract_id = client.address.clone();
        token::StellarAssetClient::new(env, token).mint(customer, &amount);
        token::Client::new(env, token).approve(customer, &contract_id, &amount, &10_000);
        let payment_id = client.create_payment(
            customer,
            merchant,
            &amount,
            token,
            &Currency::USDC,
            &3600,
            &String::from_slice(env, "test metadata"),
        );
        client.complete_payment(admin, &payment_id);
        payment_id
    }

    #[test]
    fn test_set_payment_forward_valid() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let result = client.try_set_payment_forward(&merchant, &forward_to, &5000);
        assert!(result.is_ok());

        let config = client.get_forward_config(&merchant);
        assert_eq!(config.merchant, merchant);
        assert_eq!(config.forward_to, forward_to);
        assert_eq!(config.forward_bps, 5000);
        assert!(config.active);
    }

    #[test]
    fn test_set_payment_forward_invalid_bps_zero() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let result = client.try_set_payment_forward(&merchant, &forward_to, &0);
        assert_eq!(
            result,
            Err(Ok(Error::Feature(FeatureError::InvalidForwardBps)))
        );
    }

    #[test]
    fn test_set_payment_forward_invalid_bps_too_high() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let result = client.try_set_payment_forward(&merchant, &forward_to, &10001);
        assert_eq!(
            result,
            Err(Ok(Error::Feature(FeatureError::InvalidForwardBps)))
        );
    }

    #[test]
    fn test_set_payment_forward_loop_detection() {
        let (env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        // Build a 3-hop chain that closes back to merchant: forward_to (B) → C → merchant (A)
        let c = Address::generate(&env);
        client.set_payment_forward(&c, &merchant, &5000); // C → A
        client.set_payment_forward(&forward_to, &c, &5000); // B → C

        // merchant (A) → forward_to (B) → C → merchant (A) is a cycle
        let result = client.try_set_payment_forward(&merchant, &forward_to, &5000);
        assert_eq!(result, Err(Ok(Error::Feature(FeatureError::ForwardLoop))));
    }

    #[test]
    fn test_set_payment_forward_valid_bps_boundaries() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let result1 = client.try_set_payment_forward(&merchant, &forward_to, &1);
        assert!(result1.is_ok());

        client.remove_payment_forward(&merchant);

        let result2 = client.try_set_payment_forward(&merchant, &forward_to, &10000);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_remove_payment_forward_success() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        client.set_payment_forward(&merchant, &forward_to, &5000);
        assert!(client.try_get_forward_config(&merchant).is_ok());

        let result = client.try_remove_payment_forward(&merchant);
        assert!(result.is_ok());

        assert!(client.try_get_forward_config(&merchant).is_err());
    }

    #[test]
    fn test_remove_payment_forward_not_found() {
        let (_env, client, _admin, _customer, merchant, _forward_to, _token) = setup_env();

        let result = client.try_remove_payment_forward(&merchant);
        assert_eq!(
            result,
            Err(Ok(Error::Feature(FeatureError::ForwardConfigNotFound)))
        );
    }

    #[test]
    fn test_get_forward_config_not_found() {
        let (_env, client, _admin, _customer, merchant, _forward_to, _token) = setup_env();

        let result = client.try_get_forward_config(&merchant);
        assert!(result.is_err());
    }

    #[test]
    fn test_payment_forward_on_completion() {
        let (env, client, admin, customer, merchant, forward_to, token) = setup_env();

        client.set_payment_forward(&merchant, &forward_to, &5000);

        let payment_id =
            create_and_complete_payment(&env, &client, &admin, &customer, &merchant, 1_000, &token);

        let payment = client.get_payment(&payment_id);
        assert_eq!(payment.status, PaymentStatus::Completed);
    }

    #[test]
    fn test_payment_forward_calculation() {
        let (env, client, admin, customer, merchant, forward_to, token) = setup_env();

        client.set_payment_forward(&merchant, &forward_to, &2500);

        let payment_id = create_and_complete_payment(
            &env, &client, &admin, &customer, &merchant, 1_000_000, &token,
        );

        let payment = client.get_payment(&payment_id);
        assert_eq!(payment.status, PaymentStatus::Completed);
    }

    #[test]
    fn test_payment_forward_multiple_updates() {
        let (env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let new_forward_to = Address::generate(&env);

        client.set_payment_forward(&merchant, &forward_to, &5000);
        client.set_payment_forward(&merchant, &new_forward_to, &7500);

        let config = client.get_forward_config(&merchant);
        assert_eq!(config.forward_to, new_forward_to);
        assert_eq!(config.forward_bps, 7500);
    }

    #[test]
    fn test_payment_forward_with_minimal_bps() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let result = client.try_set_payment_forward(&merchant, &forward_to, &1);
        assert!(result.is_ok());

        let config = client.get_forward_config(&merchant);
        assert_eq!(config.forward_bps, 1);
    }

    #[test]
    fn test_payment_forward_with_full_bps() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        let result = client.try_set_payment_forward(&merchant, &forward_to, &10000);
        assert!(result.is_ok());

        let config = client.get_forward_config(&merchant);
        assert_eq!(config.forward_bps, 10000);
    }

    #[test]
    fn test_payment_forward_config_persistence() {
        let (_env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        client.set_payment_forward(&merchant, &forward_to, &5000);

        let config1 = client.get_forward_config(&merchant);
        let config2 = client.get_forward_config(&merchant);

        assert_eq!(config1.forward_bps, config2.forward_bps);
        assert_eq!(config1.forward_to, forward_to);
    }

    #[test]
    fn test_remove_and_readd_forward_config() {
        let (env, client, _admin, _customer, merchant, forward_to, _token) = setup_env();

        client.set_payment_forward(&merchant, &forward_to, &5000);
        client.remove_payment_forward(&merchant);

        assert!(client.try_get_forward_config(&merchant).is_err());

        let new_forward_to = Address::generate(&env);
        client.set_payment_forward(&merchant, &new_forward_to, &7500);

        let config = client.get_forward_config(&merchant);
        assert_eq!(config.forward_to, new_forward_to);
        assert_eq!(config.forward_bps, 7500);
    }
}
