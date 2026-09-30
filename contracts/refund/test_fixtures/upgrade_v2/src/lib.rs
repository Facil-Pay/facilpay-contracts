//! Stand-in "next version" of the refund contract, used to test `upgrade` (#643).
//!
//! It exposes a function the current refund contract does not have (`version`)
//! and reads the admin the refund contract stored, proving that an in-place
//! upgrade keeps both the contract address and its storage.
#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

/// Mirrors the refund contract's `DataKey::Admin` storage key.
#[contracttype]
pub enum DataKey {
    Admin,
}

#[contract]
pub struct RefundUpgradeV2;

#[contractimpl]
impl RefundUpgradeV2 {
    pub fn version() -> u32 {
        2
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }
}
