use super::*;
use crate::{keys, FundingCloseSnapshot};

// Regression tests for issue #912: the centralized `keys` module must produce byte-for-byte
// identical storage keys to the raw `DataKey` variants it replaces, since the refactor promises
// "identical key layout; no migration needed". Each test writes a value under one construction
// (`keys::x(...)` or the raw `DataKey::X(...)`) and reads it back under the other — a typo or
// shape mismatch in the new module would surface as a defaulted/missing read, not a silent pass.

#[test]
fn investor_contribution_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&keys::investor_contribution(investor.clone()), &777i128);
    });
    let read_back: i128 = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&DataKey::InvestorContribution(investor.clone()))
            .unwrap()
    });
    assert_eq!(read_back, 777i128);
}

#[test]
fn investor_effective_yield_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::InvestorEffectiveYield(investor.clone()), &950i64);
    });
    let read_back: i64 = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&keys::investor_effective_yield(investor.clone()))
            .unwrap()
    });
    assert_eq!(read_back, 950i64);
}

#[test]
fn investor_claim_not_before_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(
            &keys::investor_claim_not_before(investor.clone()),
            &123_456u64,
        );
    });
    let read_back: u64 = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&DataKey::InvestorClaimNotBefore(investor.clone()))
            .unwrap()
    });
    assert_eq!(read_back, 123_456u64);
}

#[test]
fn investor_claimed_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::InvestorClaimed(investor.clone()), &true);
    });
    let read_back: bool = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&keys::investor_claimed(investor.clone()))
            .unwrap()
    });
    assert!(read_back);
}

#[test]
fn min_contribution_floor_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&keys::min_contribution_floor(), &42i128);
    });
    let read_back: i128 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::MinContributionFloor)
            .unwrap()
    });
    assert_eq!(read_back, 42i128);
}

#[test]
fn max_unique_investors_cap_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::MaxUniqueInvestorsCap, &7u32);
    });
    let read_back: u32 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&keys::max_unique_investors_cap())
            .unwrap()
    });
    assert_eq!(read_back, 7u32);
}

#[test]
fn max_per_investor_cap_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&keys::max_per_investor_cap(), &555i128);
    });
    let read_back: i128 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::MaxPerInvestorCap)
            .unwrap()
    });
    assert_eq!(read_back, 555i128);
}

#[test]
fn unique_funder_count_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::UniqueFunderCount, &3u32);
    });
    let read_back: u32 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&keys::unique_funder_count())
            .unwrap()
    });
    assert_eq!(read_back, 3u32);
}

#[test]
fn investor_index_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let mut index: SorobanVec<Address> = SorobanVec::new(&env);
        index.push_back(investor.clone());
        env.storage()
            .instance()
            .set(&keys::investor_index(), &index);
    });
    let read_back: SorobanVec<Address> = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::InvestorIndex)
            .unwrap()
    });
    assert_eq!(read_back.len(), 1);
    assert_eq!(read_back.get(0).unwrap(), investor);
}

#[test]
fn funding_deadline_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::FundingDeadline, &999_999u64);
    });
    let read_back: u64 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&keys::funding_deadline())
            .unwrap()
    });
    assert_eq!(read_back, 999_999u64);
}

#[test]
fn funding_close_snapshot_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);

    let snap = FundingCloseSnapshot {
        total_principal: 100i128,
        funding_target: 100i128,
        closed_at_ledger_timestamp: 0,
        closed_at_ledger_sequence: 100,
    };

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&keys::funding_close_snapshot(), &snap);
    });
    let read_back: FundingCloseSnapshot = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::FundingCloseSnapshot)
            .unwrap()
    });
    assert_eq!(read_back, snap);
}

#[test]
fn funding_token_key_matches_raw_datakey() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    default_init(&client, &env, &admin, &sme);
    let other_token = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::FundingToken, &other_token);
    });
    let read_back: Address = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&keys::funding_token())
            .unwrap()
    });
    assert_eq!(read_back, other_token);
}
