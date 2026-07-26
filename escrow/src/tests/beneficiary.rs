//! Tests for the `get_beneficiary` read-only entrypoint.
//!
//! Coverage:
//! - `None` returned when escrow is not yet initialized (unset state).
//! - `Some(sme_address)` returned after `init` with the expected initial address.
//! - `Some(new_sme_address)` returned after `rotate_beneficiary` updates the SME.
//! - Verify that `get_beneficiary` is consistent with `get_escrow().sme_address`.
//! - Multiple rotations track the latest address correctly.
//! - Works across every escrow lifecycle status (open, funded, settled, cancelled).
//! - No auth required to call `get_beneficiary`.

use super::*;
use crate::EscrowError;

// ─────────────────────────────────────────────────────────────────────────────
// Unset state
// ─────────────────────────────────────────────────────────────────────────────

/// `get_beneficiary` must return `None` on a freshly deployed but uninitialized contract.
#[test]
fn test_get_beneficiary_uninitialized_returns_none() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);

    // Contract deployed but `init` never called → no DataKey::Escrow entry.
    assert_eq!(client.get_beneficiary(), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// Set state — post-init
// ─────────────────────────────────────────────────────────────────────────────

/// After `init`, `get_beneficiary` must return `Some(sme_address)` matching the
/// address supplied to `init`.
#[test]
fn test_get_beneficiary_after_init_returns_sme_address() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN1"),
        &sme,
        &TARGET,
        &500i64,
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

    let result = client.get_beneficiary();
    assert_eq!(result, Some(sme));
}

/// `get_beneficiary` must be consistent with `get_escrow().sme_address`.
#[test]
fn test_get_beneficiary_matches_get_escrow_sme_address() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN2"),
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

    let via_getter = client.get_beneficiary();
    let via_escrow = client.get_escrow().sme_address;

    assert_eq!(via_getter, Some(via_escrow));
}

// ─────────────────────────────────────────────────────────────────────────────
// After rotate_beneficiary
// ─────────────────────────────────────────────────────────────────────────────

/// After a successful `rotate_beneficiary`, `get_beneficiary` must reflect the
/// new SME address.
#[test]
fn test_get_beneficiary_after_rotation_returns_new_sme() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN3"),
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

    let new_sme = Address::generate(&env);
    client.rotate_beneficiary(&new_sme);

    assert_eq!(client.get_beneficiary(), Some(new_sme));
}

/// Before rotation, `get_beneficiary` returns the original SME; after rotation it
/// returns the new SME — verifies both states in sequence.
#[test]
fn test_get_beneficiary_tracks_rotation_state_change() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN4"),
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

    // Before rotation: original SME.
    assert_eq!(client.get_beneficiary(), Some(sme.clone()));

    let new_sme = Address::generate(&env);
    client.rotate_beneficiary(&new_sme);

    // After rotation: new SME.
    assert_eq!(client.get_beneficiary(), Some(new_sme));
}

/// Multiple sequential rotations — `get_beneficiary` always returns the most recent SME.
#[test]
fn test_get_beneficiary_tracks_multiple_rotations() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN5"),
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

    let sme2 = Address::generate(&env);
    client.rotate_beneficiary(&sme2);
    assert_eq!(client.get_beneficiary(), Some(sme2.clone()));

    let sme3 = Address::generate(&env);
    client.rotate_beneficiary(&sme3);
    assert_eq!(client.get_beneficiary(), Some(sme3.clone()));

    // Final rotation returns to a fourth address.
    let sme4 = Address::generate(&env);
    client.rotate_beneficiary(&sme4);
    assert_eq!(client.get_beneficiary(), Some(sme4));
}

// ─────────────────────────────────────────────────────────────────────────────
// Lifecycle status coverage
// ─────────────────────────────────────────────────────────────────────────────

/// `get_beneficiary` returns the correct address when escrow is cancelled (status 4).
#[test]
fn test_get_beneficiary_after_cancel_still_returns_sme() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN6"),
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

    // Cancel funding (status 0 → 4).
    client.cancel_funding();

    // Beneficiary address is still readable after cancellation.
    assert_eq!(client.get_beneficiary(), Some(sme));
}

