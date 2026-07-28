//! Admin-configurable attestation limit tests.

use super::{assert_contract_error, default_init, setup};
use crate::{
    AttestationParameters, AttestationParametersUpdated, EscrowError, LiquifactEscrowClient,
    MAX_ATTESTATION_APPEND_BATCH, MAX_ATTESTATION_APPEND_ENTRIES, MAX_ATTESTATION_READ_PAGE,
    MAX_ATTESTATION_REVOKE_BATCH,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, Event as _, IntoVal, Vec as SorobanVec,
};

fn initialized(env: &Env) -> (LiquifactEscrowClient<'_>, Address) {
    let (client, admin, sme) = setup(env);
    default_init(&client, env, &admin, &sme);
    (client, admin)
}

fn defaults() -> AttestationParameters {
    AttestationParameters {
        max_append_entries: MAX_ATTESTATION_APPEND_ENTRIES,
        max_append_batch: MAX_ATTESTATION_APPEND_BATCH,
        max_revoke_batch: MAX_ATTESTATION_REVOKE_BATCH,
        max_read_page: MAX_ATTESTATION_READ_PAGE,
    }
}

fn configured() -> AttestationParameters {
    AttestationParameters {
        max_append_entries: 8,
        max_append_batch: 4,
        max_revoke_batch: 3,
        max_read_page: 2,
    }
}

fn digest(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

#[test]
fn defaults_are_backward_compatible_and_admin_can_update_them() {
    let env = Env::default();
    let (client, _) = initialized(&env);

    assert_eq!(client.get_attestation_parameters(), defaults());

    let new_parameters = configured();
    client.set_attestation_parameters(&new_parameters);

    assert_eq!(client.get_attestation_parameters(), new_parameters);
}

#[test]
fn update_emits_exact_old_and_new_parameters() {
    let env = Env::default();
    let (client, _) = initialized(&env);
    let invoice_id = client.get_escrow().invoice_id;
    let new_parameters = configured();

    client.set_attestation_parameters(&new_parameters);
    let events = env.events().all();

    assert_eq!(events.events().len(), 1);
    assert_eq!(
        events.events().first().unwrap().clone(),
        AttestationParametersUpdated {
            name: symbol_short!("att_cfg"),
            invoice_id,
            old_parameters: defaults(),
            new_parameters,
        }
        .to_xdr(&env, &client.address)
    );
}

#[test]
fn zero_relational_and_hard_ceiling_violations_are_rejected_atomically() {
    let env = Env::default();
    let (client, _) = initialized(&env);
    let invalid = [
        AttestationParameters {
            max_append_entries: 0,
            ..defaults()
        },
        AttestationParameters {
            max_append_entries: MAX_ATTESTATION_APPEND_ENTRIES + 1,
            ..defaults()
        },
        AttestationParameters {
            max_append_batch: 0,
            ..defaults()
        },
        AttestationParameters {
            max_append_batch: MAX_ATTESTATION_APPEND_BATCH + 1,
            ..defaults()
        },
        AttestationParameters {
            max_append_entries: 4,
            max_append_batch: 5,
            ..defaults()
        },
        AttestationParameters {
            max_revoke_batch: 0,
            ..defaults()
        },
        AttestationParameters {
            max_revoke_batch: MAX_ATTESTATION_REVOKE_BATCH + 1,
            ..defaults()
        },
        AttestationParameters {
            max_read_page: 0,
            ..defaults()
        },
        AttestationParameters {
            max_read_page: MAX_ATTESTATION_READ_PAGE + 1,
            ..defaults()
        },
    ];

    for parameters in invalid {
        assert_contract_error(
            client.try_set_attestation_parameters(&parameters),
            EscrowError::AttestationParametersOutOfRange,
        );
        assert_eq!(client.get_attestation_parameters(), defaults());
    }
}

#[test]
fn append_capacity_cannot_be_lowered_below_live_usage() {
    let env = Env::default();
    let (client, _) = initialized(&env);
    client.append_attestation_digest(&digest(&env, 1));
    client.append_attestation_digest(&digest(&env, 2));

    let below_usage = AttestationParameters {
        max_append_entries: 1,
        max_append_batch: 1,
        ..defaults()
    };
    assert_contract_error(
        client.try_set_attestation_parameters(&below_usage),
        EscrowError::AttestationParametersOutOfRange,
    );

    let at_usage = AttestationParameters {
        max_append_entries: 2,
        max_append_batch: 2,
        ..defaults()
    };
    client.set_attestation_parameters(&at_usage);
    assert_eq!(client.get_attestation_parameters(), at_usage);
}

#[test]
fn non_admin_cannot_change_parameters() {
    let env = Env::default();
    let (client, _) = initialized(&env);
    let non_admin = Address::generate(&env);
    let proposed = configured();

    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_attestation_parameters",
            args: SorobanVec::from_array(&env, [proposed.clone().into_val(&env)]),
            sub_invokes: &[],
        },
    }]);

    assert!(client.try_set_attestation_parameters(&proposed).is_err());
    assert_eq!(client.get_attestation_parameters(), defaults());
}

#[test]
fn configured_append_limits_are_enforced() {
    let env = Env::default();
    let (client, _) = initialized(&env);
    let parameters = AttestationParameters {
        max_append_entries: 2,
        max_append_batch: 1,
        max_revoke_batch: 1,
        max_read_page: 1,
    };
    client.set_attestation_parameters(&parameters);

    let oversized_batch = soroban_sdk::vec![&env, digest(&env, 1), digest(&env, 2)];
    assert_contract_error(
        client.try_append_attestation_digests(&oversized_batch),
        EscrowError::AttestationAppendBatchTooLarge,
    );

    client.append_attestation_digest(&digest(&env, 1));
    client.append_attestation_digest(&digest(&env, 2));
    assert_contract_error(
        client.try_append_attestation_digest(&digest(&env, 3)),
        EscrowError::AttestationAppendLogCapacityReached,
    );
}

#[test]
fn configured_revoke_batch_and_read_page_limits_are_enforced() {
    let env = Env::default();
    let (client, _) = initialized(&env);
    let parameters = AttestationParameters {
        max_append_entries: 4,
        max_append_batch: 4,
        max_revoke_batch: 1,
        max_read_page: 2,
    };
    client.set_attestation_parameters(&parameters);
    let digests = soroban_sdk::vec![&env, digest(&env, 1), digest(&env, 2), digest(&env, 3)];
    client.append_attestation_digests(&digests);

    let oversized_revoke = soroban_sdk::vec![&env, 0u32, 1u32];
    assert_contract_error(
        client.try_revoke_attestation_digests(&oversized_revoke),
        EscrowError::AttestationBatchTooLarge,
    );

    client.revoke_attestation_digest(&0);
    client.revoke_attestation_digest(&1);
    client.revoke_attestation_digest(&2);
    assert_eq!(client.get_revoked_attestation_digests(&0, &99).len(), 2);
}
