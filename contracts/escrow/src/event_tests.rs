#![cfg(test)]

use soroban_sdk::{symbol_short, testutils::Address as _, testutils::MockAuthInvoke, Env, InvokeContractCheckAuth, Symbol, Address, Vec};
use crate::{
    Escrow, ContractStatus, ReleaseAuthorization, Milestone, EscrowContract,
    ContractCreatedEvent, ContractFundedEvent, MilestoneReleasedEvent,
    ContractDisputedEvent, ContractClosedEvent,
};

// ============================================================================
// TEST CONSTANTS
// ============================================================================

const MILESTONE_ONE: i128 = 100_0000000;   // 100 XLM
const MILESTONE_TWO: i128 = 200_0000000;   // 200 XLM
const MILESTONE_THREE: i128 = 150_0000000; // 150 XLM
const TOTAL_AMOUNT: i128 = MILESTONE_ONE + MILESTONE_TWO + MILESTONE_THREE;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn setup_contract(
    env: &Env,
) -> (u32, Address, Address, Address) {
    let client = Address::generate(env);
    let freelancer = Address::generate(env);
    let arbiter = Address::generate(env);

    env.mock_all_auths();

    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        Some(arbiter.clone()),
        vec![env, MILESTONE_ONE, MILESTONE_TWO, MILESTONE_THREE],
        ReleaseAuthorization::ClientAndArbiter,
    );

    (contract_id, client, freelancer, arbiter)
}

fn fund_contract(env: &Env, contract_id: u32, funder: &Address, amount: i128) {
    env.mock_all_auths();
    Escrow::deposit_funds(env.clone(), contract_id, funder.clone(), amount);
}

// ============================================================================
// CONTRACT CREATION TESTS
// ============================================================================

#[test]
fn test_create_contract_success() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    // Create contract - should emit ContractCreated event
    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        None,
        vec![&env, MILESTONE_ONE, MILESTONE_TWO],
        ReleaseAuthorization::ClientOnly,
    );

    assert!(contract_id > 0);

    // Verify contract exists
    assert!(Escrow::contract_exists(env.clone(), contract_id));

    // Verify contract status
    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.status, ContractStatus::Created);
    assert_eq!(contract.client, client);
    assert_eq!(contract.freelancer, freelancer);
    assert_eq!(contract.total_amount, MILESTONE_ONE + MILESTONE_TWO);
}

#[test]
fn test_create_contract_with_arbiter() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let arbiter = Address::generate(&env);

    env.mock_all_auths();

    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        Some(arbiter.clone()),
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ClientAndArbiter,
    );

    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.arbiter, Some(arbiter));
    assert_eq!(contract.release_auth, ReleaseAuthorization::ClientAndArbiter);
}

#[test]
#[should_panic(expected = "At least one milestone required")]
fn test_create_contract_no_milestones() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    Escrow::create_contract(
        env.clone(),
        client,
        freelancer,
        None,
        vec![&env],
        ReleaseAuthorization::ClientOnly,
    );
}

#[test]
#[should_panic(expected = "Client and freelancer cannot be the same address")]
fn test_create_contract_same_client_freelancer() {
    let env = Env::default();
    let addr = Address::generate(&env);

    env.mock_all_auths();

    Escrow::create_contract(
        env.clone(),
        addr.clone(),
        addr,
        None,
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ClientOnly,
    );
}

#[test]
#[should_panic(expected = "Milestone amounts must be positive")]
fn test_create_contract_negative_milestone() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    Escrow::create_contract(
        env.clone(),
        client,
        freelancer,
        None,
        vec![&env, -100_0000000],
        ReleaseAuthorization::ClientOnly,
    );
}

#[test]
#[should_panic(expected = "Exceeds maximum contract funding size")]
fn test_create_contract_exceeds_max_size() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    Escrow::create_contract(
        env.clone(),
        client,
        freelancer,
        None,
        vec![&env, 2_000_000_000_000_i128],
        ReleaseAuthorization::ClientOnly,
    );
}

