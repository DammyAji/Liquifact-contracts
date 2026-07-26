use super::*;
use crate::{
    EscrowError, PauseMaxDurationUpdated, PauseRateLimitUpdated, PausedChanged,
    MAX_PAUSE_MAX_DURATION_SECS, MAX_PAUSE_TOGGLE_LIMIT, MAX_PAUSE_TOGGLE_WINDOW_SECS,
    MIN_PAUSE_MAX_DURATION_SECS, MIN_PAUSE_TOGGLE_LIMIT, MIN_PAUSE_TOGGLE_WINDOW_SECS,
};
use soroban_sdk::{testutils::Events, token::StellarAssetClient, Event};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn init_open(
    client: &LiquifactEscrowClient<'_>,
    env: &Env,
    admin: &Address,
    sme: &Address,
    id: &str,
) -> (Address, Address) {
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        admin,
        &soroban_sdk::String::from_str(env, id),
        sme,
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
    (token, treasury)
}

fn init_funded(
    client: &LiquifactEscrowClient<'_>,
    env: &Env,
    admin: &Address,
    sme: &Address,
    investor: &Address,
    id: &str,
) -> (Address, Address) {
    let (token, treasury) = init_open(client, env, admin, sme, id);
    client.fund(investor, &TARGET);
    (token, treasury)
}

fn init_funded_with_real_token<'a>(
    env: &'a Env,
    admin: &Address,
    sme: &Address,
    investor: &Address,
    id: &str,
) -> (LiquifactEscrowClient<'a>, Address) {
    let sac = env.register_stellar_asset_contract_v2(Address::generate(env));
    let token_id = sac.address();
    let sac_admin = StellarAssetClient::new(env, &token_id);
    let treasury = Address::generate(env);
    let escrow_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(env, &escrow_id);
    client.init(
        admin,
        &soroban_sdk::String::from_str(env, id),
        sme,
        &TARGET,
        &800i64,
        &0u64,
        &token_id,
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
    sac_admin.mint(investor, &TARGET);
    client.fund(investor, &TARGET);
    (client, escrow_id)
}

fn init_settled<'a>(
    env: &'a Env,
    admin: &Address,
    sme: &Address,
    investor: &Address,
    id: &str,
) -> (LiquifactEscrowClient<'a>, Address, Address, Address) {
    let sac = env.register_stellar_asset_contract_v2(Address::generate(env));
    let token = sac.address();
    let treasury = Address::generate(env);
    let escrow_id = env.register(LiquifactEscrow, ());
    let client = LiquifactEscrowClient::new(env, &escrow_id);
    client.init(
        admin,
        &soroban_sdk::String::from_str(env, id),
        sme,
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
    let sac_admin = StellarAssetClient::new(env, &token);
    sac_admin.mint(investor, &TARGET);
    client.fund(investor, &TARGET);
    client.settle();
    (client, escrow_id, token, treasury)
}

// ── 1. fund ──────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn fund_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU001");
    client.set_paused(&true);
    client.fund(&investor, &TARGET);
}

#[test]
fn fund_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU002");
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());
    let escrow = client.fund(&investor, &TARGET);
    assert_eq!(escrow.status, 1);
}

// ── 2. fund_with_commitment ─────────────────────────────────────────────────

#[test]
#[should_panic]
fn fund_with_commitment_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU003");
    client.set_paused(&true);
    client.fund_with_commitment(&investor, &TARGET, &0u64);
}

#[test]
fn fund_with_commitment_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU004");
    client.set_paused(&true);
    client.set_paused(&false);
    let escrow = client.fund_with_commitment(&investor, &TARGET, &0u64);
    assert_eq!(escrow.status, 1);
}

// ── 3. fund_batch ───────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn fund_batch_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU005");
    client.set_paused(&true);
    let entries = SorobanVec::from_array(&env, [(investor.clone(), TARGET)]);
    client.fund_batch(&entries);
}

