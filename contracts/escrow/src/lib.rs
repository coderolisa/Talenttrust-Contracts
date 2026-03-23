#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, Symbol, Vec,
};

/// Maximum fee basis points (100% = 10000 basis points)
pub const MAX_FEE_BASIS_POINTS: u32 = 10000;

/// Default protocol fee: 2.5% = 250 basis points
pub const DEFAULT_FEE_BASIS_POINTS: u32 = 250;

/// Default timeout duration: 30 days in seconds (30 * 24 * 60 * 60)
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 2_592_000;

/// Minimum timeout duration: 1 day in seconds
pub const MIN_TIMEOUT_SECONDS: u64 = 86_400;

/// Maximum timeout duration: 365 days in seconds
pub const MAX_TIMEOUT_SECONDS: u64 = 31_536_000;

/// Data keys for contract storage
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    TreasuryConfig,
    Contract(u32),
    Milestone(u32, u32),
    ContractStatus(u32),
    NextContractId,
    ContractTimeout(u32),
    MilestoneDeadline(u32, u32),
    DisputeDeadline(u32),
    LastActivity(u32),
    Dispute(u32),
    MilestoneComplete(u32, u32),
}

/// Status of an escrow contract
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractStatus {
    Created = 0,
    Funded = 1,
    Completed = 2,
    Disputed = 3,
    InDispute = 4,
}

/// Milestone structure for escrow payments
#[contracttype]
#[derive(Clone, Debug)]
pub struct Milestone {
    pub amount: i128,
    pub released: bool,
}

/// Timeout configuration for escrow contracts
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeoutConfig {
    /// Timeout duration in seconds
    pub duration: u64,
    /// Auto-resolve type: 0 = return to client, 1 = release to freelancer, 2 = split
    pub auto_resolve_type: u32,
}

/// Dispute structure for tracking disputes
#[contracttype]
#[derive(Clone, Debug)]
pub struct Dispute {
    /// Address that initiated the dispute
    pub initiator: Address,
    /// Reason for the dispute
    pub reason: Symbol,
    /// Timestamp when dispute was created
    pub created_at: u64,
    /// Whether dispute has been resolved
    pub resolved: bool,
}

/// Treasury configuration for protocol fee collection
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryConfig {
    /// Address where protocol fees are sent
    pub address: Address,
    /// Fee percentage in basis points (10000 = 100%)
    pub fee_basis_points: u32,
}

/// Escrow contract structure
#[contracttype]
#[derive(Clone, Debug)]
pub struct EscrowContract {
    pub client: Address,
    pub freelancer: Address,
    pub total_amount: i128,
    pub milestone_count: u32,
}

/// Custom errors for the escrow contract
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EscrowError {
    /// Treasury not initialized
    TreasuryNotInitialized = 1,
    /// Invalid fee percentage (exceeds 100%)
    InvalidFeePercentage = 2,
    /// Unauthorized access
    Unauthorized = 3,
    /// Contract not found
    ContractNotFound = 4,
    /// Milestone not found
    MilestoneNotFound = 5,
    /// Milestone already released
    MilestoneAlreadyReleased = 6,
    /// Insufficient funds
    InsufficientFunds = 7,
    /// Invalid amount
    InvalidAmount = 8,
    /// Treasury already initialized
    TreasuryAlreadyInitialized = 9,
    /// Arithmetic overflow
    ArithmeticOverflow = 10,
    /// Timeout not exceeded
    TimeoutNotExceeded = 11,
    /// Invalid timeout duration
    InvalidTimeout = 12,
    /// Milestone not marked complete
    MilestoneNotComplete = 13,
    /// Milestone already complete
    MilestoneAlreadyComplete = 14,
    /// Dispute not found
    DisputeNotFound = 15,
    /// Dispute already resolved
    DisputeAlreadyResolved = 16,
    /// Timeout already claimed
    TimeoutAlreadyClaimed = 17,
    /// No dispute active
    NoDisputeActive = 18,
}

#[contract]
pub struct Escrow;

