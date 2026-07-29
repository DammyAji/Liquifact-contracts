//! Protocol fee split conservation properties (issue #663).
//!
//! These tests assert the core accounting identity of the protocol fee leg of
//! `withdraw()`:
//!
//! ```text
//! fee      = funded_amount * protocol_fee_bps / 10_000   (floor)
//! sme_net  = funded_amount - fee
//! fee + sme_net == funded_amount                          (exact, always)
//! ```
//!
//! Coverage is intentionally additive: nothing in the existing suite is
//! modified. The generators sweep the full valid `protocol_fee_bps` range,
//! including both endpoints (`0` and `10_000`), and the assertions cover both
//! the computed legs and the observed token balance deltas of the treasury and
//! the SME after `withdraw()`.

use super::*;
use proptest::prelude::*;

/// Expected floored protocol fee for a given principal and fee rate.
///
/// Mirrors the on-chain formula exactly: integer (truncating) division by
/// `10_000`, so any rounding residue always stays with the SME leg.
fn expected_fee_for(funded_amount: i128, protocol_fee_bps: i64) -> i128 {
    funded_amount * (protocol_fee_bps as i128) / 10_000
}

/// Outcome of one fee-split scenario.
struct FeeSplitOutcome {
    expected_fee: i128,
    expected_sme_net: i128,
    treasury_delta: i128,
    sme_delta: i128,
    fee_record_count: u32,
    first_fee_record_amount: i128,
    first_fee_record_treasury_matches: bool,
    status_after_withdraw: u32,
}

/// Deploy a fresh escrow backed by a standard Stellar asset token, fund it to
/// exactly `funded_amount`, withdraw, and report the observed fee split.
///
/// `yield_bps` is pinned to `0` so the settle pool equals the principal and the
/// only split under test is `fee` versus `sme_net`.
fn run_fee_split(funded_amount: i128, protocol_fee_bps: i64, invoice_id: &str) -> FeeSplitOutcome {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let treasury = Address::generate(&env);
    let investor = Address::generate(&env);

    let token = install_stellar_asset_token(&env);
    let client = deploy(&env);

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, invoice_id),
        &sme,
        &funded_amount,
        &0i64,
        &0u64,
        &token.id,
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
        &Some(protocol_fee_bps),
    );

    // The investor must really hold the principal so `fund` moves tokens in.
    token.stellar.mint(&investor, &funded_amount);
    client.fund(&investor, &funded_amount);

    let expected_fee = expected_fee_for(funded_amount, protocol_fee_bps);
    let expected_sme_net = funded_amount - expected_fee;

    let treasury_before = token.token.balance(&treasury);
    let sme_before = token.token.balance(&sme);

    let after = client.withdraw();

    let treasury_after = token.token.balance(&treasury);
    let sme_after = token.token.balance(&sme);

    let fees = client.get_fees_page(&0, &10);
    let fee_record_count = fees.len();
    let (first_fee_record_amount, first_fee_record_treasury_matches) = match fees.get(0) {
        Some(record) => (record.amount, record.treasury == treasury),
        None => (0i128, true),
    };

    FeeSplitOutcome {
        expected_fee,
        expected_sme_net,
        treasury_delta: treasury_after - treasury_before,
        sme_delta: sme_after - sme_before,
        fee_record_count,
        first_fee_record_amount,
        first_fee_record_treasury_matches,
        status_after_withdraw: after.status,
    }
}

proptest! {
    /// # Fee split conservation (issue #663)
    ///
    /// For every generated `(funded_amount, protocol_fee_bps)` pair:
    ///
    /// 1. `fee + sme_net == funded_amount` exactly.
    /// 2. `fee >= 0` — the fee leg is never negative.
    /// 3. `fee <= funded_amount` — the fee never exceeds the principal.
    /// 4. The treasury balance delta equals the computed fee leg.
    /// 5. The SME balance delta equals the computed net leg.
    /// 6. The two observed deltas sum back to the disbursed principal.
    #[test]
    fn prop_fee_plus_sme_net_equals_disbursed_principal(
        funded_amount in 1i128..=1_000_000_000_000i128,
        protocol_fee_bps in 0i64..=10_000i64,
    ) {
        let outcome = run_fee_split(funded_amount, protocol_fee_bps, "FEESPLIT");

        prop_assert!(
            outcome.expected_fee >= 0,
            "fee must never be negative (fee={})",
            outcome.expected_fee
        );
        prop_assert!(
            outcome.expected_fee <= funded_amount,
            "fee ({}) must never exceed the principal ({})",
            outcome.expected_fee,
            funded_amount
        );
        prop_assert_eq!(
            outcome.expected_fee + outcome.expected_sme_net,
            funded_amount,
            "fee + sme_net must equal the disbursed principal"
        );

        prop_assert_eq!(
            outcome.treasury_delta,
            outcome.expected_fee,
            "treasury balance delta must equal the computed fee leg"
        );
        prop_assert_eq!(
            outcome.sme_delta,
            outcome.expected_sme_net,
            "SME balance delta must equal the computed net leg"
        );
        prop_assert_eq!(
            outcome.treasury_delta + outcome.sme_delta,
            funded_amount,
            "observed legs must sum back to the disbursed principal"
        );

        prop_assert_eq!(outcome.status_after_withdraw, 3u32, "withdraw must set status 3");
    }

    /// # Fee ledger agrees with the computed fee leg (issue #663)
    ///
    /// A non-zero fee must record exactly one `FeeRecord` whose amount equals
    /// the computed fee and whose treasury matches the configured treasury.
    /// A zero fee must record nothing at all.
    #[test]
    fn prop_fee_record_matches_computed_fee_leg(
        funded_amount in 10_000i128..=1_000_000_000_000i128,
        protocol_fee_bps in 0i64..=10_000i64,
    ) {
        let outcome = run_fee_split(funded_amount, protocol_fee_bps, "FEELEDGR");

        if outcome.expected_fee > 0 {
            prop_assert_eq!(outcome.fee_record_count, 1u32, "one fee record expected");
            prop_assert_eq!(
                outcome.first_fee_record_amount,
                outcome.expected_fee,
                "fee record amount must equal the computed fee leg"
            );
            prop_assert!(
                outcome.first_fee_record_treasury_matches,
                "fee record treasury must match the configured treasury"
            );
        } else {
            prop_assert_eq!(
                outcome.fee_record_count,
                0u32,
                "a zero fee must not write a fee record"
            );
        }
    }
}

