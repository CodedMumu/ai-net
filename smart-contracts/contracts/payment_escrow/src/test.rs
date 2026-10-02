//! Tests for the payment escrow contract (issue #192).
//!
//! Covers all state transitions and authorization checks.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Env, IntoVal,
};

struct Fixture {
    env: Env,
    client: PaymentEscrowContractClient<'static>,
    admin: Address,
    coordinator: Address,
    agent: Address,
    task_id: Symbol,
}

fn fixture() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| {
        l.sequence_number = 100;
        l.timestamp = 1_700_000_000;
    });

    let contract_id = env.register(PaymentEscrowContract, ());
    let client = PaymentEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let coordinator = Address::generate(&env);
    let agent = Address::generate(&env);
    let task_id = Symbol::new(&env, "task001");

    client.initialize(&admin);

    Fixture {
        env,
        client,
        admin,
        coordinator,
        agent,
        task_id,
    }
}

const AMOUNT: i128 = 10_000_000; // 1 XLM in stroops
const TIMEOUT: u32 = 200; // ledger 200 > current 100

// ── create_escrow ─────────────────────────────────────────────────────────────

#[test]
fn create_escrow_stores_active_record() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    let record = f.client.get_escrow(&f.task_id).expect("escrow not found");
    assert_eq!(record.state, EscrowState::Active);
    assert_eq!(record.coordinator, f.coordinator);
    assert_eq!(record.agent, f.agent);
    assert_eq!(record.amount, AMOUNT);
    assert_eq!(record.timeout_ledger, TIMEOUT);
    assert_eq!(record.dispute_hold_until, None);
}

#[test]
fn create_escrow_emits_created_event() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    let events = f.env.events().all();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events.get(0).unwrap().1,
        (symbol_short!("escrow"), symbol_short!("created")).into_val(&f.env)
    );
}

#[test]
fn create_escrow_duplicate_task_id_rejected() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    let result = f
        .client
        .try_create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    assert_eq!(result, Err(Ok(Error::AlreadyExists)));
}

#[test]
fn create_escrow_zero_amount_rejected() {
    let f = fixture();
    let result = f
        .client
        .try_create_escrow(&f.coordinator, &f.task_id, &f.agent, &0, &TIMEOUT);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn create_escrow_timeout_in_past_rejected() {
    let f = fixture();
    // current ledger is 100; timeout <= 100 is invalid
    let result = f
        .client
        .try_create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &50u32);
    assert_eq!(result, Err(Ok(Error::InvalidTimeout)));
}

// ── release_escrow ────────────────────────────────────────────────────────────

#[test]
fn coordinator_can_release_active_escrow() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    f.client.release_escrow(&f.coordinator, &f.task_id);

    let record = f.client.get_escrow(&f.task_id).expect("record removed");
    assert_eq!(record.state, EscrowState::Released);
}

#[test]
fn release_escrow_emits_released_event() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    f.client.release_escrow(&f.coordinator, &f.task_id);

    let events = f.env.events().all();
    // Last event should be "released"
    let last = events.get(events.len() - 1).unwrap();
    assert_eq!(
        last.1,
        (symbol_short!("escrow"), symbol_short!("released")).into_val(&f.env)
    );
}

#[test]
fn release_escrow_non_coordinator_rejected() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    let result = f.client.try_release_escrow(&f.agent, &f.task_id);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn double_release_rejected() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    f.client.release_escrow(&f.coordinator, &f.task_id);

    let result = f.client.try_release_escrow(&f.coordinator, &f.task_id);
    assert_eq!(result, Err(Ok(Error::AlreadyReleased)));
}

// ── dispute_escrow ────────────────────────────────────────────────────────────

#[test]
fn coordinator_can_dispute_active_escrow() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    f.client.dispute_escrow(&f.coordinator, &f.task_id);

    let record = f.client.get_escrow(&f.task_id).expect("record removed");
    assert_eq!(record.state, EscrowState::Disputed);
    assert_eq!(record.dispute_hold_until, Some(100 + DISPUTE_HOLD_LEDGERS));
}

