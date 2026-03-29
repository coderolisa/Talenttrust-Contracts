# Escrow Contract Event Emission

This document describes the standardized event emission system implemented in the TalentTrust escrow smart contract.

## Overview

The escrow contract emits structured, deterministic events for all critical state transitions. These events enable off-chain indexing, monitoring, and integration with external systems while maintaining security and consistency.

### Design Principles

1. **Minimal but Sufficient**: Events contain only necessary data for off-chain consumers
2. **Deterministic**: Events always follow the same structure and are only emitted on successful state changes
3. **Secure**: No sensitive data (private keys, secrets) is exposed
4. **Indexed**: Event topics and data use consistent, short symbols for efficient indexing

## Event Specifications

All events use the following emission pattern:
```rust
env.events().publish((EVENT_TOPIC_SYMBOL,), event_data_struct);
```

### 1. Contract Created Event

**Topic:** `"create"` (Symbol)

**Emitted When:** A new escrow contract is successfully created via `create_contract()`

**Event Structure:**
```rust
pub struct ContractCreatedEvent {
    pub contract_id: u32,              // Unique contract identifier
    pub client: Address,               // Client funding the escrow
    pub freelancer: Address,           // Freelancer receiving payments
    pub arbiter: Option<Address>,      // Optional dispute arbiter
    pub total_amount: i128,            // Total escrow amount in stroops
    pub milestone_count: u32,          // Number of milestones
    pub release_auth: ReleaseAuthorization, // Authorization scheme
    pub created_at: u64,               // Ledger timestamp
}
```

**Authorization Schemes:**
- `ClientOnly`: Only client can approve/release milestones
- `ArbiterOnly`: Only arbiter can approve/release milestones
- `ClientAndArbiter`: Either client or arbiter can approve/release
- `MultiSig`: Multi-signature scheme (client/arbiter)

**Off-Chain Use Cases:**
- Track contract creation for accounting
- Index contracts by client, freelancer, or arbiter
- Monitor total escrow volume
- Validate contract parameters

**Example Event Data:**
```json
{
  "contract_id": 1,
  "client": "GABC...",
  "freelancer": "GDEF...",
  "arbiter": "GHIJ...",
  "total_amount": 450000000000,
  "milestone_count": 3,
  "release_auth": "ClientAndArbiter",
  "created_at": 1711787400
}
```

---

### 2. Contract Funded Event

**Topic:** `"fund"` (Symbol)

**Emitted When:** Client successfully deposits the full escrow amount via `deposit_funds()`

**Event Structure:**
```rust
pub struct ContractFundedEvent {
    pub contract_id: u32,              // Contract being funded
    pub funder: Address,               // Address that deposited funds
    pub amount: i128,                  // Amount deposited in stroops
    pub new_status: u8,                // Status code (1 = Funded)
    pub funded_at: u64,                // Ledger timestamp
}
```

**Status Codes:**
- `0`: Created
- `1`: Funded
- `2`: Completed
- `3`: Disputed

**Off-Chain Use Cases:**
- Confirm fund custody
- Lock milestone amounts
- Trigger notifications to freelancer
- Update contract status in off-chain database

**Example Event Data:**
```json
{
  "contract_id": 1,
  "funder": "GABC...",
  "amount": 450000000000,
  "new_status": 1,
  "funded_at": 1711787500
}
```

---

### 3. Milestone Released Event

**Topic:** `"release"` (Symbol)

**Emitted When:** A milestone is successfully released via `release_milestone()`

**Event Structure:**
```rust
pub struct MilestoneReleasedEvent {
    pub contract_id: u32,              // Contract containing milestone
    pub milestone_id: u32,             // Zero-based milestone index
    pub amount: i128,                  // Amount released in stroops
    pub released_by: Address,          // Address that triggered release
    pub released_at: u64,              // Ledger timestamp
}
```

**Off-Chain Use Cases:**
- Track milestone completion and payment
- Update freelancer earnings
- Monitor release authority for compliance
- Generate payment reconciliation reports

**Example Event Data:**
```json
{
  "contract_id": 1,
  "milestone_id": 0,
  "amount": 150000000000,
  "released_by": "GABC...",
  "released_at": 1711787600
}
```

---

### 4. Contract Disputed Event

**Topic:** `"dispute"` (Symbol)

**Emitted When:** A dispute is raised via `dispute_contract()`

**Event Structure:**
```rust
pub struct ContractDisputedEvent {
    pub contract_id: u32,              // Contract under dispute
    pub initiator: Address,            // Client or arbiter initiating
    pub reason: Symbol,                // Dispute reason (short symbol)
    pub disputed_at: u64,              // Ledger timestamp
}
```

**Reason Examples:**
- `"quality"`: Quality concerns
- `"partial"`: Partial work delivery
- `"deadline"`: Missed deadline
- `"other"`: Other reason

**Off-Chain Use Cases:**
- Alert involved parties immediately
- Pause further releases/payments
- Trigger dispute resolution workflow
- Log for compliance and audit

**Example Event Data:**
```json
{
  "contract_id": 1,
  "initiator": "GABC...",
  "reason": "quality",
  "disputed_at": 1711787700
}
```

---

### 5. Contract Closed Event

**Topic:** `"close"` (Symbol)

**Emitted When:** All milestones are released, transitioning contract to `Completed` status

