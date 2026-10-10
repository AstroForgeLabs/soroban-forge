# Soroban Forge

[![CI](https://github.com/AstroForgeLabs/soroban-forge/actions/workflows/ci.yml/badge.svg)](https://github.com/AstroForgeLabs/soroban-forge/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Drips Wave](https://img.shields.io/badge/Drips-Stellar%20Wave-blue.svg)](https://drips.network)

**Production-ready Soroban smart contract modules and off-chain event pipeline tools for the Stellar ecosystem.**

`soroban-forge` is a toolkit of composable building blocks for Stellar custom account policy enforcement and on-chain event streaming — designed to be dropped into any Soroban project without modification.

---

## Contracts

| Contract | Description | Tests |
|---|---|---|
| [`spend-limit`](./contracts/spend-limit/) | Enforce per-transaction spend caps in stroops | 4 |
| [`rate-limit`](./contracts/rate-limit/) | Sliding-window call rate limiter | 4 |
| [`multisig-policy`](./contracts/multisig-policy/) | N-of-M signer approval gate | 5 |
| [`time-lock`](./contracts/time-lock/) | Block execution until a future ledger | 5 |

**18 unit tests — all passing.**

---

## Quickstart

```bash
git clone https://github.com/AstroForgeLabs/soroban-forge.git
cd soroban-forge

# Test all contracts
cargo test --manifest-path contracts/spend-limit/Cargo.toml
cargo test --manifest-path contracts/rate-limit/Cargo.toml
cargo test --manifest-path contracts/multisig-policy/Cargo.toml
cargo test --manifest-path contracts/time-lock/Cargo.toml

# Build WASM artifacts
cargo build --manifest-path contracts/spend-limit/Cargo.toml \
  --target wasm32-unknown-unknown --release
```

---

## Contract Details

### `spend-limit`

Enforce a maximum single-transaction spend (in stroops) for Stellar custom accounts.

```rust
// Initialise with a 500_000 stroop limit
client.initialise(&owner, &500_000);

// In __check_auth: reject payments over the limit
let allowed = client.check_spend(&amount);
```

**Storage:** `LIMIT` (i128), `OWNER` (Address)  
**Methods:** `initialise`, `set_limit`, `get_limit`, `check_spend`

---

### `rate-limit`

Limit how many times an operation can execute within a sliding ledger window.

```rust
// Allow 10 calls per 100-ledger window
client.initialise(&10, &100);

// Returns false when the limit is reached
let allowed = client.try_call();
```

**Storage:** `MAX`, `WINDOW`, `COUNT`, `START`  
**Methods:** `initialise`, `try_call`, `call_count`, `max_calls`

---

### `multisig-policy` ✨ New

N-of-M approval gate for treasury management, protocol upgrades, and governance.

```rust
// Require 2-of-3 signers
client.initialise(&signers_vec, &2);

// Each signer approves an operation ID
client.approve(&signer_a, &op_id);
client.approve(&signer_b, &op_id);

// Check if threshold is reached
let ready = client.is_approved(&op_id); // true
```

**Storage:** `THRESH`, `SIGNERS`, per-op approval maps  
**Methods:** `initialise`, `approve`, `approval_count`, `is_approved`, `threshold`

---

### `time-lock` ✨ New

Prevent execution before a future ledger sequence number — enabling vesting, delayed upgrades, and governance notice periods.

```rust
// Lock until ledger 10_000
client.initialise(&owner, &10_000);

// Before ledger 10_000
client.is_unlocked() // false

// Extend the lock (owner only)
client.extend_lock(&owner, &20_000);
```

**Storage:** `UNLOCK`, `OWNER`  
**Methods:** `initialise`, `extend_lock`, `is_unlocked`, `unlock_at`

---

## Event Pipeline

| Tool | Description |
|---|---|
| [`webhook-relay`](./event-pipeline/webhook-relay/) | Streams Horizon SSE contract events to HTTP webhooks with HMAC-SHA256 signing |

---

## Requirements

- Rust stable (see `rust-toolchain.toml`)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Stellar CLI (optional, for testnet deployment): `cargo install stellar-cli`

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Issues are labelled with Drips complexity points.

## License

[MIT](./LICENSE) © 2024 Abdulmalik Ojo
