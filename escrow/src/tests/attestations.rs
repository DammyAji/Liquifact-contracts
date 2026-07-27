//! Attestation tests: `bind_primary_attestation_hash` (single-set) and
//! `append_attestation_digest` (bounded by [`MAX_ATTESTATION_APPEND_ENTRIES`]).
//!
//! These tests prove the two chain-anchor invariants:
//! 1. The primary hash is **write-once** — a second bind panics regardless of the digest value.
//! 2. The append log is **capacity-bounded** — the 33rd entry panics; the 32nd succeeds.
//!
//! Neither entrypoint stores ZK proofs or performs off-chain verification. They record a
//! 32-byte digest (e.g. SHA-256 of an IPFS CID or a KYC/KYB document bundle) so that
//! off-chain verifiers can confirm the on-chain anchor matches their document set.

use super::*;
use soroban_sdk::{symbol_short, testutils::Events, BytesN, Error, InvokeError};
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

fn setup_with_append(env: &Env) -> (LiquifactEscrowClient<'_>, Address) {
    let (client, admin) = setup_with_init(env);
    client.append_attestation_digest(&digest(env, 0xAA));
    (client, admin)
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
// bind_primary_attestation_hash — single-set invariant
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

/// A second bind with the **same** digest must panic — single-set is unconditional.
#[test]
#[should_panic]
fn test_bind_primary_hash_same_digest_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let d = digest(&env, 0x01);
    client.bind_primary_attestation_hash(&d);
    client.bind_primary_attestation_hash(&d);
}

/// A second bind with a **different** digest must also panic — no replacement allowed.
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
// append_attestation_digest — bounded log invariant
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

/// The 32nd entry (index 31) succeeds — boundary must be inclusive.
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

/// The 33rd entry must panic — capacity is strictly bounded.
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

/// Duplicate digests are allowed — the log is an audit trail, not a set.
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
// revoke_attestation_digest — revocation tombstone invariant
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
fn test_double_revoke_typed_error() {
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
fn test_revoke_out_of_range_typed_error() {
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
fn test_revoke_at_log_len_typed_error() {
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

/// Revocation does not alter the append log contents — the digest remains readable.
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
// Event emission tests for AttestationDigestRevoked
// ---------------------------------------------------------------------------

/// AttestationDigestRevoked event is emitted on successful revocation.
#[test]
fn test_revoke_emits_event() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xAA));

    client.revoke_attestation_digest(&0);

    let events = env.events().all();
    assert_eq!(events.events().len(), 1, "expected exactly one event");
    let event = events.events().first().unwrap();
    assert_eq!(
        *event,
        crate::AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id: client.get_escrow().invoice_id,
            index: 0,
        }
        .to_xdr(&env, &client.address)
    );
}

/// AttestationDigestRevoked event contains correct invoice_id and index.
#[test]
fn test_revoke_event_fields_correct() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x01));
    client.append_attestation_digest(&digest(&env, 0x02));

    client.revoke_attestation_digest(&1);

    let events = env.events().all();
    let event = events.events().first().unwrap();
    assert_eq!(
        *event,
        crate::AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id: client.get_escrow().invoice_id,
            index: 1,
        }
        .to_xdr(&env, &client.address)
    );
}

/// Multiple revocations emit multiple events with correct indices.
#[test]
fn test_multiple_revocations_emit_events() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for i in 0u8..3 {
        client.append_attestation_digest(&digest(&env, i));
    }

    client.revoke_attestation_digest(&0);
    let events_after_first = env.events().all();
    assert_eq!(events_after_first.events().len(), 1);

    client.revoke_attestation_digest(&2);
    let events_after_second = env.events().all();
    assert_eq!(events_after_second.events().len(), 1);

    let event = events_after_second.events().first().unwrap();
    assert_eq!(
        *event,
        crate::AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id: client.get_escrow().invoice_id,
            index: 2,
        }
        .to_xdr(&env, &client.address)
    );
}

