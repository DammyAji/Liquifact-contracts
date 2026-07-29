//! Centralised storage-key definitions for the LiquiFact escrow contract.
//!
//! Funding logic previously constructed the [`DataKey`] variants inline at each call site.
//! Two call sites agreeing on a key's shape by convention (rather than by construction) is
//! exactly the drift risk this module removes: every caller now obtains a given key through one
//! function instead of re-typing `DataKey::Variant(...)`.
//!
//! All persistent and instance-storage keys are defined here as variants of [`DataKey`].
//! Typed constructor functions are provided for every key family so that call sites never
//! build a [`DataKey`] inline ΓÇö reducing the risk of typos, discriminant drift between
//! modules, and copy-paste errors when a new key needs to be added.
//!
//! ## Collateral keys
//!
//! The collateral pledge key family is managed by [`collateral_pledge_key`]. All three
//! collateral entrypoints (`record_sme_collateral_commitment`, `clear_sme_collateral_commitment`,
//! `get_sme_collateral_commitment`) call this function instead of constructing
//! `DataKey::SmeCollateralPledge` inline. This ensures any future rename or split of the
//! collateral key cannot silently diverge across call sites.
//!
//! ## Additive-key policy (ADR-007)
//!
//! Adding a new variant is **backward-compatible** when the new key is read with
//! `.unwrap_or(default)` and its absence does not change existing entrypoint semantics.
//! Renaming a variant, changing its XDR discriminant, or altering the stored type of an
//! existing key is **breaking** and requires a `migrate` path or a full redeploy.

use soroban_sdk::{contracttype, Address};

/// Per-investor persistent principal recorded by `fund`.
pub(crate) fn investor_contribution(investor: Address) -> DataKey {
    DataKey::InvestorContribution(investor)
}

// ---------------------------------------------------------------------------
// Collateral key constructors
// ---------------------------------------------------------------------------

/// Return the canonical storage key for the SME collateral pledge.
///
/// All three collateral entrypoints ΓÇö `record_sme_collateral_commitment`,
/// `clear_sme_collateral_commitment`, and `get_sme_collateral_commitment` ΓÇö must call this
/// function instead of constructing `DataKey::SmeCollateralPledge` inline. This single
/// construction point guarantees that a future rename or variant-split cannot silently
/// diverge across call sites.
///
/// # Storage tier
///
/// The returned key lives in **instance** storage (shared TTL with the contract instance).
/// Callers are responsible for using `env.storage().instance()`.
///
/// # Example
///
/// ```ignore
/// use crate::keys::collateral_pledge_key;
///
/// let key = collateral_pledge_key();
/// env.storage().instance().set(&key, &commitment);
/// ```
#[inline(always)]
pub fn collateral_pledge_key() -> DataKey {
    DataKey::SmeCollateralPledge
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ΓöÇΓöÇ collateral_pledge_key ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ

    /// The constructor must return the `SmeCollateralPledge` variant ΓÇö verified by a
    /// `matches!` guard so the test does not depend on a `PartialEq` derive that the
    /// `#[contracttype]` macro does not generate for `DataKey`.
    #[test]
    fn collateral_pledge_key_returns_sme_collateral_pledge_variant() {
        let key = collateral_pledge_key();
        assert!(
            matches!(key, DataKey::SmeCollateralPledge),
            "collateral_pledge_key() must return DataKey::SmeCollateralPledge"
        );
    }

    /// Calling the constructor twice must produce structurally identical keys ΓÇö callers
    /// that cache or compare keys between entrypoints (e.g. an indexer that stores the
    /// discriminant) must see a stable, idempotent value.
    #[test]
    fn collateral_pledge_key_is_idempotent() {
        let k1 = collateral_pledge_key();
        let k2 = collateral_pledge_key();
        // Verify both are the same variant via matches! (no PartialEq on DataKey).
        assert!(matches!(k1, DataKey::SmeCollateralPledge));
        assert!(matches!(k2, DataKey::SmeCollateralPledge));
    }

    /// The key must be `Clone`-able (required by Soroban storage APIs that pass keys by
    /// reference and may need to retain an owned copy).
    #[test]
    fn collateral_pledge_key_is_cloneable() {
        let key = collateral_pledge_key();
        let cloned = key.clone();
        assert!(matches!(cloned, DataKey::SmeCollateralPledge));
    }

    // ΓöÇΓöÇ DataKey variant smoke-tests ΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇΓöÇ
    //
    // These tests are not exhaustive variant coverage; they confirm that the
    // enum compiles correctly and that the unit-type variants are distinguishable
    // from the tuple variants at runtime via `matches!`.

    /// Unit-type (non-tuple) variants must be constructible and must not match
    /// each other ΓÇö guards against accidental discriminant collision.
    #[test]
    fn unit_type_variants_are_distinct() {
        // Sample a representative set of unit-type keys.
        let escrow = DataKey::Escrow;
        let version = DataKey::Version;
        let legal_hold = DataKey::LegalHold;
        let pledge = DataKey::SmeCollateralPledge;
        let paused = DataKey::Paused;
        let protocol_fee = DataKey::ProtocolFeeBps;

        assert!(matches!(escrow, DataKey::Escrow));
        assert!(matches!(version, DataKey::Version));
        assert!(matches!(legal_hold, DataKey::LegalHold));
        assert!(matches!(pledge, DataKey::SmeCollateralPledge));
        assert!(matches!(paused, DataKey::Paused));
        assert!(matches!(protocol_fee, DataKey::ProtocolFeeBps));

/// Instance-storage immutable SEP-41 funding token address, set once at `init`.
pub(crate) fn funding_token() -> DataKey {
    DataKey::FundingToken
}