# TalentTrust Contracts

Soroban smart contracts for the TalentTrust decentralized freelancer escrow protocol on the Stellar network.

## What's in this repo

- **Escrow contract** (`contracts/escrow`): Holds funds in escrow, supports milestone-based payments, reputation credential issuance, treasury payout integration, and timeout-based escrow actions for inactive milestones/disputes.

## Features

### Core Escrow Functionality
- **Milestone-based payments**: Break projects into milestones with individual payment amounts
- **Secure fund holding**: Client deposits funds into escrow before work begins
- **Milestone release**: Client releases payments upon milestone completion
- **Reputation system**: Issue credentials to freelancers upon contract completion

### Treasury Payout Integration
- **Protocol fee collection**: Automatically deduct fees from milestone payments
- **Configurable fee structure**: Admin can set fee percentage (in basis points, max 100%)
- **Secure treasury address**: Fees are transferred to a designated treasury address
- **Audit trail events**: All treasury operations emit events for transparency

#### Treasury Functions
- `initialize_treasury(admin, treasury_address, fee_basis_points)` - Initialize treasury configuration (one-time)
- `update_treasury_config(admin, new_address, new_fee)` - Update treasury settings (admin only)
- `get_treasury_config()` - Read current treasury configuration
- `calculate_protocol_fee(amount)` - Calculate fee for a given amount
- `payout_treasury(admin, token, amount)` - Direct payout to treasury (admin only)

#### Fee Structure
- Default fee: 2.5% (250 basis points)
- Fee calculation: `(amount * fee_basis_points) / 10000`
- Fees are deducted during milestone release and transferred to treasury

#### Audit Events
- `TR_CFG_SET` - Emitted when treasury config is set or updated
- `FEE_COLL` - Emitted when protocol fees are collected
- `TR_PAYOUT` - Emitted when direct treasury payout occurs
- `MS_RELEASE` - Emitted when a milestone is released

### Timeout-Based Escrow Actions
- **Automatic timeout handling**: Contracts can timeout after period of inactivity
- **Milestone completion tracking**: Freelancer marks milestones complete, starting timeout
- **Timeout claims**: Either party can claim funds after timeout period
- **Dispute resolution with timeout**: Disputes auto-resolve after timeout if not manually resolved
- **Configurable timeout duration**: 1 day to 365 days, default 30 days
- **Auto-resolve options**: Return to client, release to freelancer, or split 50/50

#### Timeout Functions
- `set_contract_timeout(contract_id, duration, auto_resolve_type)` - Set timeout for contract
- `get_contract_timeout(contract_id)` - Read timeout configuration
- `get_last_activity(contract_id)` - Get last activity timestamp
- `mark_milestone_complete(contract_id, milestone_id)` - Freelancer marks work done
- `is_milestone_complete(contract_id, milestone_id)` - Check milestone status
- `claim_milestone_timeout(contract_id, milestone_id)` - Claim funds after timeout
- `raise_dispute(contract_id, initiator, reason)` - Initiate dispute with timeout
- `get_dispute(contract_id)` - Get dispute information
- `resolve_dispute(admin, contract_id, resolution)` - Admin resolves dispute before timeout
- `claim_dispute_timeout(contract_id)` - Auto-resolve dispute after timeout

#### Timeout Configuration
- Default timeout: 30 days (2,592,000 seconds)
- Minimum timeout: 1 day (86,400 seconds)
- Maximum timeout: 365 days (31,536,000 seconds)
- Auto-resolve types: 0 = return to client, 1 = release to freelancer, 2 = split 50/50

#### Timeout Events
- `TIMEOUT` - Emitted when timeout is claimed
- `DISPUTE` - Emitted when dispute is raised
- `RESOLVED` - Emitted when dispute is resolved
- `COMPLETE` - Emitted when milestone is marked complete

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

## Usage Example

```rust
use soroban_sdk::{Env, Address, vec};
use escrow::{Escrow, EscrowClient};

// Initialize environment
let env = Env::default();
let contract_id = env.register(Escrow, ());
let client = EscrowClient::new(&env, &contract_id);

// Initialize treasury with 2.5% fee
let admin = Address::generate(&env);
let treasury = Address::generate(&env);
client.initialize_treasury(&admin, &treasury, &250);

// Create escrow contract
let client_addr = Address::generate(&env);
let freelancer_addr = Address::generate(&env);
let token = Address::generate(&env);
let milestones = vec![&env, 100_0000000_i128, 200_0000000_i128];

let contract_id = client.create_contract(&client_addr, &freelancer_addr, &milestones, &token);

// Deposit funds
client.deposit_funds(&contract_id, &300_0000000_i128, &token);

// Release milestone (automatically deducts 2.5% fee to treasury)
client.release_milestone(&contract_id, &0);
```

## Security Considerations

### Treasury Security
- **Admin-only access**: Only the admin can initialize or update treasury configuration
- **Authorization checks**: All admin functions require cryptographic authorization
- **Fee validation**: Fee percentage cannot exceed 100% (10000 basis points)
- **Safe arithmetic**: All calculations use checked arithmetic to prevent overflow
- **Immutable config**: Treasury can only be initialized once

### Access Control
- Admin is set during treasury initialization and cannot be changed
- Only the client who created an escrow can deposit funds and release milestones
- Only the freelancer can mark milestones as complete
- Only client or freelancer can raise disputes
- All state-changing operations require proper authorization

### Timeout Security
- **Timeout validation**: Duration must be between 1 day and 365 days
- **Activity tracking**: Last activity timestamp updated on all state changes
- **Timeout claims**: Only legitimate parties can claim after timeout
- **Dispute resolution**: Admin can resolve disputes before timeout expires
- **Auto-resolve protection**: Funds distributed according to pre-configured rules

### Audit Trail
- All treasury and timeout operations emit events with relevant data
- Events include: admin address, treasury address, fee amounts, timestamps, dispute reasons
- Complete history of configuration changes and timeouts is preserved on-chain

## Testing

The test suite covers:
- Treasury initialization (success and failure cases)
- Configuration updates (authorized and unauthorized)
- Fee calculation accuracy (various percentages and amounts)
- Milestone release with fee deduction
- Timeout configuration (valid and invalid durations)
- Milestone completion and timeout claims
- Dispute raising and resolution
- Activity tracking updates
- Edge cases (0% fee, 100% fee, overflow protection, timeout bounds)
- Access control and authorization

Run tests with:
```bash
cargo test
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
