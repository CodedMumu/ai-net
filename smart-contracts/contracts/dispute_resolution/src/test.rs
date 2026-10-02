//! # Dispute Resolution Unit Tests

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, BytesN, Env, Symbol, Vec,
};

// Timestamp constants for deadline arithmetic:
//   evidence_deadline = 0 + 259_200        = 259_200
//   voting_deadline   = 259_200 + 172_800  = 432_000
//   appeal_deadline   = 432_000 + 172_800  = 604_800
const EVIDENCE_DEADLINE: u64 = 259_200;
const VOTING_DEADLINE: u64 = 432_000;
const APPEAL_DEADLINE: u64 = 604_800;

// ────────────────────────────────────────────────────────────────────────────
// Setup helpers
// ────────────────────────────────────────────────────────────────────────────

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

fn setup_with_jurors(
) -> (
    Env,
    DisputeResolutionContractClient<'static>,
    Address,
    Vec<'static, Address>,
) {
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

/// Helper: file a dispute and return the default dispute_id Symbol.
fn file_default_dispute(
    env: &Env,
    client: &DisputeResolutionContractClient,
    filer: &Address,
) -> Symbol {
    let dispute_id = Symbol::new(env, "disp1");
    client.file_dispute(filer, &Symbol::new(env, "agent1"), &dispute_id);
    dispute_id
}

/// Helper: create a BytesN<32> with a specific first byte.
fn make_hash(env: &Env, first_byte: u8) -> BytesN<32> {
    let mut arr = [0u8; 32];
    arr[0] = first_byte;
    BytesN::from_array(env, &arr)
}

/// Helper: advance the ledger timestamp.
fn set_timestamp(env: &Env, ts: u64) {
    env.ledger().with_mut(|l| {
        l.timestamp = ts;
    });
}

// ────────────────────────────────────────────────────────────────────────────
// Existing tests (preserved, with timestamp fixes)
// ────────────────────────────────────────────────────────────────────────────

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

    let hash = make_hash(&env, 42);
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

    // Advance timestamp past voting deadline
    set_timestamp(&env, VOTING_DEADLINE + 1);

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
    set_timestamp(&env, VOTING_DEADLINE + 1);

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

    // Advance past voting deadline to resolve
    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    // Advance past appeal deadline
    set_timestamp(&env, APPEAL_DEADLINE + 1);

    let appellant = Address::generate(&env);
    assert_eq!(
        client.try_appeal_dispute(&dispute_id, &appellant),
        Err(Ok(Error::AppealWindowClosed))
    );
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

// ────────────────────────────────────────────────────────────────────────────
// New comprehensive integration tests
// ────────────────────────────────────────────────────────────────────────────

// ── 1. file_dispute: duplicate rejected ─────────────────────────────────────

#[test]
fn file_dispute_duplicate_rejected() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp_dup");

    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    // Filing the same dispute_id a second time must return AlreadyExists
    assert_eq!(
        client.try_file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id),
        Err(Ok(Error::AlreadyExists))
    );
}

// ── 2. submit_evidence: outside evidence window rejected ────────────────────

#[test]
fn submit_evidence_after_deadline_fails() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "disp_ev");
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &dispute_id);

    // Advance past the evidence deadline
    set_timestamp(&env, EVIDENCE_DEADLINE + 1);

    let hash = make_hash(&env, 7);
    assert_eq!(
        client.try_submit_evidence(&dispute_id, &filer, &hash),
        Err(Ok(Error::DisputeExpired))
    );
}

// ── 3. cast_vote: arbiter vote recorded correctly ───────────────────────────

#[test]
fn cast_vote_arbiter_recorded() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    let juror = jurors.get(2).unwrap();
    client.cast_vote(&dispute_id, &juror, &VoteSide::Agent);

    // Status transitions to Voting
    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Voting);

    // Confirm vote stored — double-vote attempt must fail with JurorAlreadyVoted
    assert_eq!(
        client.try_cast_vote(&dispute_id, &juror, &VoteSide::Client),
        Err(Ok(Error::JurorAlreadyVoted))
    );
}

