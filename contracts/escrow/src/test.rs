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
fn test_issue_reputation() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let freelancer_addr = Address::generate(&env);
    let result = client.issue_reputation(&freelancer_addr, &5);
    assert!(result);
}

#[test]
#[should_panic(expected = "Milestone amounts cannot be empty")]
fn test_create_contract_empty_milestones_panics() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env];

    client.create_contract(&client_addr, &freelancer_addr, &milestones);
}

#[test]
#[should_panic(expected = "Milestone amounts must be greater than zero")]
fn test_create_contract_negative_milestones_panics() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 100_0000000_i128, -500_i128];

    client.create_contract(&client_addr, &freelancer_addr, &milestones);
}

#[test]
#[should_panic(expected = "Deposit amount must be strictly positive")]
fn test_deposit_funds_zero_panics() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    client.deposit_funds(&1, &0);
}

#[test]
#[should_panic(expected = "Rating must be between 1 and 5")]
fn test_issue_reputation_out_of_bounds_panics() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let freelancer_addr = Address::generate(&env);
    client.issue_reputation(&freelancer_addr, &6);
}