/// Event is not emitted when revocation fails (out of range).
#[test]
fn test_revoke_out_of_range_no_event_emitted() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    // Empty log, index 0 is out of range - should return error before event emission
    assert!(client.try_revoke_attestation_digest(&0).is_err());
    assert_eq!(env.events().all().events().len(), 0);
}

// ---------------------------------------------------------------------------
// unrevoke_attestation_digest — reversal of revocation
// ---------------------------------------------------------------------------

/// Happy path: revoke then unrevoke, confirm state flips.
#[test]
fn test_unrevoke_single_entry() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    client.revoke_attestation_digest(&0);
    assert!(client.is_attestation_revoked(&0));

    client.unrevoke_attestation_digest(&0);
    assert!(!client.is_attestation_revoked(&0));
}

/// Unrevoke emits `att_unrev` with correct fields.
#[test]
fn test_unrevoke_emits_event() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    let contract_id = client.address.clone();
    client.revoke_attestation_digest(&0);

    client.unrevoke_attestation_digest(&0);

    let events = env.events().all();
    let invoice_id = client.get_escrow().invoice_id;
    assert_eq!(
        events.events().last().unwrap().clone(),
        AttestationDigestUnrevoked {
            name: symbol_short!("att_unrev"),
            invoice_id,
            index: 0,
        }
        .to_xdr(&env, &contract_id)
    );
}

/// Unrevoke on empty log returns `AttestationIndexOutOfRange`.
#[test]
fn test_unrevoke_out_of_range_empty_log() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&0),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// Unrevoke at log.len() returns `AttestationIndexOutOfRange`.
#[test]
fn test_unrevoke_at_log_len() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&1),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// Unrevoke a large out-of-range index returns `AttestationIndexOutOfRange`.
#[test]
fn test_unrevoke_large_index_out_of_range() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&99),
        EscrowError::AttestationIndexOutOfRange,
    );
}

/// Unrevoke an index that was never revoked returns `AttestationNotRevoked`.
#[test]
fn test_unrevoke_not_revoked() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    assert_contract_error(
        client.try_unrevoke_attestation_digest(&0),
        EscrowError::AttestationNotRevoked,
    );
}

/// Digest preserved through revoke → unrevoke.
#[test]
fn test_unrevoke_preserves_digest() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    let d = digest(&env, 0xAA);
    // Recreate: log has 0xAA at index 0
    let log = client.get_attestation_append_log();
    assert_eq!(log.get(0).unwrap(), d);

    client.revoke_attestation_digest(&0);
    client.unrevoke_attestation_digest(&0);

    let log = client.get_attestation_append_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log.get(0).unwrap(), d);
}

/// Revoke → unrevoke → revoke round-trip succeeds.
#[test]
fn test_revoke_unrevoke_cycle() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);

    client.revoke_attestation_digest(&0);
    assert!(client.is_attestation_revoked(&0));

    client.unrevoke_attestation_digest(&0);
    assert!(!client.is_attestation_revoked(&0));

    client.revoke_attestation_digest(&0);
    assert!(client.is_attestation_revoked(&0));
}

/// Unrevoke non-admin returns error.
#[test]
fn test_unrevoke_non_admin_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_append(&env);
    client.revoke_attestation_digest(&0);
    env.mock_auths(&[]);
    assert!(client.try_unrevoke_attestation_digest(&0).is_err());
}

// ---------------------------------------------------------------------------
// revoke_attestation_digests — batch revocation
// ---------------------------------------------------------------------------

/// Happy path: batch revoke multiple indices atomically.
#[test]
fn test_batch_revoke_happy_path() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for _ in 0..3 {
        client.append_attestation_digest(&digest(&env, 0xFF));
    }
    let indices = soroban_sdk::vec![&env, 0u32, 2u32];
    client.revoke_attestation_digests(&indices);

    assert!(client.is_attestation_revoked(&0));
    assert!(!client.is_attestation_revoked(&1));
    assert!(client.is_attestation_revoked(&2));
}

