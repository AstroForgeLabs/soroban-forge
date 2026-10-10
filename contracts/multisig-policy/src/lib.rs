//! forge-multisig-policy  (NEW — not in soroban-ops-cookbook)
//!
//! An N-of-M multisig policy contract for Stellar custom accounts.
//! A set of M authorised signers is registered at initialisation.
//! Any operation guarded by this policy only executes when at least N
//! distinct signers have called `approve()` with the same `op_id`.
//!
//! # Use-cases
//! - Treasury management: require 3-of-5 board members to approve a transfer
//! - Protocol upgrades: require 2-of-3 core team keys
//! - Emergency pause: 1-of-N is sufficient (set threshold = 1)

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, map, symbol_short, vec, Address, Env, Map, Symbol, Vec,
};

// ─── Storage keys ────────────────────────────────────────────────────────────

const THRESHOLD_KEY: Symbol = symbol_short!("THRESH");
const SIGNERS_KEY: Symbol = symbol_short!("SIGNERS");
/// Prefix for per-op approval maps: "AP:{op_id}"
const APPROVALS_PREFIX: Symbol = symbol_short!("AP");

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct MultisigPolicyContract;

#[contractimpl]
impl MultisigPolicyContract {
    /// Register M signers and require N approvals (N ≤ M).
    pub fn initialise(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&THRESHOLD_KEY) {
            panic!("already initialised");
        }
        assert!(
            threshold > 0 && threshold <= signers.len(),
            "invalid threshold"
        );
        env.storage().instance().set(&SIGNERS_KEY, &signers);
        env.storage().instance().set(&THRESHOLD_KEY, &threshold);
    }

    /// Record `signer`'s approval for `op_id`.
    /// Panics if the signer is not in the authorised set.
    pub fn approve(env: Env, signer: Address, op_id: u32) {
        signer.require_auth();

        let signers: Vec<Address> = env
            .storage()
            .instance()
            .get(&SIGNERS_KEY)
            .unwrap_or_else(|| panic!("not initialised"));

        let is_authorised = signers.iter().any(|s| s == signer);
        if !is_authorised {
            panic!("signer not authorised");
        }

        let mut approvals: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&op_id)
            .unwrap_or_else(|| map![&env]);

        approvals.set(signer, true);
        env.storage().instance().set(&op_id, &approvals);
    }

    /// Return the number of approvals recorded for `op_id`.
    pub fn approval_count(env: Env, op_id: u32) -> u32 {
        let approvals: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&op_id)
            .unwrap_or_else(|| map![&env]);
        approvals.len()
    }

    /// Return `true` if `op_id` has reached the required threshold.
    pub fn is_approved(env: Env, op_id: u32) -> bool {
        let threshold: u32 = env
            .storage()
            .instance()
            .get(&THRESHOLD_KEY)
            .unwrap_or_else(|| panic!("not initialised"));
        let count = Self::approval_count(env, op_id);
        count >= threshold
    }

    /// Return the configured threshold.
    pub fn threshold(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&THRESHOLD_KEY)
            .unwrap_or(0)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup(env: &Env, n: u32, m: usize) -> (soroban_sdk::Address, Vec<Address>, MultisigPolicyContractClient) {
        let id = env.register(MultisigPolicyContract, ());
        let client = MultisigPolicyContractClient::new(env, &id);
        let signers: Vec<Address> = (0..m)
            .map(|_| Address::generate(env))
            .collect::<soroban_sdk::Vec<Address>>();
        client.initialise(&signers, &n);
        (id, signers, client)
    }

    #[test]
    fn single_approval_not_enough_for_2of3() {
        let env = Env::default();
        env.mock_all_auths();
        let (_id, signers, client) = setup(&env, 2, 3);
        client.approve(&signers.get(0).unwrap(), &1);
        assert!(!client.is_approved(&1));
        assert_eq!(client.approval_count(&1), 1);
    }

    #[test]
    fn two_approvals_satisfy_2of3() {
        let env = Env::default();
        env.mock_all_auths();
        let (_id, signers, client) = setup(&env, 2, 3);
        client.approve(&signers.get(0).unwrap(), &1);
        client.approve(&signers.get(1).unwrap(), &1);
        assert!(client.is_approved(&1));
    }

    #[test]
    fn threshold_returned_correctly() {
        let env = Env::default();
        env.mock_all_auths();
        let (_id, _signers, client) = setup(&env, 3, 5);
        assert_eq!(client.threshold(), 3);
    }

    #[test]
    fn different_op_ids_are_independent() {
        let env = Env::default();
        env.mock_all_auths();
        let (_id, signers, client) = setup(&env, 2, 3);
        client.approve(&signers.get(0).unwrap(), &1);
        client.approve(&signers.get(1).unwrap(), &1);
        assert!(client.is_approved(&1));
        assert!(!client.is_approved(&2)); // op 2 has no approvals
    }

    #[test]
    fn one_of_n_threshold_satisfied_by_single_signer() {
        let env = Env::default();
        env.mock_all_auths();
        let (_id, signers, client) = setup(&env, 1, 5);
        client.approve(&signers.get(3).unwrap(), &99);
        assert!(client.is_approved(&99));
    }
}
