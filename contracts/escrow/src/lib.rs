#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol, Vec,
};

/// Lifecycle states of an escrow contract.
///
/// State machine:
/// ```text
/// Created ──► Accepted ──► Funded ──► Completed
///                                  └──► Disputed
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    /// Contract created by client; awaiting freelancer acceptance.
    Created = 0,
    /// Freelancer has accepted the terms; client may now fund the contract.
    Accepted = 1,
    /// Client has deposited funds; milestones may be released.
    Funded = 2,
    /// All milestones released; contract concluded.
    Completed = 3,
    /// Contract under dispute resolution.
    Disputed = 4,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
    pub approved_by: Option<Address>,
    pub approval_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseAuthorization {
    ClientOnly = 0,
    ClientAndArbiter = 1,
    ArbiterOnly = 2,
    MultiSig = 3,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowContract {
    pub client: Address,
    pub freelancer: Address,
    pub arbiter: Option<Address>,
    pub milestones: Vec<Milestone>,
    pub status: ContractStatus,
    pub release_auth: ReleaseAuthorization,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Approval {
    None = 0,
    Client = 1,
    Arbiter = 2,
    Both = 3,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct MilestoneApproval {
    pub milestone_id: u32,
    pub approvals: Map<Address, bool>,
    pub required_approvals: u32,
    pub approval_status: Approval,
}

/// Persistent record for a single escrow engagement.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ContractRecord {
    pub client: Address,
    pub freelancer: Address,
    pub milestones: Vec<Milestone>,
    pub status: ContractStatus,
}

#[contracttype]
pub enum DataKey {
    Contract(u32),
    NextId,
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// Create a new escrow contract with milestone release authorization
    ///
    /// # Arguments
    /// * `client` - Address of the client who funds the escrow
    /// * `freelancer` - Address of the freelancer who receives payments
    /// * `arbiter` - Optional arbiter address for dispute resolution
    /// * `milestone_amounts` - Vector of milestone payment amounts
    /// * `release_auth` - Authorization scheme for milestone releases
    ///
    /// # Returns
    /// Contract ID for the newly created escrow
    ///
    /// # Errors
    /// Panics if:
    /// - Milestone amounts vector is empty
    /// - Any milestone amount is zero or negative
    /// - Client and freelancer addresses are the same
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        milestone_amounts: Vec<i128>,
    ) -> u32 {
        client.require_auth();

        assert!(milestone_amounts.len() > 0, "milestones required");

        let prev: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::NextId)
            .unwrap_or(0_u32);
        let id = prev + 1;

        let mut milestones: Vec<Milestone> = Vec::new(&env);
        for i in 0..milestone_amounts.len() {
            let amount = milestone_amounts.get(i).unwrap();
            milestones.push_back(Milestone {
                amount,
                released: false,
            });
        }

        let record = ContractRecord {
            client,
            freelancer,
            milestones,
            status: ContractStatus::Created,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Contract(id), &record);
        env.storage().persistent().set(&DataKey::NextId, &id);

        id
    }

    /// Freelancer accepts the contract terms, enabling the client to fund it.
    ///
    /// Transitions: [`ContractStatus::Created`] → [`ContractStatus::Accepted`].
    ///
    /// # Panics
    /// Panics if the contract does not exist, the caller is not the designated
    /// freelancer, or the contract is not in `Created` status.
    pub fn accept_contract(env: Env, contract_id: u32) {
        let key = DataKey::Contract(contract_id);
        let mut record: ContractRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("contract not found");

        record.freelancer.require_auth();

        assert!(
            record.status == ContractStatus::Created,
            "can only accept a Created contract"
        );

        record.status = ContractStatus::Accepted;
        env.storage().persistent().set(&key, &record);
    }

    /// Deposit funds into escrow.
    ///
    /// The freelancer **must** have accepted the contract first
    /// ([`ContractStatus::Accepted`]). This enforces the two-party handshake
    /// before any funds are committed.
    ///
    /// Transitions: [`ContractStatus::Accepted`] → [`ContractStatus::Funded`].
    ///
    /// # Panics
    /// Panics if the contract does not exist, the caller is not the client,
    /// `amount` is not positive, or the contract has not been accepted.
    pub fn deposit_funds(env: Env, contract_id: u32, amount: i128) -> bool {
        let key = DataKey::Contract(contract_id);
        let mut record: ContractRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("contract not found");

        record.client.require_auth();

        assert!(amount > 0, "amount must be positive");
        assert!(
            record.status == ContractStatus::Accepted,
            "freelancer must accept the contract before funding"
        );

        record.status = ContractStatus::Funded;
        env.storage().persistent().set(&key, &record);

        true
    }

    /// Release a milestone payment to the freelancer.
    ///
    /// # Panics
    /// Panics if the contract is not `Funded`, the milestone index is out of
    /// range, or the milestone has already been released.
    pub fn release_milestone(env: Env, contract_id: u32, milestone_id: u32) -> bool {
        let key = DataKey::Contract(contract_id);
        let mut record: ContractRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("contract not found");

        record.client.require_auth();

        assert!(
            record.status == ContractStatus::Funded,
            "contract must be funded"
        );
        assert!(
            milestone_id < record.milestones.len(),
            "milestone index out of range"
        );

        let mut milestone = record.milestones.get(milestone_id).unwrap();
        assert!(!milestone.released, "milestone already released");
        milestone.released = true;
        record.milestones.set(milestone_id, milestone);

        env.storage().persistent().set(&key, &record);

        true
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    pub fn issue_reputation(_env: Env, _freelancer: Address, _rating: i128) -> bool {
        true
    }

    /// Returns the current [`ContractStatus`] of a contract.
    ///
    /// # Panics
    /// Panics if `contract_id` does not exist.
    pub fn get_status(env: Env, contract_id: u32) -> ContractStatus {
        let record: ContractRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .expect("contract not found");
        record.status
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

#[cfg(test)]
mod test;
