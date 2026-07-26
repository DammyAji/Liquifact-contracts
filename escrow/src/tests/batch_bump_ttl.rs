//! Comprehensive tests for [`LiquifactEscrow::batch_bump_ttl`].
//!
//! Coverage areas:
//! - Admin authorization required (non-admin caller rejected).
//! - Empty batch rejected with [`EscrowError::BumpTtlBatchEmpty`].
//! - Over-cap batch rejected with [`EscrowError::BumpTtlBatchTooLarge`].
//! - At-cap batch (exactly [`MAX_BUMP_TTL_BATCH`] entries) accepted.
//! - Single-key batches for each persistent-key variant succeed.
//! - Instance-storage key variants succeed.
//! - Mixed persistent + instance keys in one call succeed.
//! - Calling batch_bump_ttl on an un-initialized escrow is rejected.
//! - Repeated calls (idempotent TTL extension) do not error.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, Vec as SorobanVec,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Deploy and initialize a minimal escrow, returning `(client, admin, sme, token, treasury)`.
fn init_minimal(
    env: &Env,
) -> (
    LiquifactEscrowClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    env.mock_all_auths();
    let client = deploy(env);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, "BTT001"),
        &sme,
        &1_000i128,
        &500i64,
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
    (client, admin, sme, token, treasury)
}

/// Build a `SorobanVec<DataKey>` with `n` copies of `DataKey::Escrow` (all instance-storage keys).
fn instance_key_vec(env: &Env, n: usize) -> SorobanVec<DataKey> {
    let mut v = SorobanVec::new(env);
    for _ in 0..n {
        v.push_back(DataKey::Escrow);
    }
    v
}

/// Build a `SorobanVec<DataKey>` with `n` distinct investor contribution keys.
fn contribution_key_vec(env: &Env, n: usize) -> SorobanVec<DataKey> {
    let mut v = SorobanVec::new(env);
    for _ in 0..n {
        v.push_back(DataKey::InvestorContribution(Address::generate(env)));
    }
    v
}

// ── Negative tests — bounds / auth ───────────────────────────────────────────

#[test]
fn batch_bump_ttl_empty_keys_rejected() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let empty: SorobanVec<DataKey> = SorobanVec::new(&env);
    assert_contract_error(
        client.try_batch_bump_ttl(&empty),
        EscrowError::BumpTtlBatchEmpty,
    );
}

#[test]
fn batch_bump_ttl_over_cap_rejected() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    // MAX_BUMP_TTL_BATCH + 1 entries must be rejected.
    let over_cap = instance_key_vec(&env, (MAX_BUMP_TTL_BATCH + 1) as usize);
    assert_contract_error(
        client.try_batch_bump_ttl(&over_cap),
        EscrowError::BumpTtlBatchTooLarge,
    );
}

#[test]
fn batch_bump_ttl_exactly_at_cap_accepted() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    // Exactly MAX_BUMP_TTL_BATCH entries should succeed.
    let at_cap = instance_key_vec(&env, MAX_BUMP_TTL_BATCH as usize);
    client.batch_bump_ttl(&at_cap); // must not panic
}

#[test]
fn batch_bump_ttl_requires_admin_auth_non_admin_rejected() {
    let env = Env::default();
    // Deploy and init with mock_all_auths so init succeeds.
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    let contract_id = client.address.clone();

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "BTT_AUTH"),
        &sme,
        &1_000i128,
        &500i64,
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

    // Now set up auth as the non-admin only.
    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    let keys_val = soroban_sdk::Val::from(keys_vec.clone());

    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "batch_bump_ttl",
            args: soroban_sdk::vec![&env, keys_val],
            sub_invokes: &[],
        },
    }]);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.batch_bump_ttl(&keys_vec)
    }));
    assert!(
        result.is_err(),
        "expected panic when non-admin calls batch_bump_ttl"
    );
}

