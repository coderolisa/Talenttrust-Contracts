use soroban_sdk::{testutils::Address as _, vec, Address, Env};

use crate::{Escrow, EscrowClient};

fn setup() -> (Env, EscrowClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, client, admin)
}

#[test]
fn test_activate_emergency_sets_emergency_and_pause_flags() {
    let (_env, client, _admin) = setup();

    assert!(!client.is_emergency());
    assert!(!client.is_paused());

    assert!(client.activate_emergency_pause());

    assert!(client.is_emergency());
    assert!(client.is_paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_unpause_fails_while_emergency_is_active() {
    let (_env, client, _admin) = setup();

    client.activate_emergency_pause();
    let _ = client.unpause();
}

#[test]
fn test_resolve_emergency_restores_operations() {
    let (env, client, _admin) = setup();

    client.activate_emergency_pause();
    assert!(client.resolve_emergency());

    assert!(!client.is_emergency());
    assert!(!client.is_paused());

    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 10_i128, 20_i128];

    let created = client.create_contract(&client_addr, &freelancer_addr, &milestones);
    assert_eq!(created, 1);
}
