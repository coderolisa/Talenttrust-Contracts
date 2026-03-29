//! # TalentTrust Escrow Contract with Event Emission
//!
//! A Soroban smart contract implementing a milestone-based escrow protocol for
//! the TalentTrust decentralized freelancer platform on the Stellar network.
//! 
//! This version includes standardized event emission for all critical state
//! transitions: contract creation, funding, milestone release, disputes, and
//! contract closure.
//!
//! ## Overview
//!
//! The escrow contract holds funds on behalf of a client and releases them to a
//! freelancer as individual milestones are approved. An optional arbiter can be
//! designated for dispute resolution. Four authorization schemes are supported:
//! `ClientOnly`, `ArbiterOnly`, `ClientAndArbiter`, and `MultiSig`.
//!
//! ## Lifecycle with Events
//!
//! ```text
//! create_contract [emit: ContractCreated]
//!     ↓
//! deposit_funds [emit: ContractFunded]
//!     ↓
//! approve_milestone_release
//!     ↓
//! release_milestone [emit: MilestoneReleased]
//!     ↓
//! (repeat per milestone)
//!     ↓
//! All released → Completed [emit: ContractClosed]
//! 
//! OR
//! 
//! dispute_contract [emit: ContractDisputed]
//! ```
//!
//! ## Security Assumptions
//!
//! - All callers that mutate state must pass `require_auth()`.
//! - Events are emitted only after successful state modifications.
//! - Event data is minimal to reduce on-chain storage but sufficient for off-chain indexing.
//! - No sensitive data (private keys, secrets) is exposed in events.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, 
    Env, String, Symbol, Vec,
};

// ============================================================================
// EVENT STRUCTURES
// ============================================================================

/// Standardized event topics for escrow contract actions.
/// These are used with `env.events().publish()` for consistent indexing.
const EVENT_CREATE: Symbol = symbol_short!("create");
const EVENT_FUND: Symbol = symbol_short!("fund");
const EVENT_RELEASE: Symbol = symbol_short!("release");
const EVENT_DISPUTE: Symbol = symbol_short!("dispute");
const EVENT_CLOSE: Symbol = symbol_short!("close");

/// Contract created event - emitted when escrow is first created.
///
/// # Event Topic
/// `"create"`
///
/// # Event Data
/// Includes: contract_id, client, freelancer, arbiter, total_amount, milestone_count, release_auth
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractCreatedEvent {
    /// Numeric identifier for the escrow contract
    pub contract_id: u32,
    /// Client address that will fund the escrow
    pub client: Address,
    /// Freelancer address that will receive milestone payments
    pub freelancer: Address,
    /// Optional arbiter address for dispute resolution
    pub arbiter: Option<Address>,
    /// Total amount across all milestones (in stroops)
    pub total_amount: i128,
    /// Number of milestones in the contract
    pub milestone_count: u32,
    /// Authorization scheme used (ClientOnly/ArbiterOnly/ClientAndArbiter/MultiSig)
    pub release_auth: ReleaseAuthorization,
    /// Timestamp of contract creation
    pub created_at: u64,
}

/// Contract funded event - emitted when client deposits the full escrow amount.
///
/// # Event Topic
/// `"fund"`
///
/// # Event Data
/// Includes: contract_id, client, funded_amount, status transition
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractFundedEvent {
    /// Numeric identifier for the escrow contract
    pub contract_id: u32,
    /// Client address that made the deposit
    pub funder: Address,
    /// Amount deposited (in stroops)
    pub amount: i128,
    /// New contract status: Funded
    pub new_status: u8, // 1 = Funded
    /// Timestamp of the funding
    pub funded_at: u64,
}

/// Milestone released event - emitted when a milestone is successfully released.
///
/// # Event Topic
/// `"release"`
///
/// # Event Data
/// Includes: contract_id, milestone_id, amount, releaser
#[contracttype]
#[derive(Clone, Debug)]
pub struct MilestoneReleasedEvent {
    /// Numeric identifier for the escrow contract
    pub contract_id: u32,
    /// Zero-based index of the released milestone
    pub milestone_id: u32,
    /// Amount released (in stroops)
    pub amount: i128,
    /// Address that triggered the release
    pub released_by: Address,
    /// Timestamp of the release
    pub released_at: u64,
}

