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
#[derive(Clone, Debug)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Admin,
    Paused,
    EmergencyPaused,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ContractPaused = 3,
    NotPaused = 4,
    EmergencyActive = 5,
}

#[contract]
pub struct Escrow;

impl Escrow {
    fn read_admin(env: &Env) -> Result<Address, EscrowError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::NotInitialized)
    }

    fn require_admin(env: &Env) -> Result<(), EscrowError> {
        let admin = Self::read_admin(env)?;
        admin.require_auth();
        Ok(())
    }

    fn is_paused_internal(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    fn is_emergency_internal(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::EmergencyPaused)
            .unwrap_or(false)
    }

    fn ensure_not_paused(env: &Env) -> Result<(), EscrowError> {
        if Self::is_paused_internal(env) {
            return Err(EscrowError::ContractPaused);
        }
        Ok(())
    }
}

#[contractimpl]
impl Escrow {
    /// Initializes admin-managed pause controls.
    ///
    /// # Errors
    /// - [`EscrowError::AlreadyInitialized`] if admin is already set.
    pub fn initialize(env: Env, admin: Address) -> Result<(), EscrowError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(EscrowError::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::EmergencyPaused, &false);
        Ok(())
    }

    /// Returns the configured administrator.
    ///
    /// # Errors
    /// - [`EscrowError::NotInitialized`] if admin is not configured.
    pub fn get_admin(env: Env) -> Result<Address, EscrowError> {
        Self::read_admin(&env)
    }

    /// Pauses mutating operations for incident response.
    ///
    /// # Errors
    /// - [`EscrowError::NotInitialized`] if admin is not configured.
    pub fn pause(env: Env) -> Result<bool, EscrowError> {
        Self::require_admin(&env)?;
        env.storage().instance().set(&DataKey::Paused, &true);
        Ok(true)
    }

    /// Lifts a normal pause.
    ///
    /// # Errors
    /// - [`EscrowError::NotInitialized`] if admin is not configured.
    /// - [`EscrowError::NotPaused`] if contract is already active.
    /// - [`EscrowError::EmergencyActive`] if emergency lock is active.
    pub fn unpause(env: Env) -> Result<bool, EscrowError> {
        Self::require_admin(&env)?;

        if Self::is_emergency_internal(&env) {
            return Err(EscrowError::EmergencyActive);
        }
        if !Self::is_paused_internal(&env) {
            return Err(EscrowError::NotPaused);
        }

        env.storage().instance().set(&DataKey::Paused, &false);
        Ok(true)
    }

    /// Activates emergency mode and hard-pauses the contract.
    ///
    /// # Errors
    /// - [`EscrowError::NotInitialized`] if admin is not configured.
    pub fn activate_emergency_pause(env: Env) -> Result<bool, EscrowError> {
        Self::require_admin(&env)?;

        env.storage()
            .instance()
            .set(&DataKey::EmergencyPaused, &true);
        env.storage().instance().set(&DataKey::Paused, &true);
        Ok(true)
    }

    /// Clears emergency mode and restores active operation.
    ///
    /// # Errors
    /// - [`EscrowError::NotInitialized`] if admin is not configured.
    pub fn resolve_emergency(env: Env) -> Result<bool, EscrowError> {
        Self::require_admin(&env)?;

        env.storage()
            .instance()
            .set(&DataKey::EmergencyPaused, &false);
        env.storage().instance().set(&DataKey::Paused, &false);
        Ok(true)
    }

    /// Read-only pause status.
    pub fn is_paused(env: Env) -> bool {
        Self::is_paused_internal(&env)
    }

    /// Read-only emergency status.
    pub fn is_emergency(env: Env) -> bool {
        Self::is_emergency_internal(&env)
    }

    /// Create a new escrow contract. Client and freelancer addresses are stored
    /// for access control. Milestones define payment amounts.
    ///
    /// # Errors
    /// - [`EscrowError::ContractPaused`] if contract is paused.
    pub fn create_contract(
        env: Env,
        _client: Address,
        _freelancer: Address,
        _milestone_amounts: Vec<i128>,
    ) -> Result<u32, EscrowError> {
        Self::ensure_not_paused(&env)?;

        // Contract creation - returns a non-zero contract id placeholder.
        // Full implementation would store state in persistent storage.
        Ok(1)
    }

    /// Deposit funds into escrow. Only the client may call this.
    ///
    /// # Errors
    /// - [`EscrowError::ContractPaused`] if contract is paused.
    pub fn deposit_funds(env: Env, _contract_id: u32, _amount: i128) -> Result<bool, EscrowError> {
        Self::ensure_not_paused(&env)?;

        // Escrow deposit logic would go here.
        Ok(true)
    }

    /// Release a milestone payment to the freelancer after verification.
    ///
    /// # Errors
    /// - [`EscrowError::ContractPaused`] if contract is paused.
    pub fn release_milestone(
        env: Env,
        _contract_id: u32,
        _milestone_id: u32,
    ) -> Result<bool, EscrowError> {
        Self::ensure_not_paused(&env)?;

        // Release payment for the given milestone.
        Ok(true)
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    ///
    /// # Errors
    /// - [`EscrowError::ContractPaused`] if contract is paused.
    pub fn issue_reputation(
        env: Env,
        _freelancer: Address,
        _rating: i128,
    ) -> Result<bool, EscrowError> {
        Self::ensure_not_paused(&env)?;

        // Reputation credential issuance.
        Ok(true)
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

#[cfg(test)]
mod test;