// ============================================================================
// CONTRACT FUNDING TESTS
// ============================================================================

#[test]
fn test_deposit_funds_success() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();

    // Deposit funds - should emit ContractFunded event
    let result = Escrow::deposit_funds(
        env.clone(),
        contract_id,
        client.clone(),
        TOTAL_AMOUNT,
    );

    assert!(result);

    // Verify contract status changed to Funded
    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.status, ContractStatus::Funded);
    assert_eq!(contract.funded_amount, TOTAL_AMOUNT);
}

#[test]
#[should_panic(expected = "Contract not found")]
fn test_deposit_funds_contract_not_found() {
    let env = Env::default();
    let client = Address::generate(&env);

    env.mock_all_auths();

    Escrow::deposit_funds(
        env.clone(),
        999,
        client,
        TOTAL_AMOUNT,
    );
}

#[test]
#[should_panic(expected = "Only client can deposit funds")]
fn test_deposit_funds_not_client() {
    let env = Env::default();
    let (contract_id, _client, freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();

    Escrow::deposit_funds(
        env.clone(),
        contract_id,
        freelancer,
        TOTAL_AMOUNT,
    );
}

#[test]
#[should_panic(expected = "Deposit amount must equal total milestone amounts")]
fn test_deposit_funds_wrong_amount() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();

    Escrow::deposit_funds(
        env.clone(),
        contract_id,
        client,
        TOTAL_AMOUNT - 1_0000000,
    );
}

#[test]
#[should_panic(expected = "Contract must be in Created status to deposit funds")]
fn test_deposit_funds_wrong_status() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();

    // First deposit
    Escrow::deposit_funds(
        env.clone(),
        contract_id,
        client.clone(),
        TOTAL_AMOUNT,
    );

    // Try to deposit again
    Escrow::deposit_funds(
        env.clone(),
        contract_id,
        client,
        TOTAL_AMOUNT,
    );
}

// ============================================================================
// MILESTONE APPROVAL TESTS
// ============================================================================

#[test]
fn test_approve_milestone_success() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    // Approve milestone
    let result = Escrow::approve_milestone_release(
        env.clone(),
        contract_id,
        client.clone(),
        0,
    );

    assert!(result);

    // Verify milestone approval
    let milestone = Escrow::get_milestone(env.clone(), contract_id, 0);
    assert_eq!(milestone.approved_by, Some(client.clone()));
    assert!(milestone.approval_timestamp.is_some());
}

#[test]
#[should_panic(expected = "Invalid milestone ID")]
fn test_approve_milestone_invalid_id() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    Escrow::approve_milestone_release(
        env.clone(),
        contract_id,
        client,
        999,
    );
}

#[test]
#[should_panic(expected = "Unauthorized to approve milestone release")]
fn test_approve_milestone_unauthorized() {
    let env = Env::default();
    let (contract_id, client, freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    // Freelancer not authorized with ClientAndArbiter scheme
    Escrow::approve_milestone_release(
        env.clone(),
        contract_id,
        freelancer,
        0,
    );
}

#[test]
#[should_panic(expected = "Milestone already approved by this address")]
fn test_approve_milestone_duplicate() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    Escrow::approve_milestone_release(
        env.clone(),
        contract_id,
        client.clone(),
        0,
    );

    // Try to approve again with same address
    Escrow::approve_milestone_release(
        env.clone(),
        contract_id,
        client,
        0,
    );
}

// ============================================================================
// MILESTONE RELEASE TESTS
// ============================================================================

#[test]
fn test_release_milestone_success() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);
    Escrow::approve_milestone_release(env.clone(), contract_id, client.clone(), 0);

    // Release milestone - should emit MilestoneReleased event
    let result = Escrow::release_milestone(
        env.clone(),
        contract_id,
        client.clone(),
        0,
    );

    assert!(result);

    // Verify milestone released
    let milestone = Escrow::get_milestone(env.clone(), contract_id, 0);
    assert!(milestone.released);

    // Verify contract released_amount updated
    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.released_amount, MILESTONE_ONE);
    assert_eq!(contract.status, ContractStatus::Funded); // Not all released yet
}

