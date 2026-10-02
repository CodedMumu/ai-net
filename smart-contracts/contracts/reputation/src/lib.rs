#![no_std]

//! On-chain agent reputation derived from authorized task outcomes.
//!
//! Scores use fixed-point hundredths, start at zero, and may be negative.
//! Outcome events provide the durable reputation history.

mod errors;

pub use errors::Error;

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

pub const SUCCESS_POINTS: i64 = 1_000;
pub const FAILURE_POINTS: i64 = 2_000;
pub const FIVE_STAR_MULTIPLIER_BPS: i64 = 150;
pub const DECAY_RATE_BPS: i64 = 500;
pub const SECONDS_PER_WEEK: u64 = 604_800;
pub const INITIAL_SCORE: i64 = 0;

const TTL_THRESHOLD: u32 = 50_000;
const TTL_EXTEND_TO: u32 = 241_920;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationScore {
    pub agent_id: Symbol,
    /// Score in hundredths of a point.
    pub score: i64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub last_activity: u64,
    pub last_decay_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeRecordedEvent {
    pub agent_id: Symbol,
    pub task_id: Symbol,
    pub success: bool,
    /// Soroban contract interfaces use u32; accepted values are 0 through 5.
    pub rating: u32,
    pub new_score: i64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationDecayedEvent {
    pub agent_id: Symbol,
    pub old_score: i64,
    pub new_score: i64,
    pub weeks_elapsed: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    AuthorizedCallers,
    Reputation(Symbol),
    Outcome(Symbol, Symbol),
}

#[contract]
pub struct ReputationContract;

fn require_initialized(env: &Env) -> Result<(), Error> {
    if env.storage().instance().has(&DataKey::Admin) {
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

fn is_authorized_caller(env: &Env, addr: &Address) -> bool {
    let callers: Vec<Address> = env
        .storage()
        .instance()
        .get(&DataKey::AuthorizedCallers)
        .unwrap_or_else(|| Vec::new(env));
    callers.contains(addr)
}

fn load_or_create_score(env: &Env, agent_id: &Symbol) -> ReputationScore {
    let now = env.ledger().timestamp();
    env.storage()
        .persistent()
        .get(&DataKey::Reputation(agent_id.clone()))
        .unwrap_or(ReputationScore {
            agent_id: agent_id.clone(),
            score: INITIAL_SCORE,
            successful_tasks: 0,
            failed_tasks: 0,
            last_activity: now,
            last_decay_at: now,
        })
}

fn save_score(env: &Env, score: &ReputationScore) {
    let key = DataKey::Reputation(score.agent_id.clone());
    env.storage().persistent().set(&key, score);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

fn apply_decay(score: i64, weeks_elapsed: u64) -> i64 {
    let mut decayed = score;
    for _ in 0..weeks_elapsed {
        decayed = decayed.saturating_mul(10_000 - DECAY_RATE_BPS) / 10_000;
    }
    decayed
}

#[contractimpl]
impl ReputationContract {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::AuthorizedCallers, &Vec::<Address>::new(&env));
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
        Ok(())
    }

    pub fn add_authorized_caller(env: Env, caller: Address) -> Result<(), Error> {
        require_initialized(&env)?;
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let mut callers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AuthorizedCallers)
            .unwrap_or_else(|| Vec::new(&env));
        if callers.contains(&caller) {
            return Err(Error::CallerAlreadyRegistered);
        }
        callers.push_back(caller);
        env.storage()
            .instance()
            .set(&DataKey::AuthorizedCallers, &callers);
        Ok(())
    }

    pub fn remove_authorized_caller(env: Env, caller: Address) -> Result<(), Error> {
        require_initialized(&env)?;
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        let callers: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AuthorizedCallers)
            .unwrap_or_else(|| Vec::new(&env));
        let mut updated = Vec::new(&env);
        let mut found = false;
        for existing in callers.iter() {
            if existing == caller {
                found = true;
            } else {
                updated.push_back(existing);
            }
        }
        if !found {
            return Err(Error::CallerNotFound);
        }
        env.storage()
            .instance()
            .set(&DataKey::AuthorizedCallers, &updated);
        Ok(())
    }

    /// Record a task outcome. `rating` is represented as u32 in the Soroban ABI
    /// and is restricted to the range 0 through 5.
    pub fn record_outcome(
        env: Env,
        caller: Address,
        agent_id: Symbol,
        task_id: Symbol,
        success: bool,
        rating: u32,
    ) -> Result<(), Error> {
        require_initialized(&env)?;
        if !is_authorized_caller(&env, &caller) {
            return Err(Error::Unauthorized);
        }
        caller.require_auth();
        if rating > 5 {
            return Err(Error::InvalidRating);
        }

        let outcome_key = DataKey::Outcome(agent_id.clone(), task_id.clone());
        if env.storage().persistent().has(&outcome_key) {
            return Err(Error::TaskAlreadyRecorded);
        }

        let mut reputation = load_or_create_score(&env, &agent_id);
        let delta = if success {
            if rating == 5 {
                SUCCESS_POINTS * FIVE_STAR_MULTIPLIER_BPS / 100
            } else {
                SUCCESS_POINTS
            }
        } else {
            -FAILURE_POINTS
        };
        reputation.score = reputation.score.saturating_add(delta);
        if success {
            reputation.successful_tasks = reputation.successful_tasks.saturating_add(1);
        } else {
            reputation.failed_tasks = reputation.failed_tasks.saturating_add(1);
        }
        let now = env.ledger().timestamp();
        reputation.last_activity = now;
        reputation.last_decay_at = now;
        save_score(&env, &reputation);
        env.storage().persistent().set(&outcome_key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&outcome_key, TTL_THRESHOLD, TTL_EXTEND_TO);

        env.events().publish(
            (symbol_short!("reputat"), symbol_short!("outcome")),
            OutcomeRecordedEvent {
                agent_id,
                task_id,
                success,
                rating,
                new_score: reputation.score,
            },
        );
        Ok(())
    }

    /// Apply compound 5% decay for each complete inactive week.
    pub fn decay_reputation(env: Env, agent_id: Symbol) -> Result<(), Error> {
        require_initialized(&env)?;
        let mut reputation = load_or_create_score(&env, &agent_id);
        let now = env.ledger().timestamp();
        let weeks_elapsed = now.saturating_sub(reputation.last_decay_at) / SECONDS_PER_WEEK;
        if weeks_elapsed == 0 {
            return Ok(());
        }

        let old_score = reputation.score;
        reputation.score = apply_decay(old_score, weeks_elapsed);
        reputation.last_decay_at = reputation
            .last_decay_at
            .saturating_add(weeks_elapsed.saturating_mul(SECONDS_PER_WEEK));
        save_score(&env, &reputation);
        env.events().publish(
            (symbol_short!("reputat"), symbol_short!("decayed")),
            ReputationDecayedEvent {
                agent_id,
                old_score,
                new_score: reputation.score,
                weeks_elapsed,
            },
        );
        Ok(())
    }

    pub fn get_reputation(env: Env, agent_id: Symbol) -> Result<ReputationScore, Error> {
        require_initialized(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::Reputation(agent_id))
            .ok_or(Error::AgentNotFound)
    }

    /// Return a score in hundredths of a point, or zero for an unknown agent.
    pub fn get_score(env: Env, agent_id: Symbol) -> i64 {
        env.storage()
            .persistent()
            .get::<DataKey, ReputationScore>(&DataKey::Reputation(agent_id))
            .map(|record| record.score)
            .unwrap_or(INITIAL_SCORE)
    }

    pub fn get_authorized_callers(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::AuthorizedCallers)
            .unwrap_or_else(|| Vec::new(&env))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger},
        Address, Env, IntoVal,
    };

    fn fixture() -> (Env, ReputationContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| ledger.timestamp = 1_000_000);
        let contract_id = env.register(ReputationContract, ());
        let client = ReputationContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let caller = Address::generate(&env);
        client.initialize(&admin);
        client.add_authorized_caller(&caller);
        (env, client, caller)
    }

    fn agent(env: &Env) -> Symbol {
        Symbol::new(env, "agent1")
    }

    fn task(env: &Env) -> Symbol {
        Symbol::new(env, "task1")
    }

    #[test]
    fn success_adds_ten_points() {
        let (env, client, caller) = fixture();
        client.record_outcome(&caller, &agent(&env), &task(&env), &true, &0);
        assert_eq!(client.get_reputation(&agent(&env)).score, SUCCESS_POINTS);
    }

    #[test]
    fn failure_subtracts_twenty_points() {
        let (env, client, caller) = fixture();
        client.record_outcome(&caller, &agent(&env), &task(&env), &false, &0);
        assert_eq!(client.get_reputation(&agent(&env)).score, -FAILURE_POINTS);
    }

    #[test]
    fn five_star_success_uses_multiplier() {
        let (env, client, caller) = fixture();
        client.record_outcome(&caller, &agent(&env), &task(&env), &true, &5);
        assert_eq!(
            client.get_reputation(&agent(&env)).score,
            SUCCESS_POINTS * FIVE_STAR_MULTIPLIER_BPS / 100
        );
    }

    #[test]
    fn task_outcome_cannot_be_counted_twice() {
        let (env, client, caller) = fixture();
        let agent_id = agent(&env);
        let task_id = task(&env);
        client.record_outcome(&caller, &agent_id, &task_id, &true, &0);
        assert_eq!(
            client.try_record_outcome(&caller, &agent_id, &task_id, &true, &0),
            Err(Ok(Error::TaskAlreadyRecorded))
        );
        assert_eq!(client.get_reputation(&agent_id).successful_tasks, 1);
    }

    #[test]
    fn unauthorized_caller_cannot_record_outcome() {
        let (env, client, _) = fixture();
        let stranger = Address::generate(&env);
        assert_eq!(
            client.try_record_outcome(&stranger, &agent(&env), &task(&env), &true, &0),
            Err(Ok(Error::Unauthorized))
        );
    }

    #[test]
    fn decay_applies_five_percent_per_complete_week() {
        let (env, client, caller) = fixture();
        let agent_id = agent(&env);
        client.record_outcome(&caller, &agent_id, &task(&env), &true, &0);
        env.ledger()
            .with_mut(|ledger| ledger.timestamp += SECONDS_PER_WEEK + 1);
        client.decay_reputation(&agent_id);
        assert_eq!(
            client.get_reputation(&agent_id).score,
            SUCCESS_POINTS * (10_000 - DECAY_RATE_BPS) / 10_000
        );
    }

    #[test]
    fn new_outcome_resets_inactivity_clock() {
        let (env, client, caller) = fixture();
        let agent_id = agent(&env);
        client.record_outcome(&caller, &agent_id, &task(&env), &true, &0);
        env.ledger()
            .with_mut(|ledger| ledger.timestamp += SECONDS_PER_WEEK - 1);
        client.decay_reputation(&agent_id);
        let second_task = Symbol::new(&env, "task2");
        client.record_outcome(&caller, &agent_id, &second_task, &true, &0);
        env.ledger().with_mut(|ledger| ledger.timestamp += 1);
        client.decay_reputation(&agent_id);
        assert_eq!(client.get_reputation(&agent_id).score, SUCCESS_POINTS * 2);
    }

    #[test]
    fn outcome_event_is_emitted() {
        let (env, client, caller) = fixture();
        client.record_outcome(&caller, &agent(&env), &task(&env), &true, &3);
        let events = env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("reputat"), symbol_short!("outcome")).into_val(&env)
        );
    }
}
