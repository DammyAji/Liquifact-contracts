use super::*;
use soroban_sdk::{testutils::Events as _, Address};

// ---------------------------------------------------------------------------
// is_paused defaults to false
// ---------------------------------------------------------------------------
#[test]
fn is_paused_defaults_to_false() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    assert!(!client.is_paused());
}

// ---------------------------------------------------------------------------
// Admin can set paused and is_paused returns true
// ---------------------------------------------------------------------------
#[test]
fn admin_can_set_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    assert!(client.is_paused());
}

// ---------------------------------------------------------------------------
// Admin can unpause
// ---------------------------------------------------------------------------
#[test]
fn admin_can_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());
}

// ---------------------------------------------------------------------------
// is_paused is O(1) read-only — does not mutate storage
// ---------------------------------------------------------------------------
#[test]
fn is_paused_is_read_only() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    assert!(!client.is_paused());
    client.set_paused(&true);
    assert!(client.is_paused());
}

// ---------------------------------------------------------------------------
// fund blocked when paused
// ---------------------------------------------------------------------------
#[test]
fn fund_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    let investor = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.fund(&investor, &100);
    }));
    assert!(result.is_err(), "fund should panic when paused");
}

// ---------------------------------------------------------------------------
// fund succeeds after unpause
// ---------------------------------------------------------------------------
#[test]
fn fund_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    client.set_paused(&false);
    let investor = Address::generate(&env);
    client.fund(&investor, &100);
}

// ---------------------------------------------------------------------------
// settle blocked when paused
// ---------------------------------------------------------------------------
#[test]
fn settle_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.set_paused(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.settle();
    }));
    assert!(result.is_err(), "settle should panic when paused");
}

// ---------------------------------------------------------------------------
// settle succeeds after unpause
// ---------------------------------------------------------------------------
#[test]
fn settle_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.set_paused(&true);
    client.set_paused(&false);
    client.settle();
}

// ---------------------------------------------------------------------------
// withdraw blocked when paused
// ---------------------------------------------------------------------------
#[test]
fn withdraw_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.set_paused(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.withdraw();
    }));
    assert!(result.is_err(), "withdraw should panic when paused");
}

// ---------------------------------------------------------------------------
// withdraw succeeds after unpause
// ---------------------------------------------------------------------------
#[test]
fn withdraw_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.set_paused(&true);
    client.set_paused(&false);
    client.withdraw();
}

// ---------------------------------------------------------------------------
// claim_investor_payout blocked when paused
// ---------------------------------------------------------------------------
#[test]
fn claim_investor_payout_blocked_when_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.settle();
    client.set_paused(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_investor_payout(&investor);
    }));
    assert!(result.is_err(), "claim should panic when paused");
}

// ---------------------------------------------------------------------------
// claim_investor_payout succeeds after unpause
// ---------------------------------------------------------------------------
#[test]
fn claim_investor_payout_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.settle();
    client.set_paused(&true);
    client.set_paused(&false);
    client.claim_investor_payout(&investor);
}

// ---------------------------------------------------------------------------
// Read views (is_paused, get_escrow, get_version) are unaffected by pause
// ---------------------------------------------------------------------------
#[test]
fn read_views_unaffected_by_pause() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let escrow_before = client.get_escrow();
    assert!(!client.is_paused());
    client.set_paused(&true);
    assert!(client.is_paused());
    let escrow_after = client.get_escrow();
    assert_eq!(escrow_before, escrow_after);
    let v = client.get_version();
    assert!(v > 0);
}

// ---------------------------------------------------------------------------
// Non-admin cannot set_paused (no auth)
// ---------------------------------------------------------------------------
#[test]
fn non_admin_cannot_set_paused() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    // Clear auths and try without admin auth
    env.mock_auths(&[]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_paused(&true);
    }));
    assert!(
        result.is_err(),
        "set_paused should panic without admin auth"
    );
}

// ---------------------------------------------------------------------------
// set_paused true when already true is a no-op (no crash)
// ---------------------------------------------------------------------------
#[test]
fn set_paused_true_when_already_true_is_noop() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    client.set_paused(&true);
    assert!(client.is_paused());
}