// ── 4. resolve_dispute: 3-of-5 Client → resolution = 0 (client wins) ───────

#[test]
fn resolve_dispute_3_client_2_agent_client_wins() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(1).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(2).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(3).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(4).unwrap(), &VoteSide::Agent);

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert_eq!(dispute.resolution, Some(0)); // client wins
}

// ── 5. resolve_dispute: 3-of-5 Agent → resolution = 1 (agent wins) ─────────

#[test]
fn resolve_dispute_3_agent_2_client_agent_wins() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(1).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(2).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(3).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(4).unwrap(), &VoteSide::Client);

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert_eq!(dispute.resolution, Some(1)); // agent wins
}

// ── 6. Auto-resolve: voting period expires, majority wins ───────────────────
//
//    Simulates a scenario where no one resolves during the window;
//    resolution is called long after the deadline — the vote tally
//    still determines the outcome.

#[test]
fn resolve_dispute_long_after_deadline_majority_wins() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    // 4 agent votes, 1 client vote
    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(1).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(2).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(3).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(4).unwrap(), &VoteSide::Client);

    // Advance to long after appeal deadline
    set_timestamp(&env, APPEAL_DEADLINE + 86_400);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert_eq!(dispute.resolution, Some(1)); // agent wins (4 vs 1)
}

// ── 7. Edge case: 2-2 tie → resolution = 1 (agent wins) ────────────────────
//
//    When client_votes == agent_votes, client_votes > agent_votes is false,
//    so the condition resolves to 1 (agent wins).

#[test]
fn resolve_dispute_tie_agent_wins() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    // 2 client, 2 agent, 1 abstains (no vote)
    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(1).unwrap(), &VoteSide::Client);
    client.cast_vote(&dispute_id, &jurors.get(2).unwrap(), &VoteSide::Agent);
    client.cast_vote(&dispute_id, &jurors.get(3).unwrap(), &VoteSide::Agent);
    // juror[4] does not vote

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    // client_votes (2) is NOT > agent_votes (2) → resolution = 1
    assert_eq!(dispute.resolution, Some(1));
}

// ── 8. resolve_dispute before voting deadline fails ─────────────────────────

#[test]
fn resolve_dispute_before_deadline_fails() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client);

    // Timestamp is still 0 (before voting_deadline = 432_000)
    assert_eq!(
        client.try_resolve_dispute(&dispute_id),
        Err(Ok(Error::DisputeExpired))
    );
}

// ── 9. All 5 jurors vote Client → resolution = 0 ────────────────────────────

#[test]
fn resolve_dispute_all_five_client_wins() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    for i in 0..5 {
        client.cast_vote(&dispute_id, &jurors.get(i).unwrap(), &VoteSide::Client);
    }

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert_eq!(dispute.resolution, Some(0)); // unanimous client win
}

// ── 10. All 5 jurors vote Agent → resolution = 1 ────────────────────────────

#[test]
fn resolve_dispute_all_five_agent_wins() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    for i in 0..5 {
        client.cast_vote(&dispute_id, &jurors.get(i).unwrap(), &VoteSide::Agent);
    }

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    assert_eq!(dispute.resolution, Some(1)); // unanimous agent win
}

// ── 11. resolve_dispute twice fails ─────────────────────────────────────────

#[test]
fn resolve_dispute_twice_fails() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client);

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    assert_eq!(
        client.try_resolve_dispute(&dispute_id),
        Err(Ok(Error::DisputeAlreadyResolved))
    );
}

// ── 12. Multiple evidence submissions within window ──────────────────────────

