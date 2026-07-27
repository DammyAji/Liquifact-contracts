use super::*;

#[test]
fn test_funding_config_defaults_before_init() {
    let env = Env::default();
    let (client, _admin, _sme) = setup(&env);

    let cfg = client.get_funding_config();

    assert_eq!(cfg.funding_target, 0);
    assert_eq!(cfg.yield_bps, 0);
    assert_eq!(cfg.maturity, 0);
    assert_eq!(cfg.min_contribution_floor, 0);
    assert_eq!(cfg.max_unique_investors_cap, None);
    assert_eq!(cfg.max_per_investor_cap, None);
    assert_eq!(cfg.funding_deadline, None);
    assert_eq!(cfg.protocol_fee_bps, 0);
    assert!(cfg.yield_tiers.is_empty());
    assert!(!cfg.allowlist_active);
}

#[test]
fn test_funding_config_after_init() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INVCfg"),
        &sme,
        &TARGET,
        &800i64,
        &1000u64,
        &Address::generate(&env),
        &None,
        &Address::generate(&env),
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

    let cfg = client.get_funding_config();

    assert_eq!(cfg.funding_target, TARGET);
    assert_eq!(cfg.yield_bps, 800);
    assert_eq!(cfg.maturity, 1000);
    assert_eq!(cfg.min_contribution_floor, 0);
    assert_eq!(cfg.max_unique_investors_cap, None);
    assert_eq!(cfg.max_per_investor_cap, None);
    assert_eq!(cfg.funding_deadline, None);
    assert_eq!(cfg.protocol_fee_bps, 0);
    assert!(cfg.yield_tiers.is_empty());
    assert!(!cfg.allowlist_active);
}

#[test]
fn test_funding_config_after_target_update() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INVTgt"),
        &sme,
        &TARGET,
        &800i64,
        &0u64,
        &Address::generate(&env),
        &None,
        &Address::generate(&env),
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

    let new_target = 50_000_000_000i128;
    client.update_funding_target(&new_target);

    let cfg = client.get_funding_config();
    assert_eq!(cfg.funding_target, new_target);
}

#[test]
fn test_funding_config_with_caps_and_deadline() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    let deadline = 1_000_000u64;

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INVCap"),
        &sme,
        &TARGET,
        &500i64,
        &0u64,
        &Address::generate(&env),
        &None,
        &Address::generate(&env),
        &Some(10i128),
        &Some(5u32),
        &Some(500_000i128),
        &Some(200i64),
        &None,
        &None,
        &Some(deadline),
        &None,
        &None,
        &None::<i64>,
    );

    let cfg = client.get_funding_config();

    assert_eq!(cfg.funding_target, TARGET);
    assert_eq!(cfg.yield_bps, 500);
    assert_eq!(cfg.min_contribution_floor, 10);
    assert_eq!(cfg.max_unique_investors_cap, Some(5));
    assert_eq!(cfg.max_per_investor_cap, Some(500_000));
    assert_eq!(cfg.funding_deadline, Some(deadline));
    assert_eq!(cfg.protocol_fee_bps, 200);
    assert!(!cfg.allowlist_active);
}
