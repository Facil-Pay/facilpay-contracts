#![no_std]
use escrow::EscrowContractClient;
use payments::PaymentContractClient;
use refund::RefundContractClient;
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String};

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
}

#[contracttype]
pub enum DataKey {
    Admin,
    PaymentContract,
    EscrowContract,
    RefundContract,
}

#[contract]
pub struct AdminContract;

#[contractimpl]
impl AdminContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        payment_contract: Address,
        escrow_contract: Address,
        refund_contract: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
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

    /// Pauses the payment, escrow, and refund contracts in a single Soroban
    /// invocation so there is no window where one contract remains live
    /// during an incident response.
    pub fn emergency_pause_all(env: Env, admin: Address, reason: String) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if admin != stored_admin {
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

        PaymentContractClient::new(&env, &payment_contract).pause_contract(&admin, &reason);
        EscrowContractClient::new(&env, &escrow_contract).pause_contract(&admin, &reason);
        RefundContractClient::new(&env, &refund_contract).pause_contract(&admin, &reason);

        Ok(())
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
        let payment_contract = setup_payment(&env, &admin);
        let escrow_contract = setup_escrow(&env, &admin);
        let refund_contract = setup_refund(&env, &admin);

        client.initialize(&admin, &payment_contract, &escrow_contract, &refund_contract);

        let reason = String::from_str(&env, "security incident");
        client.emergency_pause_all(&admin, &reason);
    }
}