#[test]
fn fund_batch_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU006");
    client.set_paused(&true);
    client.set_paused(&false);
    let entries = SorobanVec::from_array(&env, [(investor.clone(), TARGET)]);
    let escrow = client.fund_batch(&entries);
    assert_eq!(escrow.status, 1);
}

// ── 4. settle ────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn settle_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU007");
    client.set_paused(&true);
    client.settle();
}

#[test]
fn settle_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU008");
    client.set_paused(&true);
    client.set_paused(&false);
    let escrow = client.settle();
    assert_eq!(escrow.status, 2);
}

// ── 5. withdraw ──────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn withdraw_blocked_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let (client, _escrow_id) = init_funded_with_real_token(&env, &admin, &sme, &investor, "PAU009");
    client.set_paused(&true);
    client.withdraw();
}

#[test]
fn withdraw_succeeds_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let (client, _escrow_id) = init_funded_with_real_token(&env, &admin, &sme, &investor, "PAU010");
    client.set_paused(&true);
    client.set_paused(&false);
    let escrow = client.withdraw();
    assert_eq!(escrow.status, 3);
}

// ── 6. claim_investor_payout ─────────────────────────────────────────────────

#[test]
#[should_panic]
fn claim_investor_payout_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU011");
    client.settle();
    client.set_paused(&true);
    client.claim_investor_payout(&investor);
}

#[test]
fn claim_investor_payout_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU012");
    client.settle();
    client.set_paused(&true);
    client.set_paused(&false);
    client.claim_investor_payout(&investor);
    assert!(client.is_investor_claimed(&investor));
}

// ── 7. Read-only views unaffected by pause ───────────────────────────────────

#[test]
fn read_views_unaffected_by_pause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU013");

    client.set_paused(&true);
    assert!(client.is_paused());

    // get_escrow
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 1);

    // get_escrow_summary
    let summary = client.get_escrow_summary();
    assert_eq!(summary.escrow.status, 1);

    // get_remaining_funding_capacity
    let cap = client.get_remaining_funding_capacity();
    assert_eq!(cap, 0);

    // get_funding_token
    let _token = client.get_funding_token();

    // get_treasury
    let _treasury = client.get_treasury();

    // get_version
    assert_eq!(client.get_version(), SCHEMA_VERSION);

    // get_contribution
    let contribution = client.get_contribution(&investor);
    assert_eq!(contribution, TARGET);

    // get_legal_hold — orthogonal
    assert!(!client.get_legal_hold());

    // get_min_contribution_floor
    let _floor = client.get_min_contribution_floor();

    // get_unique_funder_count
    assert_eq!(client.get_unique_funder_count(), 1);

    // is_allowlist_active
    let _ = client.is_allowlist_active();
}

#[test]
fn read_views_unaffected_by_pause_on_open_escrow() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU014");

    client.set_paused(&true);
    assert!(client.is_paused());

    // get_escrow on open escrow
    let escrow = client.get_escrow();
    assert_eq!(escrow.status, 0);

    // get_escrow_summary
    let summary = client.get_escrow_summary();
    assert_eq!(summary.escrow.status, 0);

    // get_remaining_funding_capacity should equal TARGET
    let cap = client.get_remaining_funding_capacity();
    assert_eq!(cap, TARGET);
}

// ── 8. Admin gating ──────────────────────────────────────────────────────────

#[test]
fn set_paused_by_admin_succeeds() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU015");
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());
}

#[test]
#[should_panic]
fn set_paused_by_non_admin_panics() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU016");
    env.mock_auths(&[]);
    client.set_paused(&true);
}

// ── 9. Redundant no-op calls ─────────────────────────────────────────────────

#[test]
fn set_paused_true_when_already_true_is_noop() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU017");
    client.set_paused(&true);
    assert!(client.is_paused());
    // Second call should succeed (no-op)
    client.set_paused(&true);
    assert!(client.is_paused());
}

#[test]
fn set_paused_false_when_already_false_is_noop() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU018");
    assert!(!client.is_paused());
    // Default is false, calling set_paused(false) should succeed
    client.set_paused(&false);
    assert!(!client.is_paused());
}

