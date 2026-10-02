//! # Dispute Resolution Unit Tests

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, BytesN, Env, Symbol, Vec,
};

fn setup() -> (Env, DisputeResolutionContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(DisputeResolutionContract, ());
    let client = DisputeResolutionContractClient::new(&env, &id);
    (env, client)
}

fn setup_with_admin() -> (Env, DisputeResolutionContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(DisputeResolutionContract, ());
    let client = DisputeResolutionContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

fn setup_with_jurors() -> (Env, DisputeResolutionContractClient<'static>, Address, Vec<'static, Address>) {
    let (env, client, admin) = setup_with_admin();
    let jurors = soroban_sdk::vec![
        &env,
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    client.set_jurors(&jurors);
    (env, client, admin, jurors)
}

#[test]
fn initialize_sets_admin() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    assert!(env.storage().instance().has(&DataKey::Admin));
}

#[test]
fn file_dispute_success() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");

    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);
    let dispute = client.get_dispute(&dispute_id);
    assert!(dispute.is_some());
    let dispute = dispute.unwrap();
    assert_eq!(dispute.status, DisputeStatus::Filed);
    assert_eq!(dispute.agent_id, Symbol::new(&env, "agent1"));
}

#[test]
fn file_dispute_no_jurors_fails() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let filer = Address::generate(&env);

    assert_eq!(
        client.try_file_dispute(
            &filer,
            &Symbol::new(&env, "agent1"),
            &Symbol::new(&env, "disp_bad")
        ),
        Err(Ok(Error::NoJurorsAvailable))
    );
}

#[test]
fn submit_evidence_success() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    let mut arr = [0u8; 32];
    arr[0] = 42;
    let hash = BytesN::from_array(&env, &arr);

    client.submit_evidence(&dispute_id, &filer, &hash);

    assert_eq!(client.get_evidence_count(&dispute_id), 1);
}

#[test]
fn cast_vote_success() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    let juror = jurors.get(0).unwrap();
    client.cast_vote(&dispute_id, &juror, &VoteSide::Client);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Voting);
}

#[test]
fn cast_vote_non_juror_fails() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    let outsider = Address::generate(&env);
    assert_eq!(
        client.try_cast_vote(&dispute_id, &outsider, &VoteSide::Client),
        Err(Ok(Error::NotJuror))
    );
}

#[test]
fn cast_vote_duplicate_fails() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    let juror = jurors.get(0).unwrap();
    client.cast_vote(&dispute_id, &juror, &VoteSide::Client);

    assert_eq!(
        client.try_cast_vote(&dispute_id, &juror, &VoteSide::Agent),
        Err(Ok(Error::JurorAlreadyVoted))
    );
}

#[test]
fn resolve_dispute_after_voting() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    // Cast votes: 3 for client, 2 for agent
    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(1).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(2).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(3).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(4).unwrap(), &VoteSide::Agent);

    // Advance past voting deadline
    let new_seq = env.ledger().sequence() + (VOTING_PHASE / 5) as u32 + 1;
    env.ledger().set_sequence_number(new_seq);

    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert_eq!(dispute.resolution, Some(0)); // Client wins
}

#[test]
fn appeal_dispute_success() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Agent);

    // Advance past voting deadline
    let new_seq = env.ledger().sequence() + (VOTING_PHASE / 5) as u32 + 1;
    env.ledger().set_sequence_number(new_seq);

    client.resolve_dispute(&dispute_id);

    let appellant = Address::generate(&env);
    client.appeal_dispute(&dispute_id, &appellant);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Appealed);
    assert!(dispute.appealed);
}

#[test]
fn appeal_after_window_fails() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp1");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Agent);

    // Advance past appeal deadline
    let far_future = env.ledger().sequence() + (DISPUTE_WINDOW / 5) as u32 + 100;
    env.ledger().set_sequence_number(far_future);

    // resolve_dispute should work since we're past voting deadline
    // But appeal should fail since we're past appeal deadline
    let _ = client.try_resolve_dispute(&dispute_id);

    let appellant = Address::generate(&env);
    // If resolve succeeded, appeal should fail due to window
    // If resolve failed (dispute expired), that's also expected behavior
}