/// Batch revoke emits one `att_rev` event per revoked index.
#[test]
fn test_batch_revoke_emits_events() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for _ in 0..3 {
        client.append_attestation_digest(&digest(&env, 0xFF));
    }
    let contract_id = client.address.clone();

    client.revoke_attestation_digests(&soroban_sdk::vec![&env, 0u32, 2u32]);

    let all_events = env.events().all();
    let events = all_events.events();
    assert_eq!(events.len(), 2, "expected 2 events for 2 revoked indices");

    let invoice_id = client.get_escrow().invoice_id;
    assert_eq!(
        events.first().unwrap().clone(),
        AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id: invoice_id.clone(),
            index: 0,
        }
        .to_xdr(&env, &contract_id)
    );
    assert_eq!(
        events.get(1).unwrap().clone(),
        AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id,
            index: 2,
        }
        .to_xdr(&env, &contract_id)
    );
}

/// Empty batch returns `AttestationBatchEmpty`.
#[test]
fn test_batch_revoke_empty_panics() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    assert_contract_error(
        client.try_revoke_attestation_digests(&soroban_sdk::vec![&env]),
        EscrowError::AttestationBatchEmpty,
    );
}

/// Batch exceeding `MAX_ATTESTATION_REVOKE_BATCH` returns `AttestationBatchTooLarge`.
#[test]
fn test_batch_revoke_oversized() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    let mut indices = SorobanVec::new(&env);
    for i in 0..=MAX_ATTESTATION_REVOKE_BATCH {
        indices.push_back(i);
    }
    assert_contract_error(
        client.try_revoke_attestation_digests(&indices),
        EscrowError::AttestationBatchTooLarge,
    );
}

/// Batch revoke at max capacity succeeds.
#[test]
fn test_batch_revoke_max_size_succeeds() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    for _ in 0..MAX_ATTESTATION_REVOKE_BATCH {
        client.append_attestation_digest(&digest(&env, 0xFF));
    }
    let mut indices = SorobanVec::new(&env);
    for i in 0..MAX_ATTESTATION_REVOKE_BATCH {
        indices.push_back(i);
    }
    client.revoke_attestation_digests(&indices);
    for i in 0..MAX_ATTESTATION_REVOKE_BATCH {
        assert!(client.is_attestation_revoked(&i));
    }
}

/// Out-of-range index in batch returns `AttestationIndexOutOfRange`.
#[test]
fn test_batch_revoke_out_of_range_rollback() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x01));
    // index 1 is out of range
    let indices = soroban_sdk::vec![&env, 0u32, 1u32];
    assert_contract_error(
        client.try_revoke_attestation_digests(&indices),
        EscrowError::AttestationIndexOutOfRange,
    );
    // Index 0 must NOT be revoked (atomic rollback)
    assert!(!client.is_attestation_revoked(&0));
}

/// Already-revoked index in batch returns `AttestationAlreadyRevoked` and rolls back.
#[test]
fn test_batch_revoke_already_revoked_rollback() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0x01));
    client.append_attestation_digest(&digest(&env, 0x02));
    client.revoke_attestation_digest(&1);
    let indices = soroban_sdk::vec![&env, 0u32, 1u32];
    assert_contract_error(
        client.try_revoke_attestation_digests(&indices),
        EscrowError::AttestationAlreadyRevoked,
    );
    // Index 0 must NOT be revoked (atomic rollback)
    assert!(!client.is_attestation_revoked(&0));
    // Index 1 remains revoked from the first call
    assert!(client.is_attestation_revoked(&1));
}

/// Batch revoke non-admin returns error.
#[test]
fn test_batch_revoke_non_admin_returns_error() {
    let env = Env::default();
    let (client, _) = setup_with_init(&env);
    client.append_attestation_digest(&digest(&env, 0xFF));
    let indices = soroban_sdk::vec![&env, 0u32];
    env.mock_auths(&[]);
    assert!(client.try_revoke_attestation_digests(&indices).is_err());
}
