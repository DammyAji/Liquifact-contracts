//! Centralised storage helpers for the LiquiFact invoice escrow contract.
//!
//! Every persistent or instance-storage read/write goes through one of the
//! functions in this module.  No entrypoint accesses `env.storage()` inline.
//!
//! # Additive-key policy
//!
//! A new `DataKey` variant is backward-compatible when:
//! 1. It is read with `.get().unwrap_or(default)` so absent keys behave as "unset".
//! 2. It does not change the XDR shape of any existing variant or stored struct.
//! 3. It does not alter semantics of existing entrypoints when absent.
//!
//! See ADR-007 for details.

use crate::types::{FundingCloseSnapshot, InvoiceEscrow, SmeCollateralCommitment};
use soroban_sdk::{contracttype, Address, Env};

// ---------------------------------------------------------------------------
// DataKey — all storage keys in one place
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    // ── Core instance keys (always present after init) ─────────────
    Escrow,
    Version,
    Admin,
    PendingAdmin,
    AdminProposalExpiry,
    FundingToken,
    Treasury,
    MinContributionFloor,
    UniqueFunderCount,
    LegalHold,

    // ── Optional instance keys ─────────────────────────────────────
    RegistryRef,
    YieldTierTable,
    FundingCloseSnapshot,
    SmeCollateralPledge,
    MaxUniqueInvestorsCap,
    MaxPerInvestorCap,
    PrimaryAttestationHash,
    AttestationAppendLog,
    SettledAt,
    FundingDeadline,
    AllowlistActive,
    PauseToggleLimit,
    PauseToggleWindowSecs,
    PauseToggleWindowStart,
    PauseToggleCountInWindow,
    ProtocolFeeBps,
    MaturityMaxHorizon,
    StorageLimit,
    InvestorIndex,

    // ── Fees limit ─────────────────────────────────────────────────
    FeesLimit,

    // ── Per-address investor keys (persistent storage) ─────────────
    InvestorContribution(Address),
    InvestorEffectiveYield(Address),
    InvestorClaimNotBefore(Address),
    InvestorClaimed(Address),
    InvestorAllowlisted(Address),
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

// ---------------------------------------------------------------------------
// Version / schema
// ---------------------------------------------------------------------------

pub fn get_version(env: &Env) -> Option<u32> {
    env.storage().instance().get(&DataKey::Version)
}

pub fn set_version(env: &Env, version: u32) {
    env.storage().instance().set(&DataKey::Version, &version);
}

// ---------------------------------------------------------------------------
// Escrow
// ---------------------------------------------------------------------------

pub fn get_escrow(env: &Env) -> Option<InvoiceEscrow> {
    env.storage().instance().get(&DataKey::Escrow)
}

pub fn set_escrow(env: &Env, escrow: &InvoiceEscrow) {
    env.storage().instance().set(&DataKey::Escrow, escrow);
}

// ---------------------------------------------------------------------------
// Legal hold
// ---------------------------------------------------------------------------

pub fn get_legal_hold(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::LegalHold)
        .unwrap_or(false)
}

pub fn set_legal_hold(env: &Env, value: &bool) {
    env.storage().instance().set(&DataKey::LegalHold, value);
}

// ---------------------------------------------------------------------------
// Paused
// ---------------------------------------------------------------------------

pub fn get_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::LegalHold)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, value: &bool) {
    env.storage().instance().set(&DataKey::LegalHold, value);
}

// ---------------------------------------------------------------------------
// SME collateral
// ---------------------------------------------------------------------------

pub fn get_sme_collateral(env: &Env) -> Option<SmeCollateralCommitment> {
    env.storage().instance().get(&DataKey::SmeCollateralPledge)
}

pub fn set_sme_collateral(env: &Env, collateral: &SmeCollateralCommitment) {
    env.storage()
        .instance()
        .set(&DataKey::SmeCollateralPledge, collateral);
}

// ---------------------------------------------------------------------------
// Funding token
// ---------------------------------------------------------------------------

pub fn get_funding_token(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::FundingToken)
}

// ---------------------------------------------------------------------------
// Treasury
// ---------------------------------------------------------------------------

pub fn get_treasury(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Treasury)
}

// ---------------------------------------------------------------------------
// Protocol fee bps
// ---------------------------------------------------------------------------

pub fn get_protocol_fee_bps(env: &Env) -> i64 {
    env.storage()
        .instance()
        .get(&DataKey::ProtocolFeeBps)
        .unwrap_or(0)
}

pub fn set_protocol_fee_bps(env: &Env, fee_bps: &i64) {
    env.storage()
        .instance()
        .set(&DataKey::ProtocolFeeBps, fee_bps);
}

// ---------------------------------------------------------------------------
// Fees limit
// ---------------------------------------------------------------------------

/// Default fees limit in basis points (10_000 = 100%).
pub const DEFAULT_FEES_LIMIT: i64 = 10_000;

/// Minimum valid fees limit (0%).
pub const MIN_FEES_LIMIT: i64 = 0;

/// Maximum valid fees limit (10_000 = 100%).
pub const MAX_FEES_LIMIT: i64 = 10_000;

/// Read the configured fees limit.  Returns [`DEFAULT_FEES_LIMIT`] when the
/// key has never been written (additive-key policy).
pub fn get_fees_limit(env: &Env) -> i64 {
    env.storage()
        .instance()
        .get(&DataKey::FeesLimit)
        .unwrap_or(DEFAULT_FEES_LIMIT)
}

/// Write a new fees limit.  The caller must validate bounds before calling this.
pub fn set_fees_limit(env: &Env, limit: &i64) {
    env.storage().instance().set(&DataKey::FeesLimit, limit);
}

/// Check whether `value` is within the acceptable range for a fees limit.
pub fn is_valid_fees_limit(value: i64) -> bool {
    (MIN_FEES_LIMIT..=MAX_FEES_LIMIT).contains(&value)
}

// ---------------------------------------------------------------------------
// Investor persistence keys (forwarded from `keys` module)
// ---------------------------------------------------------------------------

pub fn get_investor_contribution(env: &Env, investor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::InvestorContribution(investor.clone()))
        .unwrap_or(0)
}

pub fn set_investor_contribution(env: &Env, investor: &Address, amount: &i128) {
    env.storage()
        .persistent()
        .set(&DataKey::InvestorContribution(investor.clone()), amount);
}

// ---------------------------------------------------------------------------
// Misc instance helpers
// ---------------------------------------------------------------------------

pub fn get_unique_funder_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::UniqueFunderCount)
        .unwrap_or(0)
}

pub fn get_storage_limit(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::StorageLimit)
        .unwrap_or(5_000)
}

pub fn set_storage_limit(env: &Env, limit: &u32) {
    env.storage().instance().set(&DataKey::StorageLimit, limit);
}

pub fn get_funding_close_snapshot(env: &Env) -> Option<FundingCloseSnapshot> {
    env.storage().instance().get(&DataKey::FundingCloseSnapshot)
}