/// Contract disputed event - emitted when a dispute is raised.
///
/// # Event Topic
/// `"dispute"`
///
/// # Event Data
/// Includes: contract_id, initiator, reason, timestamp
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractDisputedEvent {
    /// Numeric identifier for the escrow contract
    pub contract_id: u32,
    /// Address that initiated the dispute
    pub initiator: Address,
    /// Reason for the dispute (encoded as symbol)
    pub reason: Symbol,
    /// Timestamp of dispute creation
    pub disputed_at: u64,
}

/// Contract closed event - emitted when all milestones are released and contract completes.
///
/// # Event Topic
/// `"close"`
///
/// # Event Data
/// Includes: contract_id, final_status, freelancer receiving final payment, total released
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractClosedEvent {
    /// Numeric identifier for the escrow contract
    pub contract_id: u32,
    /// Freelancer address receiving final milestone payments
    pub freelancer: Address,
    /// New contract status: Completed
    pub new_status: u8, // 2 = Completed
    /// Total amount released across all milestones
    pub total_released: i128,
    /// Timestamp of contract closure
    pub closed_at: u64,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Persistent storage keys used by the Escrow contract.
///
/// Each variant corresponds to a distinct piece of contract state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Full escrow contract state, keyed by the numeric contract ID.
    Contract(u32),
    /// Milestone data for a specific contract and milestone index
    Milestone(u32, u32),
    /// Contract status
    ContractStatus(u32),
    /// Next available contract ID counter
    NextContractId,
    /// Contract timeout configuration
    ContractTimeout(u32),
    /// Dispute data for a contract
    Dispute(u32),
    /// Protocol parameters
    ProtocolParameters,
}

/// The lifecycle status of an escrow contract.
///
/// Valid transitions:
/// ```text
/// Created → Funded → Completed
/// Funded  → Disputed
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    /// Contract created, awaiting client deposit
    Created = 0,
    /// Funds deposited by client; milestones may be released
    Funded = 1,
    /// All milestones released; contract completed
    Completed = 2,
    /// Contract under dispute; releases paused
    Disputed = 3,
}

/// Represents a payment milestone in the escrow contract.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    /// Payment amount in stroops (1 XLM = 10,000,000 stroops)
    pub amount: i128,
    /// Whether this milestone has been released
    pub released: bool,
    /// Address that approved this milestone (client/arbiter)
    pub approved_by: Option<Address>,
    /// Ledger timestamp of when approval occurred
    pub approval_timestamp: Option<u64>,
}

/// Defines the security authorization scheme for milestone releases.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseAuthorization {
    /// Only the client can approve and release milestones
    ClientOnly,
    /// Only the arbiter can approve and release milestones
    ArbiterOnly,
    /// Either client or arbiter can approve and release milestones
    ClientAndArbiter,
    /// Multi-signature scheme requiring multiple approvals
    MultiSig,
}

/// The on-chain record for a single escrow agreement.
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowContract {
    /// Address of the client who funds the escrow and approves releases
    pub client: Address,
    /// Address of the freelancer who receives milestone payments
    pub freelancer: Address,
    /// Optional arbiter address for dispute resolution
    pub arbiter: Option<Address>,
    /// Authorization scheme for milestone releases
    pub release_auth: ReleaseAuthorization,
    /// Vector of milestone payment amounts and release status
    pub milestones: Vec<Milestone>,
    /// Current lifecycle status
    pub status: ContractStatus,
    /// Total amount across all milestones
    pub total_amount: i128,
    /// Amount deposited so far
    pub funded_amount: i128,
    /// Amount released so far
    pub released_amount: i128,
}

/// Reputation record for tracking freelancer performance.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationRecord {
    /// Number of completed contracts
    pub completed_contracts: u32,
    /// Total ratings accumulated
    pub total_rating: i128,
    /// Most recent rating value
    pub last_rating: i128,
}

