//! Centralized constructors for funding-related storage keys.
//!
//! Funding logic (`fund`/`fund_with_commitment`/`fund_batch`/`fund_impl`, the cap and
//! funding-deadline admin setters, `update_funding_target`, `partial_settle`, `unfund`, and
//! `bump_ttl`) previously constructed the [`DataKey`] variants below inline at each call site.
//! Two call sites agreeing on a key's shape by convention (rather than by construction) is
//! exactly the drift risk this module removes: every caller now obtains a given key through one
//! function instead of re-typing `DataKey::Variant(...)`.
//!
//! This is a pure indirection layer. No key's on-chain shape or `DataKey` discriminant changes —
//! that contract still belongs solely to [`DataKey`] (see ADR-007) — so no migration or
//! `SCHEMA_VERSION` bump is needed (issue #912).
//!
//! The module also centralizes the collateral key construction point used by the collateral
//! entrypoints, and it documents the additive-key policy for ADR-007 so future storage variants
//! can be introduced without silent drift.

use crate::DataKey;
use soroban_sdk::{contracttype, Address};

/// Per-investor persistent principal recorded by `fund` / `fund_with_commitment` / `fund_batch`.
pub(crate) fn investor_contribution(investor: Address) -> DataKey {
    DataKey::InvestorContribution(investor)
}

/// Per-investor persistent effective yield (bps) selected on the investor's first deposit.
pub(crate) fn investor_effective_yield(investor: Address) -> DataKey {
    DataKey::InvestorEffectiveYield(investor)
}

/// Per-investor persistent claim-not-before ledger timestamp (`0` = no extra claim gate).
pub(crate) fn investor_claim_not_before(investor: Address) -> DataKey {
    DataKey::InvestorClaimNotBefore(investor)
}

/// Per-investor persistent claimed-payout marker.
pub(crate) fn investor_claimed(investor: Address) -> DataKey {
    DataKey::InvestorClaimed(investor)
}

/// Instance-storage minimum per-call contribution floor (`0` = no floor).
pub(crate) fn min_contribution_floor() -> DataKey {
    DataKey::MinContributionFloor
}

/// Instance-storage cap on distinct investor addresses (absent = unlimited).
pub(crate) fn max_unique_investors_cap() -> DataKey {
    DataKey::MaxUniqueInvestorsCap
}

/// Instance-storage cap on total principal for a single investor address (absent = unlimited).
pub(crate) fn max_per_investor_cap() -> DataKey {
    DataKey::MaxPerInvestorCap
}

/// Instance-storage count of distinct investor addresses that have funded so far.
pub(crate) fn unique_funder_count() -> DataKey {
    DataKey::UniqueFunderCount
}

/// Instance-storage ordered list of investor addresses backing paginated enumeration.
pub(crate) fn investor_index() -> DataKey {
    DataKey::InvestorIndex
}

/// Instance-storage optional funding deadline timestamp (absent = no deadline).
pub(crate) fn funding_deadline() -> DataKey {
    DataKey::FundingDeadline
}

/// Instance-storage write-once pro-rata snapshot captured at the first funded transition.
pub(crate) fn funding_close_snapshot() -> DataKey {
    DataKey::FundingCloseSnapshot
}

/// Instance-storage immutable SEP-41 funding token address, set once at `init`.
pub(crate) fn funding_token() -> DataKey {
    DataKey::FundingToken
}

