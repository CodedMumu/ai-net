#![no_std]
// Soroban contract entrypoints can legitimately have more than 7 parameters;
// suppress this lint for the whole crate rather than annotating every generated
// client function individually.
#![allow(clippy::too_many_arguments)]

//! # Task Store Contract
//!
//! Stores task metadata and manages the task lifecycle state machine.
//!
//! ## High-level lifecycle API (create_task / assign_task / complete_task / fail_task / get_task)
//!
//! This layer provides a clean, permission-enforced API for the three-state
//! lifecycle: `Created → Assigned → Completed / Failed`.
//!
//! - `create_task`   — callable by any submitter (requires `submitter.require_auth()`)
//! - `assign_task`   — coordinator-only (`coordinator.require_auth()`)
//! - `complete_task` — assigned-agent-only (`agent.require_auth()`)
//! - `fail_task`     — assigned-agent or coordinator
//! - `get_task`      — read-only, no auth required
//!
//! ## Oracle integration
//!
//! When an OracleManager is configured (via `set_oracle_manager`), the current
//! market price for the supplied `price_pair` is resolved via
//! `OracleManager::resolve_price` and stamped immutably onto the task at
//! creation time in `TaskMetadata::quoted_price_stroops`.
//!
//! If no OracleManager is configured, or if `price_pair` is `None`, the field
//! is left as `None` and no error is returned — legacy callers that do not
//! supply a pair continue to work unchanged.
//!
//! If an OracleManager *is* configured and a `price_pair` is supplied but the
//! oracle returns no usable price (stale feed + no fallback), the call is
//! **rejected** with `Error::OraclePriceUnavailable`. This prevents tasks from
//! being accepted at an unknown cost.

mod types;

pub use types::{
    DataKey, Error, OracleManagerSetEvent, TaskCreatedEvent, TaskFinalizedEvent, TaskLifecycleStatus,
    TaskMetadata, TaskRecord, TaskStoreAssignedEvent, TaskStoreCompletedEvent, TaskStoreCreatedEvent,
    TaskStoreFailedEvent, TaskStatus, TaskUpdatedEvent, DEFAULT_TTL_DAYS, LEDGERS_PER_DAY,
    MAX_COMPRESSED_DAG_BYTES, MAX_TTL_DAYS, TASK_LIFECYCLE_EVENT_VERSION,
};

use soroban_sdk::{
    contract, contractimpl, symbol_short, Address, Bytes, BytesN, Env, String, Vec,
};

const SECONDS_PER_DAY: u64 = 86_400;
const CONTRACT_VERSION: &str = "1.0.0";
/// Default TTL (in ledgers) for TaskRecord entries: 30 days.
const TASK_RECORD_TTL_LEDGERS: u32 = MAX_TTL_DAYS * LEDGERS_PER_DAY;

fn ttl_ledgers(ttl_days: u32) -> u32 {
    ttl_days.saturating_mul(LEDGERS_PER_DAY)
}

fn is_expired(env: &Env, metadata: &TaskMetadata) -> bool {
    env.ledger().timestamp() >= metadata.expires_at
}

fn read_metadata(env: &Env, task_id: &BytesN<32>) -> Result<TaskMetadata, Error> {
    let key = DataKey::Task(task_id.clone());
    let metadata: TaskMetadata = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::NotFound)?;

    if is_expired(env, &metadata) {
        return Err(Error::Expired);
    }

    Ok(metadata)
}

fn has_duplicate_agents(agents: &Vec<Address>) -> bool {
    for (index, agent) in agents.iter().enumerate() {
        for other in agents.iter().skip(index + 1) {
            if agent == other {
                return true;
            }
        }
    }
    false
}

fn can_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Pending, TaskStatus::Running)
            | (TaskStatus::Pending, TaskStatus::Failed)
            | (TaskStatus::Running, TaskStatus::Completed)
            | (TaskStatus::Running, TaskStatus::Failed)
    )
}

fn is_terminal(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::Completed | TaskStatus::Failed)
}

fn read_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

fn require_admin(env: &Env) -> Result<Address, Error> {
    let admin = read_admin(env)?;
    admin.require_auth();
    Ok(admin)
}

/// Read the coordinator from instance storage. Returns `None` if unset.
fn read_coordinator(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Coordinator)
}

/// Read a TaskRecord by its numeric task_id, or return Error::NotFound.
fn read_task_record(env: &Env, task_id: u64) -> Result<TaskRecord, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::TaskRecord(task_id))
        .ok_or(Error::NotFound)
}