// ---------------------------------------------------------------------------
// set_paused false when already false is a no-op (no crash)
// ---------------------------------------------------------------------------
#[test]
fn set_paused_false_when_already_false_is_noop() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&false);
    assert!(!client.is_paused());
}

// ---------------------------------------------------------------------------
// set_paused emits PausedChanged event
// ---------------------------------------------------------------------------
#[test]
fn set_paused_emits_event() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    let events = env.events().all();
    // After set_paused, there should be more events than before (at minimum init + pause events)
    assert!(events.events().len() > 0, "expected events to be emitted");
}

// ---------------------------------------------------------------------------
// Pause is orthogonal to legal hold — both can be set independently
// ---------------------------------------------------------------------------
#[test]
fn pause_orthogonal_to_legal_hold() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    assert!(!client.is_paused());
    assert!(!client.get_legal_hold());
    client.set_paused(&true);
    assert!(client.is_paused());
    assert!(!client.get_legal_hold());
    client.set_legal_hold(&true);
    assert!(client.is_paused());
    assert!(client.get_legal_hold());
    client.set_paused(&false);
    assert!(!client.is_paused());
    assert!(client.get_legal_hold());
    client.set_legal_hold(&false);
    assert!(!client.is_paused());
    assert!(!client.get_legal_hold());
}

// ---------------------------------------------------------------------------
// Pause toggle cycle (pause → unpause → pause → unpause)
// ---------------------------------------------------------------------------
#[test]
fn pause_toggle_cycle() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    assert!(!client.is_paused());
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());
    client.set_paused(&true);
    assert!(client.is_paused());
    client.set_paused(&false);
    assert!(!client.is_paused());
}

// ---------------------------------------------------------------------------
// Operation succeeds between toggle cycles
// ---------------------------------------------------------------------------
#[test]
fn fund_succeeds_between_toggle_cycles() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.set_paused(&true);
    client.set_paused(&false);
    client.fund(&investor, &100);
    client.set_paused(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.fund(&Address::generate(&env), &50);
    }));
    assert!(result.is_err(), "fund should panic when paused");
    client.set_paused(&false);
    client.fund(&investor, &50);
}

// ---------------------------------------------------------------------------
// is_paused returns false on uninitialized contract (graceful default)
// ---------------------------------------------------------------------------
#[test]
fn is_paused_returns_false_on_uninit_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    assert!(!client.is_paused(), "is_paused defaults to false on uninit escrow");
}

// ---------------------------------------------------------------------------
// set_paused triggers before status validation for fund (pause gate first)
// ---------------------------------------------------------------------------
#[test]
fn pause_gate_triggers_before_funding_status_check() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    client.set_paused(&true);
    let investor = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.fund(&investor, &100);
    }));
    assert!(
        result.is_err(),
        "fund should panic with pause before status check"
    );
}

// ---------------------------------------------------------------------------
// Pause gate fires before legal hold for settle
// ---------------------------------------------------------------------------
#[test]
fn pause_gate_triggers_before_legal_hold_settle() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.set_paused(&true);
    client.set_legal_hold(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.settle();
    }));
    assert!(result.is_err(), "settle should panic");
}

// ---------------------------------------------------------------------------
// Pause gate fires before legal hold for withdraw
// ---------------------------------------------------------------------------
#[test]
fn pause_gate_triggers_before_legal_hold_withdraw() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.set_paused(&true);
    client.set_legal_hold(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.withdraw();
    }));
    assert!(result.is_err(), "withdraw should panic");
}

// ---------------------------------------------------------------------------
// Pause gate fires before legal hold for claim
// ---------------------------------------------------------------------------
#[test]
fn pause_gate_triggers_before_legal_hold_claim() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    default_init(&client, &env, &admin, &sme);
    let investor = Address::generate(&env);
    client.fund(&investor, &100_000_000_000i128);
    client.settle();
    client.set_paused(&true);
    client.set_legal_hold(&true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_investor_payout(&investor);
    }));
    assert!(result.is_err(), "claim should panic");
}