/// Endpoint: `protocol_fee_bps == 0` sends the entire principal to the SME.
#[test]
fn fee_split_endpoint_zero_bps_gives_sme_everything() {
    let funded_amount = 1_000_000i128;
    let outcome = run_fee_split(funded_amount, 0i64, "FEEBPS00");

    assert_eq!(outcome.expected_fee, 0, "zero bps must produce a zero fee");
    assert_eq!(outcome.expected_sme_net, funded_amount);
    assert_eq!(outcome.treasury_delta, 0, "treasury must receive nothing");
    assert_eq!(
        outcome.sme_delta, funded_amount,
        "SME must receive the whole principal"
    );
    assert_eq!(outcome.treasury_delta + outcome.sme_delta, funded_amount);
    assert_eq!(outcome.fee_record_count, 0, "no fee record at zero bps");
}

/// Endpoint: `protocol_fee_bps == 10_000` sends the entire principal to the
/// treasury and leaves the SME leg at exactly zero.
#[test]
fn fee_split_endpoint_max_bps_gives_treasury_everything() {
    let funded_amount = 1_000_000i128;
    let outcome = run_fee_split(funded_amount, 10_000i64, "FEEBPSMX");

    assert_eq!(
        outcome.expected_fee, funded_amount,
        "10_000 bps must charge the whole principal"
    );
    assert_eq!(outcome.expected_sme_net, 0, "SME net must be exactly zero");
    assert_eq!(outcome.treasury_delta, funded_amount);
    assert_eq!(outcome.sme_delta, 0);
    assert_eq!(outcome.treasury_delta + outcome.sme_delta, funded_amount);
    assert!(
        outcome.expected_fee <= funded_amount,
        "fee must never exceed the principal even at the maximum rate"
    );
}

/// Rounding residue always stays with the SME leg: with a principal that is not
/// divisible by the fee rate the floored fee is strictly smaller than the exact
/// rational share, and conservation still holds exactly.
#[test]
fn fee_split_rounding_residue_stays_with_sme() {
    let funded_amount = 10_001i128;
    let protocol_fee_bps = 3_333i64;
    let outcome = run_fee_split(funded_amount, protocol_fee_bps, "FEEROUND");

    let exact_numerator = funded_amount * (protocol_fee_bps as i128);
    assert!(
        exact_numerator % 10_000 != 0,
        "scenario must exercise a rounding residue"
    );
    assert_eq!(outcome.expected_fee, exact_numerator / 10_000);
    assert_eq!(
        outcome.expected_fee + outcome.expected_sme_net,
        funded_amount,
        "conservation must hold under flooring"
    );
    assert_eq!(outcome.treasury_delta, outcome.expected_fee);
    assert_eq!(outcome.sme_delta, outcome.expected_sme_net);
}

/// Smallest possible principal: a single unit with a mid-range fee rate floors
/// the fee to zero, so the SME keeps the whole unit and conservation holds.
#[test]
fn fee_split_minimum_principal_floors_fee_to_zero() {
    let outcome = run_fee_split(1i128, 5_000i64, "FEEMIN01");

    assert_eq!(outcome.expected_fee, 0, "1 * 5_000 / 10_000 floors to 0");
    assert_eq!(outcome.expected_sme_net, 1);
    assert_eq!(outcome.treasury_delta, 0);
    assert_eq!(outcome.sme_delta, 1);
    assert_eq!(outcome.treasury_delta + outcome.sme_delta, 1);
}
