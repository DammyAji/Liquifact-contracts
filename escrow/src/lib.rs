#![no_std]

pub mod errors;
pub mod external_calls;
pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

use errors::EscrowError;
use soroban_sdk::{contract, contractimpl, Address, Env};
use storage::{
    get_admin, get_escrow, get_fees_limit, get_legal_hold, get_paused, get_protocol_fee_bps,
    get_version, set_admin, set_escrow, set_fees_limit, set_protocol_fee_bps, set_version,
};
use types::{EscrowStatus, InvoiceEscrow};

pub const SCHEMA_VERSION: u32 = 6;

pub const INSTANCE_TTL_MIN_EXTENSION_LEDGERS: u32 = 5_000;

#[contract]
pub struct LiquifactEscrow;

#[contractimpl]
impl LiquifactEscrow {
    pub fn init(
        env: Env,
        admin: Address,
        sme: Address,
        funding_token: Address,
        invoice_id: u64,
        target_amount: i128,
        maturity: u64,
    ) -> Result<(), EscrowError> {
        if get_version(&env).is_some() {
            return Err(EscrowError::AlreadyInitialized);
        }

        set_version(&env, SCHEMA_VERSION);
        set_admin(&env, &admin);

        let escrow = InvoiceEscrow {
            invoice_id,
            sme,
            funding_token,
            target_amount,
            funded_amount: 0,
            settled_amount: 0,
            withdrawn_amount: 0,
            status: EscrowStatus::Open,
            maturity,
        };

        set_escrow(&env, &escrow);
        Ok(())
    }

    pub fn settle(env: Env, settle_amount: i128) -> Result<(), EscrowError> {
        if get_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }
        if get_legal_hold(&env) {
            return Err(EscrowError::LegalHoldActive);
        }

        let mut escrow = get_escrow(&env).ok_or(EscrowError::NotInitialized)?;
        escrow.sme.require_auth();

        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::EscrowNotInFundedState);
        }

        let current_time = env.ledger().timestamp();
        if current_time < escrow.maturity {
            return Err(EscrowError::MaturityNotReached);
        }

        let remaining_to_settle = escrow
            .funded_amount
            .checked_sub(escrow.settled_amount)
            .ok_or(EscrowError::SettlementAmountInvalid)?;

        if settle_amount <= 0 || settle_amount > remaining_to_settle {
            return Err(EscrowError::SettlementAmountInvalid);
        }

        escrow.settled_amount = escrow
            .settled_amount
            .checked_add(settle_amount)
            .ok_or(EscrowError::SettlementAmountInvalid)?;

        if escrow.settled_amount == escrow.funded_amount {
            escrow.status = EscrowStatus::Settled;
        }

        set_escrow(&env, &escrow);
        Ok(())
    }

    pub fn partial_settle(env: Env, settle_amount: i128) -> Result<(), EscrowError> {
        if get_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }
        if get_legal_hold(&env) {
            return Err(EscrowError::LegalHoldActive);
        }

        let mut escrow = get_escrow(&env).ok_or(EscrowError::NotInitialized)?;
        escrow.sme.require_auth();

        if escrow.status != EscrowStatus::Funded {
            return Err(EscrowError::EscrowNotInFundedState);
        }

        let remaining_to_settle = escrow
            .funded_amount
            .checked_sub(escrow.settled_amount)
            .ok_or(EscrowError::SettlementAmountInvalid)?;

        if settle_amount <= 0 || settle_amount > remaining_to_settle {
            return Err(EscrowError::SettlementAmountInvalid);
        }

        escrow.settled_amount = escrow
            .settled_amount
            .checked_add(settle_amount)
            .ok_or(EscrowError::SettlementAmountInvalid)?;

        if escrow.settled_amount == escrow.funded_amount {
            escrow.status = EscrowStatus::Settled;
        }

        set_escrow(&env, &escrow);
        Ok(())
    }

    pub fn withdraw(env: Env, amount: i128) -> Result<(), EscrowError> {
        if get_paused(&env) {
            return Err(EscrowError::ContractPaused);
        }
        if get_legal_hold(&env) {
            return Err(EscrowError::LegalHoldActive);
        }

        let mut escrow = get_escrow(&env).ok_or(EscrowError::NotInitialized)?;
        escrow.sme.require_auth();

        if escrow.status != EscrowStatus::Funded && escrow.status != EscrowStatus::Settled {
            return Err(EscrowError::EscrowNotInFundedState);
        }

        let available_to_withdraw = escrow
            .funded_amount
            .checked_sub(escrow.withdrawn_amount)
            .ok_or(EscrowError::WithdrawAmountInvalid)?;

        if amount <= 0 || amount > available_to_withdraw {
            return Err(EscrowError::WithdrawAmountInvalid);
        }

        escrow.withdrawn_amount = escrow
            .withdrawn_amount
            .checked_add(amount)
            .ok_or(EscrowError::WithdrawAmountInvalid)?;

        set_escrow(&env, &escrow);
        Ok(())
    }

    pub fn get_escrow(env: Env) -> Option<InvoiceEscrow> {
        get_escrow(&env)
    }

    pub fn get_version(env: Env) -> Option<u32> {
        get_version(&env)
    }

    pub fn set_fees_limit(env: Env, limit: i64) -> Result<i64, EscrowError> {
        let admin = get_admin(&env).ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

        if !(0..=10_000).contains(&limit) {
            return Err(EscrowError::FeesLimitOutOfRange);
        }

        set_fees_limit(&env, &limit);
        Ok(limit)
    }

    pub fn get_fees_limit(env: Env) -> i64 {
        get_fees_limit(&env)
    }

    pub fn set_protocol_fee_bps(env: Env, fee_bps: i64) -> Result<i64, EscrowError> {
        let admin = get_admin(&env).ok_or(EscrowError::NotInitialized)?;
        admin.require_auth();

        let limit = get_fees_limit(&env);
        if fee_bps < 0 || fee_bps > limit {
            return Err(EscrowError::ProtocolFeeBpsOutOfRange);
        }

        set_protocol_fee_bps(&env, &fee_bps);
        Ok(fee_bps)
    }

    pub fn get_protocol_fee_bps(env: Env) -> i64 {
        get_protocol_fee_bps(&env)
    }
}
