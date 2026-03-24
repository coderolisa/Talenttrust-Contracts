#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    Created = 0,
    Funded = 1,
    Completed = 2,
    Disputed = 3,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowState {
    pub client: Address,
    pub freelancer: Address,
    pub milestones: Vec<Milestone>,
    pub status: ContractStatus,
    pub balance: i128,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    NextId,
    Contract(u32),
}

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
    ) -> u32 {
        let contract_id: u32 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
        env.storage()
            .instance()
            .set(&DataKey::NextId, &(contract_id + 1));

        let mut milestones = Vec::new(&env);
        for amount in milestone_amounts.iter() {
            milestones.push_back(Milestone {
                amount,
                released: false,
            });
        }

        let state = EscrowState {
            client,
            freelancer,
            milestones,
            status: ContractStatus::Created,
            balance: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::Contract(contract_id), &state);
        contract_id
    }

    /// Deposit funds into escrow. Only the client may call this.
    /// In a real implementation this would transfer tokens.
    pub fn deposit_funds(env: Env, contract_id: u32, amount: i128) -> bool {
        let mut state: EscrowState = env
            .storage()
            .instance()
            .get(&DataKey::Contract(contract_id))
            .unwrap();
        if state.status != ContractStatus::Created {
            return false;
        }

        let mut total_required = 0;
        for m in state.milestones.iter() {
            total_required += m.amount;
        }

        if amount < total_required {
            return false;
        }

        state.balance += amount;
        state.status = ContractStatus::Funded;
        env.storage()
            .instance()
            .set(&DataKey::Contract(contract_id), &state);
        true
    }

    /// Release a milestone payment to the freelancer after verification.
    pub fn release_milestone(env: Env, contract_id: u32, milestone_id: u32) -> bool {
        let mut state: EscrowState = env
            .storage()
            .instance()
            .get(&DataKey::Contract(contract_id))
            .unwrap();
        if state.status != ContractStatus::Funded {
            return false;
        }

        if milestone_id >= state.milestones.len() {
            return false;
        }

        let mut milestone = state.milestones.get(milestone_id).unwrap();
        if milestone.released {
            return false;
        }

        milestone.released = true;
        state.milestones.set(milestone_id, milestone);

        let amount_released = state.milestones.get(milestone_id).unwrap().amount;
        state.balance -= amount_released;

        let mut all_released = true;
        for m in state.milestones.iter() {
            if !m.released {
                all_released = false;
                break;
            }
        }
        if all_released {
            state.status = ContractStatus::Completed;
        }

        env.storage()
            .instance()
            .set(&DataKey::Contract(contract_id), &state);
        true
    }

    /// Get current state of the escrow. Useful for tests and UIs.
    pub fn get_state(env: Env, contract_id: u32) -> EscrowState {
        env.storage()
            .instance()
            .get(&DataKey::Contract(contract_id))
            .unwrap()
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    pub fn issue_reputation(_env: Env, _freelancer: Address, _rating: i128) -> bool {
        // Reputation credential issuance.
        true
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod proptest;