// ── 10. Event emission ───────────────────────────────────────────────────────

#[test]
fn set_paused_emits_event() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    init_open(&client, &env, &admin, &sme, "PAU019");

    client.set_paused(&true);
    let events = env.events().all();
    let last = events.events().last().unwrap().clone();
    assert_eq!(
        last,
        PausedChanged {
            name: symbol_short!("paused"),
            invoice_id: client.get_escrow().invoice_id,
            active: 1,
        }
        .to_xdr(&env, &contract_id)
    );

    client.set_paused(&false);
    let events = env.events().all();
    let last = events.events().last().unwrap().clone();
    assert_eq!(
        last,
        PausedChanged {
            name: symbol_short!("paused"),
            invoice_id: client.get_escrow().invoice_id,
            active: 0,
        }
        .to_xdr(&env, &contract_id)
    );
}

// ── 11. Orthogonal to legal hold ─────────────────────────────────────────────

#[test]
fn pause_orthogonal_to_legal_hold() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU020");

    // Pause doesn't affect legal hold
    client.set_paused(&true);
    assert!(!client.get_legal_hold());

    // Legal hold doesn't affect pause
    client.set_legal_hold(&true);
    assert!(client.is_paused());
    assert!(client.get_legal_hold());

    // Clearing pause leaves legal hold intact
    client.set_paused(&false);
    assert!(!client.is_paused());
    assert!(client.get_legal_hold());

    // Clearing legal hold leaves pause intact
    client.clear_legal_hold();
    assert!(!client.is_paused());
    assert!(!client.get_legal_hold());
}

// ── 12. Pause gate fires before status validation ─────────────────────────────

#[test]
#[should_panic]
fn pause_gate_fires_before_status_validation_fund() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU021");
    // Escrow is funded (status=1), but paused should block before "not open for funding"
    client.set_paused(&true);
    client.fund(&investor, &TARGET);
}

#[test]
#[should_panic]
fn pause_gate_fires_before_legal_hold_fund() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU022");
    client.set_paused(&true);
    client.set_legal_hold(&true);
    // Should panic with PausedBlocksFunding, not LegalHoldBlocksFunding
    client.fund(&investor, &TARGET);
}

#[test]
#[should_panic]
fn pause_gate_fires_before_legal_hold_settle() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU023");
    client.set_paused(&true);
    client.set_legal_hold(&true);
    // Should panic with PausedBlocksSettlement, not LegalHoldBlocksSettlement
    client.settle();
}

#[test]
#[should_panic]
fn pause_gate_fires_before_legal_hold_withdraw() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let (client, _escrow_id) = init_funded_with_real_token(&env, &admin, &sme, &investor, "PAU024");
    client.set_paused(&true);
    client.set_legal_hold(&true);
    // Should panic with PausedBlocksWithdrawal, not LegalHoldBlocksWithdrawal
    client.withdraw();
}

#[test]
#[should_panic]
fn pause_gate_fires_before_legal_hold_claim() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU025");
    client.settle();
    client.set_paused(&true);
    client.set_legal_hold(&true);
    // Should panic with PausedBlocksInvestorClaims, not LegalHoldBlocksInvestorClaims
    client.claim_investor_payout(&investor);
}

// ── 13. Typed error codes ─────────────────────────────────────────────────────

#[test]
fn fund_returns_typed_error_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU026");
    client.set_paused(&true);
    assert_contract_error(
        client.try_fund(&investor, &TARGET),
        EscrowError::PausedBlocksFunding,
    );
}

#[test]
fn settle_returns_typed_error_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU027");
    client.set_paused(&true);
    assert_contract_error(client.try_settle(), EscrowError::PausedBlocksSettlement);
}

#[test]
fn withdraw_returns_typed_error_when_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let investor = Address::generate(&env);
    let (client, _escrow_id) = init_funded_with_real_token(&env, &admin, &sme, &investor, "PAU028");
    client.set_paused(&true);
    assert_contract_error(client.try_withdraw(), EscrowError::PausedBlocksWithdrawal);
}