#[test]
fn test_release_all_milestones_triggers_close() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    // Approve and release all milestones
    for i in 0..3 {
        Escrow::approve_milestone_release(env.clone(), contract_id, client.clone(), i);
        let result = Escrow::release_milestone(env.clone(), contract_id, client.clone(), i);
        assert!(result);
    }

    // After releasing all, contract should be Completed
    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.status, ContractStatus::Completed);
    assert_eq!(contract.released_amount, TOTAL_AMOUNT);
}

#[test]
#[should_panic(expected = "Invalid milestone ID")]
fn test_release_milestone_invalid_id() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    Escrow::release_milestone(
        env.clone(),
        contract_id,
        client,
        999,
    );
}

#[test]
#[should_panic(expected = "Milestone already released")]
fn test_release_milestone_duplicate() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);
    Escrow::approve_milestone_release(env.clone(), contract_id, client.clone(), 0);

    Escrow::release_milestone(env.clone(), contract_id, client.clone(), 0);

    // Try to release again
    Escrow::release_milestone(
        env.clone(),
        contract_id,
        client,
        0,
    );
}

#[test]
#[should_panic(expected = "Milestone not approved for release")]
fn test_release_milestone_not_approved() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    // Try to release without approval
    Escrow::release_milestone(
        env.clone(),
        contract_id,
        client,
        0,
    );
}

// ============================================================================
// DISPUTE TESTS
// ============================================================================

#[test]
fn test_dispute_contract_by_client() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    // Dispute contract - should emit ContractDisputed event
    let result = Escrow::dispute_contract(
        env.clone(),
        contract_id,
        client.clone(),
        symbol_short!("quality"),
    );

    assert!(result);

    // Verify status changed to Disputed
    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.status, ContractStatus::Disputed);

    // Verify dispute record exists
    let dispute = Escrow::get_dispute(env.clone(), contract_id);
    assert!(dispute.is_some());
    assert_eq!(dispute.unwrap().initiator, client);
}

#[test]
fn test_dispute_contract_by_arbiter() {
    let env = Env::default();
    let (contract_id, client, _freelancer, arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    let result = Escrow::dispute_contract(
        env.clone(),
        contract_id,
        arbiter.clone(),
        symbol_short!("dispute"),
    );

    assert!(result);

    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.status, ContractStatus::Disputed);
}

#[test]
#[should_panic(expected = "Only client or arbiter can dispute contract")]
fn test_dispute_contract_unauthorized() {
    let env = Env::default();
    let (contract_id, client, freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    Escrow::dispute_contract(
        env.clone(),
        contract_id,
        freelancer,
        symbol_short!("reason"),
    );
}

#[test]
#[should_panic(expected = "Contract must be in Funded status to dispute")]
fn test_dispute_contract_wrong_status() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();

    // Try to dispute before funding
    Escrow::dispute_contract(
        env.clone(),
        contract_id,
        client,
        symbol_short!("reason"),
    );
}

// ============================================================================
// EDGE CASE & BOUNDARY TESTS
// ============================================================================

#[test]
fn test_large_milestone_amounts() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    let large_amount = 100_000_000_0000000; // 100,000 XLM
    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        None,
        vec![&env, large_amount],
        ReleaseAuthorization::ClientOnly,
    );

    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.total_amount, large_amount);
}

#[test]
fn test_single_milestone_contract() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        None,
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ClientOnly,
    );

    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.milestones.len(), 1);
    assert_eq!(contract.total_amount, MILESTONE_ONE);
}

