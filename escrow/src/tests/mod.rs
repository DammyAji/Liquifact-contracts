//! Test modules for the Liquifact escrow contract.
//!
//! Each module covers a specific area of the contract's behaviour.

mod admin;
mod attestations;
mod auth_matrix;
mod batch_bump_ttl;
mod beneficiary;
mod bounds_validation;
mod cap_validation;
mod coverage;
mod external_calls;
mod external_calls_mocked;
mod funding;
mod funding_deadline_tests;
mod init;
mod integration;
mod integration_status_guards;
mod keys;
mod legal_hold;
mod migration_errors;
mod paginated_views;
mod pause;
mod properties;
mod reconciliation_lifecycle;
mod settlement;
mod settlement_batch;
mod yield_tier_overflow;
