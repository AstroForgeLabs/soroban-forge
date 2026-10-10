//! forge-time-lock  (NEW — not in soroban-ops-cookbook)
//!
//! A Soroban time-lock policy contract that prevents execution before a
//! configured unlock ledger sequence number. Useful for:
//!
//! - **Vesting schedules**: tokens locked until a future ledger
//! - **Delayed governance**: proposals that take effect after a delay
//! - **Timelocked upgrades**: require on-chain notice period before activation

#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

// ─── Storage keys ────────────────────────────────────────────────────────────

const UNLOCK_KEY: Symbol = symbol_short!("UNLOCK");
const OWNER_KEY: Symbol = symbol_short!("OWNER");

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct TimeLockContract;

#[contractimpl]
impl TimeLockContract {
    /// Initialise with an unlock ledger sequence number.
    /// Operations will be blocked until `env.ledger().sequence() >= unlock_at`.
    pub fn initialise(env: Env, owner: Address, unlock_at: u32) {
        if env.storage().instance().has(&UNLOCK_KEY) {
            panic!("already initialised");
        }
        assert!(
            unlock_at > env.ledger().sequence(),
            "unlock_at must be in the future"
        );
        env.storage().instance().set(&OWNER_KEY, &owner);
        env.storage().instance().set(&UNLOCK_KEY, &unlock_at);
    }

    /// Extend the lock to a later ledger. Owner-only.
    pub fn extend_lock(env: Env, caller: Address, new_unlock_at: u32) {
        caller.require_auth();
        let owner: Address = env
            .storage()
            .instance()
            .get(&OWNER_KEY)
            .unwrap_or_else(|| panic!("not initialised"));
        if caller != owner {
            panic!("unauthorised");
        }
        let current: u32 = env.storage().instance().get(&UNLOCK_KEY).unwrap_or(0);
        assert!(new_unlock_at > current, "can only extend, not shorten");
        env.storage().instance().set(&UNLOCK_KEY, &new_unlock_at);
    }

    /// Returns `true` when the lock has expired (execution is allowed).
    pub fn is_unlocked(env: Env) -> bool {
        let unlock_at: u32 = env
            .storage()
            .instance()
            .get(&UNLOCK_KEY)
            .unwrap_or_else(|| panic!("not initialised"));
        env.ledger().sequence() >= unlock_at
    }

    /// Return the unlock ledger sequence number.
    pub fn unlock_at(env: Env) -> u32 {
        env.storage().instance().get(&UNLOCK_KEY).unwrap_or(0)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

    #[test]
    fn locked_before_unlock_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(100);
        let id = env.register(TimeLockContract, ());
        let client = TimeLockContractClient::new(&env, &id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &200);
        assert!(!client.is_unlocked());
    }

    #[test]
    fn unlocked_at_exact_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(100);
        let id = env.register(TimeLockContract, ());
        let client = TimeLockContractClient::new(&env, &id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &150);

        // Advance to unlock ledger
        env.ledger().set_sequence_number(150);
        assert!(client.is_unlocked());
    }

    #[test]
    fn unlocked_after_unlock_ledger() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(100);
        let id = env.register(TimeLockContract, ());
        let client = TimeLockContractClient::new(&env, &id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &150);
        env.ledger().set_sequence_number(500);
        assert!(client.is_unlocked());
    }

    #[test]
    fn extend_lock_pushes_unlock_further() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(100);
        let id = env.register(TimeLockContract, ());
        let client = TimeLockContractClient::new(&env, &id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &200);
        assert_eq!(client.unlock_at(), 200);

        client.extend_lock(&owner, &500);
        assert_eq!(client.unlock_at(), 500);
    }

    #[test]
    fn still_locked_after_extension() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(100);
        let id = env.register(TimeLockContract, ());
        let client = TimeLockContractClient::new(&env, &id);
        let owner = Address::generate(&env);

        client.initialise(&owner, &200);
        // Would be unlocked at 200 — but we extend
        client.extend_lock(&owner, &1000);

        env.ledger().set_sequence_number(300); // past original 200 but before 1000
        assert!(!client.is_unlocked());
    }
}