#[test]
fn claim_returns_typed_error_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_funded(&client, &env, &admin, &sme, &investor, "PAU029");
    client.settle();
    client.set_paused(&true);
    assert_contract_error(
        client.try_claim_investor_payout(&investor),
        EscrowError::PausedBlocksInvestorClaims,
    );
}

// ── 14. Multiple pauses toggle correctly ──────────────────────────────────────

#[test]
fn pause_toggle_cycle() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU030");

    assert!(!client.is_paused());
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());

    // Funding should succeed after cycle
    let escrow = client.fund(&investor, &TARGET);
    assert_eq!(escrow.status, 1);
}

// ── 15. Configurable pause max duration (default) ────────────────────────────

#[test]
fn get_pause_max_duration_defaults_to_zero() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU031");
    assert_eq!(client.get_pause_max_duration(), 0);
}

#[test]
fn pause_default_duration_never_auto_expires() {
    // Default (unconfigured) duration must reproduce the legacy behavior: a pause blocks
    // gated entrypoints indefinitely until explicitly cleared, no matter how far ledger
    // time advances.
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU032");

    client.set_paused(&true);
    env.ledger().set_timestamp(u64::MAX / 2);
    assert!(client.is_paused());
    assert_contract_error(
        client.try_fund(&investor, &TARGET),
        EscrowError::PausedBlocksFunding,
    );
}

// ── 16. set_pause_max_duration: valid updates and bounds ──────────────────────

#[test]
fn set_pause_max_duration_valid_update() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU033");

    let ret = client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
    assert_eq!(ret, MIN_PAUSE_MAX_DURATION_SECS);
    assert_eq!(client.get_pause_max_duration(), MIN_PAUSE_MAX_DURATION_SECS);
}

#[test]
fn set_pause_max_duration_accepts_lower_and_upper_bounds() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU034");

    client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
    assert_eq!(client.get_pause_max_duration(), MIN_PAUSE_MAX_DURATION_SECS);

    client.set_pause_max_duration(&MAX_PAUSE_MAX_DURATION_SECS);
    assert_eq!(client.get_pause_max_duration(), MAX_PAUSE_MAX_DURATION_SECS);
}

#[test]
fn set_pause_max_duration_zero_always_allowed_and_disables_limit() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU035");

    client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
    assert_eq!(client.get_pause_max_duration(), MIN_PAUSE_MAX_DURATION_SECS);

    let ret = client.set_pause_max_duration(&0u64);
    assert_eq!(ret, 0);
    assert_eq!(client.get_pause_max_duration(), 0);
}

#[test]
fn set_pause_max_duration_rejects_below_min() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU036");

    assert_contract_error(
        client.try_set_pause_max_duration(&(MIN_PAUSE_MAX_DURATION_SECS - 1)),
        EscrowError::PauseMaxDurationOutOfRange,
    );
    // Config unchanged after a rejected call.
    assert_eq!(client.get_pause_max_duration(), 0);
}

#[test]
fn set_pause_max_duration_rejects_above_max() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU037");

    assert_contract_error(
        client.try_set_pause_max_duration(&(MAX_PAUSE_MAX_DURATION_SECS + 1)),
        EscrowError::PauseMaxDurationOutOfRange,
    );
    assert_eq!(client.get_pause_max_duration(), 0);
}

#[test]
#[should_panic]
fn set_pause_max_duration_by_non_admin_panics() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU038");
    env.mock_auths(&[]);
    client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
}

#[test]
fn set_pause_max_duration_emits_event() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    init_open(&client, &env, &admin, &sme, "PAU039");

    client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
    let events = env.events().all();
    let last = events.events().last().unwrap().clone();
    assert_eq!(
        last,
        PauseMaxDurationUpdated {
            name: symbol_short!("pausemax"),
            invoice_id: client.get_escrow().invoice_id,
            old_value: 0,
            new_value: MIN_PAUSE_MAX_DURATION_SECS,
        }
        .to_xdr(&env, &contract_id)
    );
}

