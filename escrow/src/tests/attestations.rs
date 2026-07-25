//! Attestation tests: `bind_primary_attestation_hash` (single-set) and
//! `append_attestation_digest` (bounded by [`MAX_ATTESTATION_APPEND_ENTRIES`]).
//!
//! These tests prove the two chain-anchor invariants:
//! 1. The primary hash is **write-once** ÔÇö a second bind panics regardless of the digest value.
//! 2. The append log is **capacity-bounded** ÔÇö the 33rd entry panics; the 32nd succeeds.
//!
//! Neither entrypoint stores ZK proofs or performs off-chain verification. They record a
//! 32-byte digest (e.g. SHA-256 of an IPFS CID or a KYC/KYB document bundle) so that
//! off-chain verifiers can confirm the on-chain anchor matches their document set.

use super::*;
use soroban_sdk::{BytesN, Error, InvokeError};
use std::fmt::Debug;

fn assert_contract_error<T, E>(
    result: Result<Result<T, E>, Result<Error, InvokeError>>,
    expected: EscrowError,
) where
    T: Debug,
    E: Debug,
{
    let expected_code = expected as u32;
    match result {
        Err(Ok(error)) => assert_eq!(error, Error::from_contract_error(expected_code)),
        Err(Err(InvokeError::Contract(code))) => assert_eq!(code, expected_code),
        other => panic!("expected ContractError({expected_code}), got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A deterministic 32-byte digest seeded by `seed` for test readability.
fn digest(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Initialize a fresh escrow and return `(client, admin)`.
fn setup_with_init(env: &Env) -> (LiquifactEscrowClient<'_>, Address) {
    let (client, admin, sme) = setup(env);
    default_init(&client, env, &admin, &sme);
    (client, admin)
}

// ---------------------------------------------------------------------------
// bind_primary_attestation_hash ÔÇö single-set invariant
// ---------------------------------------------------------------------------

/// Happy path: first bind succeeds and is readable via the getter.
#[test]
fn test_bind_primary_hash_stores_and_reads() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let d = digest(&env, 0xAB);
    client.bind_primary_attestation_hash(&d);
    assert_eq!(client.get_primary_attestation_hash(), Some(d));
}

/// Before any bind the getter returns `None`.
#[test]
fn test_get_primary_hash_none_before_bind() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    assert_eq!(client.get_primary_attestation_hash(), None);
}

/// A second bind with the **same** digest must panic ÔÇö single-set is unconditional.
#[test]
#[should_panic]
fn test_bind_primary_hash_same_digest_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let d = digest(&env, 0x01);
    client.bind_primary_attestation_hash(&d);
    client.bind_primary_attestation_hash(&d);
}

/// A second bind with a **different** digest must also panic ÔÇö no replacement allowed.
#[test]
#[should_panic]
fn test_bind_primary_hash_different_digest_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.bind_primary_attestation_hash(&digest(&env, 0x01));
    client.bind_primary_attestation_hash(&digest(&env, 0x02));
}

/// Non-admin caller must not be able to bind the primary hash.
#[test]
#[should_panic]
fn test_bind_primary_hash_non_admin_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    // Clear all mocks so auth is enforced for the next call.
    env.mock_auths(&[]);
    client.bind_primary_attestation_hash(&digest(&env, 0xFF));
}

// ---------------------------------------------------------------------------
// append_attestation_digest ÔÇö bounded log invariant
// ---------------------------------------------------------------------------

/// Empty log before any append.
#[test]
fn test_append_log_empty_before_first_append() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    assert_eq!(client.get_attestation_append_log().len(), 0);
}

/// Single append is stored at index 0.
#[test]
fn test_append_single_entry_stored() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let d = digest(&env, 0x10);
    client.append_attestation_digest(&d);
    let log = client.get_attestation_append_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log.get(0).unwrap(), d);
}

