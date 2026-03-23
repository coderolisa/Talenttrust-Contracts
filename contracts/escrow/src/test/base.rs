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
fn test_create_contract_success() {
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
fn test_deposit_funds_success() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.deposit_funds(&1, &1_000_0000000);
    assert!(result);
}

#[test]
fn test_release_milestone_success() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.release_milestone(&1, &0);
    assert!(result);
}

#[test]
fn test_issue_reputation_success() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let freelancer = Address::generate(&env);
    let result = client.issue_reputation(&freelancer, &5);
    assert!(result);
}