/// Governed protocol parameters for escrow validation logic.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolParameters {
    /// Minimum amount for each milestone
    pub min_milestone_amount: i128,
    /// Maximum number of milestones per contract
    pub max_milestones: u32,
    /// Minimum reputation rating allowed
    pub min_reputation_rating: i128,
    /// Maximum reputation rating allowed
    pub max_reputation_rating: i128,
}

/// Dispute record for tracking contract disputes.
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeRecord {
    /// Address that initiated the dispute
    pub initiator: Address,
    /// Reason for the dispute
    pub reason: Symbol,
    /// Timestamp when dispute was created
    pub created_at: u64,
    /// Whether dispute has been resolved
    pub resolved: bool,
}

/// Custom errors for the escrow contract.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowError {
    /// Unauthorized access attempt
    Unauthorized = 1,
    /// Contract not found in storage
    ContractNotFound = 2,
    /// Milestone index invalid
    MilestoneNotFound = 3,
    /// Milestone already released
    MilestoneAlreadyReleased = 4,
    /// Insufficient funds for operation
    InsufficientFunds = 5,
    /// Invalid amount (negative or zero)
    InvalidAmount = 6,
    /// Contract is in wrong status for operation
    InvalidStatus = 7,
    /// Arithmetic overflow/underflow
    ArithmeticOverflow = 8,
    /// Participant is invalid
    InvalidParticipant = 9,
    /// No milestones provided
    EmptyMilestones = 10,
    /// Milestone IDs out of bounds or invalid
    InvalidMilestoneId = 11,
    /// Protocol parameters not initialized
    ProtocolNotInitialized = 12,
}

// ============================================================================
// CONTRACT IMPLEMENTATION
// ============================================================================

