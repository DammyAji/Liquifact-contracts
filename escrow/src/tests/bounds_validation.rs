use crate::tests::{assert_contract_error, setup};
use crate::{EscrowError, MAX_BUMP_TTL_BATCH};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

#[test]
fn test_bump_ttl_batch_bounds() {
    let env = Env::default();
    let (client, _admin, _sme) = setup(&env);

    // Test empty batch
    let empty_batch: Vec<Address> = Vec::new(&env);
    assert_contract_error(
        client.try_bump_ttl(&empty_batch),
        EscrowError::BumpTtlBatchEmpty,
    );

    // Test exact MAX_BUMP_TTL_BATCH (should succeed)
    let mut max_batch = Vec::new(&env);
    for _ in 0..MAX_BUMP_TTL_BATCH {
        max_batch.push_back(Address::generate(&env));
    }
    client.bump_ttl(&max_batch); // should not panic

    // Test over MAX_BUMP_TTL_BATCH
    let mut over_batch = Vec::new(&env);
    for _ in 0..=MAX_BUMP_TTL_BATCH {
        over_batch.push_back(Address::generate(&env));
    }
    assert_contract_error(
        client.try_bump_ttl(&over_batch),
        EscrowError::BumpTtlBatchTooLarge,
    );
}