// ── 17. Pause auto-expiry once a max duration is configured ───────────────────

#[test]
fn pause_auto_expires_after_configured_duration() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU040");

    client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
    let paused_at = env.ledger().timestamp();
    client.set_paused(&true);
    assert!(client.is_paused());

    // Advance ledger time to exactly the expiry boundary (paused_at + max_duration).
    env.ledger()
        .set_timestamp(paused_at + MIN_PAUSE_MAX_DURATION_SECS);

    assert!(!client.is_paused());
    let escrow = client.fund(&investor, &TARGET);
    assert_eq!(escrow.status, 1);
}

#[test]
fn pause_not_yet_expired_still_blocks() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU041");

    client.set_pause_max_duration(&MIN_PAUSE_MAX_DURATION_SECS);
    let paused_at = env.ledger().timestamp();
    client.set_paused(&true);

    // One second before expiry: still blocked.
    env.ledger()
        .set_timestamp(paused_at + MIN_PAUSE_MAX_DURATION_SECS - 1);

    assert!(client.is_paused());
    assert_contract_error(
        client.try_fund(&investor, &TARGET),
        EscrowError::PausedBlocksFunding,
    );
}

// ── 18. get_pause_rate_limit (default) ─────────────────────────────────────────

#[test]
fn get_pause_rate_limit_defaults_to_disabled() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU042");
    assert_eq!(client.get_pause_rate_limit(), (0u32, 0u64));
}

// ── 19. set_pause_rate_limit: valid updates and bounds ─────────────────────────

#[test]
fn set_pause_rate_limit_valid_update() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU043");

    let ret = client.set_pause_rate_limit(&5u32, &(MIN_PAUSE_TOGGLE_WINDOW_SECS * 10));
    assert_eq!(ret, (5u32, MIN_PAUSE_TOGGLE_WINDOW_SECS * 10));
    assert_eq!(
        client.get_pause_rate_limit(),
        (5u32, MIN_PAUSE_TOGGLE_WINDOW_SECS * 10)
    );
}

#[test]
fn set_pause_rate_limit_accepts_lower_and_upper_bounds() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU044");

    client.set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &MIN_PAUSE_TOGGLE_WINDOW_SECS);
    assert_eq!(
        client.get_pause_rate_limit(),
        (MIN_PAUSE_TOGGLE_LIMIT, MIN_PAUSE_TOGGLE_WINDOW_SECS)
    );

    client.set_pause_rate_limit(&MAX_PAUSE_TOGGLE_LIMIT, &MAX_PAUSE_TOGGLE_WINDOW_SECS);
    assert_eq!(
        client.get_pause_rate_limit(),
        (MAX_PAUSE_TOGGLE_LIMIT, MAX_PAUSE_TOGGLE_WINDOW_SECS)
    );
}

#[test]
fn set_pause_rate_limit_zero_zero_disables() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU045");

    client.set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &MIN_PAUSE_TOGGLE_WINDOW_SECS);
    let ret = client.set_pause_rate_limit(&0u32, &0u64);
    assert_eq!(ret, (0u32, 0u64));
    assert_eq!(client.get_pause_rate_limit(), (0u32, 0u64));
}

#[test]
fn set_pause_rate_limit_rejects_limit_out_of_range() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU046");

    assert_contract_error(
        client
            .try_set_pause_rate_limit(&(MAX_PAUSE_TOGGLE_LIMIT + 1), &MIN_PAUSE_TOGGLE_WINDOW_SECS),
        EscrowError::PauseToggleLimitOutOfRange,
    );
    assert_eq!(client.get_pause_rate_limit(), (0u32, 0u64));
}