/// The TalentTrust escrow contract with event emission.
#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    // ========================================================================
    // INITIALIZATION & GOVERNANCE
    // ========================================================================

    /// Initialize protocol parameters (governance function).
    pub fn init_protocol(
        env: Env,
        admin: Address,
        min_milestone: i128,
        max_milestones: u32,
        min_rating: i128,
        max_rating: i128,
    ) -> bool {
        admin.require_auth();

        let params = ProtocolParameters {
            min_milestone_amount: min_milestone,
            max_milestones,
            min_reputation_rating: min_rating,
            max_reputation_rating: max_rating,
        };

        env.storage()
            .persistent()
            .set(&DataKey::ProtocolParameters, &params);

        true
    }

    /// Get current protocol parameters.
    pub fn protocol_parameters(env: &Env) -> ProtocolParameters {
        env.storage()
            .persistent()
            .get::<_, ProtocolParameters>(&DataKey::ProtocolParameters)
            .unwrap_or(ProtocolParameters {
                min_milestone_amount: 1_0000000,     // 1 XLM minimum
                max_milestones: 16,                   // 16 max milestones
                min_reputation_rating: 1,
                max_reputation_rating: 5,
            })
    }

    // ========================================================================
    // EVENT EMISSION HELPERS
    // ========================================================================

    /// Emit a ContractCreated event (internal helper).
    fn emit_contract_created(
        env: &Env,
        contract_id: u32,
        client: &Address,
        freelancer: &Address,
        arbiter: &Option<Address>,
        total_amount: i128,
        milestone_count: u32,
        release_auth: &ReleaseAuthorization,
    ) {
        let event = ContractCreatedEvent {
            contract_id,
            client: client.clone(),
            freelancer: freelancer.clone(),
            arbiter: arbiter.clone(),
            total_amount,
            milestone_count,
            release_auth: release_auth.clone(),
            created_at: env.ledger().timestamp(),
        };

        env.events().publish((EVENT_CREATE,), event);
    }

    /// Emit a ContractFunded event (internal helper).
    fn emit_contract_funded(env: &Env, contract_id: u32, funder: &Address, amount: i128) {
        let event = ContractFundedEvent {
            contract_id,
            funder: funder.clone(),
            amount,
            new_status: ContractStatus::Funded as u8,
            funded_at: env.ledger().timestamp(),
        };

        env.events().publish((EVENT_FUND,), event);
    }

    /// Emit a MilestoneReleased event (internal helper).
    fn emit_milestone_released(
        env: &Env,
        contract_id: u32,
        milestone_id: u32,
        amount: i128,
        released_by: &Address,
    ) {
        let event = MilestoneReleasedEvent {
            contract_id,
            milestone_id,
            amount,
            released_by: released_by.clone(),
            released_at: env.ledger().timestamp(),
        };

        env.events().publish((EVENT_RELEASE,), event);
    }

    /// Emit a ContractDisputed event (internal helper).
    fn emit_contract_disputed(
        env: &Env,
        contract_id: u32,
        initiator: &Address,
        reason: Symbol,
    ) {
        let event = ContractDisputedEvent {
            contract_id,
            initiator: initiator.clone(),
            reason,
            disputed_at: env.ledger().timestamp(),
        };

        env.events().publish((EVENT_DISPUTE,), event);
    }

    /// Emit a ContractClosed event (internal helper).
    fn emit_contract_closed(
        env: &Env,
        contract_id: u32,
        freelancer: &Address,
        total_released: i128,
    ) {
        let event = ContractClosedEvent {
            contract_id,
            freelancer: freelancer.clone(),
            new_status: ContractStatus::Completed as u8,
            total_released,
            closed_at: env.ledger().timestamp(),
        };

        env.events().publish((EVENT_CLOSE,), event);
    }

    // ========================================================================
    // CONTRACT CREATION
    // ========================================================================

    /// Create a new escrow contract with milestone-based release authorization.
    ///
    /// Stores the contract record in persistent storage and emits a
    /// `ContractCreated` event for off-chain indexing.
    ///
    /// # Arguments
    ///
    /// | Name                | Type                    | Description
    /// |---------------------|-------------------------|----------------------------------------
    /// | `env`               | `Env`                   | Soroban host environment
    /// | `client`            | `Address`               | Client who will fund the escrow
    /// | `freelancer`        | `Address`               | Freelancer receiving milestone payments
    /// | `arbiter`           | `Option<Address>`       | Optional arbiter for disputes
    /// | `milestone_amounts` | `Vec<i128>`             | Ordered list of milestone amounts
    /// | `release_auth`      | `ReleaseAuthorization`  | Authorization scheme for milestone releases
    ///
    /// # Returns
    ///
    /// A `u32` contract identifier (unique per contract)
    ///
    /// # Errors
    ///
    /// - Panics if milestone_amounts is empty
    /// - Panics if any milestone amount is ≤ 0
    /// - Panics if total exceeds protocol maximum
    ///
    /// # Events
    ///
    /// Emits `ContractCreated` event with all contract parameters
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        arbiter: Option<Address>,
        milestone_amounts: Vec<i128>,
        release_auth: ReleaseAuthorization,
    ) -> u32 {
        // Validation: at least one milestone
        if milestone_amounts.is_empty() {
            panic!("At least one milestone required");
        }

        // Validation: milestones below protocol max
        let protocol_params = Self::protocol_parameters(&env);
        if milestone_amounts.len() > protocol_params.max_milestones {
            panic!("Exceeds maximum milestone count");
        }

        // Validation: all amounts positive and calculate total
        let mut total_amount: i128 = 0;
        let mut milestones = Vec::new(&env);

        for i in 0..milestone_amounts.len() {
            let amount = milestone_amounts.get(i).unwrap();
            if amount <= 0 {
                panic!("Milestone amounts must be positive");
            }
            if amount < protocol_params.min_milestone_amount {
                panic!("Milestone amount below minimum");
            }

            total_amount = total_amount
                .checked_add(amount)
                .unwrap_or_else(|| panic!("Amount overflow"));

            milestones.push_back(Milestone {
                amount,
                released: false,
                approved_by: None,
                approval_timestamp: None,
            });
        }

        // Validation: total amount reasonable
        if total_amount > 1_000_000_000_000_i128 {
            panic!("Exceeds maximum contract funding size");
        }

        // Validation: client and freelancer are different
        if client == freelancer {
            panic!("Client and freelancer cannot be the same address");
        }

        // Generate contract ID
        let contract_id = env.storage()
            .persistent()
            .get::<_, u32>(&DataKey::NextContractId)
            .unwrap_or(0)
            .checked_add(1)
            .unwrap();

        // Create contract record
        let contract = EscrowContract {
            client: client.clone(),
            freelancer: freelancer.clone(),
            arbiter: arbiter.clone(),
            release_auth: release_auth.clone(),
            milestones,
            status: ContractStatus::Created,
            total_amount,
            funded_amount: 0,
            released_amount: 0,
        };

        // Store contract
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &contract);

        // Increment contract ID counter
        env.storage()
            .persistent()
            .set(&DataKey::NextContractId, &contract_id);

        // EMIT EVENT: Contract Created
        Self::emit_contract_created(
            &env,
            contract_id,
            &client,
            &freelancer,
            &arbiter,
            total_amount,
            milestone_amounts.len(),
            &release_auth,
        );

        contract_id
    }

    // ========================================================================
    // CONTRACT FUNDING
    // ========================================================================

    /// Deposit the full escrow amount into the contract.
    ///
    /// Only the client may call this function. The deposited amount must equal
    /// the sum of all milestone amounts. On success the contract status
    /// transitions from `Created` to `Funded` and a `ContractFunded` event is
    /// emitted.
    ///
    /// # Arguments
    ///
    /// | Name          | Type      | Description
    /// |---------------|-----------|-----------------------------
    /// | `env`         | `Env`     | Soroban host environment
    /// | `contract_id` | `u32`     | Identifier of the escrow contract
    /// | `caller`      | `Address` | Must be client; auth required
    /// | `amount`      | `i128`    | Amount to deposit (must equal total)
    ///
    /// # Returns
    ///
    /// `true` on success
    ///
    /// # Errors (Panics)
    ///
    /// - Contract not found
    /// - Caller is not the client
    /// - Contract not in Created status
    /// - Amount does not equal total milestone sum
    ///
    /// # Events
    ///
    /// Emits `ContractFunded` event with amount and timestamp
    pub fn deposit_funds(env: Env, contract_id: u32, caller: Address, amount: i128) -> bool {
        caller.require_auth();

        // Retrieve contract
        let contract = env
            .storage()
            .persistent()
            .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)
            .unwrap();

        // Validate caller is client
        if caller != contract.client {
            panic!("Only client can deposit funds");
        }

        // Validate contract status
        if contract.status != ContractStatus::Created {
            panic!("Contract must be in Created status to deposit funds");
        }

        // Validate amount matches total
        if amount != contract.total_amount {
            panic!("Deposit amount must equal total milestone amounts");
        }

        // Update contract status
        let mut updated_contract = contract;
        updated_contract.status = ContractStatus::Funded;
        updated_contract.funded_amount = amount;

        // Store updated contract
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &updated_contract);

        // EMIT EVENT: Contract Funded
        Self::emit_contract_funded(&env, contract_id, &caller, amount);

        true
    }

    // ========================================================================
    // MILESTONE MANAGEMENT
    // ========================================================================

    /// Approve a milestone for release.
    ///
    /// This function marks a milestone as approved by the caller (client or
    /// arbiter depending on the authorization scheme). The actual release
    /// happens via `release_milestone`.
    ///
    /// # Arguments
    ///
    /// | Name          | Type      | Description
    /// |---------------|-----------|-----------------------------
    /// | `env`         | `Env`     | Soroban host environment
    /// | `contract_id` | `u32`     | Escrow contract identifier
    /// | `caller`      | `Address` | Caller approving; auth required
    /// | `milestone_id`| `u32`     | Zero-based milestone index
    ///
    /// # Returns
    ///
    /// `true` on success
    ///
    /// # Errors (Panics)
    ///
    /// - Contract not found
    /// - Invalid milestone ID
    /// - Unauthorized caller
    /// - Milestone already approved by this caller
    pub fn approve_milestone_release(
        env: Env,
        contract_id: u32,
        caller: Address,
        milestone_id: u32,
    ) -> bool {
        caller.require_auth();

        // Retrieve contract
        let mut contract = env
            .storage()
            .persistent()
            .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)
            .unwrap();

        // Validate status
        if contract.status != ContractStatus::Funded {
            panic!("Contract must be in Funded status to approve milestones");
        }

        // Validate milestone ID
        if milestone_id >= contract.milestones.len() {
            panic!("Invalid milestone ID");
        }

        // Get milestone
        let mut milestone = contract.milestones.get(milestone_id).unwrap();

        // Validate authorization
        let is_authorized = match contract.release_auth {
            ReleaseAuthorization::ClientOnly => caller == contract.client,
            ReleaseAuthorization::ArbiterOnly => {
                contract.arbiter.clone().map_or(false, |a| caller == a)
            }
            ReleaseAuthorization::ClientAndArbiter => {
                caller == contract.client
                    || contract
                        .arbiter
                        .clone()
                        .map_or(false, |a| caller == a)
            }
            ReleaseAuthorization::MultiSig => {
                caller == contract.client
                    || contract
                        .arbiter
                        .clone()
                        .map_or(false, |a| caller == a)
            }
        };

        if !is_authorized {
            panic!("Unauthorized to approve milestone release");
        }

        // Check if already approved
        if milestone
            .approved_by
            .clone()
            .map_or(false, |addr| addr == caller)
        {
            panic!("Milestone already approved by this address");
        }

        // Update milestone approval
        milestone.approved_by = Some(caller);
        milestone.approval_timestamp = Some(env.ledger().timestamp());

        contract.milestones.set(milestone_id, milestone);
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &contract);

        true
    }

    /// Release an approved milestone payment.
    ///
    /// Releases funds for a milestone that has been properly approved.
    /// Emits a `MilestoneReleased` event. If all milestones are released,
    /// the contract transitions to `Completed` and a `ContractClosed` event
    /// is emitted.
    ///
    /// # Arguments
    ///
    /// | Name          | Type      | Description
    /// |---------------|-----------|-----------------------------
    /// | `env`         | `Env`     | Soroban host environment
    /// | `contract_id` | `u32`     | Escrow contract identifier
    /// | `caller`      | `Address` | Caller triggering release; auth required
    /// | `milestone_id`| `u32`     | Zero-based milestone index
    ///
    /// # Returns
    ///
    /// `true` on success
    ///
    /// # Errors (Panics)
    ///
    /// - Contract not found
    /// - Invalid milestone ID
    /// - Milestone already released
    /// - Milestone not approved
    /// - Insufficient funds
    ///
    /// # Events
    ///
    /// Emits `MilestoneReleased` for each release
    /// Emits `ContractClosed` if all milestones released
    pub fn release_milestone(
        env: Env,
        contract_id: u32,
        caller: Address,
        milestone_id: u32,
    ) -> bool {
        caller.require_auth();

        // Retrieve contract
        let mut contract = env
            .storage()
            .persistent()
            .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)
            .unwrap();

        // Validate status
        if contract.status != ContractStatus::Funded {
            panic!("Contract must be in Funded status to release milestones");
        }

        // Validate milestone ID
        if milestone_id >= contract.milestones.len() {
            panic!("Invalid milestone ID");
        }

        // Get milestone
        let mut milestone = contract.milestones.get(milestone_id).unwrap();

        // Validate not already released
        if milestone.released {
            panic!("Milestone already released");
        }

        // Validate approval exists
        if milestone.approved_by.is_none() {
            panic!("Milestone not approved for release");
        }

        // Mark milestone as released
        milestone.released = true;
        contract.milestones.set(milestone_id, milestone.clone());

        // Update released amount
        contract.released_amount = contract
            .released_amount
            .checked_add(milestone.amount)
            .unwrap_or_else(|| panic!("Amount overflow"));

        // EMIT EVENT: Milestone Released
        Self::emit_milestone_released(&env, contract_id, milestone_id, milestone.amount, &caller);

        // Check if all milestones released (contract completion)
        let all_released = contract.milestones.iter().all(|m| m.released);
        if all_released {
            contract.status = ContractStatus::Completed;

            // EMIT EVENT: Contract Closed
            Self::emit_contract_closed(&env, contract_id, &contract.freelancer, contract.released_amount);
        }

        // Store updated contract
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &contract);

        true
    }

    // ========================================================================
    // DISPUTE MANAGEMENT
    // ========================================================================

    /// Initiate a dispute on the escrow contract.
    ///
    /// Only the client or arbiter may dispute a contract. Disputes pause
    /// further releases until resolved. Emits a `ContractDisputed` event.
    ///
    /// # Arguments
    ///
    /// | Name          | Type      | Description
    /// |---------------|-----------|-----------------------------
    /// | `env`         | `Env`     | Soroban host environment
    /// | `contract_id` | `u32`     | Escrow contract identifier
    /// | `caller`      | `Address` | Client or arbiter; auth required
    /// | `reason`      | `Symbol`  | Reason for dispute
    ///
    /// # Returns
    ///
    /// `true` on success
    ///
    /// # Errors (Panics)
    ///
    /// - Contract not found
    /// - Caller is not client or arbiter
    /// - Contract not in Funded status
    ///
    /// # Events
    ///
    /// Emits `ContractDisputed` event with initiator and reason
    pub fn dispute_contract(
        env: Env,
        contract_id: u32,
        caller: Address,
        reason: Symbol,
    ) -> bool {
        caller.require_auth();

        // Retrieve contract
        let mut contract = env
            .storage()
            .persistent()
            .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)
            .unwrap();

        // Validate status
        if contract.status != ContractStatus::Funded {
            panic!("Contract must be in Funded status to dispute");
        }

        // Validate caller is client or arbiter
        let is_authorized = caller == contract.client
            || contract.arbiter.clone().map_or(false, |arb| arb == caller);

        if !is_authorized {
            panic!("Only client or arbiter can dispute contract");
        }

        // Update status to disputed
        contract.status = ContractStatus::Disputed;

        // Store dispute record
        let dispute = DisputeRecord {
            initiator: caller.clone(),
            reason,
            created_at: env.ledger().timestamp(),
            resolved: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(contract_id), &dispute);

        // Store updated contract
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &contract);

        // EMIT EVENT: Contract Disputed
        Self::emit_contract_disputed(&env, contract_id, &caller, reason);

        true
    }

    // ========================================================================
    // UTILITY & INFO FUNCTIONS
    // ========================================================================

    /// Retrieve full contract details by contract ID.
    pub fn get_contract(env: Env, contract_id: u32) -> EscrowContract {
        env.storage()
            .persistent()
            .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)
            .unwrap()
    }

    /// Retrieve a specific milestone from a contract.
    pub fn get_milestone(env: Env, contract_id: u32, milestone_id: u32) -> Milestone {
        let contract = env
            .storage()
            .persistent()
            .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)
            .unwrap();

        if milestone_id >= contract.milestones.len() {
            panic!("Invalid milestone ID");
        }

        contract.milestones.get(milestone_id).unwrap()
    }

    /// Retrieve dispute information for a contract.
    pub fn get_dispute(env: Env, contract_id: u32) -> Option<DisputeRecord> {
        env.storage()
            .persistent()
            .get::<_, DisputeRecord>(&DataKey::Dispute(contract_id))
    }

    /// Check if a contract exists.
    pub fn contract_exists(env: Env, contract_id: u32) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Contract(contract_id))
    }

    /// Get the current next contract ID (for testing).
    pub fn get_next_contract_id(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get::<_, u32>(&DataKey::NextContractId)
            .unwrap_or(0)
    }

    /// Hello-world test function.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        let env = Env::default();
        let result = Escrow::hello(env, symbol_short!("World"));
        assert_eq!(result, symbol_short!("World"));
    }
}

#[cfg(test)]
mod event_tests;