#[test]
fn batch_bump_ttl_requires_admin_auth_no_auth_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "BTT_NOAUTH"),
        &sme,
        &1_000i128,
        &500i64,
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

    // Clear all auths — call should panic because admin auth is missing.
    env.mock_auths(&[]);
    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.batch_bump_ttl(&keys_vec)
    }));
    assert!(
        result.is_err(),
        "expected panic when no auth is present for batch_bump_ttl"
    );
}

#[test]
fn batch_bump_ttl_not_initialized_panics() {
    let env = Env::default();
    env.mock_all_auths();
    // Do NOT call init — the contract has no escrow state.
    let client = deploy(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);

    // load_escrow_require_admin will panic with EscrowNotInitialized.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.batch_bump_ttl(&keys_vec)
    }));
    assert!(
        result.is_err(),
        "expected panic when escrow is not initialized"
    );
}

// ── Positive tests — instance-storage keys ───────────────────────────────────

#[test]
fn batch_bump_ttl_single_instance_key_escrow() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    client.batch_bump_ttl(&keys_vec); // must not panic
}

#[test]
fn batch_bump_ttl_single_instance_key_version() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Version);
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_single_instance_key_legal_hold() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::LegalHold);
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_single_instance_key_funding_token() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::FundingToken);
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_single_instance_key_min_contribution_floor() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::MinContributionFloor);
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_multiple_instance_keys() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    keys_vec.push_back(DataKey::Version);
    keys_vec.push_back(DataKey::LegalHold);
    keys_vec.push_back(DataKey::FundingToken);
    keys_vec.push_back(DataKey::Treasury);
    keys_vec.push_back(DataKey::AllowlistActive);
    keys_vec.push_back(DataKey::UniqueFunderCount);
    keys_vec.push_back(DataKey::MinContributionFloor);
    keys_vec.push_back(DataKey::FundingCloseSnapshot);
    client.batch_bump_ttl(&keys_vec);
}

// ── Positive tests — persistent per-investor keys ────────────────────────────

#[test]
fn batch_bump_ttl_single_persistent_key_investor_contribution() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorContribution(investor.clone()));
    client.batch_bump_ttl(&keys_vec); // no panic even if key absent
}

#[test]
fn batch_bump_ttl_single_persistent_key_investor_effective_yield() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorEffectiveYield(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_single_persistent_key_investor_claim_not_before() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorClaimNotBefore(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_single_persistent_key_investor_claimed() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorClaimed(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_single_persistent_key_investor_allowlisted() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorAllowlisted(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_multiple_persistent_keys_multiple_investors() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorContribution(alice.clone()));
    keys_vec.push_back(DataKey::InvestorEffectiveYield(alice.clone()));
    keys_vec.push_back(DataKey::InvestorClaimNotBefore(alice.clone()));
    keys_vec.push_back(DataKey::InvestorClaimed(alice.clone()));
    keys_vec.push_back(DataKey::InvestorAllowlisted(alice.clone()));
    keys_vec.push_back(DataKey::InvestorContribution(bob.clone()));
    keys_vec.push_back(DataKey::InvestorAllowlisted(bob.clone()));
    keys_vec.push_back(DataKey::InvestorContribution(carol.clone()));
    client.batch_bump_ttl(&keys_vec);
}

// ── Positive tests — mixed keys ───────────────────────────────────────────────

#[test]
fn batch_bump_ttl_mixed_persistent_and_instance_keys() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    // Instance keys
    keys_vec.push_back(DataKey::Escrow);
    keys_vec.push_back(DataKey::Version);
    keys_vec.push_back(DataKey::LegalHold);
    // Persistent keys
    keys_vec.push_back(DataKey::InvestorContribution(investor.clone()));
    keys_vec.push_back(DataKey::InvestorEffectiveYield(investor.clone()));
    keys_vec.push_back(DataKey::InvestorClaimNotBefore(investor.clone()));
    keys_vec.push_back(DataKey::InvestorClaimed(investor.clone()));
    keys_vec.push_back(DataKey::InvestorAllowlisted(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

// ── Positive tests — after state transitions ─────────────────────────────────

#[test]
fn batch_bump_ttl_after_funding_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "BTT_FUND"),
        &sme,
        &1_000i128,
        &500i64,
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
    client.fund(&investor, &1_000i128);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    keys_vec.push_back(DataKey::InvestorContribution(investor.clone()));
    keys_vec.push_back(DataKey::InvestorEffectiveYield(investor.clone()));
    keys_vec.push_back(DataKey::InvestorClaimNotBefore(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_after_settlement_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "BTT_SETTLE"),
        &sme,
        &1_000i128,
        &500i64,
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
    client.fund(&investor, &1_000i128);
    client.settle();

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    keys_vec.push_back(DataKey::InvestorContribution(investor.clone()));
    keys_vec.push_back(DataKey::InvestorClaimed(investor.clone()));
    client.batch_bump_ttl(&keys_vec);
}

#[test]
fn batch_bump_ttl_after_allowlist_set_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "BTT_ALWL"),
        &sme,
        &1_000i128,
        &500i64,
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
    client.set_investor_allowlisted(&investor, &true);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::InvestorAllowlisted(investor.clone()));
    keys_vec.push_back(DataKey::AllowlistActive);
    client.batch_bump_ttl(&keys_vec);
}

