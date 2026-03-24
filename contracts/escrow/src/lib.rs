#![no_std]

mod types;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec, String, panic_with_error};
use types::*;

#[contract]
pub struct Escrow;

#[contractimpl]
impl Escrow {
    /// Create a new escrow contract. Client and freelancer addresses are stored
    /// for access control. Milestones define payment amounts.
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        milestone_amounts: Vec<i128>,
    ) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }

        let mut milestones = Vec::new(&env);
        for amount in milestone_amounts.iter() {
            milestones.push_back(Milestone {
                amount,
                released: false,
                work_evidence: None,
            });
        }

        env.storage().instance().set(&DataKey::Client, &client);
        env.storage().instance().set(&DataKey::Freelancer, &freelancer);
        env.storage().instance().set(&DataKey::Milestones, &milestones);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    /// Deposit funds into escrow. Only the client may call this.
    /// In a real implementation, this would involve transferring tokens.
    pub fn deposit_funds(env: Env, _amount: i128) {
        let client: Address = env.storage().instance().get(&DataKey::Client).unwrap();
        client.require_auth();
        
        // Deposit logic (e.g., token transfer) would go here.
        // For now, we assume funds are handled externally or by the contract balance.
    }

    /// Release a milestone payment to the freelancer after verification.
    /// This implementation includes idempotent protection and metadata storage.
    pub fn release_milestone(env: Env, milestone_id: u32, work_evidence: String) {
        let client: Address = env.storage().instance().get(&DataKey::Client).expect("not initialized");
        client.require_auth();

        let mut milestones = Self::get_milestones(env.clone());

        if let Err(e) = Self::validate_milestone_release(&milestones, milestone_id) {
            panic_with_error!(&env, e);
        }

        let mut milestone = milestones.get(milestone_id).unwrap();
        milestone.released = true;
        milestone.work_evidence = Some(work_evidence);

        milestones.set(milestone_id, milestone);
        env.storage().instance().set(&DataKey::Milestones, &milestones);
    }

    // --- Internal Logic ---

    fn validate_milestone_release(milestones: &Vec<Milestone>, id: u32) -> Result<(), Error> {
        if id >= milestones.len() {
            return Err(Error::IndexOutOfBounds);
        }
        if milestones.get(id).unwrap().released {
            return Err(Error::AlreadyReleased);
        }
        Ok(())
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    pub fn issue_reputation(env: Env, _rating: i128) {
        let client: Address = env.storage().instance().get(&DataKey::Client).unwrap();
        client.require_auth();
        // Reputation credential issuance logic.
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }

    /// Getter for milestones (useful for verification and UI)
    pub fn get_milestones(env: Env) -> Vec<Milestone> {
        env.storage().instance().get(&DataKey::Milestones).unwrap_or(Vec::new(&env))
    }
}

#[cfg(test)]
mod test;
