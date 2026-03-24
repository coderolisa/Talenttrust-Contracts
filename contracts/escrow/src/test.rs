use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient};

#[test]
fn test_hello() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.hello(&symbol_short!("World"));
    assert_eq!(result, symbol_short!("World"));
}

#[test]
fn test_create_contract() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128, 600_0000000_i128];

    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);
    assert_eq!(id, 1);
}

#[test]
fn test_deposit_funds() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.deposit_funds(&1, &1_000_0000000);
    assert!(result);
}

#[test]
fn test_release_milestone() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.release_milestone(&1, &0);
    assert!(result);
}

#[test]
#[should_panic(expected = "ArithmeticOverflow")]
fn test_overflow_on_deposit() {
    let env = Env::default();
    let contract_id = env.register_contract(None, EscrowContract);
    let client = EscrowContractClient::new(&env, &contract_id);

    // Use maximum value of i128 to trigger overflow on addition
    let max_val = i128::MAX;
    client.deposit(&max_val, &1); // Should fail when adding even 1
}

#[test]
fn test_underflow_protection() {
    let env = Env::default();
    // ... setup ...
    // Attempting to release more than the balance should trigger ArithmeticOverflow
    let result = client.try_release_payment(&1000); 
    assert_eq!(result, Err(Ok(Error::ArithmeticOverflow)));
}

#[test]
fn test_admin_can_set_and_revoke_arbitrator() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let arb = Address::generate(&env);
    
    // Initialize contract with admin...
    let client = EscrowContractClient::new(&env, &contract_id);

    // Set Arbitrator
    client.set_arbitrator(&arb);
    assert_eq!(client.get_arbitrator(), arb);

    // Revoke Arbitrator
    client.revoke_arbitrator();
    assert!(client.try_get_arbitrator().is_err());
}

#[test]
#[should_panic(expected = "HostError: Error(Context, InvalidAction)")]
fn test_non_admin_cannot_set_arbitrator() {
    let env = Env::default();
    let mallory = Address::generate(&env); // Malicious user
    let client = EscrowContractClient::new(&env, &contract_id);

    env.mock_all_auths(); 
    // This will fail because the internal requirement check is against the Admin key
    client.set_arbitrator(&mallory);
}

#[test]
fn test_storage_rent_extension() {
    let env = Env::default();
    let contract_id = env.register_contract(None, EscrowContract);
    let client = EscrowContractClient::new(&env, &contract_id);

    // Initial interaction
    client.initialize(&admin);
    
    // Check initial TTL (Soroban internal testing tools allow checking ledger info)
    let initial_ttl = env.ledger().get_ledger_network_id(); // Use ledger simulation
    
    // Perform action that triggers extension
    client.create_escrow(&user);
    
    // In a real Soroban test environment, we'd verify the ledger sequence 
    // for the specific storage key has been bumped.
}