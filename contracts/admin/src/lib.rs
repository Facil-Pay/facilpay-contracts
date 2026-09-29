#![no_std]
use escrow::EscrowContractClient;
use payments::PaymentContractClient;
use refund::RefundContractClient;
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Map, String};

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ContractAddresses {
    pub payment: Address,
    pub escrow: Address,
    pub refund: Address,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Pauser,
    PaymentContract,
    EscrowContract,
    RefundContract,
}

#[contract]
pub struct AdminContract;

#[contractimpl]
impl AdminContract {
    /// Initializes the admin contract with the addresses of the payment, escrow,
    /// and refund contracts.
    ///
    /// # Parameters
    /// - `admin`: the address authorized to manage the contract.
    /// - `pauser`: the address authorized to pause/unpause the platform.
    /// - `payment_contract`: the deployed payment contract address.
    /// - `escrow_contract`: the deployed escrow contract address.
    /// - `refund_contract`: the deployed refund contract address.
    ///
    /// # Returns
    /// Returns `Ok(())` when initialization succeeds.
    ///
    /// # Errors
    /// Returns `Error::AlreadyInitialized` if the contract has already been set up.
    pub fn initialize(
        env: Env,
        admin: Address,
        pauser: Address,
        payment_contract: Address,
        escrow_contract: Address,
        refund_contract: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Pauser, &pauser);
        env.storage()
            .instance()
            .set(&DataKey::PaymentContract, &payment_contract);
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .instance()
            .set(&DataKey::RefundContract, &refund_contract);

