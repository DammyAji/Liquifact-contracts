use super::super::{LiquifactEscrow, LiquifactEscrowClient, YieldTier, YieldTierPreview};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Vec as SorobanVec};

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
        &soroban_sdk::String::from_str(env, "YTPREV01"),
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
fn test_preview_yield_tier_returns_struct() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let result: YieldTierPreview = client.preview_yield_tier(&1_000i128, &0u64);
    assert_eq!(result.effective_yield_bps, 800);
    assert_eq!(result.matched_lock_secs, 0);
}

#[test]
fn test_preview_yield_tier_matches_tier() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    tiers.push_back(make_tier(&env, 604_800, 500));
    client.set_yield_tiers(&tiers);

    let result = client.preview_yield_tier(&1_000i128, &604_800u64);
    assert_eq!(result.effective_yield_bps, 500);
    assert_eq!(result.matched_lock_secs, 604_800);
}

#[test]
fn test_preview_yield_tier_picks_highest_qualifying() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    tiers.push_back(make_tier(&env, 604_800, 500));
    client.set_yield_tiers(&tiers);

    let result = client.preview_yield_tier(&1_000i128, &1_000_000u64);
    assert_eq!(result.effective_yield_bps, 500);
    assert_eq!(result.matched_lock_secs, 604_800);
}

#[test]
fn test_preview_yield_tier_falls_back_to_base_when_no_tier_qualifies() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    client.set_yield_tiers(&tiers);

    let result = client.preview_yield_tier(&1_000i128, &10u64);
    assert_eq!(result.effective_yield_bps, 800);
    assert_eq!(result.matched_lock_secs, 0);
}

#[test]
fn test_preview_yield_tier_zero_lock_falls_back_to_base() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let (admin, _) = init_escrow(&env, &client);

    let mut tiers: SorobanVec<YieldTier> = SorobanVec::new(&env);
    tiers.push_back(make_tier(&env, 86_400, 200));
    client.set_yield_tiers(&tiers);

    let result = client.preview_yield_tier(&1_000i128, &0u64);
    assert_eq!(result.effective_yield_bps, 800);
    assert_eq!(result.matched_lock_secs, 0);
}
