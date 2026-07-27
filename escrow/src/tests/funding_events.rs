use super::*;

use soroban_sdk::testutils::Events as _;

// ---------------------------------------------------------------------------
// Funding event topic and payload coverage (issue #871)
//
// Each test captures events immediately after the emitting call and asserts
// the topic symbols and payload field values.  The Soroban test host retains
// only the most recent invocation's event buffer, so we snapshot right after
// every emitting entrypoint.
// ---------------------------------------------------------------------------

fn funded_init(
    env: &Env,
    client: &LiquifactEscrowClient<'_>,
    admin: &Address,
    sme: &Address,
    invoice: &str,
) -> (Address, Symbol) {
    let token = Address::generate(env);
    let treasury = Address::generate(env);
    client.init(
        admin,
        &soroban_sdk::String::from_str(env, invoice),
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
    let contract_id = client.address.clone();
    (token, Symbol::new(env, invoice))
}

// --- EscrowInitialized ----------------------------------------------------

#[test]
fn test_escrow_initialized_event_topics_and_payload() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);

    let token = Address::generate(&env);
    let treasury = Address::generate(&env);
    let contract_id = client.address.clone();

    client.init(
        &admin,
        &soroban_sdk::String::from_str(&env, "INVEvt"),
        &sme,
        &TARGET,
        &800i64,
        &1000u64,
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

    let events = env.events().all();
    let event = events.events().last().unwrap().clone();

    let expected = EscrowInitialized {
        name: symbol_short!("escrow_ii"),
        escrow: client.get_escrow(),
        funding_token: token,
        treasury,
        registry: None,
        has_maturity_lock: true,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event, expected);
}

// --- EscrowFunded ---------------------------------------------------------

#[test]
fn test_escrow_funded_event_topics_and_payload() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (token, invoice_id) = funded_init(&env, &client, &admin, &sme, "INVFnd");

    let investor = Address::generate(&env);
    client.fund(&investor, &TARGET);

    let events = env.events().all();
    let event = events.events().last().unwrap().clone();

    let expected = EscrowFunded {
        name: symbol_short!("funded"),
        invoice_id,
        investor,
        amount: TARGET,
        funded_amount: TARGET,
        status: 1,
        investor_effective_yield_bps: 800,
        tier_lock_secs: 0,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event, expected);
}

// --- EscrowUnfunded -------------------------------------------------------

#[test]
fn test_escrow_unfunded_event_topics_and_payload() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (_token, invoice_id) = funded_init(&env, &client, &admin, &sme, "INVUfn");

    let investor = Address::generate(&env);
    let half = TARGET / 2;
    client.fund(&investor, &half);

    let unfunded = client.unfund(&investor, &half);

    let events = env.events().all();
    let event = events.events().last().unwrap().clone();

    let expected = EscrowUnfunded {
        name: symbol_short!("unfund"),
        invoice_id,
        investor,
        amount: half,
        funded_amount: unfunded.funded_amount,
        status: unfunded.status,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event, expected);
}

// --- FundingTargetUpdated -------------------------------------------------

#[test]
fn test_funding_target_updated_event_topics_and_payload() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (_token, invoice_id) = funded_init(&env, &client, &admin, &sme, "INVTgt");

    let new_target = 200_000_000_000i128;
    client.update_funding_target(&new_target);

    let events = env.events().all();
    let event = events.events().last().unwrap().clone();

    let expected = FundingTargetUpdated {
        name: symbol_short!("fund_tgt"),
        invoice_id,
        old_target: TARGET,
        new_target,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event, expected);
}

// --- FundingCancelled -----------------------------------------------------

#[test]
fn test_funding_cancelled_event_topics_and_payload() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (_token, invoice_id) = funded_init(&env, &client, &admin, &sme, "INVCanc");

    let investor = Address::generate(&env);
    client.fund(&investor, &(TARGET / 2));

    client.cancel_funding();

    let events = env.events().all();
    let event = events.events().last().unwrap().clone();

    let expected = FundingCancelled {
        name: symbol_short!("fund_cn"),
        invoice_id,
        funded_amount: TARGET / 2,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event, expected);
}

// --- EscrowPartialSettle --------------------------------------------------

#[test]
fn test_escrow_partial_settle_event_topics_and_payload() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (_token, invoice_id) = funded_init(&env, &client, &admin, &sme, "INVPstl");

    let investor = Address::generate(&env);
    client.fund(&investor, &TARGET);

    let partial_amount = TARGET / 4;
    client.partial_settle(&partial_amount);

    let events = env.events().all();
    let event = events.events().last().unwrap().clone();

    let expected = EscrowPartialSettle {
        name: symbol_short!("prtstle"),
        invoice_id,
        funded_amount: TARGET,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event, expected);
}

// --- No topic collision across funding events --------------------------------

#[test]
fn test_no_funding_event_topic_collision() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let _ = funded_init(&env, &client, &admin, &sme, "INVColl");

    let investor = Address::generate(&env);
    client.fund(&investor, &TARGET);

    let funded_events = env.events().all();
    let funded_event = funded_events.events().last().unwrap().clone();

    client.update_funding_target(&(TARGET * 2));

    let target_events = env.events().all();
    let target_event = target_events.events().last().unwrap().clone();

    // Both events must exist and not be identical (topics differ).
    assert_ne!(funded_event, target_event);
}

// --- Multiple funders, sequential topic integrity ---------------------------

#[test]
fn test_sequential_funded_events_carry_distinct_investor_topics() {
    use soroban_sdk::testutils::Events as _;

    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, sme) = setup(&env);
    let contract_id = client.address.clone();

    let (_token, invoice_id) = funded_init(&env, &client, &admin, &sme, "INVMult");

    let inv1 = Address::generate(&env);
    let inv2 = Address::generate(&env);
    let half = TARGET / 2;

    client.fund(&inv1, &half);
    let events1 = env.events().all();
    let event1 = events1.events().last().unwrap().clone();

    client.fund(&inv2, &half);
    let events2 = env.events().all();
    let event2 = events2.events().last().unwrap().clone();

    let expected1 = EscrowFunded {
        name: symbol_short!("funded"),
        invoice_id: invoice_id.clone(),
        investor: inv1,
        amount: half,
        funded_amount: half,
        status: 0,
        investor_effective_yield_bps: 800,
        tier_lock_secs: 0,
    }
    .to_xdr(&env, &contract_id);

    let expected2 = EscrowFunded {
        name: symbol_short!("funded"),
        invoice_id,
        investor: inv2,
        amount: half,
        funded_amount: TARGET,
        status: 1,
        investor_effective_yield_bps: 800,
        tier_lock_secs: 0,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(event1, expected1);
    assert_eq!(event2, expected2);

    // Events differ because investor topic differs.
    assert_ne!(expected1, expected2);
}