/// Multiple appends preserve insertion order.
#[test]
fn test_append_multiple_entries_ordered() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for i in 0u8..5 {
        client.append_attestation_digest(&digest(&env, i));
    }
    let log = client.get_attestation_append_log();
    assert_eq!(log.len(), 5);
    for i in 0u8..5 {
        assert_eq!(log.get(i as u32).unwrap(), digest(&env, i));
    }
}

/// The 32nd entry (index 31) succeeds ÔÇö boundary must be inclusive.
#[test]
fn test_append_exactly_max_entries_succeeds() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    // MAX_ATTESTATION_APPEND_ENTRIES = 32, safely fits in u8.
    for i in 0u8..(MAX_ATTESTATION_APPEND_ENTRIES as u8) {
        client.append_attestation_digest(&digest(&env, i));
    }
    assert_eq!(
        client.get_attestation_append_log().len(),
        MAX_ATTESTATION_APPEND_ENTRIES
    );
}

/// The 33rd entry must panic ÔÇö capacity is strictly bounded.
#[test]
#[should_panic]
fn test_append_beyond_max_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    // Append MAX+1 entries; the last one must panic.
    for i in 0u8..=(MAX_ATTESTATION_APPEND_ENTRIES as u8) {
        client.append_attestation_digest(&digest(&env, i));
    }
}

/// Duplicate digests are allowed ÔÇö the log is an audit trail, not a set.
#[test]
fn test_append_duplicate_digest_allowed() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let d = digest(&env, 0x42);
    client.append_attestation_digest(&d);
    client.append_attestation_digest(&d);
    assert_eq!(client.get_attestation_append_log().len(), 2);
}

/// Non-admin caller must not be able to append.
#[test]
#[should_panic]
fn test_append_non_admin_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    // Clear all mocks so auth is enforced for the next call.
    env.mock_auths(&[]);
    client.append_attestation_digest(&digest(&env, 0x01));
}

// ---------------------------------------------------------------------------
// Interaction: primary hash and append log are independent
// ---------------------------------------------------------------------------

/// Binding the primary hash does not affect the append log.
#[test]
fn test_primary_bind_does_not_affect_append_log() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.bind_primary_attestation_hash(&digest(&env, 0xAA));
    assert_eq!(client.get_attestation_append_log().len(), 0);
}

/// Appending does not affect the primary hash.
#[test]
fn test_append_does_not_affect_primary_hash() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xBB));
    assert_eq!(client.get_primary_attestation_hash(), None);
}

/// Both can coexist: bind primary then fill part of the append log.
#[test]
fn test_primary_and_append_coexist() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let primary = digest(&env, 0xCC);
    client.bind_primary_attestation_hash(&primary);
    for i in 0u8..4 {
        client.append_attestation_digest(&digest(&env, i));
    }
    assert_eq!(client.get_primary_attestation_hash(), Some(primary));
    assert_eq!(client.get_attestation_append_log().len(), 4);
}

// ---------------------------------------------------------------------------
// revoke_attestation_digest ÔÇö revocation tombstone invariant
// ---------------------------------------------------------------------------

/// Happy path: revoke index 0 and confirm via `is_attestation_revoked`.
#[test]
fn test_revoke_single_entry() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xAA));

    assert!(!client.is_attestation_revoked(&0));
    client.revoke_attestation_digest(&0);
    assert!(client.is_attestation_revoked(&0));
}

/// Revoking index 1 (after two appends) leaves index 0 unaffected.
#[test]
fn test_revoke_later_index_does_not_affect_earlier() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x01));
    client.append_attestation_digest(&digest(&env, 0x02));

    client.revoke_attestation_digest(&1);
    assert!(!client.is_attestation_revoked(&0));
    assert!(client.is_attestation_revoked(&1));
}

