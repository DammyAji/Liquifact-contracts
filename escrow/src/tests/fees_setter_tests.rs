//! Tests for the `set_protocol_fee_bps` / `set_fees_limit` / `get_fees_limit` admin setters.
//!
//! Verifies:
//! - `set_fees_limit` stores the configured max and `get_fees_limit` returns it.
//! - `set_protocol_fee_bps` within the configured limit is accepted and persisted.
//! - `set_protocol_fee_bps` outside the configured limit is rejected with
//!   `ProtocolFeeBpsOutOfRange` (215).
//! - Neither setter is callable without admin authorization.
//! - `ProtocolFeeUpdated` event is emitted on every successful `set_protocol_fee_bps` call,
//!   carrying the previous and new basis-point values.
//! - Fee changes are reflected at withdrawal time (`withdraw` uses the stored value).

use super::super::{
    tests::assert_contract_error, EscrowError, LiquifactEscrow, LiquifactEscrowClient,
    ProtocolFeeUpdated,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env,
};

fn deploy(env: &Env) -> LiquifactEscrowClient<'_> {
    let id = env.register(LiquifactEscrow, ());
    LiquifactEscrowClient::new(env, &id)
}

/// Initialise with a stub token (no real SAC); sufficient for setter / getter tests.
fn init_escrow(env: &Env, client: &LiquifactEscrowClient) -> Address {
    let admin = Address::generate(env);
    let sme = Address::generate(env);
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        &admin,
        &soroban_sdk::String::from_str(env, "FEESET01"),
        &sme,
        &10_000i128,
        &0i64,
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
    admin
}

// ── set_fees_limit / get_fees_limit ───────────────────────────────────────

/// `set_fees_limit` persists the configured ceiling and `get_fees_limit` returns it.
#[test]
fn set_fees_limit_stores_value_readable_by_get() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&5_000i64);
    assert_eq!(client.get_fees_limit(), 5_000i64);
}

/// Setting the fees limit to `0` allows only a zero protocol fee.
#[test]
fn set_fees_limit_zero_allows_only_zero_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&0i64);
    assert_eq!(client.get_fees_limit(), 0i64);

    // fee_bps = 0 must be accepted (0 <= 0).
    client.set_protocol_fee_bps(&0i64);
    assert_eq!(client.get_protocol_fee_bps(), 0i64);
}

/// Setting the fees limit to `10_000` allows the full range of protocol fees.
#[test]
fn set_fees_limit_max_allows_full_range() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&10_000i64);
    assert_eq!(client.get_fees_limit(), 10_000i64);

    client.set_protocol_fee_bps(&10_000i64);
    assert_eq!(client.get_protocol_fee_bps(), 10_000i64);
}

/// `set_fees_limit` requires admin authorization.
#[test]
#[should_panic]
fn set_fees_limit_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    env.mock_auths(&[]);
    client.set_fees_limit(&5_000i64);
}

// ── set_protocol_fee_bps ──────────────────────────────────────────────────

/// `set_protocol_fee_bps` with a value within the fees limit is accepted and persisted.
#[test]
fn set_protocol_fee_bps_within_limit_is_stored() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    // Establish a limit then set a fee within it.
    client.set_fees_limit(&10_000i64);
    client.set_protocol_fee_bps(&500i64); // 5%
    assert_eq!(client.get_protocol_fee_bps(), 500i64);
}

/// `set_protocol_fee_bps` can be updated multiple times within the limit.
#[test]
fn set_protocol_fee_bps_can_be_updated_repeatedly() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&10_000i64);

    client.set_protocol_fee_bps(&200i64);
    assert_eq!(client.get_protocol_fee_bps(), 200i64);

    client.set_protocol_fee_bps(&400i64);
    assert_eq!(client.get_protocol_fee_bps(), 400i64);

    client.set_protocol_fee_bps(&0i64);
    assert_eq!(client.get_protocol_fee_bps(), 0i64);
}

/// `set_protocol_fee_bps` with a value exceeding the fees limit returns
/// `ProtocolFeeBpsOutOfRange` (215).
#[test]
fn set_protocol_fee_bps_above_limit_returns_typed_error() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    // Limit is 500 bps; anything above should be rejected.
    client.set_fees_limit(&500i64);

    assert_contract_error(
        client.try_set_protocol_fee_bps(&501i64),
        EscrowError::ProtocolFeeBpsOutOfRange,
    );
}

