//! forge-spend-limit
//!
//! A Soroban policy contract that enforces per-transaction spend limits for
//! Stellar custom accounts. When integrated into a custom account's
//! `__check_auth` hook this contract vetoes any payment whose amount exceeds
//! the configured limit, providing a simple but effective guardrail against
//! fat-finger transfers and key-compromise scenarios.

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Env, Symbol};

// ─── Storage keys ────────────────────────────────────────────────────────────

const LIMIT_KEY: Symbol = symbol_short!("LIMIT");
const OWNER_KEY: Symbol = symbol_short!("OWNER");

// ─── Errors ──────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ForgeError {
    /// Caller is not the contract owner.
    Unauthorized = 1,
    /// The amount exceeds the configured spend limit.
    ExceedsLimit = 2,
    /// Contract has not been initialised yet.
    NotInitialised = 3,
}

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct SpendLimitContract;

#[contractimpl]
impl SpendLimitContract {
    /// Initialise the contract with an owner and a spend limit (in stroops).
    ///
    /// # Panics
    /// Panics if the contract has already been initialised.
    pub fn initialise(env: Env, owner: soroban_sdk::Address, limit_stroops: i128) {
        if env.storage().instance().has(&OWNER_KEY) {
            panic!("already initialised");
        }
        env.storage().instance().set(&OWNER_KEY, &owner);
        env.storage().instance().set(&LIMIT_KEY, &limit_stroops);
    }

    /// Update the spend limit. Only callable by the owner.
    pub fn set_limit(env: Env, caller: soroban_sdk::Address, new_limit: i128) {
        caller.require_auth();
        let owner: soroban_sdk::Address = env
            .storage()
            .instance()
            .get(&OWNER_KEY)
            .unwrap_or_else(|| panic!("not initialised"));
        if caller != owner {
            panic!("unauthorised");
        }
        env.storage().instance().set(&LIMIT_KEY, &new_limit);
    }

    /// Return the current spend limit.
    pub fn get_limit(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&LIMIT_KEY)
            .unwrap_or(0)
    }

    /// Check whether `amount` is within the spend limit.
    /// Returns `true` when allowed, `false` when the limit is exceeded.
    ///
    /// Call this from a custom account's `__check_auth` hook.
    pub fn check_spend(env: Env, amount: i128) -> bool {
        let limit: i128 = env
            .storage()
            .instance()
            .get(&LIMIT_KEY)
            .unwrap_or_else(|| panic!("not initialised"));
        amount <= limit
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn initialise_sets_limit_and_owner() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SpendLimitContract, ());
        let client = SpendLimitContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &1_000_000);
        assert_eq!(client.get_limit(), 1_000_000);
    }

    #[test]
    fn check_spend_allows_amount_under_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SpendLimitContract, ());
        let client = SpendLimitContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &500_000);
        assert!(client.check_spend(&499_999));
        assert!(client.check_spend(&500_000));
    }

    #[test]
    fn check_spend_rejects_amount_over_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SpendLimitContract, ());
        let client = SpendLimitContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &500_000);
        assert!(!client.check_spend(&500_001));
        assert!(!client.check_spend(&1_000_000_000));
    }

    #[test]
    fn set_limit_updates_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(SpendLimitContract, ());
        let client = SpendLimitContractClient::new(&env, &contract_id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &100_000);
        assert_eq!(client.get_limit(), 100_000);
        client.set_limit(&owner, &999_999);
        assert_eq!(client.get_limit(), 999_999);
    }
}
