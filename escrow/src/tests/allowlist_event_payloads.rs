use super::super::{
    AllowlistEnabledChanged, AllowlistStateChanged, InvestorAllowlistChanged, LiquifactEscrow,
    LiquifactEscrowClient,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, Event, Vec as SorobanVec,
};

fn deploy(env: &Env) -> LiquifactEscrowClient<'_> {
    let id = env.register(LiquifactEscrow, ());
    LiquifactEscrowClient::new(env, &id)
}

fn init_escrow(env: &Env, client: &LiquifactEscrowClient) -> (Address, Address) {
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, "ALEVT001"),
        &sme,
        &10_000i128,
        &800i64,
        &0u64,
        &token,
        &None,
        &treasury,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None::<i64>,
    );
    (admin, sme)
}

#[test]
fn test_allowlist_enabled_changed_topics_and_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();

    client.set_allowlist_active(&true);
    let events = env.events().all();

    let expected = AllowlistEnabledChanged {
        name: symbol_short!("al_ena"),
        invoice_id,
        active: 1,
    };
    assert!(events.contains(&expected.to_xdr(&env, &contract_id)));
}

#[test]
fn test_allowlist_disabled_changed_topics_and_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();

    client.set_allowlist_active(&true);
    client.set_allowlist_active(&false);
    let events = env.events().all();

    let enabled = AllowlistEnabledChanged {
        name: symbol_short!("al_ena"),
        invoice_id: invoice_id.clone(),
        active: 1,
    };
    let disabled = AllowlistEnabledChanged {
        name: symbol_short!("al_ena"),
        invoice_id,
        active: 0,
    };
    assert!(events.contains(&enabled.to_xdr(&env, &contract_id)));
    assert!(events.contains(&disabled.to_xdr(&env, &contract_id)));
}

#[test]
fn test_investor_allowlist_changed_add_and_remove() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();
    let investor = Address::generate(&env);

    client.set_investor_allowlisted(&investor, &true);
    let events = env.events().all();

    let added = InvestorAllowlistChanged {
        name: symbol_short!("al_set"),
        invoice_id: invoice_id.clone(),
        investor: investor.clone(),
        allowed: 1,
    };
    assert!(events.contains(&added.to_xdr(&env, &contract_id)));
}

#[test]
fn test_investor_allowlist_changed_remove_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();
    let investor = Address::generate(&env);

    client.set_investor_allowlisted(&investor, &true);
    client.set_investor_allowlisted(&investor, &false);
    let events = env.events().all();

    let removed = InvestorAllowlistChanged {
        name: symbol_short!("al_set"),
        invoice_id,
        investor,
        allowed: 0,
    };
    assert!(events.contains(&removed.to_xdr(&env, &contract_id)));
}

#[test]
fn test_allowlist_state_changed_batch_add() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();

    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let mut batch: SorobanVec<Address> = SorobanVec::new(&env);
    batch.push_back(a);
    batch.push_back(b);

    client.set_allowlist_active(&true);
    client.set_investors_allowlisted(&batch, &true);
    let events = env.events().all();

    let expected = AllowlistStateChanged {
        name: symbol_short!("al_st"),
        invoice_id,
        total_count: 2,
    };
    assert!(events.contains(&expected.to_xdr(&env, &contract_id)));
}

#[test]
fn test_allowlist_state_changed_single_add() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();
    let investor = Address::generate(&env);

    client.set_investor_allowlisted(&investor, &true);
    let events = env.events().all();

    let expected = AllowlistStateChanged {
        name: symbol_short!("al_st"),
        invoice_id,
        total_count: 1,
    };
    assert!(events.contains(&expected.to_xdr(&env, &contract_id)));
}

#[test]
fn test_allowlist_state_changed_after_remove() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();
    let investor = Address::generate(&env);

    client.set_investor_allowlisted(&investor, &true);
    client.set_investor_allowlisted(&investor, &false);
    let events = env.events().all();

    let expected = AllowlistStateChanged {
        name: symbol_short!("al_st"),
        invoice_id,
        total_count: 0,
    };
    assert!(events.contains(&expected.to_xdr(&env, &contract_id)));
}

#[test]
fn test_allowlist_state_changed_batch_remove() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();

    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let mut batch: SorobanVec<Address> = SorobanVec::new(&env);
    batch.push_back(a);
    batch.push_back(b);

    client.set_investors_allowlisted(&batch, &true);
    client.set_investors_allowlisted(&batch, &false);
    let events = env.events().all();

    let expected = AllowlistStateChanged {
        name: symbol_short!("al_st"),
        invoice_id,
        total_count: 0,
    };
    assert!(events.contains(&expected.to_xdr(&env, &contract_id)));
}
