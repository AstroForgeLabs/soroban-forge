# Contributing to soroban-forge

Thanks for your interest in contributing! This project is a set of composable
Soroban contract modules and off-chain event tooling for the Stellar ecosystem.

## Getting started

1. Install Rust stable (see `rust-toolchain.toml`).
2. Add the wasm target:

   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. Clone the repository and build the workspace:

   ```bash
   git clone https://github.com/AstroForgeLabs/soroban-forge.git
   cd soroban-forge
   cargo build --workspace
   ```

## Development workflow

- `cargo test --workspace` — run every unit test.
- `cargo test -p <package>` — run a single crate's tests, e.g.
  `cargo test -p forge-spend-limit`.
- `cargo build -p <package> --target wasm32-unknown-unknown --release` —
  produce a deployable wasm artifact for a contract.

Soroban's SDK writes state snapshots to `test_snapshots/`. These are committed
and reviewed alongside code changes, so include any snapshot updates in your
pull request.

## Pull requests

- Target the `main` (or `master`) branch.
- Keep changes focused and add or update tests for new behaviour.
- Run `cargo test --workspace` before opening a pull request.
- Describe the motivation and the resulting behaviour in the PR description.

## Issues

Issues are labelled with Drips complexity points.

## License

By contributing you agree that your contributions are licensed under the
[MIT License](./LICENSE).