#[test]
fn set_pause_rate_limit_rejects_window_out_of_range() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU047");

    assert_contract_error(
        client
            .try_set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &(MAX_PAUSE_TOGGLE_WINDOW_SECS + 1)),
        EscrowError::PauseToggleWindowOutOfRange,
    );
    assert_contract_error(
        client
            .try_set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &(MIN_PAUSE_TOGGLE_WINDOW_SECS - 1)),
        EscrowError::PauseToggleWindowOutOfRange,
    );
    assert_eq!(client.get_pause_rate_limit(), (0u32, 0u64));
}

#[test]
fn set_pause_rate_limit_rejects_nonzero_toggles_with_zero_window() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU048");

    assert_contract_error(
        client.try_set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &0u64),
        EscrowError::PauseRateLimitInvalidCombination,
    );
}

#[test]
fn set_pause_rate_limit_rejects_zero_toggles_with_nonzero_window() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU049");

    assert_contract_error(
        client.try_set_pause_rate_limit(&0u32, &MIN_PAUSE_TOGGLE_WINDOW_SECS),
        EscrowError::PauseRateLimitInvalidCombination,
    );
}

#[test]
#[should_panic]
fn set_pause_rate_limit_by_non_admin_panics() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU050");
    env.mock_auths(&[]);
    client.set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &MIN_PAUSE_TOGGLE_WINDOW_SECS);
}

#[test]
fn set_pause_rate_limit_emits_event() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    init_open(&client, &env, &admin, &sme, "PAU051");

    client.set_pause_rate_limit(&MIN_PAUSE_TOGGLE_LIMIT, &MIN_PAUSE_TOGGLE_WINDOW_SECS);
    let events = env.events().all();
    let last = events.events().last().unwrap().clone();
    assert_eq!(
        last,
        PauseRateLimitUpdated {
            name: symbol_short!("pause_rl"),
            invoice_id: client.get_escrow().invoice_id,
            old_limit: 0,
            new_limit: MIN_PAUSE_TOGGLE_LIMIT,
            old_window_secs: 0,
            new_window_secs: MIN_PAUSE_TOGGLE_WINDOW_SECS,
        }
        .to_xdr(&env, &contract_id)
    );
}

// ── 20. Pause-toggle rate limit enforcement ────────────────────────────────────

#[test]
fn pause_toggle_rate_limit_enforced() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU052");

    client.set_pause_rate_limit(&2u32, &3_600u64);

    // Two toggles allowed within the window.
    client.set_paused(&true);
    client.set_paused(&false);

    // Third toggle within the same window is rejected.
    assert_contract_error(
        client.try_set_paused(&true),
        EscrowError::PauseToggleRateLimitExceeded,
    );
}

#[test]
#[should_panic]
fn pause_toggle_rate_limit_exceeded_panics() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU053");

    client.set_pause_rate_limit(&1u32, &3_600u64);
    client.set_paused(&true);
    client.set_paused(&false);
}

#[test]
fn pause_toggle_rate_limit_resets_after_window_elapses() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU054");

    client.set_pause_rate_limit(&1u32, &3_600u64);
    let window_start = env.ledger().timestamp();
    client.set_paused(&true);

    // Still within the window: blocked.
    assert_contract_error(
        client.try_set_paused(&false),
        EscrowError::PauseToggleRateLimitExceeded,
    );

    // Advance past the window boundary: allowed again.
    env.ledger().set_timestamp(window_start + 3_600);
    client.set_paused(&false);
    assert!(!client.is_paused());
}

#[test]
fn pause_toggle_rate_limit_resets_on_reconfiguration() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU055");

    client.set_pause_rate_limit(&1u32, &3_600u64);
    client.set_paused(&true);
    assert_contract_error(
        client.try_set_paused(&false),
        EscrowError::PauseToggleRateLimitExceeded,
    );

    // Reconfiguring the limit (even to the same values) resets the window immediately.
    client.set_pause_rate_limit(&1u32, &3_600u64);
    client.set_paused(&false);
    assert!(!client.is_paused());
}

