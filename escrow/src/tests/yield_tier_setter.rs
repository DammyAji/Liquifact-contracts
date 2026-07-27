use super::super::tests::assert_contract_error;
use super::super::{
    EscrowError, LiquifactEscrow, LiquifactEscrowClient, YieldTier, YieldTierTableUpdated,
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
        &soroban_sdk::String::from_str(env, "YTTSET01"),
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

fn make_tier(env: &Env, lock: u64, bps: i64) -> YieldTier {
    YieldTier {
        min_lock_secs: lock,
        yield_bps: bps,
    }
}

#[test]
fn test_set_yield_tiers_and_read_back() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    tiers.push_back(make_tier(&env, 604_800, 500));

    client.set_yield_tiers(&tiers);

    let read_back = client.get_yield_tiers();
    assert_eq!(read_back.len(), 2);
    assert_eq!(read_back.get_unchecked(0), tiers.get_unchecked(0));
    assert_eq!(read_back.get_unchecked(1), tiers.get_unchecked(1));
}

#[test]
#[should_panic]
fn test_set_yield_tiers_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));

    env.mock_auths(&[]);
    client.set_yield_tiers(&tiers);
}

#[test]
fn test_set_yield_tiers_rejects_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    assert_contract_error(
        client.try_set_yield_tiers(&tiers),
        EscrowError::YieldTierTableInvalid,
    );
}

#[test]
fn test_set_yield_tiers_rejects_negative_bps() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, -1));

    assert_contract_error(
        client.try_set_yield_tiers(&tiers),
        EscrowError::YieldTierTableInvalid,
    );
}

#[test]
fn test_set_yield_tiers_rejects_bps_over_10000() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 10_001));

    assert_contract_error(
        client.try_set_yield_tiers(&tiers),
        EscrowError::YieldTierTableInvalid,
    );
}

#[test]
fn test_set_yield_tiers_rejects_non_increasing_locks() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    tiers.push_back(make_tier(&env, 86_400, 300));

    assert_contract_error(
        client.try_set_yield_tiers(&tiers),
        EscrowError::YieldTierTableInvalid,
    );
}

#[test]
fn test_set_yield_tiers_rejects_decreasing_yields() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 300));
    tiers.push_back(make_tier(&env, 604_800, 200));

    assert_contract_error(
        client.try_set_yield_tiers(&tiers),
        EscrowError::YieldTierTableInvalid,
    );
}

#[test]
fn test_set_yield_tiers_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);
    let invoice_id = client.get_escrow().invoice_id;
    let contract_id = client.address.clone();

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    tiers.push_back(make_tier(&env, 604_800, 500));

    client.set_yield_tiers(&tiers);
    let events = env.events().all();

    let expected = YieldTierTableUpdated {
        name: symbol_short!("yt_upd"),
        invoice_id,
        tier_count: 2,
    };
    assert!(events.contains(&expected.to_xdr(&env, &contract_id)));
}

#[test]
fn test_set_yield_tiers_accepts_max_bps() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 10_000));

    client.set_yield_tiers(&tiers);
    let read_back = client.get_yield_tiers();
    assert_eq!(read_back.len(), 1);
    assert_eq!(read_back.get_unchecked(0).yield_bps, 10_000);
}
