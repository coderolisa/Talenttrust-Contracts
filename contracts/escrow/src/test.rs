use soroban_sdk::{symbol_short, testutils::Address as _, vec, Address, Env, Vec};

use crate::{Escrow, EscrowClient, ReleaseAuthorization};

pub(crate) const MILESTONE_ONE: i128 = 200_0000000;
pub(crate) const MILESTONE_TWO: i128 = 400_0000000;
pub(crate) const MILESTONE_THREE: i128 = 600_0000000;

pub(crate) fn register_client(env: &Env) -> EscrowClient<'_> {
    let contract_id = env.register(Escrow, ());
    EscrowClient::new(env, &contract_id)
}

pub(crate) fn default_milestones(env: &Env) -> Vec<i128> {
    vec![&env, MILESTONE_ONE, MILESTONE_TWO, MILESTONE_THREE]
}

pub(crate) fn total_milestones() -> i128 {
    MILESTONE_ONE + MILESTONE_TWO + MILESTONE_THREE
}

pub(crate) fn generated_participants(env: &Env) -> (Address, Address, Address) {
    (
        Address::generate(env),
        Address::generate(env),
        Address::generate(env),
    )
}

pub(crate) fn create_default_contract(
    client: &EscrowClient<'_>,
    env: &Env,
    release_auth: ReleaseAuthorization,
) -> (u32, Address, Address, Address) {
    let (client_addr, freelancer_addr, arbiter_addr) = generated_participants(env);
    let arbiter = match release_auth {
        ReleaseAuthorization::ClientOnly => None,
        _ => Some(arbiter_addr.clone()),
    };

    let contract_id = client.create_contract(
        &client_addr,
        &freelancer_addr,
        &arbiter,
        &default_milestones(env),
        &release_auth,
    );

    (contract_id, client_addr, freelancer_addr, arbiter_addr)
}

pub(crate) fn world_symbol() -> soroban_sdk::Symbol {
    symbol_short!("World")
}

mod access_control;
mod flows;
mod storage;