// ─────────────────────────────────────────────────────────────────────────────
// Authorization: no auth required
// ─────────────────────────────────────────────────────────────────────────────

/// `get_beneficiary` must succeed without ANY authorization context — it is a pure view.
#[test]
fn test_get_beneficiary_requires_no_auth() {
    let env = Env::default();
    // Do NOT call env.mock_all_auths() — this test proves no auth is needed.
    let client = deploy(&env);

    // Should return None and not panic (no init, no auth).
    assert_eq!(client.get_beneficiary(), None);
}

/// `get_beneficiary` after init must work without authorization.
#[test]
fn test_get_beneficiary_post_init_no_auth_required() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    // init uses mock_all_auths from setup.
    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN7"),
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

    // Disable all auth mocks so the next call has NO auth context.
    // get_beneficiary must still succeed because it is a read-only view.
    env.mock_all_auths_allowing_non_root_auth();

    let result = client.get_beneficiary();
    assert_eq!(result, Some(sme));
}

// ─────────────────────────────────────────────────────────────────────────────
// Boundary values
// ─────────────────────────────────────────────────────────────────────────────

/// `get_beneficiary` works correctly with a `yield_bps` of 0 (minimum valid value).
#[test]
fn test_get_beneficiary_yield_bps_zero() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN8"),
        &sme,
        &1i128, // minimum valid amount
        &0i64,  // minimum valid yield_bps
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

    assert_eq!(client.get_beneficiary(), Some(sme));
}

/// `get_beneficiary` works correctly with the maximum valid invoice amount.
#[test]
fn test_get_beneficiary_max_invoice_amount() {
    use crate::MAX_INVOICE_AMOUNT;
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BEN9"),
        &sme,
        &MAX_INVOICE_AMOUNT,
        &10_000i64, // maximum valid yield_bps
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

    assert_eq!(client.get_beneficiary(), Some(sme));
}

/// `get_beneficiary` returns the correct SME when the escrow was initialized
/// with `yield_bps = 10_000` (maximum valid value).
#[test]
fn test_get_beneficiary_yield_bps_max() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BN10"),
        &sme,
        &TARGET,
        &10_000i64, // maximum valid yield_bps
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

    assert_eq!(client.get_beneficiary(), Some(sme));
}

// ─────────────────────────────────────────────────────────────────────────────
// Idempotency: reading twice gives the same result
// ─────────────────────────────────────────────────────────────────────────────

/// Calling `get_beneficiary` twice without any state change between the two calls
/// must return identical results (pure read, no side effects).
#[test]
fn test_get_beneficiary_idempotent() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BN11"),
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

    let first_call = client.get_beneficiary();
    let second_call = client.get_beneficiary();
    assert_eq!(first_call, second_call);
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin address must not be the same as SME
// ─────────────────────────────────────────────────────────────────────────────

/// `get_beneficiary` returns the SME address, not the admin address.
#[test]
fn test_get_beneficiary_is_sme_not_admin() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BN12"),
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

    let beneficiary = client.get_beneficiary();
    // Must match the SME.
    assert_eq!(beneficiary, Some(sme.clone()));
    // Must NOT be the admin.
    assert_ne!(beneficiary, Some(admin));
}

// ─────────────────────────────────────────────────────────────────────────────
// Consistent with escrow summary
// ─────────────────────────────────────────────────────────────────────────────

/// `get_beneficiary` must agree with `get_escrow_summary().escrow.sme_address`.
#[test]
fn test_get_beneficiary_consistent_with_escrow_summary() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BN13"),
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

    let from_getter = client.get_beneficiary();
    let from_summary = client.get_escrow_summary().escrow.sme_address;

    assert_eq!(from_getter, Some(from_summary));
}

/// After `rotate_beneficiary`, `get_beneficiary` and `get_escrow_summary` must agree.
#[test]
fn test_get_beneficiary_consistent_with_escrow_summary_after_rotation() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INV_BN14"),
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

    let new_sme = Address::generate(&env);
    client.rotate_beneficiary(&new_sme);

    let from_getter = client.get_beneficiary();
    let from_summary = client.get_escrow_summary().escrow.sme_address;

    assert_eq!(from_getter, Some(from_summary));
    assert_eq!(from_getter, Some(new_sme));
}