/// Event topics for audit trail
pub mod topics {
    use soroban_sdk::symbol_short;
    pub const TREASURY_CONFIG_SET: soroban_sdk::Symbol = symbol_short!("TR_CFG");
    pub const PROTOCOL_FEE_COLLECTED: soroban_sdk::Symbol = symbol_short!("FEE");
    pub const TREASURY_PAYOUT: soroban_sdk::Symbol = symbol_short!("PAYOUT");
    pub const MILESTONE_RELEASED: soroban_sdk::Symbol = symbol_short!("RELEASE");
    pub const TIMEOUT_CLAIMED: soroban_sdk::Symbol = symbol_short!("TIMEOUT");
    pub const DISPUTE_RAISED: soroban_sdk::Symbol = symbol_short!("DISPUTE");
    pub const DISPUTE_RESOLVED: soroban_sdk::Symbol = symbol_short!("RESOLVED");
    pub const MILESTONE_COMPLETE: soroban_sdk::Symbol = symbol_short!("COMPLETE");
}

#[contractimpl]
impl Escrow {
    // ==================== TREASURY FUNCTIONS ====================

    /// Initialize the treasury configuration.
    /// Can only be called once by the contract deployer (admin).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must be authorized)
    /// * `treasury_address` - The address where protocol fees are sent
    /// * `fee_basis_points` - Fee percentage in basis points (10000 = 100%, 250 = 2.5%)
    ///
    /// # Errors
    /// * `TreasuryAlreadyInitialized` - If treasury is already configured
    /// * `InvalidFeePercentage` - If fee exceeds 100%
    /// * `Unauthorized` - If caller is not the admin
    pub fn initialize_treasury(
        env: Env,
        admin: Address,
        treasury_address: Address,
        fee_basis_points: u32,
    ) -> Result<(), EscrowError> {
        // Verify admin authorization
        admin.require_auth();

        // Check if treasury is already initialized
        if env.storage().persistent().has(&DataKey::TreasuryConfig) {
            return Err(EscrowError::TreasuryAlreadyInitialized);
        }

        // Validate fee percentage (max 100%)
        if fee_basis_points > MAX_FEE_BASIS_POINTS {
            return Err(EscrowError::InvalidFeePercentage);
        }

        // Store admin
        env.storage().persistent().set(&DataKey::Admin, &admin);

        // Create and store treasury config
        let config = TreasuryConfig {
            address: treasury_address.clone(),
            fee_basis_points,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TreasuryConfig, &config);

        // Emit audit event
        env.events().publish(
            (topics::TREASURY_CONFIG_SET,),
            (admin, treasury_address, fee_basis_points),
        );

        Ok(())
    }

    /// Update the treasury configuration.
    /// Only callable by the admin.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must be authorized)
    /// * `new_treasury_address` - The new treasury address
    /// * `new_fee_basis_points` - New fee percentage in basis points
    ///
    /// # Errors
    /// * `TreasuryNotInitialized` - If treasury not yet initialized
    /// * `InvalidFeePercentage` - If fee exceeds 100%
    /// * `Unauthorized` - If caller is not the admin
    pub fn update_treasury_config(
        env: Env,
        admin: Address,
        new_treasury_address: Address,
        new_fee_basis_points: u32,
    ) -> Result<(), EscrowError> {
        // Verify admin authorization
        admin.require_auth();

        // Verify caller is the stored admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        // Validate fee percentage
        if new_fee_basis_points > MAX_FEE_BASIS_POINTS {
            return Err(EscrowError::InvalidFeePercentage);
        }

        // Update treasury config
        let config = TreasuryConfig {
            address: new_treasury_address.clone(),
            fee_basis_points: new_fee_basis_points,
        };
        env.storage()
            .persistent()
            .set(&DataKey::TreasuryConfig, &config);

        // Emit audit event
        env.events().publish(
            (topics::TREASURY_CONFIG_SET,),
            (admin, new_treasury_address, new_fee_basis_points),
        );

        Ok(())
    }