/// Call `OracleManager::resolve_price(pair)` via a low-level cross-contract
/// call and return the resolved price in stroops on success, or `None` on any
/// failure (stale feed, no fallback, call error).  The oracle manager expresses
/// its error by trapping, which we catch with `try_invoke_contract`.
fn try_resolve_price(env: &Env, oracle_manager: &Address, pair: &soroban_sdk::Symbol) -> Option<i128> {
    use soroban_sdk::{InvokeError, Map, TryIntoVal, Val};

    let fn_name = soroban_sdk::Symbol::new(env, "resolve_price");
    let args = soroban_sdk::vec![env, pair.into_val(env)];

    // try_invoke_contract<T, E> returns Result<Result<T, T::Error>, Result<E, InvokeError>>.
    let result: Result<Result<Val, _>, Result<InvokeError, InvokeError>> =
        env.try_invoke_contract(oracle_manager, &fn_name, args);

    match result {
        Ok(Ok(val)) => {
            // ResolvedPrice is a contracttype struct — serialised as a Map keyed
            // by field-name Symbols.  Extract the `price` field.
            let map: Result<Map<soroban_sdk::Symbol, Val>, _> = val.try_into_val(env);
            if let Ok(m) = map {
                let price_key = soroban_sdk::Symbol::new(env, "price");
                m.get(price_key)
                    .and_then(|v| v.try_into_val(env).ok())
                    .filter(|p: &i128| *p > 0)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[contract]
pub struct TaskStoreContract;

#[contractimpl]
impl TaskStoreContract {
    // ─────────────────────────────────────────────────────────────────────────
    // Admin / initialization
    // ─────────────────────────────────────────────────────────────────────────

    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, CONTRACT_VERSION));
        // Initialise task counter to 0.
        env.storage().instance().set(&DataKey::TaskCounter, &0u64);
        env.storage().instance().set(&DataKey::TaskCount, &0u64);
        Ok(())
    }

    pub fn admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn contract_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, CONTRACT_VERSION))
    }

    pub fn upgrade(
        env: Env,
        new_wasm_hash: BytesN<32>,
        new_version: String,
    ) -> Result<(), Error> {
        let admin = require_admin(&env)?;
        let old_version = Self::contract_version(env.clone());
        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());
        env.storage().instance().set(&DataKey::Version, &new_version);
        env.events().publish(
            (symbol_short!("task_str"), symbol_short!("upgraded")),
            (old_version, new_version, new_wasm_hash, admin, env.ledger().sequence()),
        );
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Coordinator management
    // ─────────────────────────────────────────────────────────────────────────

    /// Set (or clear) the coordinator address. Only the admin can call this.
    pub fn set_coordinator(env: Env, coordinator: Option<Address>) -> Result<(), Error> {
        require_admin(&env)?;
        match coordinator {
            Some(ref c) => {
                env.storage().instance().set(&DataKey::Coordinator, c);
            }
            None => {
                env.storage().instance().remove(&DataKey::Coordinator);
            }
        }
        Ok(())
    }

    /// Return the current coordinator address (if any).
    pub fn get_coordinator(env: Env) -> Option<Address> {
        read_coordinator(&env)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // High-level lifecycle API
    // ─────────────────────────────────────────────────────────────────────────

    /// Create a new task on-chain. Any wallet can submit a task; the submitter
    /// must authorise the call via `require_auth()`.
    ///
    /// Returns the newly-assigned numeric `task_id`.
    ///
    /// # State transition
    /// `(none) → Created`
    ///
    /// # Events
    /// Emits `(task_store, task_created)` with [`TaskStoreCreatedEvent`].
    pub fn create_task(
        env: Env,
        submitter: Address,
        description_hash: BytesN<32>,
        budget_xlm: i128,
    ) -> Result<u64, Error> {
        // Auth: the submitter must authorise this call.
        submitter.require_auth();

        // Validate budget (must be > 0 stroops).
        if budget_xlm <= 0 {
            return Err(Error::InvalidBudget);
        }

        // Assign a sequential task_id.
        let task_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCounter)
            .unwrap_or(0u64);
        let next_id = task_id.saturating_add(1);
        env.storage().instance().set(&DataKey::TaskCounter, &next_id);

        // Increment total count for pagination.
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0u64);
        env.storage()
            .instance()
            .set(&DataKey::TaskCount, &count.saturating_add(1));

        let now = env.ledger().timestamp();

        let record = TaskRecord {
            task_id: next_id,
            submitter: submitter.clone(),
            description_hash: description_hash.clone(),
            budget_xlm,
            assigned_agent: None,
            result_hash: None,
            fail_reason: None,
            status: TaskLifecycleStatus::Created,
            created_at: now,
            updated_at: now,
        };

        env.storage().persistent().set(&DataKey::TaskRecord(next_id), &record);
        env.storage().persistent().extend_ttl(
            &DataKey::TaskRecord(next_id),
            TASK_RECORD_TTL_LEDGERS.saturating_sub(1),
            TASK_RECORD_TTL_LEDGERS,
        );

        // Emit event.
        env.events().publish(
            (symbol_short!("task_str"), symbol_short!("ts_created")),
            TaskStoreCreatedEvent {
                version: TASK_LIFECYCLE_EVENT_VERSION,
                task_id: next_id,
                submitter,
                description_hash,
                budget_xlm,
                created_at: now,
            },
        );

        Ok(next_id)
    }

    /// Assign an agent to a task. Only the coordinator may call this.
    ///
    /// # State transition
    /// `Created → Assigned`
    ///
    /// # Events
    /// Emits `(task_store, task_assigned)` with [`TaskStoreAssignedEvent`].
    pub fn assign_task(env: Env, task_id: u64, agent_id: Address) -> Result<(), Error> {
        // Auth: the coordinator must authorise.
        let coordinator = read_coordinator(&env).ok_or(Error::NotInitialized)?;
        coordinator.require_auth();

        let mut record = read_task_record(&env, task_id)?;

        // Only a Created task can be assigned.
        if record.status != TaskLifecycleStatus::Created {
            return Err(Error::InvalidStatusTransition);
        }

        let now = env.ledger().timestamp();
        record.assigned_agent = Some(agent_id.clone());
        record.status = TaskLifecycleStatus::Assigned;
        record.updated_at = now;

        env.storage().persistent().set(&DataKey::TaskRecord(task_id), &record);

        // Emit event.
        env.events().publish(
            (symbol_short!("task_str"), symbol_short!("ts_assigned")),
            TaskStoreAssignedEvent {
                version: TASK_LIFECYCLE_EVENT_VERSION,
                task_id,
                agent_id,
                assigned_at: now,
            },
        );

        Ok(())
    }

    /// Mark a task as completed. Only the agent assigned to the task may call
    /// this. The result hash must be provided.
    ///
    /// # State transition
    /// `Assigned → Completed`
    ///
    /// # Events
    /// Emits `(task_store, task_completed)` with [`TaskStoreCompletedEvent`].
    pub fn complete_task(
        env: Env,
        task_id: u64,
        result_hash: BytesN<32>,
    ) -> Result<(), Error> {
        let mut record = read_task_record(&env, task_id)?;

        // Task must be in Assigned state.
        if record.status != TaskLifecycleStatus::Assigned {
            return Err(Error::InvalidStatusTransition);
        }

        // Auth: the assigned agent must authorise.
        let agent_id = record.assigned_agent.clone().ok_or(Error::TaskNotAssigned)?;
        agent_id.require_auth();

        let now = env.ledger().timestamp();
        record.result_hash = Some(result_hash.clone());
        record.status = TaskLifecycleStatus::Completed;
        record.updated_at = now;

        env.storage().persistent().set(&DataKey::TaskRecord(task_id), &record);

        // Emit event.
        env.events().publish(
            (symbol_short!("task_str"), symbol_short!("ts_cmplt")),
            TaskStoreCompletedEvent {
                version: TASK_LIFECYCLE_EVENT_VERSION,
                task_id,
                agent_id,
                result_hash,
                completed_at: now,
            },
        );

        Ok(())
    }

    /// Mark a task as failed. The assigned agent or the coordinator may call
    /// this. A reason string must be provided.
    ///
    /// # State transition
    /// `Assigned → Failed`  (also accepts `Created → Failed` for early abort by coordinator)
    ///
    /// # Events
    /// Emits `(task_store, task_failed)` with [`TaskStoreFailedEvent`].
    pub fn fail_task(env: Env, task_id: u64, actor: Address, reason: String) -> Result<(), Error> {
        // Auth: actor must authorise.
        actor.require_auth();

        let mut record = read_task_record(&env, task_id)?;

        // Verify caller is either the assigned agent or the coordinator.
        let coordinator_opt = read_coordinator(&env);
        let is_coordinator = coordinator_opt.as_ref().map_or(false, |c| *c == actor);
        let is_assigned_agent = record
            .assigned_agent
            .as_ref()
            .map_or(false, |a| *a == actor);

        if !is_coordinator && !is_assigned_agent {
            return Err(Error::Unauthorized);
        }

        // Cannot fail an already-finalized task.
        if record.status == TaskLifecycleStatus::Completed
            || record.status == TaskLifecycleStatus::Failed
        {
            return Err(Error::TaskAlreadyFinalized);
        }

        let now = env.ledger().timestamp();
        record.fail_reason = Some(reason.clone());
        record.status = TaskLifecycleStatus::Failed;
        record.updated_at = now;

        env.storage().persistent().set(&DataKey::TaskRecord(task_id), &record);

        // Emit event.
        env.events().publish(
            (symbol_short!("task_str"), symbol_short!("ts_failed")),
            TaskStoreFailedEvent {
                version: TASK_LIFECYCLE_EVENT_VERSION,
                task_id,
                actor,
                reason,
                failed_at: now,
            },
        );

        Ok(())
    }

    /// Read a task record. No auth required (public read).
    pub fn get_task(env: Env, task_id: u64) -> Result<TaskRecord, Error> {
        read_task_record(&env, task_id)
    }

    /// Paginated list of task IDs. Returns up to `page_size` task IDs
    /// starting from `offset`. Use in conjunction with `get_task`.
    ///
    /// Returns `(task_ids, total_count)`.
    pub fn list_tasks(env: Env, offset: u64, page_size: u32) -> (Vec<u64>, u64) {
        let total: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCount)
            .unwrap_or(0u64);
        let counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TaskCounter)
            .unwrap_or(0u64);

        let mut ids: Vec<u64> = Vec::new(&env);
        if counter == 0 || offset >= counter {
            return (ids, total);
        }

        // IDs are 1-based sequential; iterate from (offset+1) up to min(offset+page_size, counter).
        let start = offset.saturating_add(1);
        let end = (offset.saturating_add(u64::from(page_size))).min(counter);

        let mut current = start;
        while current <= end {
            // Only include IDs that still have a stored record (not expired).
            if env
                .storage()
                .persistent()
                .has(&DataKey::TaskRecord(current))
            {
                ids.push_back(current);
            }
            current = current.saturating_add(1);
        }

        (ids, total)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Legacy low-level metadata API (preserved for backwards compatibility)
    // ─────────────────────────────────────────────────────────────────────────

    pub fn store_task_metadata(
        env: Env,
        submitter: Address,
        task_id: BytesN<32>,
        prompt_hash: BytesN<32>,
        assigned_agents: Vec<Address>,
        compressed_dag: Bytes,
        ttl_days: u32,
        price_pair: Option<soroban_sdk::Symbol>,
    ) -> Result<(), Error> {
        submitter.require_auth();

        let key = DataKey::Task(task_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyExists);
        }
        if assigned_agents.is_empty() {
            return Err(Error::NoAssignedAgents);
        }
        if has_duplicate_agents(&assigned_agents) {
            return Err(Error::DuplicateAgent);
        }
        if compressed_dag.is_empty() || compressed_dag.len() > MAX_COMPRESSED_DAG_BYTES {
            return Err(Error::InvalidDag);
        }

        let retention_days = if ttl_days == 0 {
            DEFAULT_TTL_DAYS
        } else {
            ttl_days
        };
        if retention_days > MAX_TTL_DAYS {
            return Err(Error::InvalidTtl);
        }

        // ── Oracle price resolution ────────────────────────────────────────────
        let (quoted_price_stroops, resolved_pair) = if let Some(oracle_manager) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::OracleManager)
        {
            // OracleManager is configured: a price_pair is mandatory.
            let pair = price_pair.clone().ok_or(Error::MissingPricePair)?;
            let price = try_resolve_price(&env, &oracle_manager, &pair)
                .ok_or(Error::OraclePriceUnavailable)?;
            (Some(price), Some(pair))
        } else {
            // No OracleManager: pricing is optional (legacy path).
            (None, None)
        };

        let created_at = env.ledger().timestamp();
        let expires_at =
            created_at.saturating_add(u64::from(retention_days).saturating_mul(SECONDS_PER_DAY));

        let metadata = TaskMetadata {
            task_id: task_id.clone(),
            prompt_hash: prompt_hash.clone(),
            assigned_agents,
            compressed_dag,
            status: TaskStatus::Pending,
            created_at,
            expires_at,
            quoted_price_stroops,
            price_pair: resolved_pair,
        };

        env.storage().persistent().set(&key, &metadata);
        let ledgers = ttl_ledgers(retention_days);
        env.storage()
            .persistent()
            .extend_ttl(&key, ledgers.saturating_sub(1), ledgers);

        env.events().publish(
            (symbol_short!("task_meta"), symbol_short!("created")),
            TaskCreatedEvent {
                version: TASK_LIFECYCLE_EVENT_VERSION,
                task_id,
                prompt_hash,
                assigned_agents: metadata.assigned_agents,
                created_at,
                expires_at,
                quoted_price_stroops,
            },
        );

        Ok(())
    }

    pub fn get_task_metadata(env: Env, task_id: BytesN<32>) -> Result<TaskMetadata, Error> {
        read_metadata(&env, &task_id)
    }

    pub fn get_task_status(env: Env, task_id: BytesN<32>) -> Result<TaskStatus, Error> {
        Ok(read_metadata(&env, &task_id)?.status)
    }

    pub fn update_task_status(
        env: Env,
        task_id: BytesN<32>,
        agent: Address,
        new_status: TaskStatus,
    ) -> Result<(), Error> {
        agent.require_auth();

        let key = DataKey::Task(task_id.clone());
        let mut metadata = read_metadata(&env, &task_id)?;
        if !metadata.assigned_agents.contains(&agent) {
            return Err(Error::NotAssignedAgent);
        }
        if !can_transition(metadata.status, new_status) {
            return Err(Error::InvalidStatusTransition);
        }

        let old_status = metadata.status;
        metadata.status = new_status;
        env.storage().persistent().set(&key, &metadata);

        // Every successful transition emits exactly one lifecycle event:
        // terminal transitions (-> Completed / -> Failed) emit `finalized`,
        // everything else emits `updated` — never both.
        let timestamp = env.ledger().timestamp();
        if is_terminal(new_status) {
            env.events().publish(
                (symbol_short!("task_meta"), symbol_short!("finalized")),
                TaskFinalizedEvent {
                    version: TASK_LIFECYCLE_EVENT_VERSION,
                    task_id,
                    agent,
                    old_status,
                    final_status: new_status,
                    finalized_at: timestamp,
                },
            );
        } else {
            env.events().publish(
                (symbol_short!("task_meta"), symbol_short!("updated")),
                TaskUpdatedEvent {
                    version: TASK_LIFECYCLE_EVENT_VERSION,
                    task_id,
                    agent,
                    old_status,
                    new_status,
                    updated_at: timestamp,
                },
            );
        }

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Oracle manager
    // ─────────────────────────────────────────────────────────────────────────

    pub fn set_oracle_manager(env: Env, oracle_manager: Option<Address>) -> Result<(), Error> {
        require_admin(&env)?;
        match oracle_manager {
            Some(ref mgr) => {
                env.storage().instance().set(&DataKey::OracleManager, mgr);
            }
            None => {
                env.storage().instance().remove(&DataKey::OracleManager);
            }
        }

        env.events().publish(
            (symbol_short!("task_str"), symbol_short!("ora_set")),
            OracleManagerSetEvent {
                oracle_manager,
            },
        );

        Ok(())
    }

    pub fn get_oracle_manager(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::OracleManager)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger},
        Address, Bytes, Env, IntoVal,
    };

    struct Fixture {
        env: Env,
        client: TaskStoreContractClient<'static>,
        submitter: Address,
        agent: Address,
        task_id: BytesN<32>,
        prompt_hash: BytesN<32>,
    }

    fn fixture() -> Fixture {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| {
            ledger.timestamp = 1_700_000_000;
            ledger.sequence_number = 100;
        });
        let contract_id = env.register(TaskStoreContract, ());
        let client = TaskStoreContractClient::new(&env, &contract_id);

        Fixture {
            submitter: Address::generate(&env),
            agent: Address::generate(&env),
            task_id: BytesN::from_array(&env, &[1; 32]),
            prompt_hash: BytesN::from_array(&env, &[2; 32]),
            env,
            client,
        }
    }

    fn store(fixture: &Fixture, ttl_days: u32) {
        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        fixture.client.store_task_metadata(
            &fixture.submitter,
            &fixture.task_id,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &ttl_days,
            &None,
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // High-level API: create_task / assign_task / complete_task / fail_task
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn create_task_returns_sequential_id() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let id1 = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        let id2 = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &20_000_000i128);

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn create_task_emits_created_event() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_str"), symbol_short!("ts_created")).into_val(&fixture.env)
        );
    }

    #[test]
    fn create_task_rejects_zero_or_negative_budget() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        assert_eq!(
            fixture
                .client
                .try_create_task(&fixture.submitter, &description_hash, &0i128),
            Err(Ok(Error::InvalidBudget))
        );
        assert_eq!(
            fixture
                .client
                .try_create_task(&fixture.submitter, &description_hash, &-1i128),
            Err(Ok(Error::InvalidBudget))
        );
    }

    #[test]
    fn assign_task_transitions_created_to_assigned() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        fixture.client.assign_task(&task_id, &fixture.agent);

        let record = fixture.client.get_task(&task_id);
        assert_eq!(record.status, TaskLifecycleStatus::Assigned);
        assert_eq!(record.assigned_agent, Some(fixture.agent));
    }

    #[test]
    fn assign_task_emits_assigned_event() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        fixture.env.events().all(); // drain created event
        fixture.client.assign_task(&task_id, &fixture.agent);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_str"), symbol_short!("ts_assigned")).into_val(&fixture.env)
        );
    }

    #[test]
    fn assign_task_rejects_non_created_task() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        // Assign once — success.
        fixture.client.assign_task(&task_id, &fixture.agent);

        // Try to assign again — should fail (already Assigned).
        assert_eq!(
            fixture.client.try_assign_task(&task_id, &fixture.agent),
            Err(Ok(Error::InvalidStatusTransition))
        );
    }

    #[test]
    fn complete_task_transitions_assigned_to_completed() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture.client.assign_task(&task_id, &fixture.agent);

        let result_hash = BytesN::from_array(&fixture.env, &[9u8; 32]);
        fixture.client.complete_task(&task_id, &result_hash);

        let record = fixture.client.get_task(&task_id);
        assert_eq!(record.status, TaskLifecycleStatus::Completed);
        assert_eq!(record.result_hash, Some(result_hash));
    }

    #[test]
    fn complete_task_emits_completed_event() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture.client.assign_task(&task_id, &fixture.agent);
        fixture.env.events().all(); // drain

        let result_hash = BytesN::from_array(&fixture.env, &[9u8; 32]);
        fixture.client.complete_task(&task_id, &result_hash);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_str"), symbol_short!("ts_cmplt")).into_val(&fixture.env)
        );
    }

    #[test]
    fn complete_task_rejects_unassigned_task() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        // Task is still in Created state — completing it must be rejected.
        let result_hash = BytesN::from_array(&fixture.env, &[9u8; 32]);
        assert_eq!(
            fixture.client.try_complete_task(&task_id, &result_hash),
            Err(Ok(Error::InvalidStatusTransition))
        );
    }

    #[test]
    fn fail_task_transitions_assigned_to_failed() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture.client.assign_task(&task_id, &fixture.agent);

        let reason = String::from_str(&fixture.env, "Venice AI unavailable");
        fixture
            .client
            .fail_task(&task_id, &fixture.agent, &reason);

        let record = fixture.client.get_task(&task_id);
        assert_eq!(record.status, TaskLifecycleStatus::Failed);
    }

    #[test]
    fn fail_task_emits_failed_event() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture.client.assign_task(&task_id, &fixture.agent);
        fixture.env.events().all(); // drain

        let reason = String::from_str(&fixture.env, "Timeout");
        fixture.client.fail_task(&task_id, &fixture.agent, &reason);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_str"), symbol_short!("ts_failed")).into_val(&fixture.env)
        );
    }

    #[test]
    fn fail_task_coordinator_can_abort_created_task() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        // Coordinator aborts before assignment.
        let reason = String::from_str(&fixture.env, "No agents available");
        fixture.client.fail_task(&task_id, &coordinator, &reason);

        let record = fixture.client.get_task(&task_id);
        assert_eq!(record.status, TaskLifecycleStatus::Failed);
    }

    #[test]
    fn fail_task_rejects_unauthorized_caller() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture.client.assign_task(&task_id, &fixture.agent);

        // A stranger cannot fail the task.
        let stranger = Address::generate(&fixture.env);
        let reason = String::from_str(&fixture.env, "Unauthorized");
        assert_eq!(
            fixture
                .client
                .try_fail_task(&task_id, &stranger, &reason),
            Err(Ok(Error::Unauthorized))
        );
    }

    #[test]
    fn fail_task_rejects_already_completed_task() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let coordinator = Address::generate(&fixture.env);
        fixture.client.set_coordinator(&Some(coordinator.clone()));

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture.client.assign_task(&task_id, &fixture.agent);

        let result_hash = BytesN::from_array(&fixture.env, &[9u8; 32]);
        fixture.client.complete_task(&task_id, &result_hash);

        // Trying to fail a completed task must be rejected.
        let reason = String::from_str(&fixture.env, "Late failure");
        assert_eq!(
            fixture
                .client
                .try_fail_task(&task_id, &fixture.agent, &reason),
            Err(Ok(Error::TaskAlreadyFinalized))
        );
    }

    #[test]
    fn get_task_returns_stored_record() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        let task_id = fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);

        let record = fixture.client.get_task(&task_id);
        assert_eq!(record.task_id, task_id);
        assert_eq!(record.submitter, fixture.submitter);
        assert_eq!(record.description_hash, description_hash);
        assert_eq!(record.budget_xlm, 10_000_000i128);
        assert_eq!(record.status, TaskLifecycleStatus::Created);
    }

    #[test]
    fn get_task_returns_not_found_for_unknown_id() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        assert_eq!(
            fixture.client.try_get_task(&999u64),
            Err(Ok(Error::NotFound))
        );
    }

    #[test]
    fn list_tasks_returns_paginated_ids() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);

        let description_hash = BytesN::from_array(&fixture.env, &[3u8; 32]);
        fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &10_000_000i128);
        fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &20_000_000i128);
        fixture
            .client
            .create_task(&fixture.submitter, &description_hash, &30_000_000i128);

        // Page 1: first 2 items.
        let (ids, total) = fixture.client.list_tasks(&0u64, &2u32);
        assert_eq!(total, 3);
        assert_eq!(ids.len(), 2);

        // Page 2: last 1 item.
        let (ids2, total2) = fixture.client.list_tasks(&2u64, &2u32);
        assert_eq!(total2, 3);
        assert_eq!(ids2.len(), 1);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Legacy low-level API tests (unchanged)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn stores_and_retrieves_metadata() {
        let fixture = fixture();
        store(&fixture, 0);

        let metadata = fixture.client.get_task_metadata(&fixture.task_id);
        assert_eq!(metadata.task_id, fixture.task_id);
        assert_eq!(metadata.prompt_hash, fixture.prompt_hash);
        assert_eq!(metadata.assigned_agents.get(0), Some(fixture.agent));
        assert_eq!(metadata.status, TaskStatus::Pending);
        assert_eq!(
            metadata.expires_at,
            metadata.created_at + u64::from(DEFAULT_TTL_DAYS) * SECONDS_PER_DAY
        );
        assert_eq!(metadata.quoted_price_stroops, None);
    }

    #[test]
    fn assigned_agent_updates_status() {
        let fixture = fixture();
        store(&fixture, 1);

        fixture
            .client
            .update_task_status(&fixture.task_id, &fixture.agent, &TaskStatus::Running);
        assert_eq!(
            fixture.client.get_task_status(&fixture.task_id),
            TaskStatus::Running
        );
    }

    #[test]
    fn unassigned_agent_cannot_update_status() {
        let fixture = fixture();
        store(&fixture, 1);
        let stranger = Address::generate(&fixture.env);

        let result = fixture.client.try_update_task_status(
            &fixture.task_id,
            &stranger,
            &TaskStatus::Running,
        );
        assert_eq!(result, Err(Ok(Error::NotAssignedAgent)));
    }

    #[test]
    fn rejects_invalid_status_transition() {
        let fixture = fixture();
        store(&fixture, 1);

        let result = fixture.client.try_update_task_status(
            &fixture.task_id,
            &fixture.agent,
            &TaskStatus::Completed,
        );
        assert_eq!(result, Err(Ok(Error::InvalidStatusTransition)));
    }

    #[test]
    fn metadata_expires_after_configured_period() {
        let fixture = fixture();
        store(&fixture, 1);
        fixture.env.ledger().with_mut(|ledger| {
            ledger.timestamp += SECONDS_PER_DAY;
        });

        assert_eq!(
            fixture.client.try_get_task_metadata(&fixture.task_id),
            Err(Ok(Error::Expired))
        );
    }

    #[test]
    fn emits_exactly_one_created_event_on_store() {
        let fixture = fixture();
        store(&fixture, 1);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_meta"), symbol_short!("created")).into_val(&fixture.env)
        );
    }

    #[test]
    fn emits_exactly_one_updated_event_on_non_terminal_transition() {
        let fixture = fixture();
        store(&fixture, 1);

        fixture
            .client
            .update_task_status(&fixture.task_id, &fixture.agent, &TaskStatus::Running);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_meta"), symbol_short!("updated")).into_val(&fixture.env)
        );
    }

    #[test]
    fn emits_exactly_one_finalized_event_on_terminal_transition() {
        let fixture = fixture();
        store(&fixture, 1);
        fixture
            .client
            .update_task_status(&fixture.task_id, &fixture.agent, &TaskStatus::Running);

        fixture
            .client
            .update_task_status(&fixture.task_id, &fixture.agent, &TaskStatus::Completed);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_meta"), symbol_short!("finalized")).into_val(&fixture.env)
        );
    }

    #[test]
    fn finalized_event_fires_for_the_failed_terminal_status_too() {
        let fixture = fixture();
        store(&fixture, 1);

        fixture
            .client
            .update_task_status(&fixture.task_id, &fixture.agent, &TaskStatus::Failed);

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_meta"), symbol_short!("finalized")).into_val(&fixture.env)
        );
    }

    #[test]
    fn created_event_payload_matches_stored_metadata() {
        let fixture = fixture();
        store(&fixture, 1);

        let events = fixture.env.events().all();
        let (_contract_id, _topics, data) = events.get(0).unwrap();
        let payload: TaskCreatedEvent = data.into_val(&fixture.env);

        let metadata = fixture.client.get_task_metadata(&fixture.task_id);

        assert_eq!(payload.version, TASK_LIFECYCLE_EVENT_VERSION);
        assert_eq!(payload.task_id, fixture.task_id);
        assert_eq!(payload.prompt_hash, fixture.prompt_hash);
        assert_eq!(payload.assigned_agents, metadata.assigned_agents);
        assert_eq!(payload.created_at, metadata.created_at);
        assert_eq!(payload.expires_at, metadata.expires_at);
        assert_eq!(payload.quoted_price_stroops, None);
    }

    #[test]
    fn a_rejected_transition_emits_no_lifecycle_event() {
        let fixture = fixture();
        store(&fixture, 1);

        let _ = fixture.client.try_update_task_status(
            &fixture.task_id,
            &fixture.agent,
            &TaskStatus::Completed,
        );

        assert_eq!(fixture.env.events().all().len(), 0);
    }

    #[test]
    fn initialize_sets_admin() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
    }

    #[test]
    fn double_initialize_is_rejected() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        assert_eq!(
            fixture.client.try_initialize(&admin),
            Err(Ok(Error::AlreadyInitialized))
        );
    }

    #[test]
    fn set_oracle_manager_stores_address() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let mgr = Address::generate(&fixture.env);
        fixture.client.set_oracle_manager(&Some(mgr.clone()));
        assert_eq!(fixture.client.get_oracle_manager(), Some(mgr));
    }

    #[test]
    fn set_oracle_manager_none_clears_address() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let mgr = Address::generate(&fixture.env);
        fixture.client.set_oracle_manager(&Some(mgr));
        fixture.client.set_oracle_manager(&None);
        assert_eq!(fixture.client.get_oracle_manager(), None);
    }

    #[test]
    fn set_oracle_manager_emits_event() {
        let fixture = fixture();
        let admin = Address::generate(&fixture.env);
        fixture.client.initialize(&admin);
        let mgr = Address::generate(&fixture.env);
        fixture.client.set_oracle_manager(&Some(mgr));

        let events = fixture.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("task_str"), symbol_short!("ora_set")).into_val(&fixture.env)
        );
    }

    #[test]
    fn no_oracle_manager_means_no_quoted_price() {
        let fixture = fixture();
        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        let pair = soroban_sdk::Symbol::new(&fixture.env, "XLM_USD");

        fixture.client.store_task_metadata(
            &fixture.submitter,
            &fixture.task_id,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &Some(pair),
        );

        let metadata = fixture.client.get_task_metadata(&fixture.task_id);
        assert_eq!(metadata.quoted_price_stroops, None);
    }

    #[test]
    fn fresh_oracle_price_is_stamped_on_task() {
        use oracle_manager::OracleManagerContract;
        use price_oracle::PriceOracleContract;

        let fixture = fixture();

        let oracle_id = fixture.env.register(PriceOracleContract, ());
        let oracle_client = price_oracle::PriceOracleContractClient::new(&fixture.env, &oracle_id);
        let admin_oracle = Address::generate(&fixture.env);
        oracle_client.initialize(&admin_oracle, &3_600u64);
        let now = fixture.env.ledger().timestamp();
        let pair = soroban_sdk::Symbol::new(&fixture.env, "XLM_USD");
        oracle_client.submit_price(&pair, &10_000_000i128, &now);

        let mgr_id = fixture.env.register(OracleManagerContract, ());
        let mgr_client = oracle_manager::OracleManagerContractClient::new(&fixture.env, &mgr_id);
        let admin_mgr = Address::generate(&fixture.env);
        mgr_client.initialize(&admin_mgr);
        mgr_client.set_oracle(&Some(oracle_id));

        let admin_ts = Address::generate(&fixture.env);
        fixture.client.initialize(&admin_ts);
        fixture.client.set_oracle_manager(&Some(mgr_id));

        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        fixture.client.store_task_metadata(
            &fixture.submitter,
            &fixture.task_id,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &Some(pair),
        );

        let metadata = fixture.client.get_task_metadata(&fixture.task_id);
        assert_eq!(metadata.quoted_price_stroops, Some(10_000_000i128));
    }

    #[test]
    fn stale_oracle_with_no_fallback_rejects_task() {
        use oracle_manager::OracleManagerContract;
        use price_oracle::PriceOracleContract;

        let fixture = fixture();

        let oracle_id = fixture.env.register(PriceOracleContract, ());
        let oracle_client = price_oracle::PriceOracleContractClient::new(&fixture.env, &oracle_id);
        let admin_oracle = Address::generate(&fixture.env);
        oracle_client.initialize(&admin_oracle, &3_600u64);
        let now = fixture.env.ledger().timestamp();
        let pair = soroban_sdk::Symbol::new(&fixture.env, "XLM_USD");
        oracle_client.submit_price(&pair, &10_000_000i128, &now);

        fixture.env.ledger().with_mut(|l| {
            l.timestamp = now + 3_601;
        });

        let mgr_id = fixture.env.register(OracleManagerContract, ());
        let mgr_client = oracle_manager::OracleManagerContractClient::new(&fixture.env, &mgr_id);
        let admin_mgr = Address::generate(&fixture.env);
        mgr_client.initialize(&admin_mgr);
        mgr_client.set_oracle(&Some(oracle_id));

        let admin_ts = Address::generate(&fixture.env);
        fixture.client.initialize(&admin_ts);
        fixture.client.set_oracle_manager(&Some(mgr_id));

        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        let result = fixture.client.try_store_task_metadata(
            &fixture.submitter,
            &fixture.task_id,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &Some(pair),
        );

        assert_eq!(result, Err(Ok(Error::OraclePriceUnavailable)));
    }

    #[test]
    fn stale_oracle_with_fallback_uses_fallback_price() {
        use oracle_manager::OracleManagerContract;
        use price_oracle::PriceOracleContract;

        let fixture = fixture();

        let oracle_id = fixture.env.register(PriceOracleContract, ());
        let oracle_client = price_oracle::PriceOracleContractClient::new(&fixture.env, &oracle_id);
        let admin_oracle = Address::generate(&fixture.env);
        oracle_client.initialize(&admin_oracle, &3_600u64);
        let now = fixture.env.ledger().timestamp();
        let pair = soroban_sdk::Symbol::new(&fixture.env, "XLM_USD");
        oracle_client.submit_price(&pair, &10_000_000i128, &now);

        fixture.env.ledger().with_mut(|l| {
            l.timestamp = now + 3_601;
        });

        let mgr_id = fixture.env.register(OracleManagerContract, ());
        let mgr_client = oracle_manager::OracleManagerContractClient::new(&fixture.env, &mgr_id);
        let admin_mgr = Address::generate(&fixture.env);
        mgr_client.initialize(&admin_mgr);
        mgr_client.set_oracle(&Some(oracle_id));
        mgr_client.set_fallback_price(&pair, &8_000_000i128);

        let admin_ts = Address::generate(&fixture.env);
        fixture.client.initialize(&admin_ts);
        fixture.client.set_oracle_manager(&Some(mgr_id));

        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        fixture.client.store_task_metadata(
            &fixture.submitter,
            &fixture.task_id,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &Some(pair),
        );

        let metadata = fixture.client.get_task_metadata(&fixture.task_id);
        assert_eq!(metadata.quoted_price_stroops, Some(8_000_000i128));
    }

    #[test]
    fn oracle_configured_but_no_pair_supplied_returns_error() {
        let fixture = fixture();

        let admin_ts = Address::generate(&fixture.env);
        fixture.client.initialize(&admin_ts);
        let mgr = Address::generate(&fixture.env);
        fixture.client.set_oracle_manager(&Some(mgr));

        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        let result = fixture.client.try_store_task_metadata(
            &fixture.submitter,
            &fixture.task_id,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &None,
        );

        assert_eq!(result, Err(Ok(Error::MissingPricePair)));
    }

    #[test]
    fn oracle_switching_uses_new_oracle_manager() {
        use oracle_manager::OracleManagerContract;
        use price_oracle::PriceOracleContract;

        let fixture = fixture();

        let oracle_a = fixture.env.register(PriceOracleContract, ());
        let client_a = price_oracle::PriceOracleContractClient::new(&fixture.env, &oracle_a);
        client_a.initialize(&Address::generate(&fixture.env), &3_600u64);
        let now = fixture.env.ledger().timestamp();
        let pair = soroban_sdk::Symbol::new(&fixture.env, "XLM_USD");
        client_a.submit_price(&pair, &10_000_000i128, &now);

        let mgr_a = fixture.env.register(OracleManagerContract, ());
        let mgr_a_client = oracle_manager::OracleManagerContractClient::new(&fixture.env, &mgr_a);
        mgr_a_client.initialize(&Address::generate(&fixture.env));
        mgr_a_client.set_oracle(&Some(oracle_a));

        let oracle_b = fixture.env.register(PriceOracleContract, ());
        let client_b = price_oracle::PriceOracleContractClient::new(&fixture.env, &oracle_b);
        client_b.initialize(&Address::generate(&fixture.env), &3_600u64);
        client_b.submit_price(&pair, &20_000_000i128, &now);

        let mgr_b = fixture.env.register(OracleManagerContract, ());
        let mgr_b_client = oracle_manager::OracleManagerContractClient::new(&fixture.env, &mgr_b);
        mgr_b_client.initialize(&Address::generate(&fixture.env));
        mgr_b_client.set_oracle(&Some(oracle_b));

        let admin_ts = Address::generate(&fixture.env);
        fixture.client.initialize(&admin_ts);

        fixture.client.set_oracle_manager(&Some(mgr_a));

        let agents = Vec::from_array(&fixture.env, [fixture.agent.clone()]);
        let dag = Bytes::from_slice(&fixture.env, &[0x78, 0x9c, 0x03, 0x00]);
        let task_a = BytesN::from_array(&fixture.env, &[1; 32]);
        fixture.client.store_task_metadata(
            &fixture.submitter,
            &task_a,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &Some(pair.clone()),
        );
        assert_eq!(
            fixture
                .client
                .get_task_metadata(&task_a)
                .quoted_price_stroops,
            Some(10_000_000i128)
        );

        fixture.client.set_oracle_manager(&Some(mgr_b));

        let task_b = BytesN::from_array(&fixture.env, &[2; 32]);
        fixture.client.store_task_metadata(
            &fixture.submitter,
            &task_b,
            &fixture.prompt_hash,
            &agents,
            &dag,
            &1u32,
            &Some(pair),
        );
        assert_eq!(
            fixture
                .client
                .get_task_metadata(&task_b)
                .quoted_price_stroops,
            Some(20_000_000i128)
        );
    }
}
