use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient};

#[test]
fn test_hello() {use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient};

/// Test the hello function, ensures basic contract call works.
#[test]
fn test_hello() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.hello(&symbol_short!("World"));
    assert_eq!(result, symbol_short!("World"));
}

/// Test creating a new escrow contract.
#[test]
fn test_create_contract() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 2_000_000_000_i128, 4_000_000_000_i128, 6_000_000_000_i128];

    // Step 1: Create contract first
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    assert_eq!(id, 1);
}

/// Test depositing funds into the escrow contract.
#[test]
fn test_deposit_funds() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 1_000_000_000_i128];

    // Step 1: Create contract first
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    let token = Address::generate(&env);

    // Step 2: Deposit funds using the correct contract_id
    let result = client.deposit_funds(&id, &token, &client_addr, &1_000_000_000);
    assert!(result);
}

/// Test releasing a milestone payment to the freelancer.
#[test]
fn test_release_milestone() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 500_000_000_i128];

    // Step 1: Create contract first
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    let token = Address::generate(&env);

    // Step 2: Deposit funds first (simulate funding)
    let deposit_result = client.deposit_funds(&id, &token, &client_addr, &500_000_000);
    assert!(deposit_result);

    // Step 3: Release milestone to freelancer
    let result = client.release_milestone(&id, &token, &freelancer_addr, &500_000_000);
    assert!(result);
}

/// Test that depositing an invalid (zero) amount fails.
#[test]
fn test_deposit_invalid_amount() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 100_i128];

    // Step 1: Create contract first
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    let token = Address::generate(&env);

    // Step 2: Try depositing 0, should fail
    let result = client.deposit_funds(&id, &token, &client_addr, &0);
    assert!(!result);
}
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

    // Step 1: create contract first
    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128];
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    let token = Address::generate(&env);

    // Step 2: use the contract_id returned from create_contract
    let result = client.deposit_funds(&id, &token, &client_addr, &1_000_0000000);
    assert!(result);
}

#[test]
fn test_release_milestone() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    // Step 1: create contract first
    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128];
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    let token = Address::generate(&env);

    // Step 2: deposit funds first so milestone can be released
    let _ = client.deposit_funds(&id, &token, &client_addr, &1_000_0000000);

    // Step 3: release milestone using the contract_id
    let result = client.release_milestone(&id, &token, &freelancer_addr, &500_0000000);
    assert!(result);
}

#[test]
fn test_deposit_invalid_amount() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128];
    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);

    let token = Address::generate(&env);

    let result = client.deposit_funds(&id, &token, &client_addr, &0);
    assert!(!result);
}