// ---------------------------------------------------------------------------
// DataKey enum
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
/// Storage discriminator for persisted contract state.
///
/// Most variants live in **instance** storage (shared TTL with the contract instance, bounded
/// aggregate size). Per-investor variants
/// [`InvestorContribution`], [`InvestorEffectiveYield`], [`InvestorClaimNotBefore`], and
/// [`InvestorClaimed`] use **persistent** storage (independent per-address TTL; see ADR-007 and
/// `docs/escrow-gas-storage-notes.md`). [`InvestorAllowlisted`] also uses persistent storage.
///
/// Optional keys are always read with `.get(...).unwrap_or(default)` so that deployments predating
/// a key behave as "unset / default" without panicking.
///
/// ## Additive-key policy (see ADR-007)
///
/// Adding a new variant is **backward-compatible** when the new key is read with
/// `.unwrap_or(default)` and its absence does not change existing entrypoint semantics.
/// Renaming a variant, changing its XDR discriminant, or altering the stored type of an
/// existing key is **breaking** and requires a `migrate` path or a full redeploy.
///
/// Derive rationale:
/// - `Clone`: required because keys are passed by reference into storage APIs and reused
///   across lookups/sets in the same execution path.
pub enum DataKey {
    /// Full escrow snapshot ([`InvoiceEscrow`]); rewritten atomically on every state transition.
    Escrow,
    /// Stored schema version; written once by [`LiquifactEscrow::init`] to [`SCHEMA_VERSION`]
    /// and updated by [`LiquifactEscrow::migrate`] when a migration path is implemented.
    /// Read with [`LiquifactEscrow::get_version`]. Never delete or rename this variant.
    Version,
    /// Per-investor contributed principal recorded during [`LiquifactEscrow::fund`].
    /// **Persistent** storage. Absent ⇒ `0`. One entry per investor address.
    InvestorContribution(Address),
    /// When true, compliance/legal hold blocks payouts and settlement finalization.
    /// Absent ⇒ `false` (no hold). Toggled by admin via [`LiquifactEscrow::set_legal_hold`].
    LegalHold,
    /// Optional minimum ledger timestamp when `LegalHold` may be cleared after a
    /// [`LiquifactEscrow::request_clear_legal_hold`] call.
    /// Absent ⇒ no clear request is pending.
    LegalHoldClearableAt,
    /// Configured minimum delay between [`LiquifactEscrow::request_clear_legal_hold`] and
    /// [`LiquifactEscrow::set_legal_hold(env, false)`]. Absent ⇒ `0`.
    LegalHoldClearDelay,
    /// Optional SME collateral commitment metadata (record-only — not an on-chain asset lock).
    /// Absent when no commitment has been recorded. Replaceable by the SME.
    ///
    /// **Do not construct this variant directly.** Use [`collateral_pledge_key`] to get the
    /// canonical key so all collateral call sites share a single construction point.
    SmeCollateralPledge,
    /// Set to `true` when an investor has exercised a claim after settlement.
    /// **Persistent** storage. Absent ⇒ `false`. Written once; a second claim returns without re-emitting.
    InvestorClaimed(Address),
    /// SEP-41 funding asset for this invoice instance; set once in [`LiquifactEscrow::init`].
    /// Immutable after init.
    FundingToken,
    /// Protocol treasury that may receive [`LiquifactEscrow::sweep_terminal_dust`]; set once in init.
    /// Immutable after init.
    Treasury,
    /// Optional registry contract id for indexers; **hint only**, not authority (see module rustdoc).
    /// Omitted from storage when unset at init. Absent ⇒ `None`.
    RegistryRef,
    /// Immutable tier table when configured at [`LiquifactEscrow::init`]; omitted when tiering is off.
    /// Absent ⇒ no tiering (base `yield_bps` applies to all investors).
    /// **Trust:** values are protocol-supplied at deploy; the contract never mutates this key after init.
    YieldTierTable,
    /// Set once when status first becomes **funded** (1); immutable thereafter (pro-rata denominator).
    /// Absent until the escrow reaches `status == 1`. See [`FundingCloseSnapshot`].
    FundingCloseSnapshot,
    /// Effective annualized yield in bps chosen at this investor's **first** deposit (see tiered yield).
    /// **Persistent** storage. Absent ⇒ falls back to [`InvoiceEscrow::yield_bps`]. One entry per investor address.
    InvestorEffectiveYield(Address),
    /// Minimum [`Env::ledger`] timestamp before [`LiquifactEscrow::claim_investor_payout`] (0 = no extra gate).
    /// **Persistent** storage. Absent ⇒ `0`. One entry per investor address; set on first deposit.
    InvestorClaimNotBefore(Address),
    /// Minimum [`LiquifactEscrow::fund`] / [`LiquifactEscrow::fund_with_commitment`] amount per call (0 = no floor).
    /// Written as `0` even when unconfigured so reads always succeed.
    MinContributionFloor,
    /// When set at [`LiquifactEscrow::init`], caps distinct investor addresses that may contribute.
    /// Absent ⇒ unlimited. Checked against [`DataKey::UniqueFunderCount`] on each new investor.
    MaxUniqueInvestorsCap,
    /// Optional immutable per-investor cap on total principal credited to a single address.
    /// Absent ⇒ unlimited. Checked against [`DataKey::InvestorContribution`] on every deposit.
    MaxPerInvestorCap,
    /// Proposed successor admin waiting for [`LiquifactEscrow::accept_admin`].
    /// Absent ⇒ no pending handover. Cleared after successful acceptance.
    PendingAdmin,
    /// Ledger timestamp (seconds) after which [`LiquifactEscrow::accept_admin`] rejects the
    /// pending proposal. Written alongside [`DataKey::PendingAdmin`] on every
    /// [`LiquifactEscrow::propose_admin`] call; cleared on acceptance or cancellation.
    PendingAdminExpiry,
    /// Count of distinct investor addresses that have a non-zero [`DataKey::InvestorContribution`].
    /// Written as `0` at init; incremented once per new investor in `fund_impl`.
    UniqueFunderCount,
    /// Admin-only **single-set** off-chain attestation digest (e.g. SHA-256 of a legal/KYC bundle).
    /// Absent until [`LiquifactEscrow::bind_primary_attestation_hash`] is called; single-set thereafter.
    PrimaryAttestationHash,
    /// Append-only audit chain of digests (bounded by [`MAX_ATTESTATION_APPEND_ENTRIES`]).
    /// Absent ⇒ empty log. See [`LiquifactEscrow::append_attestation_digest`].
    AttestationAppendLog,
    /// Per-index revocation marker for [`DataKey::AttestationAppendLog`] entries.
    /// Absent ⇒ not revoked. Written as `true` by [`LiquifactEscrow::revoke_attestation_digest`].
    /// Preserves the original digest for auditability while signalling supersession.
    AttestationRevoked(u32),
    /// When true, only allowlisted addresses may call [`LiquifactEscrow::fund`] or [`LiquifactEscrow::fund_with_commitment`].
    AllowlistActive,
    /// Whether a specific address is permitted to fund when [`DataKey::AllowlistActive`] is true.
    InvestorAllowlisted(Address),
    /// Index of allowlisted addresses for paginated enumeration.
    AllowlistIndex,
    /// Set to `true` once an investor's principal has been refunded in a cancelled escrow.
    /// Absent ⇒ `false`. Written once; prevents double-refund.
    InvestorRefunded(Address),
    /// Running total of principal already returned to investors via [`LiquifactEscrow::refund`].
    /// Absent ⇒ `0`. Incremented atomically with each successful refund transfer.
    /// Used by [`LiquifactEscrow::sweep_terminal_dust`] to compute outstanding liabilities:
    /// `outstanding = funded_amount - distributed_principal`.
    DistributedPrincipal,
    /// Configured maximum maturity horizon in seconds from current ledger time.
    /// Absent ⇒ falls back to [`DEFAULT_MATURITY_MAX_HORIZON_SECS`].
    /// Set at init and updatable via [`LiquifactEscrow::update_maturity_max_horizon`].
    MaturityMaxHorizon,
    /// Optional funding deadline timestamp; absent ⇒ no deadline.
    /// Written by [`LiquifactEscrow::init`] and extended by
    /// [`LiquifactEscrow::extend_funding_deadline`]; checked during [`LiquifactEscrow::fund`].
    FundingDeadline,
    /// Ordered list of all investor addresses; used for pagination via [`LiquifactEscrow::get_investors`].
    /// Absent ⇒ empty list (no investors yet funded).
    InvestorIndex,
    /// Ledger timestamp recorded when [`LiquifactEscrow::settle`] transitions status to 2.
    /// Absent ⇒ not yet settled, or legacy instance. Read via [`LiquifactEscrow::get_settled_at`].
    SettledAt,
    /// When true, a lightweight **operational pause** blocks risk-bearing entrypoints
    /// (`fund`, `settle`, `withdraw`, `claim_investor_payout`) for incident response.
    /// Absent ⇒ `false` (not paused). Toggled by admin via [`LiquifactEscrow::set_paused`].
    ///
    /// Orthogonal to [`DataKey::LegalHold`]: the pause has **no** compliance semantics and
    /// **no** two-phase clear delay — it is a single-call admin switch for incidents such as a
    /// suspected token bug. Either flag independently blocks the gated entrypoints.
    Paused,
    /// Immutable protocol fee in basis points (0..=10_000) applied to the SME disbursement
    /// at [`LiquifactEscrow::withdraw`]; set once in [`LiquifactEscrow::init`].
    /// Written as `0` even when unconfigured so reads always succeed (`.unwrap_or(0)`).
    /// Stored as `i64` to match the [`InvoiceEscrow::yield_bps`] basis-point convention.
    /// **Additive key (ADR-007):** absent on instances predating this key ⇒ read as `0`
    /// (no fee), preserving legacy full-principal disbursement semantics.
    ProtocolFeeBps,
}

