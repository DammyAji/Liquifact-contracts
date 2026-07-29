use crate::tests::{assert_contract_error, setup};
use crate::EscrowError;
use crate::{DEFAULT_SETTLEMENT_LIMIT, MAX_SETTLEMENT_LIMIT, MIN_SETTLEMENT_LIMIT};
use soroban_sdk::{testutils::Address as _, Address, Env, Error, IntoVal};

#[test]
fn default_settlement_limit() {
    let env = Env::default();
    let (client, _admin, _sme) = setup(&env);

    // Check default limit
    let limit = client.get_settlement_limit();
    assert_eq!(limit, DEFAULT_SETTLEMENT_LIMIT);
}

#[test]
fn admin_sets_settlement_limit() {
    let env = Env::default();
    let (client, admin, _sme) = setup(&env);

    client.set_settlement_limit(&50);
    assert_eq!(client.get_settlement_limit(), 50);

    client.set_settlement_limit(&MIN_SETTLEMENT_LIMIT);
    assert_eq!(client.get_settlement_limit(), MIN_SETTLEMENT_LIMIT);

    client.set_settlement_limit(&MAX_SETTLEMENT_LIMIT);
    assert_eq!(client.get_settlement_limit(), MAX_SETTLEMENT_LIMIT);
}

#[test]
fn set_settlement_limit_out_of_range() {
    let env = Env::default();
    let (client, _admin, _sme) = setup(&env);

    assert_contract_error(
        client.try_set_settlement_limit(&(MIN_SETTLEMENT_LIMIT - 1)),
        EscrowError::SettlementLimitOutOfRange,
    );

    assert_contract_error(
        client.try_set_settlement_limit(&(MAX_SETTLEMENT_LIMIT + 1)),
        EscrowError::SettlementLimitOutOfRange,
    );
}

#[test]
fn non_admin_cannot_set_settlement_limit() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_escrow(&env, &client, &admin, &sme);
    let non_admin = Address::generate(&env);

    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_settlement_limit",
            args: SorobanVec::from_array(&env, [50u32.into_val(&env)]),
            sub_invokes: &[],
        },
    }]);

    // Should fail with an auth error or admin required error depending on require_auth/require_admin implementation
    let result = client.try_set_settlement_limit(&50u32);
    assert!(result.is_err());
}

#[test]
fn set_settlement_limit_boundary_zero_and_max() {
    let env = Env::default();
    let (client, admin, sme) = setup(&env);
    init_escrow(&env, &client, &admin, &sme);

    // zero
    assert_contract_error(
        client.try_set_settlement_limit(&0),
        EscrowError::SettlementLimitOutOfRange,
    );

    // max
    assert_contract_error(
        client.try_set_settlement_limit(&u32::MAX),
        EscrowError::SettlementLimitOutOfRange,
    );
}

use proptest::prelude::*;
proptest! {
    #[test]
    fn fuzz_set_settlement_limit(limit in 0u32..=u32::MAX) {
        let env = Env::default();
        let (client, admin, sme) = setup(&env);
        init_escrow(&env, &client, &admin, &sme);

        let result = client.try_set_settlement_limit(&limit);
        if (MIN_SETTLEMENT_LIMIT..=MAX_SETTLEMENT_LIMIT).contains(&limit) {
            assert!(result.is_ok());
            assert_eq!(client.get_settlement_limit(), limit);
        } else {
            assert_contract_error(
                result,
                EscrowError::SettlementLimitOutOfRange,
            );
        }
    }
}