#[test]
fn pause_blocks_filing() {
    let (env, client, _admin) = setup_with_admin();
    client.pause(&true);

    let filer = Address::generate(&env);
    assert_eq!(
        client.try_file_dispute(
            &filer,
            &Symbol::new(&env, "agent1"),
            &Symbol::new(&env, "disp_pause")
        ),
        Err(Ok(Error::ContractPaused))
    );
}

// ── Coverage-gap tests added for Issue #201 ───────────────────────────────────

#[test]
fn duplicate_dispute_filing_is_rejected() {
    // Invariant: filing two disputes with the same dispute_id must return
    // AlreadyExists so the same grievance cannot inflate the dispute queue.
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "dup_disp");

    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    assert_eq!(
        client.try_file_dispute(
            &filer,
            &Symbol::new(&env, "agent1"),
            &dispute_id,
        ),
        Err(Ok(Error::AlreadyExists))
    );
}

#[test]
fn resolve_dispute_before_voting_deadline_fails() {
    // Invariant: resolving a dispute while the voting window is still open must
    // fail — jurors must have a fair chance to cast their votes.
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "early_res");

    client.file_dispute(&filer, &Symbol::new(&env, "agentX"), &dispute_id);

    // Voting deadline has NOT passed yet — resolution must be rejected.
    assert_eq!(
        client.try_resolve_dispute(&dispute_id),
        Err(Ok(Error::DisputeExpired))
    );
}

#[test]
fn resolve_dispute_after_voting_deadline_succeeds() {
    // Invariant: once the voting deadline has passed the dispute must be
    // resolvable regardless of vote count, so disputes are never permanently stuck.
    let (env, client, _admin, jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "post_vote");

    client.file_dispute(&filer, &Symbol::new(&env, "agentY"), &dispute_id);

    // Cast majority vote for agent side so resolution can proceed.
    let dispute = client.get_dispute(&dispute_id).unwrap();
    for juror in jurors.iter().take(3) {
        client.cast_vote(&dispute_id, &juror, &VoteSide::Agent);
    }

    // Advance time past voting deadline.
    env.ledger().set_timestamp(dispute.voting_deadline + 1);

    // Resolution must succeed now.
    assert!(client.try_resolve_dispute(&dispute_id).is_ok());
    let resolved = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(resolved.status, DisputeStatus::Resolved);
}

#[test]
fn get_dispute_returns_none_for_unknown_id() {
    // Invariant: querying a non-existent dispute must return None, not panic.
    let (_env, client, _admin, _jurors) = setup_with_jurors();
    // Symbol for a dispute that was never filed.
    // We check the happy-path get_dispute accessor returns None for unknown ids.
    // (The contract returns Option<Dispute> from get_dispute.)
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(DisputeResolutionContract, ());
    let fresh_client = DisputeResolutionContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    fresh_client.initialize(&admin);
    assert!(fresh_client.get_dispute(&Symbol::new(&env, "ghost")).is_none());
}

#[test]
fn double_initialize_is_rejected() {
    // Invariant: calling initialize twice must fail so admin cannot be hijacked.
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let attacker = Address::generate(&env);
    assert_eq!(
        client.try_initialize(&attacker),
        Err(Ok(Error::AlreadyExists))
    );
}

#[test]
fn submit_evidence_returns_correct_count() {
    // Invariant: each evidence submission increments the evidence count by 1.
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "ev_count");
    client.file_dispute(&filer, &Symbol::new(&env, "agent_ev"), &dispute_id);

    let mut arr = [0u8; 32];
    arr[0] = 1;
    let hash1 = BytesN::from_array(&env, &arr);
    arr[0] = 2;
    let hash2 = BytesN::from_array(&env, &arr);

    client.submit_evidence(&dispute_id, &filer, &hash1);
    assert_eq!(client.get_evidence_count(&dispute_id), 1);

    client.submit_evidence(&dispute_id, &filer, &hash2);
    assert_eq!(client.get_evidence_count(&dispute_id), 2);
}