#[test]
fn get_paused_at_none_before_first_pause_then_tracks_activation_timestamps() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU057");

    assert_eq!(client.get_paused_at(), None);

    let first_pause_at = env.ledger().timestamp();
    client.set_paused(&true);
    assert_eq!(client.get_paused_at(), Some(first_pause_at));

    // Unpausing does not clear the recorded activation timestamp.
    client.set_paused(&false);
    assert_eq!(client.get_paused_at(), Some(first_pause_at));

    // Re-activating records the new timestamp.
    env.ledger().set_timestamp(first_pause_at + 500);
    client.set_paused(&true);
    assert_eq!(client.get_paused_at(), Some(first_pause_at + 500));
}

#[test]
fn pause_toggle_rate_limit_disabled_by_default_matches_legacy_behavior() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU056");

    // Many toggles in immediate succession, no rate limit configured: all succeed.
    for _ in 0..10 {
        client.set_paused(&true);
        client.set_paused(&false);
    }
    assert!(!client.is_paused());
}

// ── 21. Pause arithmetic overflow/saturation safety (issue #823) ──────────────
//
// The pause subsystem has exactly three raw arithmetic operations (no subtraction is
// performed anywhere in this module — only two additions and a comparison):
//   1. `paused_at.checked_add(max_duration)`         in `paused_active`      — already checked.
//   2. `start.saturating_add(window_secs)`            in `set_paused`         — already saturating.
//   3. `window_count + 1` (now `window_count.saturating_add(1)`) in `set_paused`.
//
// (3) was hardened defensively: the preceding `ensure(window_count < toggle_limit, ...)`
// already made overflow mathematically unreachable (`window_count < toggle_limit <= u32::MAX`
// implies `window_count + 1 <= u32::MAX`), but the guard now holds by construction instead of
// by relying on that check never being reordered. The tests below exercise all three at their
// extreme `u64`/`u32` boundaries and assert typed-error rejection (never a wrapped/panicking
// wraparound) wherever rejection is the correct outcome.

#[test]
fn pause_expiry_checked_add_exact_boundary_no_overflow() {
    // Boundary case for `paused_at.checked_add(max_duration)`: choose `paused_at` so the sum
    // lands exactly on `u64::MAX` (no overflow). Confirms `checked_add` resolves the correct
    // expiry timestamp right up to the edge of the representable range instead of failing safe
    // prematurely.
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU058");

    client.set_pause_max_duration(&MAX_PAUSE_MAX_DURATION_SECS);
    let paused_at = u64::MAX - MAX_PAUSE_MAX_DURATION_SECS;
    env.ledger().set_timestamp(paused_at);
    client.set_paused(&true);
    assert!(client.is_paused());

    // One second before the exact-boundary expiry: still blocked.
    env.ledger().set_timestamp(u64::MAX - 1);
    assert!(client.is_paused());
    assert_contract_error(
        client.try_fund(&investor, &TARGET),
        EscrowError::PausedBlocksFunding,
    );

    // Exactly at the boundary (paused_at + max_duration == u64::MAX): expired.
    env.ledger().set_timestamp(u64::MAX);
    assert!(!client.is_paused());
    let escrow = client.fund(&investor, &TARGET);
    assert_eq!(escrow.status, 1);
}

#[test]
fn pause_expiry_checked_add_overflow_fails_safe_stays_paused() {
    // When `paused_at + max_duration` would overflow `u64`, `checked_add` returns `None` and
    // `paused_active` fails safe by treating the pause as still active. It must never wrap
    // around to a small timestamp and falsely report "expired".
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let investor = Address::generate(&env);
    init_open(&client, &env, &admin, &sme, "PAU059");

    client.set_pause_max_duration(&MAX_PAUSE_MAX_DURATION_SECS);
    // paused_at + MAX_PAUSE_MAX_DURATION_SECS overflows u64::MAX by construction.
    let paused_at = u64::MAX - 100;
    env.ledger().set_timestamp(paused_at);
    client.set_paused(&true);
    assert!(client.is_paused());

    // Advance to the maximum representable ledger timestamp: still paused, no wraparound.
    env.ledger().set_timestamp(u64::MAX);
    assert!(client.is_paused());
    assert_contract_error(
        client.try_fund(&investor, &TARGET),
        EscrowError::PausedBlocksFunding,
    );
}

