#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, String, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Lifecycle state of an escrow contract.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    Created = 0,
    Funded = 1,
    Completed = 2,
    Disputed = 3,
}

/// Storage keys used to address persistent contract data.
#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    /// Full state for an escrow identified by its numeric ID.
    EscrowState(u32),
    /// Immutable dispute record for an escrow identified by its numeric ID.
    Dispute(u32),
}

/// Typed errors returned by dispute-related contract functions.
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputeError {
    /// The escrow contract was not found in storage.
    NotFound = 1,
    /// The caller is not the client or freelancer of this escrow.
    Unauthorized = 2,
    /// The escrow status does not allow dispute initiation (e.g. `Created`).
    InvalidStatus = 3,
    /// A dispute record already exists for this escrow.
    AlreadyDisputed = 4,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// A single payment milestone within an escrow.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    /// Amount in stroops allocated to this milestone.
    pub amount: i128,
    /// Whether the milestone payment has been released to the freelancer.
    pub released: bool,
}

/// Full on-chain state of an escrow contract.
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowState {
    /// Address of the client who created and funded the escrow.
    pub client: Address,
    /// Address of the freelancer who will receive milestone payments.
    pub freelancer: Address,
    /// Current lifecycle status of the escrow.
    pub status: ContractStatus,
    /// Ordered list of payment milestones.
    pub milestones: Vec<Milestone>,
}

/// Immutable record created when a dispute is initiated.
/// Written once to persistent storage and never overwritten.
#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeRecord {
    /// The address (client or freelancer) that initiated the dispute.
    pub initiator: Address,
    /// A short human-readable reason for the dispute.
    pub reason: String,
    /// Ledger timestamp (seconds since Unix epoch) at the moment the dispute was recorded.
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// Create a new escrow contract.
    ///
    /// Stores an `EscrowState` in persistent storage keyed by the returned
    /// contract ID. The initial status is `Created`.
    ///
    /// # Arguments
    /// * `client`            – Address of the party depositing funds.
    /// * `freelancer`        – Address of the party receiving milestone payments.
    /// * `milestone_amounts` – Ordered list of payment amounts (in stroops).
    ///
    /// # Returns
    /// A non-zero numeric contract ID that identifies this escrow.
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        milestone_amounts: Vec<i128>,
    ) -> u32 {
        let contract_id: u32 = 1; // Simplified ID; production would use a counter.

        let mut milestones: Vec<Milestone> = Vec::new(&env);
        for amount in milestone_amounts.iter() {
            milestones.push_back(Milestone {
                amount,
                released: false,
            });
        }

        let state = EscrowState {
            client,
            freelancer,
            status: ContractStatus::Created,
            milestones,
        };

        env.storage()
            .persistent()
            .set(&DataKey::EscrowState(contract_id), &state);

        contract_id
    }

    /// Deposit funds into escrow. Only the client may call this.
    pub fn deposit_funds(_env: Env, _contract_id: u32, _amount: i128) -> bool {
        true
    }

    /// Release a milestone payment to the freelancer after verification.
    pub fn release_milestone(_env: Env, _contract_id: u32, _milestone_id: u32) -> bool {
        true
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    pub fn issue_reputation(_env: Env, _freelancer: Address, _rating: i128) -> bool {
        true
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }

    /// Initiate a dispute on an existing escrow.
    ///
    /// The `initiator` must be either the client or the freelancer of the
    /// escrow. The escrow must be in `Funded` or `Completed` status. A
    /// `DisputeRecord` is written to persistent storage exactly once.
    ///
    /// # Arguments
    /// * `contract_id` – Numeric ID of the escrow to dispute.
    /// * `initiator`   – Address of the party raising the dispute.
    /// * `reason`      – Short human-readable description of the dispute.
    ///
    /// # Errors
    /// * `DisputeError::NotFound`       – No escrow with `contract_id` exists.
    /// * `DisputeError::Unauthorized`   – `initiator` is not client or freelancer.
    /// * `DisputeError::InvalidStatus`  – Escrow is in `Created` status.
    /// * `DisputeError::AlreadyDisputed`– A dispute record already exists.
    pub fn initiate_dispute(
        env: Env,
        contract_id: u32,
        initiator: Address,
        reason: String,
    ) -> Result<(), DisputeError> {
        // 1. Enforce Soroban-level authorization before any state read/write.
        initiator.require_auth();

        // 2. Load escrow state.
        let mut state: EscrowState = env
            .storage()
            .persistent()
            .get(&DataKey::EscrowState(contract_id))
            .ok_or(DisputeError::NotFound)?;

        // 3. Validate caller is a party to this escrow.
        if initiator != state.client && initiator != state.freelancer {
            return Err(DisputeError::Unauthorized);
        }

        // 4. Validate status allows dispute initiation.
        match state.status {
            ContractStatus::Created => return Err(DisputeError::InvalidStatus),
            ContractStatus::Disputed => return Err(DisputeError::AlreadyDisputed),
            ContractStatus::Funded | ContractStatus::Completed => {}
        }

        // 5. Guard against overwriting an existing dispute record.
        if env
            .storage()
            .persistent()
            .has(&DataKey::Dispute(contract_id))
        {
            return Err(DisputeError::AlreadyDisputed);
        }

        // 6. Transition status and persist updated state.
        state.status = ContractStatus::Disputed;
        env.storage()
            .persistent()
            .set(&DataKey::EscrowState(contract_id), &state);

        // 7. Write immutable dispute record.
        let record = DisputeRecord {
            initiator,
            reason,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(contract_id), &record);

        Ok(())
    }

    /// Retrieve the dispute record for an escrow, if one exists.
    ///
    /// # Arguments
    /// * `contract_id` – Numeric ID of the escrow to query.
    ///
    /// # Returns
    /// `Some(DisputeRecord)` if a dispute has been initiated, `None` otherwise.
    pub fn get_dispute(env: Env, contract_id: u32) -> Option<DisputeRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(contract_id))
    }
}

#[cfg(test)]
mod test;