/// Revoking all entries sequentially succeeds.
#[test]
fn test_revoke_all_entries() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for i in 0u8..5 {
        client.append_attestation_digest(&digest(&env, i));
    }
    for i in 0u8..5 {
        assert!(!client.is_attestation_revoked(&(i as u32)));
        client.revoke_attestation_digest(&(i as u32));
        assert!(client.is_attestation_revoked(&(i as u32)));
    }
}

/// Revoking the same index twice returns `AttestationAlreadyRevoked`.
#[test]
fn test_double_revoke_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x42));
    client.revoke_attestation_digest(&0);
    assert_contract_error(
        client.try_revoke_attestation_digest(&0),
        EscrowError::AttestationAlreadyRevoked,
    );
}

/// Revoking an index beyond the current log length returns `AttestationIndexOutOfRange`.
#[test]
fn test_revoke_out_of_range_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    // Empty log, index 0 is out of range.
    assert_contract_error(
        client.try_revoke_attestation_digest(&0),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// Revoking an index equal to log length returns `AttestationIndexOutOfRange` (0-indexed).
#[test]
fn test_revoke_at_log_len_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x10));
    // log.len() == 1, so index 1 is out of range.
    assert_contract_error(
        client.try_revoke_attestation_digest(&1),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// `is_attestation_revoked` returns `false` for any index on an empty log.
#[test]
fn test_is_revoked_empty_log() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    assert!(!client.is_attestation_revoked(&0));
    assert!(!client.is_attestation_revoked(&99));
}

/// Non-admin caller must not be able to revoke.
#[test]
#[should_panic]
fn test_revoke_non_admin_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xFF));
    env.mock_auths(&[]);
    client.revoke_attestation_digest(&0);
}

/// Revocation does not alter the append log contents ÔÇö the digest remains readable.
#[test]
fn test_revoke_preserves_log_entry() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let d = digest(&env, 0xBB);
    client.append_attestation_digest(&d);
    client.revoke_attestation_digest(&0);
    let log = client.get_attestation_append_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log.get(0).unwrap(), d);
}

/// Revocation does not affect the primary attestation hash.
#[test]
fn test_revoke_does_not_affect_primary_hash() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let primary = digest(&env, 0xCC);
    client.bind_primary_attestation_hash(&primary);
    client.append_attestation_digest(&digest(&env, 0xDD));
    client.revoke_attestation_digest(&0);
    assert_eq!(client.get_primary_attestation_hash(), Some(primary));
}

// ---------------------------------------------------------------------------
// revoke_attestation_digest — typed EscrowError edge cases (issue #378)
// ---------------------------------------------------------------------------

/// index > log.len() (large value) returns `AttestationIndexOutOfRange`.
#[test]
fn test_revoke_large_index_out_of_range() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x01));
    assert_contract_error(
        client.try_revoke_attestation_digest(&99),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// Revoking the first entry (index 0) in a multi-entry log succeeds.
#[test]
fn test_revoke_first_entry() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x01));
    client.append_attestation_digest(&digest(&env, 0x02));
    client.revoke_attestation_digest(&0);
    assert!(client.is_attestation_revoked(&0));
    assert!(!client.is_attestation_revoked(&1));
}

/// Revoking the last entry in a multi-entry log succeeds.
#[test]
fn test_revoke_last_entry() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for i in 0u8..3 {
        client.append_attestation_digest(&digest(&env, i));
    }
    client.revoke_attestation_digest(&2);
    assert!(!client.is_attestation_revoked(&0));
    assert!(!client.is_attestation_revoked(&1));
    assert!(client.is_attestation_revoked(&2));
}

/// Third revoke attempt on same index still returns `AttestationAlreadyRevoked`.
#[test]
fn test_repeated_revoke_returns_typed_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x10));
    client.revoke_attestation_digest(&0);
    assert_contract_error(
        client.try_revoke_attestation_digest(&0),
        EscrowError::AttestationAlreadyRevoked,
    );
    // A second retry also returns the same typed error.
    assert_contract_error(
        client.try_revoke_attestation_digest(&0),
        EscrowError::AttestationAlreadyRevoked,
    );
}