        Ok(())
    }

    /// Pauses the payment, escrow, and refund contracts in one Soroban call.
    ///
    /// # Parameters
    /// - `pauser`: the pauser address that must be authorized.
    /// - `reason`: a human-readable explanation for the emergency pause.
    ///
    /// # Returns
    /// Returns `Ok(())` when all child contracts are paused successfully.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// and `Error::Unauthorized` if the provided pauser address does not match the
    /// stored pauser.
    pub fn emergency_pause_all(env: Env, pauser: Address, reason: String) -> Result<(), Error> {
        pauser.require_auth();

        let stored_pauser: Address = env
            .storage()
            .instance()
            .get(&DataKey::Pauser)
            .ok_or(Error::NotInitialized)?;
        if pauser != stored_pauser {
            return Err(Error::Unauthorized);
        }

        let payment_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::PaymentContract)
            .ok_or(Error::NotInitialized)?;
        let escrow_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .ok_or(Error::NotInitialized)?;
        let refund_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::RefundContract)
            .ok_or(Error::NotInitialized)?;

        PaymentContractClient::new(&env, &payment_contract).pause_contract(&pauser, &reason);
        EscrowContractClient::new(&env, &escrow_contract).pause_contract(&pauser, &reason);
        RefundContractClient::new(&env, &refund_contract).pause_contract(&pauser, &reason);

        Ok(())
    }

    /// Unpauses the payment, escrow, and refund contracts in one Soroban call.
    ///
    /// # Parameters
    /// - `pauser`: the pauser address that must be authorized.
    ///
    /// # Returns
    /// Returns `Ok(())` when all child contracts are unpaused successfully.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// and `Error::Unauthorized` if the provided pauser address does not match the
    /// stored pauser.
    pub fn emergency_unpause_all(env: Env, pauser: Address) -> Result<(), Error> {
        pauser.require_auth();

        let stored_pauser: Address = env
            .storage()
            .instance()
            .get(&DataKey::Pauser)
            .ok_or(Error::NotInitialized)?;
        if pauser != stored_pauser {
            return Err(Error::Unauthorized);
        }

        let payment_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::PaymentContract)
            .ok_or(Error::NotInitialized)?;
        let escrow_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .ok_or(Error::NotInitialized)?;
        let refund_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::RefundContract)
            .ok_or(Error::NotInitialized)?;

        PaymentContractClient::new(&env, &payment_contract).unpause_contract(&pauser);
        EscrowContractClient::new(&env, &escrow_contract).unpause_contract(&pauser);
        RefundContractClient::new(&env, &refund_contract).unpause_contract(&pauser);

        Ok(())
    }

    /// Updates the stored payment contract address.
    ///
    /// # Parameters
    /// - `admin`: the admin address that must be authorized.
    /// - `payment_contract`: the new payment contract address.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// and `Error::Unauthorized` if the provided admin address does not match the
    /// stored admin.
    pub fn set_payment_contract(
        env: Env,
        admin: Address,
        payment_contract: Address,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&DataKey::PaymentContract, &payment_contract);

        Ok(())
    }

    /// Updates the stored escrow contract address.
    ///
    /// # Parameters
    /// - `admin`: the admin address that must be authorized.
    /// - `escrow_contract`: the new escrow contract address.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// and `Error::Unauthorized` if the provided admin address does not match the
    /// stored admin.
    pub fn set_escrow_contract(
        env: Env,
        admin: Address,
        escrow_contract: Address,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);

        Ok(())
    }

    /// Updates the stored refund contract address.
    ///
    /// # Parameters
    /// - `admin`: the admin address that must be authorized.
    /// - `refund_contract`: the new refund contract address.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the admin contract has not been initialized,
    /// and `Error::Unauthorized` if the provided admin address does not match the
    /// stored admin.
    pub fn set_refund_contract(
        env: Env,
        admin: Address,
        refund_contract: Address,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .instance()
            .set(&DataKey::RefundContract, &refund_contract);

        Ok(())
    }

    /// Returns the current admin address.
    ///
    /// # Returns
    /// The `Address` of the current admin.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    /// Returns the current pauser address.
    ///
    /// # Returns
    /// The `Address` of the current pauser.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized.
    pub fn get_pauser(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Pauser)
            .ok_or(Error::NotInitialized)
    }

    /// Returns all child contract addresses.
    ///
    /// # Returns
    /// A `ContractAddresses` struct containing the payment, escrow, and refund contract addresses.
    ///
    /// # Errors
    /// Returns `Error::NotInitialized` if the contract has not been initialized.
    pub fn get_contracts(env: Env) -> Result<ContractAddresses, Error> {
        let payment = env
            .storage()
            .instance()
            .get(&DataKey::PaymentContract)
            .ok_or(Error::NotInitialized)?;
        let escrow = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .ok_or(Error::NotInitialized)?;
        let refund = env
            .storage()
            .instance()
            .get(&DataKey::RefundContract)
            .ok_or(Error::NotInitialized)?;

        Ok(ContractAddresses {
            payment,
            escrow,
            refund,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup_payment(env: &Env, admin: &Address) -> Address {
        let contract_id = env.register(payments::PaymentContract, ());
        let client = PaymentContractClient::new(env, &contract_id);
        client.initialize(admin);
        contract_id
    }

    fn setup_escrow(env: &Env, admin: &Address) -> Address {
        let contract_id = env.register(escrow::EscrowContract, ());
        let client = EscrowContractClient::new(env, &contract_id);
        client.initialize(admin);
        contract_id
    }

    fn setup_refund(env: &Env, admin: &Address) -> Address {
        let contract_id = env.register(refund::RefundContract, ());
        let client = RefundContractClient::new(env, &contract_id);
        client.initialize(admin);
        contract_id
    }

    #[test]
    fn test_initialize_and_pause_all() {
        let env = Env::default();
        env.mock_all_auths();

        let admin_contract_id = env.register(AdminContract, ());
        let client = AdminContractClient::new(&env, &admin_contract_id);

        let admin = Address::generate(&env);
        let pauser = Address::generate(&env);
        let payment_contract = setup_payment(&env, &pauser);
        let escrow_contract = setup_escrow(&env, &pauser);
        let refund_contract = setup_refund(&env, &pauser);

        client.initialize(
            &admin,
            &pauser,
            &payment_contract,
            &escrow_contract,
            &refund_contract,
        );

        let reason = String::from_str(&env, "security incident");
        client.emergency_pause_all(&pauser, &reason);
        client.emergency_unpause_all(&pauser);
    }

    /// Registers the admin contract and three child contracts (all administered by
    /// `pauser`), then initializes the admin contract.
    /// Returns (client, admin, pauser, payment, escrow, refund).
    fn setup_initialized(
        env: &Env,
    ) -> (
        AdminContractClient<'_>,
        Address,
        Address,
        Address,
        Address,
        Address,
    ) {
        let admin_contract_id = env.register(AdminContract, ());
        let client = AdminContractClient::new(env, &admin_contract_id);

        let admin = Address::generate(env);
        let pauser = Address::generate(env);
        let payment_contract = setup_payment(env, &pauser);
        let escrow_contract = setup_escrow(env, &pauser);
        let refund_contract = setup_refund(env, &pauser);

        client.initialize(
            &admin,
            &pauser,
            &payment_contract,
            &escrow_contract,
            &refund_contract,
        );

        (
            client,
            admin,
            pauser,
            payment_contract,
            escrow_contract,
            refund_contract,
        )
    }

    #[test]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, pauser, payment_contract, escrow_contract, refund_contract) =
            setup_initialized(&env);

        let result = client.try_initialize(
            &admin,
            &pauser,
            &payment_contract,
            &escrow_contract,
            &refund_contract,
        );
        assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
    }

    #[test]
    fn test_pause_and_unpause_all_updates_child_contracts() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin, pauser, payment_contract, escrow_contract, refund_contract) =
            setup_initialized(&env);
        let payment = PaymentContractClient::new(&env, &payment_contract);
        let escrow = EscrowContractClient::new(&env, &escrow_contract);
        let refund = RefundContractClient::new(&env, &refund_contract);

        let reason = String::from_str(&env, "security incident");
        client.emergency_pause_all(&pauser, &reason);

        assert!(payment.get_pause_state().globally_paused);
        assert!(escrow.get_pause_state().globally_paused);
        assert!(refund.get_pause_state().globally_paused);
        assert_eq!(payment.get_pause_state().pause_reason, reason);

        client.emergency_unpause_all(&pauser);

        assert!(!payment.get_pause_state().globally_paused);
        assert!(!escrow.get_pause_state().globally_paused);
        assert!(!refund.get_pause_state().globally_paused);
    }

    #[test]
    fn test_pause_all_rejects_non_pauser_and_uninitialized() {
        let env = Env::default();
        env.mock_all_auths();

        // Uninitialized contract returns NotInitialized.
        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);
        let someone = Address::generate(&env);
        let reason = String::from_str(&env, "incident");
        assert_eq!(
            fresh.try_emergency_pause_all(&someone, &reason),
            Err(Ok(Error::NotInitialized))
        );
        assert_eq!(
            fresh.try_emergency_unpause_all(&someone),
            Err(Ok(Error::NotInitialized))
        );

        // Initialized contract rejects anyone but the stored pauser, including the admin.
        let (client, admin, _pauser, payment_contract, _, _) = setup_initialized(&env);
        assert_eq!(
            client.try_emergency_pause_all(&admin, &reason),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_emergency_unpause_all(&someone),
            Err(Ok(Error::Unauthorized))
        );
        assert!(
            !PaymentContractClient::new(&env, &payment_contract)
                .get_pause_state()
                .globally_paused
        );
    }

    #[test]
    fn test_contract_setters_admin_only_and_take_effect() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, pauser, old_payment, old_escrow, old_refund) = setup_initialized(&env);
        let new_payment = setup_payment(&env, &pauser);
        let new_escrow = setup_escrow(&env, &pauser);
        let new_refund = setup_refund(&env, &pauser);

        // Non-admin callers (including the pauser) are rejected.
        let stranger = Address::generate(&env);
        assert_eq!(
            client.try_set_payment_contract(&stranger, &new_payment),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_set_escrow_contract(&pauser, &new_escrow),
            Err(Ok(Error::Unauthorized))
        );
        assert_eq!(
            client.try_set_refund_contract(&stranger, &new_refund),
            Err(Ok(Error::Unauthorized))
        );

        // Setters on an uninitialized contract return NotInitialized.
        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);
        assert_eq!(
            fresh.try_set_payment_contract(&admin, &new_payment),
            Err(Ok(Error::NotInitialized))
        );

        client.set_payment_contract(&admin, &new_payment);
        client.set_escrow_contract(&admin, &new_escrow);
        client.set_refund_contract(&admin, &new_refund);

        // Pausing now targets the new contracts and leaves the old ones untouched.
        client.emergency_pause_all(&pauser, &String::from_str(&env, "rotation"));

        assert!(
            PaymentContractClient::new(&env, &new_payment)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            EscrowContractClient::new(&env, &new_escrow)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            RefundContractClient::new(&env, &new_refund)
                .get_pause_state()
                .globally_paused
        );

        assert!(
            !PaymentContractClient::new(&env, &old_payment)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            !EscrowContractClient::new(&env, &old_escrow)
                .get_pause_state()
                .globally_paused
        );
        assert!(
            !RefundContractClient::new(&env, &old_refund)
                .get_pause_state()
                .globally_paused
        );
    }

    #[test]
    fn test_get_admin_returns_stored_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, _, _, _, _) = setup_initialized(&env);

        assert_eq!(client.get_admin(), admin.clone());
    }

    #[test]
    fn test_get_admin_returns_not_initialized_before_init() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);

        assert_eq!(fresh.try_get_admin(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_get_pauser_returns_stored_pauser() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, pauser, _, _, _) = setup_initialized(&env);

        assert_eq!(client.get_pauser(), pauser.clone());
    }

    #[test]
    fn test_get_pauser_returns_not_initialized_before_init() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);

        assert_eq!(fresh.try_get_pauser(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_get_contracts_returns_all_addresses() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _, _, payment, escrow, refund) = setup_initialized(&env);

        let contracts = client.get_contracts();
        assert_eq!(contracts.payment, payment);
        assert_eq!(contracts.escrow, escrow);
        assert_eq!(contracts.refund, refund);
    }

    #[test]
    fn test_get_contracts_returns_not_initialized_before_init() {
        let env = Env::default();
        env.mock_all_auths();

        let fresh_id = env.register(AdminContract, ());
        let fresh = AdminContractClient::new(&env, &fresh_id);

        assert_eq!(fresh.try_get_contracts(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn test_get_contracts_reflects_updated_addresses() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, pauser, _, _, _) = setup_initialized(&env);
        let new_payment = setup_payment(&env, &pauser);
        let new_escrow = setup_escrow(&env, &pauser);
        let new_refund = setup_refund(&env, &pauser);

        client.set_payment_contract(&admin, &new_payment);
        client.set_escrow_contract(&admin, &new_escrow);
        client.set_refund_contract(&admin, &new_refund);

        let contracts = client.get_contracts();
        assert_eq!(contracts.payment, new_payment);
        assert_eq!(contracts.escrow, new_escrow);
        assert_eq!(contracts.refund, new_refund);
    }
}