    /// Get the current treasury configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Ok(TreasuryConfig)` - The current treasury configuration
    ///
    /// # Errors
    /// * `TreasuryNotInitialized` - If treasury not yet initialized
    pub fn get_treasury_config(env: Env) -> Result<TreasuryConfig, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::TreasuryConfig)
            .ok_or(EscrowError::TreasuryNotInitialized)
    }

    /// Calculate the protocol fee for a given amount.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `amount` - The payment amount to calculate fee for
    ///
    /// # Returns
    /// * `Ok(i128)` - The calculated fee amount
    ///
    /// # Errors
    /// * `TreasuryNotInitialized` - If treasury not yet initialized
    /// * `ArithmeticOverflow` - If calculation overflows
    pub fn calculate_protocol_fee(env: Env, amount: i128) -> Result<i128, EscrowError> {
        if amount < 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let config = Self::get_treasury_config(env)?;

        // Calculate fee: (amount * fee_basis_points) / 10000
        // Use checked arithmetic to prevent overflow
        let fee = amount
            .checked_mul(config.fee_basis_points as i128)
            .ok_or(EscrowError::ArithmeticOverflow)?
            .checked_div(MAX_FEE_BASIS_POINTS as i128)
            .ok_or(EscrowError::ArithmeticOverflow)?;

        Ok(fee)
    }

    /// Transfer protocol fees to the treasury address.
    /// Internal function used during milestone releases.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `token` - The token contract address
    /// * `amount` - The total payment amount
    ///
    /// # Returns
    /// * `Ok(i128)` - The net amount after fee deduction
    ///
    /// # Errors
    /// * Various `EscrowError` variants on failure
    fn transfer_protocol_fee(
        env: &Env,
        token: &Address,
        from: &Address,
        amount: i128,
    ) -> Result<i128, EscrowError> {
        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let config = Self::get_treasury_config(env.clone())?;
        let fee = Self::calculate_protocol_fee(env.clone(), amount)?;
        let net_amount = amount
            .checked_sub(fee)
            .ok_or(EscrowError::ArithmeticOverflow)?;

        if fee > 0 {
            // Transfer fee to treasury
            let token_client = token::Client::new(env, token);
            token_client.transfer(from, &config.address, &fee);

            // Emit audit event
            env.events().publish(
                (topics::PROTOCOL_FEE_COLLECTED,),
                (config.address.clone(), fee, amount),
            );
        }

        Ok(net_amount)
    }

    /// Direct payout to treasury (for manual fee collection or other purposes).
    /// Only callable by admin.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address (must be authorized)
    /// * `token` - The token contract address
    /// * `amount` - The amount to transfer to treasury
    ///
    /// # Errors
    /// * `Unauthorized` - If caller is not admin
    /// * `TreasuryNotInitialized` - If treasury not configured
    /// * `InvalidAmount` - If amount is invalid
    pub fn payout_treasury(
        env: Env,
        admin: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), EscrowError> {
        // Verify admin authorization
        admin.require_auth();

        // Verify caller is the stored admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        let config = Self::get_treasury_config(env.clone())?;

        // Transfer to treasury
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &config.address, &amount);

        // Emit audit event
        env.events()
            .publish((topics::TREASURY_PAYOUT,), (config.address, amount));

        Ok(())
    }

    // ==================== TIMEOUT FUNCTIONS ====================

    /// Set timeout configuration for a contract.
    /// Can be called by client during contract creation or later.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `duration` - Timeout duration in seconds
    /// * `auto_resolve_type` - Auto-resolve type (0=client, 1=freelancer, 2=split)
    ///
    /// # Errors
    /// * `ContractNotFound` - If contract doesn't exist
    /// * `InvalidTimeout` - If duration is outside valid range
    /// * `Unauthorized` - If caller is not the client
    pub fn set_contract_timeout(
        env: Env,
        contract_id: u32,
        duration: u64,
        auto_resolve_type: u32,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (_, client, _): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Client must authorize
        client.require_auth();

        // Validate timeout duration
        if duration < MIN_TIMEOUT_SECONDS || duration > MAX_TIMEOUT_SECONDS {
            return Err(EscrowError::InvalidTimeout);
        }

        // Validate auto-resolve type (0, 1, or 2)
        if auto_resolve_type > 2 {
            return Err(EscrowError::InvalidTimeout);
        }

        // Store timeout config
        let timeout_config = TimeoutConfig {
            duration,
            auto_resolve_type,
        };
        env.storage()
            .persistent()
            .set(&DataKey::ContractTimeout(contract_id), &timeout_config);

        // Initialize last activity
        Self::update_last_activity(&env, contract_id);

        Ok(())
    }

    /// Get timeout configuration for a contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    ///
    /// # Returns
    /// * `Ok(TimeoutConfig)` - The timeout configuration
    pub fn get_contract_timeout(env: Env, contract_id: u32) -> Result<TimeoutConfig, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::ContractTimeout(contract_id))
            .ok_or(EscrowError::ContractNotFound)
    }

    /// Check if timeout has been exceeded for a contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `last_activity` - The last activity timestamp
    ///
    /// # Returns
    /// * `Ok(bool)` - True if timeout exceeded, false otherwise
    fn is_timeout_exceeded(
        env: &Env,
        contract_id: u32,
        last_activity: u64,
    ) -> Result<bool, EscrowError> {
        let timeout_config: TimeoutConfig = env
            .storage()
            .persistent()
            .get(&DataKey::ContractTimeout(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        let current_time = env.ledger().timestamp();
        let deadline = last_activity
            .checked_add(timeout_config.duration)
            .ok_or(EscrowError::ArithmeticOverflow)?;

        Ok(current_time > deadline)
    }

    /// Update last activity timestamp for a contract.
    /// Internal function called on state-changing operations.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    fn update_last_activity(env: &Env, contract_id: u32) {
        let current_time = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::LastActivity(contract_id), &current_time);
    }

    /// Get last activity timestamp for a contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    ///
    /// # Returns
    /// * `Ok(u64)` - The last activity timestamp
    pub fn get_last_activity(env: Env, contract_id: u32) -> Result<u64, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::LastActivity(contract_id))
            .ok_or(EscrowError::ContractNotFound)
    }

    /// Mark a milestone as complete by the freelancer.
    /// This starts the timeout period for client to release payment.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `milestone_id` - The milestone index
    ///
    /// # Errors
    /// * `ContractNotFound` - If contract doesn't exist
    /// * `MilestoneNotFound` - If milestone doesn't exist
    /// * `MilestoneAlreadyComplete` - If already marked complete
    /// * `Unauthorized` - If caller is not the freelancer
    pub fn mark_milestone_complete(
        env: Env,
        contract_id: u32,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (_, _, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Freelancer must authorize
        freelancer.require_auth();

        // Retrieve milestone
        let milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(contract_id, milestone_id))
            .ok_or(EscrowError::MilestoneNotFound)?;

        // Check if already released
        if milestone.released {
            return Err(EscrowError::MilestoneAlreadyReleased);
        }

        // Check if already marked complete
        if env
            .storage()
            .persistent()
            .has(&DataKey::MilestoneComplete(contract_id, milestone_id))
        {
            return Err(EscrowError::MilestoneAlreadyComplete);
        }

        // Mark as complete with timestamp
        let current_time = env.ledger().timestamp();
        env.storage().persistent().set(
            &DataKey::MilestoneComplete(contract_id, milestone_id),
            &current_time,
        );

        // Update last activity
        Self::update_last_activity(&env, contract_id);

        // Emit event
        env.events().publish(
            (topics::MILESTONE_COMPLETE,),
            (contract_id, milestone_id, freelancer, current_time),
        );

        Ok(())
    }

    /// Check if a milestone is marked complete.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `milestone_id` - The milestone index
    ///
    /// # Returns
    /// * `Ok(bool)` - True if milestone is complete
    pub fn is_milestone_complete(env: Env, contract_id: u32, milestone_id: u32) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::MilestoneComplete(contract_id, milestone_id))
    }

    /// Claim milestone timeout - can be called by freelancer or client after timeout.
    /// Freelancer can claim if milestone marked complete but not released.
    /// Client can claim refund if milestone not marked complete.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `milestone_id` - The milestone index
    ///
    /// # Errors
    /// * `TimeoutNotExceeded` - If timeout period not yet passed
    /// * `Unauthorized` - If caller is not client or freelancer
    pub fn claim_milestone_timeout(
        env: Env,
        contract_id: u32,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (token, client, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Retrieve milestone
        let milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(contract_id, milestone_id))
            .ok_or(EscrowError::MilestoneNotFound)?;

        // Check if already released
        if milestone.released {
            return Err(EscrowError::MilestoneAlreadyReleased);
        }

        // Check if milestone is marked complete
        let is_complete = env
            .storage()
            .persistent()
            .has(&DataKey::MilestoneComplete(contract_id, milestone_id));

        // Get last activity timestamp
        let last_activity = if is_complete {
            env.storage()
                .persistent()
                .get(&DataKey::MilestoneComplete(contract_id, milestone_id))
                .unwrap_or(0)
        } else {
            Self::get_last_activity(env.clone(), contract_id)?
        };

        // Check if timeout exceeded
        if !Self::is_timeout_exceeded(&env, contract_id, last_activity)? {
            return Err(EscrowError::TimeoutNotExceeded);
        }

        // Determine who can claim and what action to take
        if is_complete {
            // Milestone marked complete - freelancer can claim payment
            // For simplicity, we require freelancer authorization
            freelancer.require_auth();

            // Calculate and transfer protocol fee
            let net_amount = Self::transfer_protocol_fee(
                &env,
                &token,
                &env.current_contract_address(),
                milestone.amount,
            )?;

            // Transfer net amount to freelancer
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(&env.current_contract_address(), &freelancer, &net_amount);
        } else {
            // Milestone not complete - client can claim refund
            client.require_auth();

            // Transfer full amount back to client
            let token_client = token::Client::new(&env, &token);
            token_client.transfer(&env.current_contract_address(), &client, &milestone.amount);
        }

        // Store milestone amount before moving
        let milestone_amount = milestone.amount;

        // Mark milestone as released
        let mut updated_milestone = milestone;
        updated_milestone.released = true;
        env.storage().persistent().set(
            &DataKey::Milestone(contract_id, milestone_id),
            &updated_milestone,
        );

        // Emit timeout claimed event
        env.events().publish(
            (topics::TIMEOUT_CLAIMED,),
            (
                contract_id,
                milestone_id,
                if is_complete { freelancer } else { client },
                milestone_amount,
            ),
        );

        Ok(())
    }

    /// Raise a dispute for a contract.
    /// Can be called by client or freelancer.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `initiator` - The address initiating the dispute (client or freelancer)
    /// * `reason` - Reason for the dispute
    ///
    /// # Errors
    /// * `ContractNotFound` - If contract doesn't exist
    /// * `DisputeAlreadyResolved` - If dispute already resolved
    /// * `Unauthorized` - If initiator is not client or freelancer
    pub fn raise_dispute(
        env: Env,
        contract_id: u32,
        initiator: Address,
        reason: Symbol,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (_, client, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Verify initiator is either client or freelancer
        if initiator != client && initiator != freelancer {
            return Err(EscrowError::Unauthorized);
        }

        // Initiator must authorize
        initiator.require_auth();

        // Check if dispute already exists and not resolved
        if let Some(dispute) = env
            .storage()
            .persistent()
            .get::<DataKey, Dispute>(&DataKey::Dispute(contract_id))
        {
            if dispute.resolved {
                return Err(EscrowError::DisputeAlreadyResolved);
            }
            // Dispute already active
            return Ok(());
        }

        // Create dispute
        let current_time = env.ledger().timestamp();
        let reason_clone = reason.clone();
        let dispute = Dispute {
            initiator: initiator.clone(),
            reason,
            created_at: current_time,
            resolved: false,
        };

        // Store dispute
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(contract_id), &dispute);

        // Update contract status
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::InDispute,
        );

        // Update last activity
        Self::update_last_activity(&env, contract_id);

        // Emit dispute raised event
        env.events().publish(
            (topics::DISPUTE_RAISED,),
            (contract_id, initiator, reason_clone, current_time),
        );

        Ok(())
    }

    /// Get dispute information for a contract.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    ///
    /// # Returns
    /// * `Ok(Dispute)` - The dispute information
    pub fn get_dispute(env: Env, contract_id: u32) -> Result<Dispute, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Dispute(contract_id))
            .ok_or(EscrowError::DisputeNotFound)
    }

    /// Resolve a dispute before timeout.
    /// Can only be called by admin.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - The admin address
    /// * `contract_id` - The escrow contract ID
    /// * `resolution` - Resolution type (0=client, 1=freelancer, 2=split)
    ///
    /// # Errors
    /// * `Unauthorized` - If caller is not admin
    /// * `DisputeNotFound` - If no dispute exists
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        contract_id: u32,
        resolution: u32,
    ) -> Result<(), EscrowError> {
        // Verify admin
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)?;
        if admin != stored_admin {
            return Err(EscrowError::Unauthorized);
        }

        // Retrieve dispute
        let mut dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(contract_id))
            .ok_or(EscrowError::DisputeNotFound)?;

        if dispute.resolved {
            return Err(EscrowError::DisputeAlreadyResolved);
        }

        // Retrieve contract
        let (token, client, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Get remaining funds
        let _contract_status: ContractStatus = env
            .storage()
            .persistent()
            .get(&DataKey::ContractStatus(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Calculate total remaining amount
        let mut total_remaining: i128 = 0;
        let milestone_count = 10; // Assume max 10 milestones for simplicity
        for i in 0..milestone_count {
            if let Some(milestone) = env
                .storage()
                .persistent()
                .get::<DataKey, Milestone>(&DataKey::Milestone(contract_id, i))
            {
                if !milestone.released {
                    total_remaining += milestone.amount;
                }
            }
        }

        // Apply resolution
        let token_client = token::Client::new(&env, &token);
        match resolution {
            0 => {
                // Return to client
                token_client.transfer(&env.current_contract_address(), &client, &total_remaining);
            }
            1 => {
                // Release to freelancer
                token_client.transfer(
                    &env.current_contract_address(),
                    &freelancer,
                    &total_remaining,
                );
            }
            2 => {
                // Split 50/50
                let half = total_remaining / 2;
                token_client.transfer(&env.current_contract_address(), &client, &half);
                token_client.transfer(&env.current_contract_address(), &freelancer, &half);
            }
            _ => return Err(EscrowError::InvalidTimeout),
        }

        // Mark dispute as resolved
        dispute.resolved = true;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(contract_id), &dispute);

        // Update contract status
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::Completed,
        );

        // Emit dispute resolved event
        env.events().publish(
            (topics::DISPUTE_RESOLVED,),
            (contract_id, admin, resolution),
        );

        Ok(())
    }

    /// Claim dispute timeout - auto-resolve after timeout period.
    /// Can be called by anyone after timeout period.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    ///
    /// # Errors
    /// * `TimeoutNotExceeded` - If timeout period not yet passed
    /// * `NoDisputeActive` - If no active dispute
    pub fn claim_dispute_timeout(env: Env, contract_id: u32) -> Result<(), EscrowError> {
        // Retrieve dispute
        let dispute: Dispute = env
            .storage()
            .persistent()
            .get(&DataKey::Dispute(contract_id))
            .ok_or(EscrowError::NoDisputeActive)?;

        if dispute.resolved {
            return Err(EscrowError::DisputeAlreadyResolved);
        }

        // Check if timeout exceeded
        if !Self::is_timeout_exceeded(&env, contract_id, dispute.created_at)? {
            return Err(EscrowError::TimeoutNotExceeded);
        }

        // Get timeout config for auto-resolve type
        let timeout_config = Self::get_contract_timeout(env.clone(), contract_id)?;

        // Get contract details
        let (token, client, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Calculate total remaining amount
        let mut total_remaining: i128 = 0;
        let milestone_count = 10; // Assume max 10 milestones for simplicity
        for i in 0..milestone_count {
            if let Some(milestone) = env
                .storage()
                .persistent()
                .get::<DataKey, Milestone>(&DataKey::Milestone(contract_id, i))
            {
                if !milestone.released {
                    total_remaining += milestone.amount;
                }
            }
        }

        // Apply auto-resolution
        let token_client = token::Client::new(&env, &token);
        match timeout_config.auto_resolve_type {
            0 => {
                // Return to client
                token_client.transfer(&env.current_contract_address(), &client, &total_remaining);
            }
            1 => {
                // Release to freelancer
                token_client.transfer(
                    &env.current_contract_address(),
                    &freelancer,
                    &total_remaining,
                );
            }
            2 => {
                // Split 50/50
                let half = total_remaining / 2;
                token_client.transfer(&env.current_contract_address(), &client, &half);
                token_client.transfer(&env.current_contract_address(), &freelancer, &half);
            }
            _ => return Err(EscrowError::InvalidTimeout),
        }

        // Mark dispute as resolved
        let mut updated_dispute = dispute;
        updated_dispute.resolved = true;
        env.storage()
            .persistent()
            .set(&DataKey::Dispute(contract_id), &updated_dispute);

        // Update contract status
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::Completed,
        );

        // Emit dispute auto-resolved event
        env.events().publish(
            (topics::DISPUTE_RESOLVED,),
            (
                contract_id,
                env.current_contract_address(),
                timeout_config.auto_resolve_type,
            ),
        );

        Ok(())
    }

    // ==================== ESCROW FUNCTIONS ====================

    /// Create a new escrow contract. Client and freelancer addresses are stored
    /// for access control. Milestones define payment amounts.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `client` - The client address (must authorize)
    /// * `freelancer` - The freelancer address
    /// * `milestone_amounts` - Vector of milestone payment amounts
    /// * `token` - The token contract address for payments
    ///
    /// # Returns
    /// * `Ok(u32)` - The contract ID
    pub fn create_contract(
        env: Env,
        client: Address,
        freelancer: Address,
        milestone_amounts: Vec<i128>,
        token: Address,
    ) -> Result<u32, EscrowError> {
        // Client must authorize
        client.require_auth();

        // Get or initialize next contract ID
        let contract_id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::NextContractId)
            .unwrap_or(1);

        // Calculate total amount
        let mut total_amount: i128 = 0;
        for i in 0..milestone_amounts.len() {
            let amount = milestone_amounts.get(i).ok_or(EscrowError::InvalidAmount)?;
            if amount <= 0 {
                return Err(EscrowError::InvalidAmount);
            }
            total_amount = total_amount
                .checked_add(amount)
                .ok_or(EscrowError::ArithmeticOverflow)?;

            // Store milestone
            let milestone = Milestone {
                amount,
                released: false,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Milestone(contract_id, i as u32), &milestone);
        }

        // Store contract
        let escrow_contract = EscrowContract {
            client: client.clone(),
            freelancer: freelancer.clone(),
            total_amount,
            milestone_count: milestone_amounts.len() as u32,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Contract(contract_id), &escrow_contract);

        // Store contract status
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::Created,
        );

        // Store token address for this contract
        env.storage().persistent().set(
            &DataKey::Contract(contract_id),
            &(token, client, freelancer),
        );

        // Increment next contract ID
        env.storage()
            .persistent()
            .set(&DataKey::NextContractId, &(contract_id + 1));

        Ok(contract_id)
    }

    /// Deposit funds into escrow. Only the client may call this.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `amount` - The amount to deposit
    /// * `token` - The token contract address
    ///
    /// # Returns
    /// * `Ok(())` on success
    pub fn deposit_funds(
        env: Env,
        contract_id: u32,
        amount: i128,
        token: Address,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (stored_token, client, _): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Verify token matches
        if token != stored_token {
            return Err(EscrowError::InvalidAmount);
        }

        // Client must authorize
        client.require_auth();

        if amount <= 0 {
            return Err(EscrowError::InvalidAmount);
        }

        // Transfer tokens from client to contract
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&client, &env.current_contract_address(), &amount);

        // Update status to funded
        env.storage().persistent().set(
            &DataKey::ContractStatus(contract_id),
            &ContractStatus::Funded,
        );

        // Update last activity
        Self::update_last_activity(&env, contract_id);

        Ok(())
    }

    /// Release a milestone payment to the freelancer after verification.
    /// Deducts protocol fee and transfers to treasury.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `contract_id` - The escrow contract ID
    /// * `milestone_id` - The milestone index to release
    ///
    /// # Returns
    /// * `Ok(())` on success
    pub fn release_milestone(
        env: Env,
        contract_id: u32,
        milestone_id: u32,
    ) -> Result<(), EscrowError> {
        // Retrieve contract
        let (token, client, freelancer): (Address, Address, Address) = env
            .storage()
            .persistent()
            .get(&DataKey::Contract(contract_id))
            .ok_or(EscrowError::ContractNotFound)?;

        // Client must authorize
        client.require_auth();

        // Retrieve milestone
        let mut milestone: Milestone = env
            .storage()
            .persistent()
            .get(&DataKey::Milestone(contract_id, milestone_id))
            .ok_or(EscrowError::MilestoneNotFound)?;

        // Check if already released
        if milestone.released {
            return Err(EscrowError::MilestoneAlreadyReleased);
        }

        // Calculate and transfer protocol fee
        let net_amount = Self::transfer_protocol_fee(
            &env,
            &token,
            &env.current_contract_address(),
            milestone.amount,
        )?;

        // Transfer net amount to freelancer
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &freelancer, &net_amount);

        // Mark milestone as released
        milestone.released = true;
        env.storage()
            .persistent()
            .set(&DataKey::Milestone(contract_id, milestone_id), &milestone);

        // Emit milestone released event
        env.events().publish(
            (topics::MILESTONE_RELEASED,),
            (contract_id, milestone_id, freelancer, net_amount),
        );

        // Update last activity
        Self::update_last_activity(&env, contract_id);

        Ok(())
    }

    /// Issue a reputation credential for the freelancer after contract completion.
    pub fn issue_reputation(_env: Env, _freelancer: Address, _rating: i128) -> bool {
        // Reputation credential issuance.
        true
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Result<Address, EscrowError> {
        env.storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EscrowError::Unauthorized)
    }

    /// Hello-world style function for testing and CI.
    pub fn hello(_env: Env, to: Symbol) -> Symbol {
        to
    }
}

#[cfg(test)]
mod test;
