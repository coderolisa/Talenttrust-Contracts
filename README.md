# TalentTrust Contracts

Soroban smart contracts for the TalentTrust decentralized freelancer escrow protocol on the Stellar network.

## What's in this repo

- **Escrow contract** (`contracts/escrow`): Holds funds in escrow, supports milestone-based payments and reputation credential issuance.

### Release Readiness Checklist

The escrow contract includes an on-chain **release readiness checklist** that automatically tracks and enforces deployment, verification, and post-deploy monitoring gates:

| Phase | Items |
|---|---|
| Deployment | Contract created, funds deposited |
| Verification | Parties authenticated, milestones defined |
| Post-Deploy Monitoring | All milestones released, reputation issued |

`release_milestone` is **hard-blocked** until all Deployment and Verification items are satisfied.  
Query checklist state with `get_release_checklist`, `is_release_ready`, and `is_post_deploy_complete`.

See [docs/escrow/release-readiness-checklist.md](docs/escrow/release-readiness-checklist.md) for full details, function reference, error codes, and security model.

## Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.75+)
- `rustfmt`: `rustup component add rustfmt`
- Optional: [Stellar CLI](https://developers.stellar.org/docs/tools/stellar-cli) for deployment

## Setup

```bash
# Clone (or you're already in the repo)
git clone <your-repo-url>
cd talenttrust-contracts

# Build
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --all -- --check

# Format code
cargo fmt --all
```

## Contributing

1. Fork the repo and create a branch from `main`.
2. Make changes; keep tests and formatting passing:
   - `cargo fmt --all`
   - `cargo test`
   - `cargo build`
3. Open a pull request. CI runs `cargo fmt --all -- --check`, `cargo build`, and `cargo test` on push/PR to `main`.

## CI/CD

On every push and pull request to `main`, GitHub Actions:

- Checks formatting (`cargo fmt --all -- --check`)
- Builds the workspace (`cargo build`)
- Runs tests (`cargo test`)

Ensure these pass locally before pushing.

## License

MIT
