use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient};

fn setup() -> (Env, EscrowClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let freelancer = Address::generate(&env);
    (env, client, admin, freelancer)
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_only_once_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_pause_then_unpause_toggles_state() {
    let (_env, client, _admin, _freelancer) = setup();

    assert!(!client.is_paused());
    assert!(client.pause());
    assert!(client.is_paused());

    assert!(client.unpause());
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_pause_blocks_create_contract() {
    let (env, client, _admin, freelancer) = setup();
    client.pause();

    let client_addr = Address::generate(&env);
    let milestones = vec![&env, 50_i128, 75_i128];
    let _ = client.create_contract(&client_addr, &freelancer, &milestones);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_pause_blocks_deposit_funds() {
    let (_env, client, _admin, _freelancer) = setup();
    client.pause();

    let _ = client.deposit_funds(&1, &1_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_pause_blocks_release_milestone() {
    let (_env, client, _admin, _freelancer) = setup();
    client.pause();

    let _ = client.release_milestone(&1, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_pause_blocks_issue_reputation() {
    let (env, client, _admin, _freelancer) = setup();
    client.pause();

    let freelancer = Address::generate(&env);
    let _ = client.issue_reputation(&freelancer, &5);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_unpause_fails_when_not_paused() {
    let (_env, client, _admin, _freelancer) = setup();
    let _ = client.unpause();
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_pause_requires_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    let _ = client.pause();
}