/// Non-admin `try_revoke_attestation_digest` returns an authorization error.
#[test]
fn test_revoke_non_admin_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xFF));
    env.mock_auths(&[]);
    // Any error (not Ok) satisfies the auth-rejection requirement.
    assert!(client.try_revoke_attestation_digest(&0).is_err());
}

// ---------------------------------------------------------------------------
// unrevoke_attestation_digest — clears a revocation marker
// ---------------------------------------------------------------------------

/// Happy path: revoke then unrevoke clears the marker.
#[test]
fn test_unrevoke_clears_revoked_marker() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xAA));
    client.revoke_attestation_digest(&0);
    assert!(client.is_attestation_revoked(&0));

    client.unrevoke_attestation_digest(&0);
    assert!(!client.is_attestation_revoked(&0));
}

/// Unrevoking an index beyond the log length returns `AttestationIndexOutOfRange`.
#[test]
fn test_unrevoke_out_of_range_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&0),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// Unrevoking an index that was never revoked returns `AttestationNotRevoked`.
#[test]
fn test_unrevoke_not_revoked_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xBB));
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&0),
        EscrowError::AttestationNotRevoked,
    );
}

/// A second unrevoke on an already-cleared index returns `AttestationNotRevoked`.
#[test]
fn test_unrevoke_twice_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xCC));
    client.revoke_attestation_digest(&0);
    client.unrevoke_attestation_digest(&0);
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&0),
        EscrowError::AttestationNotRevoked,
    );
}

/// Non-admin `try_unrevoke_attestation_digest` returns an authorization error.
#[test]
fn test_unrevoke_non_admin_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xDD));
    client.revoke_attestation_digest(&0);
    env.mock_auths(&[]);
    assert!(client.try_unrevoke_attestation_digest(&0).is_err());
}

// ---------------------------------------------------------------------------
// Dedicated events — issue #702
//
// Each attestation state change (bind, append, revoke, unrevoke) must emit its
// own event, captured here immediately after the mutating call (before any
// further contract call clears the host event buffer). Payloads are compared
// as full XDR to catch any field/shape drift, not just topic presence.
// ---------------------------------------------------------------------------

/// `bind_primary_attestation_hash` emits `PrimaryAttestationBound` under the
/// `att_bind` topic, carrying the escrow's invoice id and the bound digest.
#[test]
fn test_bind_primary_hash_emits_dedicated_event() {
    use soroban_sdk::{symbol_short, testutils::Events as _, Event as _};

    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;
    let d = digest(&env, 0xAB);

    client.bind_primary_attestation_hash(&d);

    assert_eq!(
        env.events().all().events(),
        std::vec![PrimaryAttestationBound {
            name: symbol_short!("att_bind"),
            invoice_id,
            digest: d,
        }
        .to_xdr(&env, &contract_id)]
    );
}

/// `append_attestation_digest` emits `AttestationDigestAppended` under the
/// `att_app` topic, carrying the invoice id, the new entry's index, and digest.
#[test]
fn test_append_digest_emits_dedicated_event() {
    use soroban_sdk::{symbol_short, testutils::Events as _, Event as _};

    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;
    let d = digest(&env, 0x11);

    client.append_attestation_digest(&d);

    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestAppended {
            name: symbol_short!("att_app"),
            invoice_id,
            index: 0,
            digest: d,
        }
        .to_xdr(&env, &contract_id)]
    );
}

/// A second `append_attestation_digest` call carries the correct incremented index.
#[test]
fn test_append_digest_second_entry_emits_index_one() {
    use soroban_sdk::{symbol_short, testutils::Events as _, Event as _};

    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;
    client.append_attestation_digest(&digest(&env, 0x01));

    let d = digest(&env, 0x02);
    client.append_attestation_digest(&d);

    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestAppended {
            name: symbol_short!("att_app"),
            invoice_id,
            index: 1,
            digest: d,
        }
        .to_xdr(&env, &contract_id)]
    );
}