// ---------------------------------------------------------------------------
// Collateral key constructors
// ---------------------------------------------------------------------------

/// Return the canonical storage key for the SME collateral pledge.
///
/// All three collateral entrypoints — `record_sme_collateral_commitment`,
/// `clear_sme_collateral_commitment`, and `get_sme_collateral_commitment` — must call this
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

    // ── collateral_pledge_key ────────────────────────────────────────────────

    /// The constructor must return the `SmeCollateralPledge` variant — verified by a
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

    /// Calling the constructor twice must produce structurally identical keys — callers
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

    // ── DataKey variant smoke-tests ──────────────────────────────────────────
    //
    // These tests are not exhaustive variant coverage; they confirm that the
    // enum compiles correctly and that the unit-type variants are distinguishable
    // from the tuple variants at runtime via `matches!`.

    /// Unit-type (non-tuple) variants must be constructible and must not match
    /// each other — guards against accidental discriminant collision.
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

        // Spot-check cross-variant inequality via exhaustive matches.
        assert!(!matches!(escrow, DataKey::Version));
        assert!(!matches!(version, DataKey::LegalHold));
        assert!(!matches!(legal_hold, DataKey::SmeCollateralPledge));
        assert!(!matches!(pledge, DataKey::Paused));
        assert!(!matches!(paused, DataKey::ProtocolFeeBps));
        assert!(!matches!(protocol_fee, DataKey::Escrow));
    }

    /// `SmeCollateralPledge` must not match any other unit-type variant — this is the key
    /// property that prevents silent cross-key reads/writes in the collateral entrypoints.
    #[test]
    fn collateral_pledge_key_does_not_match_other_unit_variants() {
        let key = collateral_pledge_key();
        assert!(!matches!(key, DataKey::Escrow));
        assert!(!matches!(key, DataKey::Version));
        assert!(!matches!(key, DataKey::LegalHold));
        assert!(!matches!(key, DataKey::LegalHoldClearableAt));
        assert!(!matches!(key, DataKey::LegalHoldClearDelay));
        assert!(!matches!(key, DataKey::FundingToken));
        assert!(!matches!(key, DataKey::Treasury));
        assert!(!matches!(key, DataKey::RegistryRef));
        assert!(!matches!(key, DataKey::YieldTierTable));
        assert!(!matches!(key, DataKey::FundingCloseSnapshot));
        assert!(!matches!(key, DataKey::MinContributionFloor));
        assert!(!matches!(key, DataKey::MaxUniqueInvestorsCap));
        assert!(!matches!(key, DataKey::MaxPerInvestorCap));
        assert!(!matches!(key, DataKey::PendingAdmin));
        assert!(!matches!(key, DataKey::PendingAdminExpiry));
        assert!(!matches!(key, DataKey::UniqueFunderCount));
        assert!(!matches!(key, DataKey::PrimaryAttestationHash));
        assert!(!matches!(key, DataKey::AttestationAppendLog));
        assert!(!matches!(key, DataKey::AllowlistActive));
        assert!(!matches!(key, DataKey::AllowlistIndex));
        assert!(!matches!(key, DataKey::DistributedPrincipal));
        assert!(!matches!(key, DataKey::MaturityMaxHorizon));
        assert!(!matches!(key, DataKey::FundingDeadline));
        assert!(!matches!(key, DataKey::InvestorIndex));
        assert!(!matches!(key, DataKey::SettledAt));
        assert!(!matches!(key, DataKey::Paused));
        assert!(!matches!(key, DataKey::ProtocolFeeBps));
    }
}