#[test]
fn agent_can_dispute_active_escrow() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    f.client.dispute_escrow(&f.agent, &f.task_id);

    let record = f.client.get_escrow(&f.task_id).expect("record removed");
    assert_eq!(record.state, EscrowState::Disputed);
}

#[test]
fn dispute_escrow_emits_disputed_event() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    f.client.dispute_escrow(&f.coordinator, &f.task_id);

    let events = f.env.events().all();
    let last = events.get(events.len() - 1).unwrap();
    assert_eq!(
        last.1,
        (symbol_short!("escrow"), symbol_short!("disputed")).into_val(&f.env)
    );
}

#[test]
fn third_party_cannot_dispute() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    let third_party = Address::generate(&f.env);
    let result = f.client.try_dispute_escrow(&third_party, &f.task_id);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn cannot_dispute_already_released_escrow() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    f.client.release_escrow(&f.coordinator, &f.task_id);

    let result = f.client.try_dispute_escrow(&f.coordinator, &f.task_id);
    assert_eq!(result, Err(Ok(Error::AlreadyReleased)));
}

#[test]
fn cannot_dispute_already_disputed_escrow() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    f.client.dispute_escrow(&f.coordinator, &f.task_id);

    let result = f.client.try_dispute_escrow(&f.agent, &f.task_id);
    assert_eq!(result, Err(Ok(Error::AlreadyDisputed)));
}

// ── expire_escrow ─────────────────────────────────────────────────────────────

#[test]
fn anyone_can_expire_after_timeout() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    // Advance ledger past timeout
    f.env.ledger().with_mut(|l| {
        l.sequence_number = TIMEOUT;
    });

    f.client.expire_escrow(&f.task_id);

    let record = f.client.get_escrow(&f.task_id).expect("record removed");
    assert_eq!(record.state, EscrowState::Expired);
}

#[test]
fn expire_escrow_emits_expired_event() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    f.env.ledger().with_mut(|l| {
        l.sequence_number = TIMEOUT;
    });
    f.client.expire_escrow(&f.task_id);

    let events = f.env.events().all();
    let last = events.get(events.len() - 1).unwrap();
    assert_eq!(
        last.1,
        (symbol_short!("escrow"), symbol_short!("expired")).into_val(&f.env)
    );
}

#[test]
fn expire_before_timeout_rejected() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    // Still at ledger 100 < TIMEOUT (200)
    let result = f.client.try_expire_escrow(&f.task_id);
    assert_eq!(result, Err(Ok(Error::NotYetExpired)));
}

#[test]
fn cannot_expire_released_escrow() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);
    f.client.release_escrow(&f.coordinator, &f.task_id);

    f.env.ledger().with_mut(|l| {
        l.sequence_number = TIMEOUT;
    });
    let result = f.client.try_expire_escrow(&f.task_id);
    assert_eq!(result, Err(Ok(Error::AlreadyReleased)));
}

#[test]
fn double_expire_rejected() {
    let f = fixture();
    f.client
        .create_escrow(&f.coordinator, &f.task_id, &f.agent, &AMOUNT, &TIMEOUT);

    f.env.ledger().with_mut(|l| {
        l.sequence_number = TIMEOUT;
    });
    f.client.expire_escrow(&f.task_id);

    let result = f.client.try_expire_escrow(&f.task_id);
    assert_eq!(result, Err(Ok(Error::AlreadyExpired)));
}

// ── get_escrow on unknown task_id ─────────────────────────────────────────────

#[test]
fn get_escrow_returns_none_for_unknown_task() {
    let f = fixture();
    let unknown = Symbol::new(&f.env, "unknown");
    assert_eq!(f.client.get_escrow(&unknown), None);
}
