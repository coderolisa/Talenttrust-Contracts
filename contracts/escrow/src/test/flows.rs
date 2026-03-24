use super::{
    create_default_contract, default_milestones, register_client, total_milestones, world_symbol,
};
use crate::{ContractStatus, ReleaseAuthorization};
use soroban_sdk::Env;

#[test]
fn test_hello() {
    let env = Env::default();
    let client = register_client(&env);

    assert_eq!(client.hello(&world_symbol()), world_symbol());
}

#[test]
fn test_create_contract_persists_expected_roles_and_status() {
    let env = Env::default();
    env.mock_all_auths();

    let client = register_client(&env);
    let (contract_id, client_addr, freelancer_addr, _arbiter_addr) =
        create_default_contract(&client, &env, ReleaseAuthorization::ClientOnly);

    let contract = client.get_contract(&contract_id);
    assert_eq!(contract.client, client_addr);
    assert_eq!(contract.freelancer, freelancer_addr);
    assert_eq!(contract.status, ContractStatus::Created);
    assert_eq!(contract.milestones.len(), 3);
}

#[test]
fn test_client_only_flow_releases_all_milestones_and_completes() {
    let env = Env::default();
    env.mock_all_auths();

    let client = register_client(&env);
    let (contract_id, client_addr, freelancer_addr, _arbiter_addr) =
        create_default_contract(&client, &env, ReleaseAuthorization::ClientOnly);

    assert!(client.deposit_funds(&contract_id, &client_addr, &total_milestones()));
    assert!(client.approve_milestone_release(&contract_id, &client_addr, &0));
    assert!(client.release_milestone(&contract_id, &client_addr, &0));
    assert!(client.approve_milestone_release(&contract_id, &client_addr, &1));
    assert!(client.release_milestone(&contract_id, &client_addr, &1));
    assert!(client.approve_milestone_release(&contract_id, &client_addr, &2));
    assert!(client.release_milestone(&contract_id, &client_addr, &2));

    let final_contract = client.get_contract(&contract_id);
    assert_eq!(final_contract.status, ContractStatus::Completed);

    assert!(client.issue_reputation(&contract_id, &client_addr, &freelancer_addr, &5));
    let reputation = client.get_reputation(&freelancer_addr);
    assert_eq!(reputation.total_rating, 5);
    assert_eq!(reputation.ratings_count, 1);
}

#[test]
fn test_multisig_requires_client_and_arbiter_approval() {
    let env = Env::default();
    env.mock_all_auths();

    let client = register_client(&env);
    let (contract_id, client_addr, _freelancer_addr, arbiter_addr) =
        create_default_contract(&client, &env, ReleaseAuthorization::MultiSig);

    assert!(client.deposit_funds(&contract_id, &client_addr, &total_milestones()));

    // Client approval alone is insufficient for MultiSig release.
    assert!(client.approve_milestone_release(&contract_id, &client_addr, &0));
    let failed_release = client.try_release_milestone(&contract_id, &client_addr, &0);
    assert!(failed_release.is_err());

    assert!(client.approve_milestone_release(&contract_id, &arbiter_addr, &0));
    assert!(client.release_milestone(&contract_id, &client_addr, &0));
}

#[test]
fn test_deposit_requires_exact_total_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let client = register_client(&env);
    let (contract_id, client_addr, _freelancer_addr, _arbiter_addr) =
        create_default_contract(&client, &env, ReleaseAuthorization::ClientOnly);

    let wrong_amount = total_milestones() - 1;
    let result = client.try_deposit_funds(&contract_id, &client_addr, &wrong_amount);
    assert!(result.is_err());

    // Sanity check the contract remains in Created state.
    assert_eq!(
        client.get_contract(&contract_id).status,
        ContractStatus::Created
    );

    // Success path still works.
    assert!(client.deposit_funds(&contract_id, &client_addr, &total_milestones()));

    // Keep default_milestones reachable in module for coverage of helper behavior.
    assert_eq!(default_milestones(&env).len(), 3);
}