#[test]
fn test_many_milestones_contract() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    env.mock_all_auths();

    let mut milestones = vec![&env];
    for i in 1..=10 {
        milestones.push_back(i as i128 * 10_0000000);
    }

    let contract_id = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        None,
        milestones,
        ReleaseAuthorization::ClientOnly,
    );

    let contract = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract.milestones.len(), 10);
}

#[test]
fn test_different_authorization_schemes() {
    let env = Env::default();
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let arbiter = Address::generate(&env);

    env.mock_all_auths();

    // ClientOnly
    let id1 = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        Some(arbiter.clone()),
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ClientOnly,
    );
    assert_eq!(Escrow::get_contract(env.clone(), id1).release_auth, ReleaseAuthorization::ClientOnly);

    // ArbiterOnly
    let id2 = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        Some(arbiter.clone()),
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ArbiterOnly,
    );
    assert_eq!(Escrow::get_contract(env.clone(), id2).release_auth, ReleaseAuthorization::ArbiterOnly);

    // ClientAndArbiter
    let id3 = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        Some(arbiter.clone()),
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ClientAndArbiter,
    );
    assert_eq!(Escrow::get_contract(env.clone(), id3).release_auth, ReleaseAuthorization::ClientAndArbiter);

    // MultiSig
    let id4 = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        Some(arbiter.clone()),
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::MultiSig,
    );
    assert_eq!(Escrow::get_contract(env.clone(), id4).release_auth, ReleaseAuthorization::MultiSig);
}

// ============================================================================
// STATE TRANSITION TESTS
// ============================================================================

#[test]
fn test_created_to_funded_transition() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    let contract_before = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract_before.status, ContractStatus::Created);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    let contract_after = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract_after.status, ContractStatus::Funded);
}

#[test]
fn test_funded_to_disputed_transition() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    let contract_before = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract_before.status, ContractStatus::Funded);

    Escrow::dispute_contract(env.clone(), contract_id, client.clone(), symbol_short!("reason"));

    let contract_after = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract_after.status, ContractStatus::Disputed);
}

#[test]
fn test_funded_to_completed_transition() {
    let env = Env::default();
    let (contract_id, client, _freelancer, _arbiter) = setup_contract(&env);

    env.mock_all_auths();
    fund_contract(&env, contract_id, &client, TOTAL_AMOUNT);

    let contract_before = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract_before.status, ContractStatus::Funded);

    // Release all milestones
    for i in 0..3 {
        Escrow::approve_milestone_release(env.clone(), contract_id, client.clone(), i);
        Escrow::release_milestone(env.clone(), contract_id, client.clone(), i);
    }

    let contract_after = Escrow::get_contract(env.clone(), contract_id);
    assert_eq!(contract_after.status, ContractStatus::Completed);
}

// ============================================================================
// UTILITY FUNCTION TESTS
// ============================================================================

#[test]
fn test_hello() {
    let env = Env::default();
    let result = Escrow::hello(env, symbol_short!("World"));
    assert_eq!(result, symbol_short!("World"));
}

#[test]
fn test_contract_exists() {
    let env = Env::default();
    let (contract_id, _, _, _) = setup_contract(&env);

    assert!(Escrow::contract_exists(env.clone(), contract_id));
    assert!(!Escrow::contract_exists(env.clone(), 999));
}

#[test]
fn test_get_next_contract_id() {
    let env = Env::default();

    // Initial ID
    let next_id_1 = Escrow::get_next_contract_id(env.clone());

    env.mock_all_auths();

    // Create a contract
    let client = Address::generate(&env);
    let freelancer = Address::generate(&env);

    let id1 = Escrow::create_contract(
        env.clone(),
        client.clone(),
        freelancer.clone(),
        None,
        vec![&env, MILESTONE_ONE],
        ReleaseAuthorization::ClientOnly,
    );

    let next_id_2 = Escrow::get_next_contract_id(env.clone());

    // Next ID should have incremented
    assert_eq!(next_id_2, next_id_1 + 1);
}
