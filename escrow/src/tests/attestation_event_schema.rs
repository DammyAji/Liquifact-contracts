//! Attestation event schema regression tests.
//!
//! Soroban's test event buffer represents the latest contract invocation, so
//! every test captures it immediately after the emitting call. Comparing the
//! complete XDR guards both indexed topics and data payloads against drift.

use super::*;
use soroban_sdk::{symbol_short, testutils::Events as _, BytesN, Event as _};

fn digest(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn initialized(env: &Env) -> LiquifactEscrowClient<'_> {
    let (client, admin, sme) = setup(env);
    default_init(&client, env, &admin, &sme);
    client
}

fn only_event(env: &Env) -> soroban_sdk::xdr::ContractEvent {
    let events = env.events().all();
    assert_eq!(
        events.events().len(),
        1,
        "expected exactly one event from the latest invocation"
    );
    events.events().first().unwrap().clone()
}

#[test]
fn bind_event_has_exact_topic_and_payload() {
    let env = Env::default();
    let client = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    let value = digest(&env, 0xA1);

    client.bind_primary_attestation_hash(&value);
    let actual = only_event(&env);

    assert_eq!(
        actual,
        PrimaryAttestationBound {
            name: symbol_short!("att_bind"),
            invoice_id,
            digest: value,
        }
        .to_xdr(&env, &client.address)
    );
}

#[test]
fn append_event_has_exact_topic_index_and_digest() {
    let env = Env::default();
    let client = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    let value = digest(&env, 0xA2);

    client.append_attestation_digest(&value);
    let actual = only_event(&env);

    assert_eq!(
        actual,
        AttestationDigestAppended {
            name: symbol_short!("att_app"),
            invoice_id,
            index: 0,
            digest: value,
        }
        .to_xdr(&env, &client.address)
    );
}

#[test]
fn batch_append_emits_ordered_event_per_digest() {
    let env = Env::default();
    let client = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    let first = digest(&env, 0xB1);
    let second = digest(&env, 0xB2);
    let third = digest(&env, 0xB3);
    let values = soroban_sdk::vec![&env, first.clone(), second.clone(), third.clone()];

    client.append_attestation_digests(&values);
    let events = env.events().all();
    let actual = events.events();

    assert_eq!(actual.len(), 3, "batch append must emit once per digest");
    for (position, value) in [first, second, third].into_iter().enumerate() {
        assert_eq!(
            actual.get(position).unwrap().clone(),
            AttestationDigestAppended {
                name: symbol_short!("att_app"),
                invoice_id: invoice_id.clone(),
                index: position as u32,
                digest: value,
            }
            .to_xdr(&env, &client.address),
            "batch append event mismatch at position {position}"
        );
    }
}

#[test]
fn revoke_event_has_exact_topic_and_index() {
    let env = Env::default();
    let client = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    client.append_attestation_digest(&digest(&env, 0xC1));

    client.revoke_attestation_digest(&0);
    let actual = only_event(&env);

    assert_eq!(
        actual,
        AttestationDigestRevoked {
            name: symbol_short!("att_rev"),
            invoice_id,
            index: 0,
        }
        .to_xdr(&env, &client.address)
    );
}

#[test]
fn batch_revoke_emits_ordered_event_per_index() {
    let env = Env::default();
    let client = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    for seed in 0xD1..=0xD3 {
        client.append_attestation_digest(&digest(&env, seed));
    }
    let indices = soroban_sdk::vec![&env, 2u32, 0u32, 1u32];

    client.revoke_attestation_digests(&indices);
    let events = env.events().all();
    let actual = events.events();

    assert_eq!(actual.len(), 3, "batch revoke must emit once per index");
    for (position, index) in [2u32, 0, 1].into_iter().enumerate() {
        assert_eq!(
            actual.get(position).unwrap().clone(),
            AttestationDigestRevoked {
                name: symbol_short!("att_rev"),
                invoice_id: invoice_id.clone(),
                index,
            }
            .to_xdr(&env, &client.address),
            "batch revoke event mismatch at position {position}"
        );
    }
}

#[test]
fn unrevoke_event_has_exact_topic_and_index() {
    let env = Env::default();
    let client = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    client.append_attestation_digest(&digest(&env, 0xE1));
    client.revoke_attestation_digest(&0);

    client.unrevoke_attestation_digest(&0);
    let actual = only_event(&env);

    assert_eq!(
        actual,
        AttestationDigestUnrevoked {
            name: symbol_short!("att_unrev"),
            invoice_id,
            index: 0,
        }
        .to_xdr(&env, &client.address)
    );
}

#[test]
fn attestation_event_topics_are_pairwise_distinct() {
    let topics = [
        symbol_short!("att_bind"),
        symbol_short!("att_app"),
        symbol_short!("att_rev"),
        symbol_short!("att_unrev"),
    ];

    for left in 0..topics.len() {
        for right in (left + 1)..topics.len() {
            assert_ne!(
                topics[left], topics[right],
                "attestation topic collision at indexes {left} and {right}"
            );
        }
    }
}