#[test]
fn submit_multiple_evidence_within_window() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    let other_party = Address::generate(&env);

    client.submit_evidence(&dispute_id, &filer, &make_hash(&env, 1));
    client.submit_evidence(&dispute_id, &other_party, &make_hash(&env, 2));
    client.submit_evidence(&dispute_id, &filer, &make_hash(&env, 3));

    assert_eq!(client.get_evidence_count(&dispute_id), 3);
}

// ── 13. Evidence submission moves status to EvidenceSubmission ───────────────

#[test]
fn submit_evidence_transitions_status() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    let before = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(before.status, DisputeStatus::Filed);

    client.submit_evidence(&dispute_id, &filer, &make_hash(&env, 10));

    let after = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(after.status, DisputeStatus::EvidenceSubmission);
}

// ── 14. Appeal on unresolved dispute fails ───────────────────────────────────

#[test]
fn appeal_unresolved_dispute_fails() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    let appellant = Address::generate(&env);
    // Dispute is still in Filed status — not Resolved
    assert_eq!(
        client.try_appeal_dispute(&dispute_id, &appellant),
        Err(Ok(Error::DisputeAlreadyResolved))
    );
}

// ── 15. Appeal twice fails ───────────────────────────────────────────────────

#[test]
fn appeal_twice_fails() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    client.cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client);

    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let appellant = Address::generate(&env);
    client.appeal_dispute(&dispute_id, &appellant);

    // Second appeal must fail with AlreadyExists
    assert_eq!(
        client.try_appeal_dispute(&dispute_id, &appellant),
        Err(Ok(Error::AlreadyExists))
    );
}

// ── 16. Casting vote on already-resolved dispute fails ───────────────────────

#[test]
fn cast_vote_on_resolved_dispute_fails() {
    let (env, client, _admin, jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    // Advance past voting deadline and resolve without any votes
    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    // Now try to cast a vote — voting_deadline is in the past so DisputeExpired
    assert_eq!(
        client.try_cast_vote(&dispute_id, &jurors.get(0).unwrap(), &VoteSide::Client),
        Err(Ok(Error::DisputeExpired))
    );
}

// ── 17. Pausing unpauses correctly ───────────────────────────────────────────

#[test]
fn pause_and_unpause() {
    let (env, client, _admin) = setup_with_admin();

    // Re-set jurors so we can test filing after unpause
    let jurors = soroban_sdk::vec![
        &env,
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    client.set_jurors(&jurors);

    // Pause
    client.pause(&true);
    let filer = Address::generate(&env);
    assert_eq!(
        client.try_file_dispute(
            &filer,
            &Symbol::new(&env, "agent1"),
            &Symbol::new(&env, "disp_p")
        ),
        Err(Ok(Error::ContractPaused))
    );

    // Unpause — filing should succeed
    client.pause(&false);
    client.file_dispute(&filer, &Symbol::new(&env, "agent1"), &Symbol::new(&env, "disp_p"));
    let dispute = client.get_dispute(&Symbol::new(&env, "disp_p")).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Filed);
}

// ── 18. get_dispute returns None for unknown dispute_id ──────────────────────

#[test]
fn get_dispute_unknown_returns_none() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    let result = client.get_dispute(&Symbol::new(&env, "nonexistent"));
    assert!(result.is_none());
}

// ── 19. No-vote resolution defaults to agent wins (0 client vs 0 agent) ─────

#[test]
fn resolve_dispute_no_votes_agent_wins() {
    let (env, client, _admin, _jurors) = setup_with_jurors();
    env.ledger().set_max_entry_ttl(100_000_000);

    let filer = Address::generate(&env);
    let dispute_id = file_default_dispute(&env, &client, &filer);

    // No one votes; advance past deadline
    set_timestamp(&env, VOTING_DEADLINE + 1);
    client.resolve_dispute(&dispute_id);

    let dispute = client.get_dispute(&dispute_id).unwrap();
    assert_eq!(dispute.status, DisputeStatus::Resolved);
    // 0 client vs 0 agent: client_votes (0) > agent_votes (0) is false → 1
    assert_eq!(dispute.resolution, Some(1));
}
