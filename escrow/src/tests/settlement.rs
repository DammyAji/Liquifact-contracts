#![cfg(test)]

use crate::{
    errors::EscrowError,
    types::{EscrowStatus, InvoiceEscrow},
    LiquifactEscrow, LiquifactEscrowClient,
};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup_test() -> (Env, LiquifactEscrowClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, LiquifactEscrow);
    let client = LiquifactEscrowClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sme = Address::generate(&env);
    let funding_token = Address::generate(&env);

    (env, client, admin, sme, funding_token)
}

#[test]
fn test_settle_bounds_and_maturity() {
    let (env, client, admin, sme, funding_token) = setup_test();

    let invoice_id = 1001;
    let target_amount = 1_000_000;
    let maturity = 10_000;

    client.init(&admin, &sme, &funding_token, &invoice_id, &target_amount, &maturity);

    // Manually mark escrow as funded for unit testing
    env.as_contract(&client.address, || {
        let mut escrow = crate::storage::get_escrow(&env).unwrap();
        escrow.funded_amount = 1_000_000;
        escrow.status = EscrowStatus::Funded;
        crate::storage::set_escrow(&env, &escrow);
    });

    // Case 1: Attempt settlement prior to maturity timestamp
    env.ledger().set_timestamp(9_999);
    let res = client.try_settle(&100_000);
    assert_eq!(res, Err(Ok(EscrowError::MaturityNotReached)));

    // Fast-forward timestamp past maturity
    env.ledger().set_timestamp(10_000);

    // Case 2: Reject zero or negative settlement amounts
    let res_zero = client.try_settle(&0);
    assert_eq!(res_zero, Err(Ok(EscrowError::SettlementAmountInvalid)));

    let res_neg = client.try_settle(&-100);
    assert_eq!(res_neg, Err(Ok(EscrowError::SettlementAmountInvalid)));

    // Case 3: Reject amounts exceeding remaining unsettled principal
    let res_over = client.try_settle(&1_000_001);
    assert_eq!(res_over, Err(Ok(EscrowError::SettlementAmountInvalid)));

    // Case 4: Process valid partial settlement via settle
    client.settle(&400_000);
    let escrow = client.get_escrow().unwrap();
    assert_eq!(escrow.settled_amount, 400_000);
    assert_eq!(escrow.status, EscrowStatus::Funded);

    // Case 5: Settle remaining balance to conclude escrow
    client.settle(&600_000);
    let escrow_final = client.get_escrow().unwrap();
    assert_eq!(escrow_final.settled_amount, 1_000_000);
    assert_eq!(escrow_final.status, EscrowStatus::Settled);
}

#[test]
fn test_partial_settle_bounds() {
    let (env, client, admin, sme, funding_token) = setup_test();

    client.init(&admin, &sme, &funding_token, &1002, &500_000, &10_000);

    env.as_contract(&client.address, || {
        let mut escrow = crate::storage::get_escrow(&env).unwrap();
        escrow.funded_amount = 500_000;
        escrow.status = EscrowStatus::Funded;
        crate::storage::set_escrow(&env, &escrow);
    });

    // Case 1: Zero / negative amount rejected
    assert_eq!(
        client.try_partial_settle(&0),
        Err(Ok(EscrowError::SettlementAmountInvalid))
    );
    assert_eq!(
        client.try_partial_settle(&-50),
        Err(Ok(EscrowError::SettlementAmountInvalid))
    );

    // Case 2: Exceeding amount rejected
    assert_eq!(
        client.try_partial_settle(&500_001),
        Err(Ok(EscrowError::SettlementAmountInvalid))
    );

    // Case 3: Valid partial settlement accepted
    client.partial_settle(&200_000);
    assert_eq!(client.get_escrow().unwrap().settled_amount, 200_000);
}

#[test]
fn test_withdraw_bounds() {
    let (env, client, admin, sme, funding_token) = setup_test();

    client.init(&admin, &sme, &funding_token, &1003, &300_000, &10_000);

    env.as_contract(&client.address, || {
        let mut escrow = crate::storage::get_escrow(&env).unwrap();
        escrow.funded_amount = 300_000;
        escrow.status = EscrowStatus::Funded;
        crate::storage::set_escrow(&env, &escrow);
    });

    // Case 1: Zero or negative withdraw amount
    assert_eq!(
        client.try_withdraw(&0),
        Err(Ok(EscrowError::WithdrawAmountInvalid))
    );
    assert_eq!(
        client.try_withdraw(&-10),
        Err(Ok(EscrowError::WithdrawAmountInvalid))
    );

    // Case 2: Over-limit withdraw amount
    assert_eq!(
        client.try_withdraw(&300_001),
        Err(Ok(EscrowError::WithdrawAmountInvalid))
    );

    // Case 3: Valid withdrawal within bounds
    client.withdraw(&150_000);
    assert_eq!(client.get_escrow().unwrap().withdrawn_amount, 150_000);

    // Case 4: Subsequent withdrawal up to maximum limit
    client.withdraw(&150_000);
    assert_eq!(client.get_escrow().unwrap().withdrawn_amount, 300_000);

    // Case 5: Attempting withdrawal after limit reached
    assert_eq!(
        client.try_withdraw(&1),
        Err(Ok(EscrowError::WithdrawAmountInvalid))
    );
}