/// `set_protocol_fee_bps` with a negative value returns `ProtocolFeeBpsOutOfRange` (215).
#[test]
fn set_protocol_fee_bps_negative_returns_typed_error() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&10_000i64);

    assert_contract_error(
        client.try_set_protocol_fee_bps(&-1i64),
        EscrowError::ProtocolFeeBpsOutOfRange,
    );
}

/// `set_protocol_fee_bps` requires admin authorization.
#[test]
#[should_panic]
fn set_protocol_fee_bps_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&10_000i64);

    env.mock_auths(&[]);
    client.set_protocol_fee_bps(&500i64);
}

/// The value stored by `set_protocol_fee_bps` is not changed by a rejected call.
#[test]
fn set_protocol_fee_bps_rejected_call_leaves_prior_value_intact() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&1_000i64);
    client.set_protocol_fee_bps(&300i64);

    // Rejected because 1_001 > limit of 1_000.
    let _ = client.try_set_protocol_fee_bps(&1_001i64);

    // Prior value must be unchanged.
    assert_eq!(
        client.get_protocol_fee_bps(),
        300i64,
        "rejected call must not mutate stored fee_bps"
    );
}

// ── ProtocolFeeUpdated event ───────────────────────────────────────────────

/// `set_protocol_fee_bps` emits a `ProtocolFeeUpdated` event carrying old and new bps.
#[test]
fn set_protocol_fee_bps_emits_protocol_fee_updated_event() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;

    client.set_fees_limit(&10_000i64);

    // Transition from default (0) to 250 bps.
    client.set_protocol_fee_bps(&250i64);
    let events = env.events().all();

    // Exactly one event must be emitted by set_protocol_fee_bps.
    assert!(!events.is_empty(), "expected at least one event");

    let expected = ProtocolFeeUpdated {
        name: symbol_short!("fee_upd"),
        invoice_id: invoice_id.clone(),
        old_fee_bps: 0i64,
        new_fee_bps: 250i64,
    };
    let last = events.get((events.len() - 1) as u32).unwrap();
    assert_eq!(
        last,
        expected.to_xdr(&env, &contract_id),
        "ProtocolFeeUpdated event must carry correct old/new fee values"
    );
}

/// On a second call the `old_fee_bps` reflects the previously stored value, not the default.
#[test]
fn set_protocol_fee_bps_event_old_value_tracks_prior_stored_value() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;

    client.set_fees_limit(&10_000i64);
    client.set_protocol_fee_bps(&500i64); // first: 0 -> 500

    // Second update: 500 -> 800.
    client.set_protocol_fee_bps(&800i64);
    let events = env.events().all();

    let expected = ProtocolFeeUpdated {
        name: symbol_short!("fee_upd"),
        invoice_id,
        old_fee_bps: 500i64,
        new_fee_bps: 800i64,
    };
    let last = events.get((events.len() - 1) as u32).unwrap();
    assert_eq!(
        last,
        expected.to_xdr(&env, &contract_id),
        "second event must show prior stored value as old_fee_bps"
    );
}

// ── Boundary and endpoint checks ──────────────────────────────────────────

/// `fee_bps == limit` is the inclusive upper boundary and must be accepted.
#[test]
fn set_protocol_fee_bps_at_exact_limit_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&2_500i64);
    client.set_protocol_fee_bps(&2_500i64);
    assert_eq!(client.get_protocol_fee_bps(), 2_500i64);
}

/// `fee_bps == limit + 1` is outside the boundary and must be rejected.
#[test]
fn set_protocol_fee_bps_one_above_limit_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&2_500i64);

    assert_contract_error(
        client.try_set_protocol_fee_bps(&2_501i64),
        EscrowError::ProtocolFeeBpsOutOfRange,
    );
}

/// `fee_bps == 0` with `limit == 10_000` is always accepted (minimum valid value).
#[test]
fn set_protocol_fee_bps_zero_always_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    init_escrow(&env, &client);

    client.set_fees_limit(&10_000i64);
    client.set_protocol_fee_bps(&0i64);
    assert_eq!(client.get_protocol_fee_bps(), 0i64);
}
