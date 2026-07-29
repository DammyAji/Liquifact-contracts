//! Contract-specific types and events for the LiquiFact invoice escrow.

use soroban_sdk::{contracttype, Address, Symbol};

/// Forward-only state machine for an invoice escrow.
///
/// Transitions: Open → Funded → Settled → Withdrawn (terminal)
/// or Open → Cancelled (terminal).
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Open = 0,
    Funded = 1,
    Settled = 2,
    Withdrawn = 3,
    Cancelled = 4,
}

/// Core invoice escrow state stored at [`super::DataKey::Escrow`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvoiceEscrow {
    pub invoice_id: u64,
    pub sme: Address,
    pub funding_token: Address,
    pub target_amount: i128,
    pub funded_amount: i128,
    pub settled_amount: i128,
    pub withdrawn_amount: i128,
    pub status: EscrowStatus,
    pub maturity: u64,
}

/// Snapshot captured atomically on first transition to funded.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundingCloseSnapshot {
    pub total_principal: i128,
    pub funding_target: i128,
    pub closed_at_ledger_timestamp: u64,
    pub closed_at_ledger_sequence: u32,
}

/// SME collateral metadata (record-only; no token custody).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmeCollateralCommitment {
    pub asset: Symbol,
    pub amount: i128,
    pub recorded_at: u64,
}

/// Yield tier for time-locked contributions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct YieldTier {
    pub min_lock_secs: u64,
    pub yield_bps: i64,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowInitialized {
    pub name: Symbol,
    pub escrow: InvoiceEscrow,
    pub funding_token: Address,
    pub treasury: Address,
    pub registry: Option<Address>,
    pub has_maturity_lock: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeesLimitUpdated {
    pub name: Symbol,
    pub invoice_id: u64,
    pub old_limit: i64,
    pub new_limit: i64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowCloseSnapshot {
    None,
    Some(FundingCloseSnapshot),
}