#[test]
fn pause_rate_limit_window_saturating_add_no_wraparound_at_extreme_start() {
    // Boundary case for `start.saturating_add(window_secs)`: when `start` is close to
    // `u64::MAX`, the sum must saturate instead of wrapping. A wraparound bug would make the
    // window appear to have already elapsed (a small wrapped timestamp <= now), incorrectly
    // resetting the toggle count and bypassing the rate limit.
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU060");

    client.set_pause_rate_limit(&1u32, &MAX_PAUSE_TOGGLE_WINDOW_SECS);
    let start = u64::MAX - 100;
    env.ledger().set_timestamp(start);
    client.set_paused(&true);

    // Still within the (saturated) window at the same timestamp: rejected, not bypassed.
    assert_contract_error(
        client.try_set_paused(&false),
        EscrowError::PauseToggleRateLimitExceeded,
    );

    // One tick before the saturated ceiling: still rejected (no early/wrapped elapse).
    env.ledger().set_timestamp(u64::MAX - 1);
    assert_contract_error(
        client.try_set_paused(&false),
        EscrowError::PauseToggleRateLimitExceeded,
    );

    // At the saturated ceiling (u64::MAX): the window is correctly treated as elapsed.
    env.ledger().set_timestamp(u64::MAX);
    client.set_paused(&false);
    assert!(!client.is_paused());
}

#[test]
fn pause_toggle_rate_limit_reaches_configured_maximum_without_overflow() {
    // Exercises `window_count` incrementing up to the maximum configurable toggle limit
    // (MAX_PAUSE_TOGGLE_LIMIT = 1000) within a single window entirely through the public API,
    // proving the counter never panics as it approaches its bound and that the guard correctly
    // rejects the call that would exceed it.
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_open(&client, &env, &admin, &sme, "PAU061");

    client.set_pause_rate_limit(&MAX_PAUSE_TOGGLE_LIMIT, &MAX_PAUSE_TOGGLE_WINDOW_SECS);

    for i in 0..MAX_PAUSE_TOGGLE_LIMIT {
        client.set_paused(&(i % 2 == 0));
    }

    // window_count has reached MAX_PAUSE_TOGGLE_LIMIT without overflowing; the next toggle
    // within the same window must be rejected with the typed error, not a panic.
    assert_contract_error(
        client.try_set_paused(&true),
        EscrowError::PauseToggleRateLimitExceeded,
    );
}

#[test]
fn pause_toggle_count_increment_never_overflows_at_forced_extreme_storage_values() {
    // Adversarial boundary case for the `window_count` increment itself: force
    // `PauseToggleCountInWindow` and `PauseToggleLimit` to values at the very edge of `u32`
    // directly in storage (bypassing the setter's MAX_PAUSE_TOGGLE_LIMIT bound) to prove the
    // increment cannot panic even in a scenario more extreme than the public API can ever
    // produce.
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();
    init_open(&client, &env, &admin, &sme, "PAU062");

    let now = env.ledger().timestamp();
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::PauseToggleLimit, &u32::MAX);
        env.storage()
            .instance()
            .set(&DataKey::PauseToggleWindowSecs, &3_600u64);
        env.storage()
            .instance()
            .set(&DataKey::PauseToggleWindowStart, &now);
        env.storage()
            .instance()
            .set(&DataKey::PauseToggleCountInWindow, &(u32::MAX - 1));
    });

    // window_count (u32::MAX - 1) < toggle_limit (u32::MAX): allowed. The increment computes
    // `u32::MAX` via `saturating_add`, never an overflowing `+`.
    client.set_paused(&true);

    // window_count is now u32::MAX; the next call within the window is correctly rejected by
    // the `<` guard rather than ever re-attempting an overflowing increment.
    assert_contract_error(
        client.try_set_paused(&false),
        EscrowError::PauseToggleRateLimitExceeded,
    );
}
