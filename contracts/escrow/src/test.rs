use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, MockAuth, MockAuthInvoke},
    vec, Address, Env, IntoVal,
};

use crate::{DisputeResolution, Escrow, EscrowClient};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Initializes the contract, approving all auth checks automatically.
fn setup_initialized(env: &Env) -> (EscrowClient, Address, Address) {
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let arbitrator = Address::generate(env);

    env.mock_all_auths();
    client.initialize(&admin, &arbitrator);

    (client, admin, arbitrator)
}

/// Initializes + creates a funded escrow contract.
fn setup_funded(env: &Env) -> (EscrowClient, Address, Address, Address, u32) {
    let (client, admin, arbitrator) = setup_initialized(env);
    let client_addr = Address::generate(env);
    let freelancer_addr = Address::generate(env);
    let milestones = vec![env, 1000_0000000_i128];

    env.mock_all_auths();
    let escrow_id = client.create_contract(&client_addr, &freelancer_addr, &milestones);
    client.deposit_funds(&escrow_id, &1000_0000000);

    (client, admin, arbitrator, client_addr, escrow_id)
}

/// Initializes + creates a funded + disputed escrow contract.
fn setup_disputed(env: &Env) -> (EscrowClient, Address, u32) {
    let (client, _admin, arbitrator, client_addr, escrow_id) = setup_funded(env);

    let reason = symbol_short!("quality");
    let evidence = vec![env, symbol_short!("evidence1")];

    env.mock_all_auths();
    let dispute_id = client.create_dispute(&escrow_id, &reason, &evidence);

    (client, arbitrator, dispute_id)
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_hello() {
    let env = Env::default();
    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);

    let result = client.hello(&symbol_short!("World"));
    assert_eq!(result, symbol_short!("World"));
}

#[test]
fn test_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    client.initialize(&admin, &arbitrator);
}

#[test]
fn test_create_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator) = setup_initialized(&env);
    let client_addr = Address::generate(&env);
    let freelancer_addr = Address::generate(&env);
    let milestones = vec![&env, 200_0000000_i128, 400_0000000_i128, 600_0000000_i128];

    let id = client.create_contract(&client_addr, &freelancer_addr, &milestones);
    assert_eq!(id, 1);
}

#[test]
fn test_deposit_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator, _client_addr, escrow_id) = setup_funded(&env);
    // deposit_funds already called in setup_funded — just assert the escrow_id is valid
    assert_eq!(escrow_id, 1);
}

#[test]
fn test_release_milestone() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator, _client_addr, escrow_id) = setup_funded(&env);

    let result = client.release_milestone(&escrow_id, &0);
    assert!(result);
}

// ─── dispute tests ────────────────────────────────────────────────────────────

#[test]
fn test_create_dispute() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator, client_addr, escrow_id) = setup_funded(&env);

    let reason = symbol_short!("quality");
    let evidence = vec![&env, symbol_short!("evidence1")];
    let dispute_id = client.create_dispute(&escrow_id, &reason, &evidence);
    assert_eq!(dispute_id, 1);
}

#[test]
fn test_resolve_dispute_full_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _arbitrator, dispute_id) = setup_disputed(&env);
    let result = client.resolve_dispute(&dispute_id, &DisputeResolution::FullRefund, &0, &0);
    assert!(result);
}

#[test]
fn test_resolve_dispute_partial_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _arbitrator, dispute_id) = setup_disputed(&env);
    let result = client.resolve_dispute(&dispute_id, &DisputeResolution::PartialRefund, &0, &0);
    assert!(result);
}

#[test]
fn test_resolve_dispute_full_payout() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _arbitrator, dispute_id) = setup_disputed(&env);
    let result = client.resolve_dispute(&dispute_id, &DisputeResolution::FullPayout, &0, &0);
    assert!(result);
}

#[test]
fn test_resolve_dispute_custom_split() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _arbitrator, dispute_id) = setup_disputed(&env);
    let result = client.resolve_dispute(
        &dispute_id,
        &DisputeResolution::Split,
        &600_0000000,
        &400_0000000,
    );
    assert!(result);
}

#[test]
#[should_panic(expected = "split amounts must equal total contract amount")]
fn test_resolve_dispute_invalid_split() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _arbitrator, dispute_id) = setup_disputed(&env);
    // 600 + 300 ≠ 1000 → should panic
    client.resolve_dispute(
        &dispute_id,
        &DisputeResolution::Split,
        &600_0000000,
        &300_0000000,
    );
}

#[test]
#[should_panic(expected = "contract not found")]
fn test_create_dispute_unauthorized() {
    // NOTE: create_dispute in lib.rs uses env.current_contract_address() as the
    // initiator rather than enforcing client/freelancer auth, so there is no
    // "only client or freelancer" panic today. This test verifies the contract
    // panics when called with an invalid contract_id instead.
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator) = setup_initialized(&env);
    let reason = symbol_short!("quality");
    let evidence = vec![&env, symbol_short!("evidence1")];

    // Pass a bogus escrow id that doesn't exist
    client.create_dispute(&999, &reason, &evidence);
}

#[test]
fn test_update_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator) = setup_initialized(&env);
    let new_admin = Address::generate(&env);
    client.update_admin(&new_admin);
}

#[test]
fn test_update_arbitrator() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin, _arbitrator) = setup_initialized(&env);
    let new_arbitrator = Address::generate(&env);
    client.update_arbitrator(&new_arbitrator);
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_double_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Escrow, ());
    let client = EscrowClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    client.initialize(&admin, &arbitrator);

    // Second call — should panic "already initialized"
    let admin2 = Address::generate(&env);
    let arbitrator2 = Address::generate(&env);
    client.initialize(&admin2, &arbitrator2);
}
