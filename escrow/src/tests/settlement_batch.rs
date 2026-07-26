//! Batch settlement tests for the LiquiFact escrow contract.
//!
//! Covers `settle_batch` happy path, edge cases, and atomic rejection semantics.
//!
//! # Architecture note
//! `settle_batch` makes cross-contract calls to each target escrow's `settle()`.
//! Soroban prohibits re-entry, so the batch executor must be a **separate**
//! contract instance from any target escrow. In these tests we deploy an
//! uninitialized `LiquifactEscrow` instance to serve as the batch executor.
//!
//! # State model recap (ADR-001)
//! `settle` transitions status from 1 (funded) to 2 (settled).
//! `settle_batch` calls `settle()` on each target escrow atomically.

#[cfg(test)]
use super::{deploy, TARGET};
use crate::LiquifactEscrow;
use soroban_sdk::{testutils::Address as _, Address, Env, Vec as SorobanVec};

/// Deploy an uninitialized escrow contract to serve as the batch executor.
/// This contract has no escrow state — it only hosts `settle_batch`.
fn deploy_batch_executor(env: &Env) -> super::LiquifactEscrowClient<'_> {
    let id = env.register(LiquifactEscrow, ());
    super::LiquifactEscrowClient::new(env, &id)
}

/// Deploy and fund an escrow to `target` (status 1). Returns `(client, escrow_id)`.
fn setup_funded_escrow<'a>(
    env: &'a Env,
    target: i128,
    invoice_id: &str,
) -> (super::LiquifactEscrowClient<'a>, Address) {
    let client = deploy(env);
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, invoice_id),
        &sme,
        &target,
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

    let investor = Address::generate(env);
    client.fund(&investor, &target);

    let escrow_id = client.address.clone();
    (client, escrow_id)
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — happy path
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` with two funded escrows must settle both.
#[test]
fn settle_batch_success_two_escrows() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "BATCH_A");
    let (client_b, addr_b) = setup_funded_escrow(&env, TARGET, "BATCH_B");

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(addr_b.clone());

    executor.settle_batch(&batch);

    assert_eq!(
        client_a.get_escrow().status,
        2u32,
        "escrow_a must be settled"
    );
    assert_eq!(
        client_b.get_escrow().status,
        2u32,
        "escrow_b must be settled"
    );
}

/// `settle_batch` with a single escrow must work identically to `settle()`.
#[test]
fn settle_batch_single_escrow() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "SINGLE_A");

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());

    executor.settle_batch(&batch);

    assert_eq!(client_a.get_escrow().status, 2u32);
}

/// `settle_batch` with three escrows must settle all three atomically.
#[test]
fn settle_batch_success_three_escrows() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "BATCH3_A");
    let (client_b, addr_b) = setup_funded_escrow(&env, TARGET, "BATCH3_B");
    let (client_c, addr_c) = setup_funded_escrow(&env, TARGET, "BATCH3_C");

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a);
    batch.push_back(addr_b);
    batch.push_back(addr_c);

    executor.settle_batch(&batch);

    assert_eq!(client_a.get_escrow().status, 2u32);
    assert_eq!(client_b.get_escrow().status, 2u32);
    assert_eq!(client_c.get_escrow().status, 2u32);
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — empty and over-limit rejection
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must reject an empty escrows vector.
#[test]
fn settle_batch_empty_rejected() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);
    let batch = SorobanVec::<Address>::new(&env);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(result.is_err(), "empty batch must be rejected");
}

/// `settle_batch` must reject when the batch exceeds `MAX_SETTLE_BATCH`.
#[test]
fn settle_batch_over_limit_rejected() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let mut batch = SorobanVec::<Address>::new(&env);
    for _ in 0..51 {
        batch.push_back(Address::generate(&env));
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(result.is_err(), "over-limit batch must be rejected");
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — atomic rejection: one invalid entry fails the entire batch
// ──────────────────────────────────────────────────────────────────────────────

/// If one escrow in the batch is not yet funded (status 0), the entire batch must revert.
#[test]
fn settle_batch_one_unfunded_fails_all() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "ATOM_A");

    // Deploy an open (status 0) escrow.
    let open_client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    open_client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "ATOM_OPEN"),
        &sme,
        &TARGET,
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
    let addr_open = open_client.address.clone();

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(addr_open.clone());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(
        result.is_err(),
        "batch must revert when one escrow is not funded"
    );

    // Verify escrow_a was NOT settled (atomic revert).
    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — paused escrow blocks the batch
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must fail if any escrow in the batch is paused.
#[test]
fn settle_batch_paused_escrow_fails_batch() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "PAUSA");
    let (client_b, addr_b) = setup_funded_escrow(&env, TARGET, "PAUSB");

    client_b.set_paused(&true);

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(addr_b.clone());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(result.is_err(), "batch must fail when one escrow is paused");

    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — legal hold blocks the batch
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must fail if any escrow in the batch has a legal hold active.
#[test]
fn settle_batch_legal_hold_fails_batch() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "HOLDA");
    let (client_b, addr_b) = setup_funded_escrow(&env, TARGET, "HOLDB");

    client_b.set_legal_hold(&true);

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(addr_b.clone());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(
        result.is_err(),
        "batch must fail when one escrow has legal hold"
    );

    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — already-settled escrow fails the batch
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must fail if any escrow in the batch is already settled.
#[test]
fn settle_batch_already_settled_fails_batch() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "DONEA");
    let (client_b, addr_b) = setup_funded_escrow(&env, TARGET, "DONEB");

    client_b.settle();
    assert_eq!(client_b.get_escrow().status, 2u32);

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(addr_b.clone());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(
        result.is_err(),
        "batch must fail when one escrow is already settled"
    );

    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — all-or-nothing with multiple funded escrows
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must settle all escrows when all are in funded status.
#[test]
fn settle_batch_all_funded_settles_all() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "ALL_A");
    let (client_b, addr_b) = setup_funded_escrow(&env, TARGET, "ALL_B");
    let (client_c, addr_c) = setup_funded_escrow(&env, TARGET, "ALL_C");

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a);
    batch.push_back(addr_b);
    batch.push_back(addr_c);

    executor.settle_batch(&batch);

    assert_eq!(client_a.get_escrow().status, 2u32);
    assert_eq!(client_b.get_escrow().status, 2u32);
    assert_eq!(client_c.get_escrow().status, 2u32);
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — duplicate addresses in the batch
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` with duplicate escrow addresses: first settle succeeds, second fails
/// (already settled), causing the entire batch to revert.
#[test]
fn settle_batch_duplicate_escrow_fails() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "DUP_A");

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(addr_a.clone());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(
        result.is_err(),
        "batch with duplicate escrow must fail on second (already settled) entry"
    );

    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — non-existent escrow address
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must fail if any address in the batch does not correspond
/// to an initialized escrow contract.
#[test]
fn settle_batch_nonexistent_escrow_fails() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "NOESC");

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(Address::generate(&env));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(
        result.is_err(),
        "batch must fail when one address is not an initialized escrow"
    );

    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// `settle_batch` — open escrow (status 0) fails the batch
// ──────────────────────────────────────────────────────────────────────────────

/// `settle_batch` must fail if any escrow in the batch is open (status 0).
#[test]
fn settle_batch_open_escrow_fails_batch() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let executor = deploy_batch_executor(&env);

    let (client_a, addr_a) = setup_funded_escrow(&env, TARGET, "OPEN_BT");

    let open_client = deploy(&env);
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    open_client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "OPEN_B2"),
        &sme,
        &TARGET,
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

    let mut batch = SorobanVec::<Address>::new(&env);
    batch.push_back(addr_a.clone());
    batch.push_back(open_client.address.clone());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        executor.settle_batch(&batch);
    }));
    assert!(
        result.is_err(),
        "batch must fail when one escrow is open (status 0)"
    );

    assert_eq!(
        client_a.get_escrow().status,
        1u32,
        "escrow_a must remain funded after atomic revert"
    );
}