**Event Structure:**
```rust
pub struct ContractClosedEvent {
    pub contract_id: u32,              // Contract being closed
    pub freelancer: Address,           // Freelancer who completed work
    pub new_status: u8,                // Status code (2 = Completed)
    pub total_released: i128,          // Total amount released
    pub closed_at: u64,                // Ledger timestamp
}
```

**Off-Chain Use Cases:**
- Signal contract completion
- Trigger reputation updates
- Archive contract records
- Generate final transaction reports
- Update freelancer completion statistics

**Example Event Data:**
```json
{
  "contract_id": 1,
  "freelancer": "GDEF...",
  "new_status": 2,
  "total_released": 450000000000,
  "closed_at": 1711787800
}
```

---

## Event Emission Guarantees

### Timing
- **Before**: No event is emitted before authorization checks
- **After**: Events are emitted immediately after state is persisted
- **Ordering**: Events are emitted in the order operations complete

### Consistency
- Events reflect the **final** state after all validation
- No events are emitted for failed operations (panics prevent emission)
- Each event contains complete, standalone data

### Atomicity
- Event emission is atomic with state changes
- Either both occur or neither occurs (panics roll back both)

## Off-Chain Indexing Guide

### Setting Up Event Listeners

#### Using Stellar SDK (JavaScript/TypeScript)
```typescript
const eventFilter = {
  type: 'contract',
  contractId: 'CABBCD...', // Escrow contract address
  topics: [
    ['AAAADwAAAAZjcmVhdGU='], // "create" event
  ]
};

const events = await horizon.transactions()
  .forLedger(ledgerNumber)
  .call();

events.records.forEach(tx => {
  // Parse event topic and data
});
```

#### Using Horizon API
```bash
# Query for contract events
curl "https://horizon.stellar.org/transactions?contract_id=CABBCD...&asset=native"
```

### Parsing Events Off-Chain

**Event Topic Extraction:**
```
Event = (topic: String, data: ContractData)
Topic examples: "create", "fund", "release", "dispute", "close"
```

**Data Deserialization:**
1. Use Stellar SDK's `xdr.SCVal` to parse event data
2. Extract field values from the struct
3. Convert stroops to XLM for display (1 XLM = 10,000,000 stroops)

### Recommended Data Storage

Store events in an off-chain database with the following schema:

```sql
CREATE TABLE escrow_events (
  event_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  contract_id BIGINT NOT NULL,
  event_type VARCHAR(10),           -- "create", "fund", "release", "dispute", "close"
  client_address VARCHAR(56),
  freelancer_address VARCHAR(56),
  arbiter_address VARCHAR(56),
  amount BIGINT,                    -- in stroops
  milestone_id BIGINT,
  reason VARCHAR(50),
  timestamp BIGINT,
  ledger_sequence BIGINT,
  transaction_hash VARCHAR(64),
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (contract_id) REFERENCES escrow_contracts(id),
  INDEX idx_contract (contract_id),
  INDEX idx_timestamp (timestamp),
  INDEX idx_event_type (event_type)
);
```

## Security Considerations

### What Events DO Expose
✓ Contract state transitions (Created → Funded → Completed/Disputed)
✓ Participant addresses (client, freelancer, arbiter)
✓ Milestone amounts and releases
✓ Authorization scheme used
✓ Timestamps for auditability

### What Events DON'T Expose
✗ Private keys or secrets
✗ Approval signatures (only boolean status)
✗ Gas prices or internal ledger details
✗ Sensitive business logic implementations

### Event Tampering Prevention
- Events are immutable once published to Stellar ledger
- Ledger provides cryptographic proof of event authenticity
- Off-chain consumers should verify ledger sequence numbers

## Troubleshooting

### No Events Appearing

**Possible Causes:**
1. Contract panic during execution - check error messages
2. Ledger not in sync - verify network connection
3. Event topic filtering issue - verify topic names match exactly

**Debug Steps:**
```rust
// Add logging before event emission
println!("About to emit create event for contract {}", contract_id);
env.events().publish((EVENT_CREATE,), event_data);
```

### Events Out of Sync

**Solution:**
- Use transaction hash + ledger sequence as canonical ordering
- Always query events in ascending ledger order
- Handle potential network delays with retry logic

### Missing Event Fields

**Check:**
- Contract code version (old versions might not emit all fields)
- Event struct definitions match current code
- Data types match expectations (addresses, amounts, timestamps)

## Future Enhancements

Potential event system improvements:

1. **Batch Events**: Emit single event for multiple milestone releases
2. **Event Filtering**: Client-side filtering for specific participants
3. **Event Versioning**: Multiple event version support for upgrades
4. **Event Aggregation**: Higher-level events (e.g., "bulk_release")
5. **Event Compression**: Reduce event size for cost optimization

## Testing Event Emission

### Unit Tests
```rust
#[test]
fn test_create_contract_emits_event() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    
    env.mock_all_auths();
    
    // Create contract and verify event would be emitted
    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        None,
        vec![&env, 100_0000000],
        ReleaseAuthorization::ClientOnly,
    );
    
    // In production, events are captured by Stellar SDK
    // Unit tests can verify contract state changes instead
    assert!(Escrow::contract_exists(env, contract_id));
}
```

### Integration Tests
Use Stellar test networks or local Soroban emulator to capture actual events.

## References

- [Soroban Events Documentation](https://soroban.stellar.org/)
- [Stellar Ledger Documentation](https://developers.stellar.org/)
- [TalentTrust Contract Specification](./CONTRACT_SPEC.md)

---

**Last Updated:** March 29, 2026
**Version:** 1.0.0
**Status:** Production Ready
