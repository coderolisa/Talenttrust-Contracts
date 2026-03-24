use super::{default_milestones, generated_participants, register_client, total_milestones};
use crate::{ContractStatus, EscrowError, ReleaseAuthorization};
use soroban_sdk::Env;

#[test]
fn test_contract_ids_increment_and_are_distinct() {
    let env = Env::default();
    env.mock_all_auths();
    let client = register_client(&env);

    let (client_a, freelancer_a, _arbiter_a) = generated_participants(&env);
    let (client_b, freelancer_b, _arbiter_b) = generated_participants(&env);

    let id_a = client.create_contract(
        &client_a,
        &freelancer_a,
        &None,
        &default_milestones(&env),
        &ReleaseAuthorization::ClientOnly,
    );

    let id_b = client.create_contract(
        &client_b,
        &freelancer_b,
        &None,
        &default_milestones(&env),
        &ReleaseAuthorization::ClientOnly,
    );

    assert_eq!(id_a, 1);
    assert_eq!(id_b, 2);
}

#[test]
fn test_state_is_isolated_per_contract_id() {
    let env = Env::default();
    env.mock_all_auths();
    let client = register_client(&env);

    let (client_a, freelancer_a, _arbiter_a) = generated_participants(&env);
    let (client_b, freelancer_b, _arbiter_b) = generated_participants(&env);

    let id_a = client.create_contract(
        &client_a,
        &freelancer_a,
        &None,
        &default_milestones(&env),
        &ReleaseAuthorization::ClientOnly,
    );

    let id_b = client.create_contract(
        &client_b,
        &freelancer_b,
        &None,
        &default_milestones(&env),
        &ReleaseAuthorization::ClientOnly,
    );

    assert!(client.deposit_funds(&id_a, &client_a, &total_milestones()));

    let contract_a = client.get_contract(&id_a);
    let contract_b = client.get_contract(&id_b);

    assert_eq!(contract_a.status, ContractStatus::Funded);
    assert_eq!(contract_b.status, ContractStatus::Created);
}

#[test]
fn test_get_contract_missing_id_returns_error() {
    let env = Env::default();
    let client = register_client(&env);

    let result = client.try_get_contract(&42);
    assert_eq!(result, Err(Ok(EscrowError::ContractNotFound)));
}

#[test]
fn test_reputation_defaults_to_zero_for_new_freelancer() {
    let env = Env::default();
    let client = register_client(&env);

    let (_client_addr, freelancer_addr, _arbiter_addr) = generated_participants(&env);
    let reputation = client.get_reputation(&freelancer_addr);

    assert_eq!(reputation.total_rating, 0);
    assert_eq!(reputation.ratings_count, 0);
}