// ── Idempotency ───────────────────────────────────────────────────────────────

#[test]
fn batch_bump_ttl_repeated_calls_idempotent() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);
    let investor = Address::generate(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    keys_vec.push_back(DataKey::InvestorContribution(investor.clone()));

    // Call multiple times — Soroban's extend_ttl never shortens, so repeated calls are safe.
    client.batch_bump_ttl(&keys_vec);
    client.batch_bump_ttl(&keys_vec);
    client.batch_bump_ttl(&keys_vec);
}

// ── Boundary values ───────────────────────────────────────────────────────────

#[test]
fn batch_bump_ttl_single_key_is_minimum_valid() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    client.batch_bump_ttl(&keys_vec); // n == 1 is the minimum accepted
}

#[test]
fn batch_bump_ttl_persistent_keys_at_cap() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    // Fill exactly MAX_BUMP_TTL_BATCH with distinct InvestorContribution keys.
    let at_cap = contribution_key_vec(&env, MAX_BUMP_TTL_BATCH as usize);
    client.batch_bump_ttl(&at_cap); // must succeed
}

#[test]
fn batch_bump_ttl_persistent_keys_over_cap_rejected() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let over_cap = contribution_key_vec(&env, (MAX_BUMP_TTL_BATCH + 1) as usize);
    assert_contract_error(
        client.try_batch_bump_ttl(&over_cap),
        EscrowError::BumpTtlBatchTooLarge,
    );
}

// ── No state mutation ─────────────────────────────────────────────────────────

#[test]
fn batch_bump_ttl_does_not_change_escrow_state() {
    let env = Env::default();
    let (client, _admin, _sme, _token, _treasury) = init_minimal(&env);

    let escrow_before = client.get_escrow();

    let mut keys_vec: SorobanVec<DataKey> = SorobanVec::new(&env);
    keys_vec.push_back(DataKey::Escrow);
    keys_vec.push_back(DataKey::Version);
    keys_vec.push_back(DataKey::LegalHold);
    client.batch_bump_ttl(&keys_vec);

    let escrow_after = client.get_escrow();

    // All escrow fields must be identical before and after the TTL extension.
    assert_eq!(escrow_before.invoice_id, escrow_after.invoice_id);
    assert_eq!(escrow_before.admin, escrow_after.admin);
    assert_eq!(escrow_before.sme_address, escrow_after.sme_address);
    assert_eq!(escrow_before.amount, escrow_after.amount);
    assert_eq!(escrow_before.funded_amount, escrow_after.funded_amount);
    assert_eq!(escrow_before.yield_bps, escrow_after.yield_bps);
    assert_eq!(escrow_before.maturity, escrow_after.maturity);
    assert_eq!(escrow_before.status, escrow_after.status);
}
