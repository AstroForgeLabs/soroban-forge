//! forge-rate-limit
//!
//! A Soroban policy contract that limits how many times an operation can be
//! called within a configurable sliding time window (in ledgers). Useful for
//! DeFi bots, oracle updates, and any on-chain action that should be
//! rate-throttled to prevent abuse or runaway automation.

#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Env, Symbol};

// ─── Storage keys ────────────────────────────────────────────────────────────

const MAX_CALLS_KEY: Symbol = symbol_short!("MAX");
const WINDOW_KEY: Symbol = symbol_short!("WINDOW");
const COUNT_KEY: Symbol = symbol_short!("COUNT");
const WINDOW_START_KEY: Symbol = symbol_short!("START");

// ─── Contract ────────────────────────────────────────────────────────────────

#[contract]
pub struct RateLimitContract;

#[contractimpl]
impl RateLimitContract {
    /// Initialise with `max_calls` allowed per `window_ledgers` window.
    pub fn initialise(env: Env, max_calls: u32, window_ledgers: u32) {
        if env.storage().instance().has(&MAX_CALLS_KEY) {
            panic!("already initialised");
        }
        env.storage().instance().set(&MAX_CALLS_KEY, &max_calls);
        env.storage().instance().set(&WINDOW_KEY, &window_ledgers);
        env.storage()
            .instance()
            .set(&WINDOW_START_KEY, &env.ledger().sequence());
        env.storage().instance().set(&COUNT_KEY, &0u32);
    }

    /// Attempt to record a call. Returns `true` if allowed, `false` if the
    /// rate limit has been reached for the current window.
    pub fn try_call(env: Env) -> bool {
        let max: u32 = env
            .storage()
            .instance()
            .get(&MAX_CALLS_KEY)
            .unwrap_or_else(|| panic!("not initialised"));
        let window: u32 = env.storage().instance().get(&WINDOW_KEY).unwrap_or(1);
        let start: u32 = env.storage().instance().get(&WINDOW_START_KEY).unwrap_or(0);
        let current_ledger = env.ledger().sequence();

        // Roll the window forward if it has expired.
        let (count, new_start) = if current_ledger.saturating_sub(start) >= window {
            (0u32, current_ledger)
        } else {
            let c: u32 = env.storage().instance().get(&COUNT_KEY).unwrap_or(0);
            (c, start)
        };

        if count >= max {
            return false;
        }

        env.storage()
            .instance()
            .set(&COUNT_KEY, &(count + 1));
        env.storage()
            .instance()
            .set(&WINDOW_START_KEY, &new_start);
        true
    }

    /// Return the number of calls recorded in the current window.
    pub fn call_count(env: Env) -> u32 {
        env.storage().instance().get(&COUNT_KEY).unwrap_or(0)
    }

    /// Return the configured maximum calls per window.
    pub fn max_calls(env: Env) -> u32 {
        env.storage().instance().get(&MAX_CALLS_KEY).unwrap_or(0)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Ledger as _, Env};

    #[test]
    fn allows_calls_within_limit() {
        let env = Env::default();
        let id = env.register(RateLimitContract, ());
        let client = RateLimitContractClient::new(&env, &id);
        client.initialise(&3, &100);

        assert!(client.try_call());
        assert!(client.try_call());
        assert!(client.try_call());
        assert_eq!(client.call_count(), 3);
    }

    #[test]
    fn rejects_calls_over_limit() {
        let env = Env::default();
        let id = env.register(RateLimitContract, ());
        let client = RateLimitContractClient::new(&env, &id);
        client.initialise(&2, &100);

        assert!(client.try_call());
        assert!(client.try_call());
        assert!(!client.try_call()); // 3rd call rejected
    }

    #[test]
    fn window_expiry_resets_counter() {
        let env = Env::default();
        let id = env.register(RateLimitContract, ());
        let client = RateLimitContractClient::new(&env, &id);
        client.initialise(&2, &10);

        assert!(client.try_call());
        assert!(client.try_call());
        assert!(!client.try_call()); // at limit

        // Advance ledger past window
        env.ledger().set_sequence_number(env.ledger().sequence() + 11);

        // Window reset — calls allowed again
        assert!(client.try_call());
        assert_eq!(client.call_count(), 1);
    }

    #[test]
    fn max_calls_reflects_config() {
        let env = Env::default();
        let id = env.register(RateLimitContract, ());
        let client = RateLimitContractClient::new(&env, &id);
        client.initialise(&7, &50);
        assert_eq!(client.max_calls(), 7);
    }
}