/// `revoke_attestation_digest` emits `AttestationDigestRevoked` under the
/// `att_rev` topic, carrying the invoice id and the revoked index.
#[test]
fn test_revoke_digest_emits_dedicated_event() {
    use soroban_sdk::{symbol_short, testutils::Events as _, Event as _};

    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;
    client.append_attestation_digest(&digest(&env, 0xEE));

    client.revoke_attestation_digest(&0);

    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id,
            index: 0,
        }
        .to_xdr(&env, &contract_id)]
    );
}

/// `unrevoke_attestation_digest` emits `AttestationDigestUnrevoked` under the
/// `att_unrev` topic, carrying the invoice id and the cleared index.
#[test]
fn test_unrevoke_digest_emits_dedicated_event() {
    use soroban_sdk::{symbol_short, testutils::Events as _, Event as _};

    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;
    client.append_attestation_digest(&digest(&env, 0xFF));
    client.revoke_attestation_digest(&0);

    client.unrevoke_attestation_digest(&0);

    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestUnrevoked {
            name: symbol_short!("att_unrev"),
            invoice_id,
            index: 0,
        }
        .to_xdr(&env, &contract_id)]
    );
}

/// Every attestation event topic is distinct from the others (and from the
/// fixed-width `symbol_short!` limit of 9 characters), so indexers filtering
/// on `topic[1]` can never confuse one attestation state change for another.
#[test]
fn test_attestation_event_topics_do_not_collide() {
    use soroban_sdk::symbol_short;

    let topics = [
        symbol_short!("att_bind"),
        symbol_short!("att_app"),
        symbol_short!("att_rev"),
        symbol_short!("att_unrev"),
    ];

    for t in &topics {
        // symbol_short! already enforces <= 9 chars at compile time; this is a
        // belt-and-suspenders runtime check that survives refactors.
        assert!(t.to_string().len() <= 9, "topic {t:?} exceeds 9 chars");
    }

    for i in 0..topics.len() {
        for j in (i + 1)..topics.len() {
            assert_ne!(
                topics[i], topics[j],
                "attestation event topics must be pairwise distinct"
            );
        }
    }
}

/// End-to-end: bind, append, revoke, unrevoke in one escrow instance each emit
/// their own event with no topic collisions, captured immediately after each
/// mutating call.
#[test]
fn test_full_attestation_lifecycle_emits_four_distinct_events() {
    use soroban_sdk::{symbol_short, testutils::Events as _, Event as _};

    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let contract_id = client.address.clone();
    let invoice_id = client.get_escrow().invoice_id;

    let primary = digest(&env, 0x01);
    client.bind_primary_attestation_hash(&primary);
    assert_eq!(
        env.events().all().events(),
        std::vec![PrimaryAttestationBound {
            name: symbol_short!("att_bind"),
            invoice_id: invoice_id.clone(),
            digest: primary,
        }
        .to_xdr(&env, &contract_id)]
    );

    let appended = digest(&env, 0x02);
    client.append_attestation_digest(&appended);
    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestAppended {
            name: symbol_short!("att_app"),
            invoice_id: invoice_id.clone(),
            index: 0,
            digest: appended,
        }
        .to_xdr(&env, &contract_id)]
    );

    client.revoke_attestation_digest(&0);
    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id: invoice_id.clone(),
            index: 0,
        }
        .to_xdr(&env, &contract_id)]
    );

    client.unrevoke_attestation_digest(&0);
    assert_eq!(
        env.events().all().events(),
        std::vec![AttestationDigestUnrevoked {
            name: symbol_short!("att_unrev"),
            invoice_id,
            index: 0,
        }
        .to_xdr(&env, &contract_id)]
    );
}
