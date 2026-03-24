#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    Created = 0,
    Funded = 1,
    Completed = 2,
    Disputed = 3,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
    pub approved_by_client: bool,
    pub approved_by_arbiter: bool,
    pub last_approval_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowContract {
    pub client: Address,
    pub freelancer: Address,
    pub arbiter: Option<Address>,
    pub milestones: Vec<Milestone>,
    pub status: ContractStatus,
    pub release_auth: ReleaseAuthorization,
    pub created_at: u64,
    pub reputation_issued: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reputation {
    pub total_rating: i128,
    pub ratings_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    NextContractId,
    Contract(u32),
    Reputation(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    InvalidParticipants = 1,
    EmptyMilestones = 2,
    InvalidMilestoneAmount = 3,
    ContractNotFound = 4,
    AmountMustBePositive = 5,
    InvalidState = 6,
    InvalidMilestoneId = 7,
    MilestoneAlreadyReleased = 8,
    UnauthorizedRole = 9,
    InsufficientApprovals = 10,
    AlreadyApproved = 11,
    MissingArbiter = 12,
    InvalidArbiter = 13,
    InvalidDepositAmount = 14,
    InvalidRating = 15,
    ReputationAlreadyIssued = 16,
    FreelancerMismatch = 17,
    ArithmeticOverflow = 18,
}

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// Creates a new escrow contract and enforces role authentication.
    ///
    /// Security requirements:
    /// - `client` and `freelancer` must be distinct addresses.
    /// - `client` and `freelancer` must both authorize contract creation.
    /// - If an `arbiter` is supplied, it must be distinct and required by the
    ///   selected release authorization mode.
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        arbiter: Option<Address>,
        milestone_amounts: Vec<i128>,
        release_auth: ReleaseAuthorization,
    ) -> Result<u32, EscrowError> {
        if client == freelancer {
            return Err(EscrowError::InvalidParticipants);
        }

        client.require_auth();
        freelancer.require_auth();

        validate_arbiter_participants(&client, &freelancer, &arbiter)?;
        validate_release_mode_arbiter(&release_auth, &arbiter)?;

        if milestone_amounts.is_empty() {
            return Err(EscrowError::EmptyMilestones);
        }

        let mut milestones = Vec::new(&env);
        let mut i = 0_u32;
        while i < milestone_amounts.len() {
            let amount = milestone_amounts
                .get(i)
                .ok_or(EscrowError::InvalidMilestoneAmount)?;
            if amount <= 0 {
                return Err(EscrowError::InvalidMilestoneAmount);
            }
            milestones.push_back(Milestone {
                amount,
                released: false,
                approved_by_client: false,
                approved_by_arbiter: false,
                last_approval_timestamp: None,
            });
            i += 1;
        }

        let contract_id = next_contract_id(&env)?;

        let contract_data = EscrowContract {
            client,
            freelancer,
            arbiter,
            milestones,
            status: ContractStatus::Created,
            release_auth,
            created_at: env.ledger().timestamp(),
            reputation_issued: false,
        };

        save_contract(&env, contract_id, &contract_data);
        Ok(contract_id)
    }

    /// Deposits escrow funds.
    ///
    /// Access control:
    /// - Caller must be the contract `client`.
    pub fn deposit_funds(
        env: Env,
        contract_id: u32,
        caller: Address,
        amount: i128,
    ) -> Result<bool, EscrowError> {
        caller.require_auth();

        if amount <= 0 {
            return Err(EscrowError::AmountMustBePositive);
        }

        let mut contract = load_contract(&env, contract_id)?;

        if caller != contract.client {
            return Err(EscrowError::UnauthorizedRole);
        }

        if contract.status != ContractStatus::Created {
            return Err(EscrowError::InvalidState);
        }

        let required_amount = total_milestone_amount(&contract.milestones)?;
        if amount != required_amount {
            return Err(EscrowError::InvalidDepositAmount);
        }

        contract.status = ContractStatus::Funded;
        save_contract(&env, contract_id, &contract);
        Ok(true)
    }

    /// Approves a milestone for release.
    ///
    /// Access control is based on `release_auth` mode and enforces explicit
    /// client/arbiter roles for the caller.
    pub fn approve_milestone_release(
        env: Env,
        contract_id: u32,
        caller: Address,
        milestone_id: u32,
    ) -> Result<bool, EscrowError> {
        caller.require_auth();

        let mut contract = load_contract(&env, contract_id)?;

        if contract.status != ContractStatus::Funded {
            return Err(EscrowError::InvalidState);
        }

        let mut milestone = contract
            .milestones
            .get(milestone_id)
            .ok_or(EscrowError::InvalidMilestoneId)?;

        if milestone.released {
            return Err(EscrowError::MilestoneAlreadyReleased);
        }

        ensure_approval_actor_authorized(&contract, &caller)?;

        if caller == contract.client {
            if milestone.approved_by_client {
                return Err(EscrowError::AlreadyApproved);
            }
            milestone.approved_by_client = true;
        } else {
            let _arbiter = contract
                .arbiter
                .clone()
                .ok_or(EscrowError::MissingArbiter)?;
            if milestone.approved_by_arbiter {
                return Err(EscrowError::AlreadyApproved);
            }
            milestone.approved_by_arbiter = true;
        }

        milestone.last_approval_timestamp = Some(env.ledger().timestamp());
        contract.milestones.set(milestone_id, milestone);

        save_contract(&env, contract_id, &contract);
        Ok(true)
    }

    /// Releases a milestone payment.
    ///
    /// Access control:
    /// - Caller must be role-authorized for the configured release mode.
    /// - Required approvals must be present for the configured release mode.
    pub fn release_milestone(
        env: Env,
        contract_id: u32,
        caller: Address,
        milestone_id: u32,
    ) -> Result<bool, EscrowError> {
        caller.require_auth();

        let mut contract = load_contract(&env, contract_id)?;

        if contract.status != ContractStatus::Funded {
            return Err(EscrowError::InvalidState);
        }

        let mut milestone = contract
            .milestones
            .get(milestone_id)
            .ok_or(EscrowError::InvalidMilestoneId)?;

        if milestone.released {
            return Err(EscrowError::MilestoneAlreadyReleased);
        }

        ensure_release_actor_authorized(&contract, &caller)?;

        if !has_required_approvals(&contract, &milestone) {
            return Err(EscrowError::InsufficientApprovals);
        }

        milestone.released = true;
        contract.milestones.set(milestone_id, milestone);

        let all_released = contract.milestones.iter().all(|m| m.released);
        if all_released {
            contract.status = ContractStatus::Completed;
        }

        save_contract(&env, contract_id, &contract);
        Ok(true)
    }

    /// Issues a reputation credential after contract completion.
    ///
    /// Access control:
    /// - Caller must be the contract `client`.
    /// - Supplied `freelancer` must match the contract `freelancer`.
    pub fn issue_reputation(
        env: Env,
        contract_id: u32,
        caller: Address,
        freelancer: Address,
        rating: i128,
    ) -> Result<bool, EscrowError> {
        caller.require_auth();

        if !(1..=5).contains(&rating) {
            return Err(EscrowError::InvalidRating);
        }

        let mut contract = load_contract(&env, contract_id)?;

        if contract.status != ContractStatus::Completed {
            return Err(EscrowError::InvalidState);
        }

        if caller != contract.client {
            return Err(EscrowError::UnauthorizedRole);
        }

        if freelancer != contract.freelancer {
            return Err(EscrowError::FreelancerMismatch);
        }

        if contract.reputation_issued {
            return Err(EscrowError::ReputationAlreadyIssued);
        }

        let key = DataKey::Reputation(freelancer.clone());
        let mut reputation = env
            .storage()
            .persistent()
            .get::<_, Reputation>(&key)
            .unwrap_or(Reputation {
                total_rating: 0,
                ratings_count: 0,
            });

        reputation.total_rating = reputation
            .total_rating
            .checked_add(rating)
            .ok_or(EscrowError::ArithmeticOverflow)?;
        reputation.ratings_count = reputation
            .ratings_count
            .checked_add(1)
            .ok_or(EscrowError::ArithmeticOverflow)?;

        env.storage().persistent().set(&key, &reputation);

        contract.reputation_issued = true;
        save_contract(&env, contract_id, &contract);

        Ok(true)
    }

    /// Returns an escrow contract by id.
    pub fn get_contract(env: Env, contract_id: u32) -> Result<EscrowContract, EscrowError> {
        load_contract(&env, contract_id)
    }

    /// Returns reputation aggregate for the provided freelancer.
    pub fn get_reputation(env: Env, freelancer: Address) -> Reputation {
        env.storage()
            .persistent()
            .get::<_, Reputation>(&DataKey::Reputation(freelancer))
            .unwrap_or(Reputation {
                total_rating: 0,
                ratings_count: 0,
            })
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

fn validate_arbiter_participants(
    client: &Address,
    freelancer: &Address,
    arbiter: &Option<Address>,
) -> Result<(), EscrowError> {
    if let Some(a) = arbiter {
        if a == client || a == freelancer {
            return Err(EscrowError::InvalidArbiter);
        }
    }
    Ok(())
}

fn validate_release_mode_arbiter(
    mode: &ReleaseAuthorization,
    arbiter: &Option<Address>,
) -> Result<(), EscrowError> {
    match mode {
        ReleaseAuthorization::ClientOnly => Ok(()),
        ReleaseAuthorization::ClientAndArbiter
        | ReleaseAuthorization::ArbiterOnly
        | ReleaseAuthorization::MultiSig => {
            if arbiter.is_none() {
                return Err(EscrowError::MissingArbiter);
            }
            Ok(())
        }
    }
}

fn total_milestone_amount(milestones: &Vec<Milestone>) -> Result<i128, EscrowError> {
    let mut total = 0_i128;
    let mut i = 0_u32;
    while i < milestones.len() {
        let amount = milestones
            .get(i)
            .ok_or(EscrowError::InvalidMilestoneId)?
            .amount;
        total = total
            .checked_add(amount)
            .ok_or(EscrowError::ArithmeticOverflow)?;
        i += 1;
    }
    Ok(total)
}

fn has_required_approvals(contract: &EscrowContract, milestone: &Milestone) -> bool {
    match contract.release_auth {
        ReleaseAuthorization::ClientOnly => milestone.approved_by_client,
        ReleaseAuthorization::ArbiterOnly => milestone.approved_by_arbiter,
        ReleaseAuthorization::ClientAndArbiter => {
            milestone.approved_by_client || milestone.approved_by_arbiter
        }
        ReleaseAuthorization::MultiSig => {
            milestone.approved_by_client && milestone.approved_by_arbiter
        }
    }
}

fn ensure_approval_actor_authorized(
    contract: &EscrowContract,
    caller: &Address,
) -> Result<(), EscrowError> {
    match contract.release_auth {
        ReleaseAuthorization::ClientOnly => {
            if *caller != contract.client {
                return Err(EscrowError::UnauthorizedRole);
            }
            Ok(())
        }
        ReleaseAuthorization::ArbiterOnly => {
            let arbiter = contract
                .arbiter
                .clone()
                .ok_or(EscrowError::MissingArbiter)?;
            if *caller != arbiter {
                return Err(EscrowError::UnauthorizedRole);
            }
            Ok(())
        }
        ReleaseAuthorization::ClientAndArbiter | ReleaseAuthorization::MultiSig => {
            if *caller == contract.client {
                return Ok(());
            }

            let arbiter = contract
                .arbiter
                .clone()
                .ok_or(EscrowError::MissingArbiter)?;
            if *caller != arbiter {
                return Err(EscrowError::UnauthorizedRole);
            }
            Ok(())
        }
    }
}

fn ensure_release_actor_authorized(
    contract: &EscrowContract,
    caller: &Address,
) -> Result<(), EscrowError> {
    ensure_approval_actor_authorized(contract, caller)
}

fn next_contract_id(env: &Env) -> Result<u32, EscrowError> {
    let key = DataKey::NextContractId;
    let storage = env.storage().persistent();

    let id = storage.get::<_, u32>(&key).unwrap_or(1);
    let next = id.checked_add(1).ok_or(EscrowError::ArithmeticOverflow)?;
    storage.set(&key, &next);

    Ok(id)
}

fn load_contract(env: &Env, contract_id: u32) -> Result<EscrowContract, EscrowError> {
    env.storage()
        .persistent()
        .get::<_, EscrowContract>(&DataKey::Contract(contract_id))
        .ok_or(EscrowError::ContractNotFound)
}

fn save_contract(env: &Env, contract_id: u32, contract: &EscrowContract) {
    env.storage()
        .persistent()
        .set(&DataKey::Contract(contract_id), contract);
}

#[cfg(test)]
mod test;
