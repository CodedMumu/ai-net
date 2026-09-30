use soroban_sdk::{contracterror, contracttype, Address, Bytes, BytesN, Symbol, Vec};

pub const DEFAULT_TTL_DAYS: u32 = 14;
pub const MAX_TTL_DAYS: u32 = 30;
pub const MAX_COMPRESSED_DAG_BYTES: u32 = 4 * 1024;
pub const LEDGERS_PER_DAY: u32 = 17_280;

/// Schema version stamped on every task lifecycle event payload (see
/// `docs/TASK_LIFECYCLE_EVENTS.md`). Bump only for a breaking payload
/// change; additive fields do not require a bump.
pub const TASK_LIFECYCLE_EVENT_VERSION: u32 = 1;

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TaskStatus {
    Pending = 0,
    Running = 1,
    Completed = 2,
    Failed = 3,
    /// Task lifecycle state added for the create_task/assign_task/complete_task API.
    Created = 4,
    Assigned = 5,
}

/// Stored state for a single task.
///
/// `quoted_price_stroops` is `None` when no OracleManager is configured at the
/// time the task was submitted.  When an OracleManager _is_ configured and
/// successfully returns a price, the value is stamped here at creation time so
/// that the agreed price is immutable for the lifetime of the task.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskMetadata {
    pub task_id: BytesN<32>,
    pub prompt_hash: BytesN<32>,
    pub assigned_agents: Vec<Address>,
    pub compressed_dag: Bytes,
    pub status: TaskStatus,
    pub created_at: u64,
    pub expires_at: u64,
    /// Oracle-quoted price in stroops, stamped at creation time.
    /// `None` if no OracleManager is configured.
    pub quoted_price_stroops: Option<i128>,
    /// Asset pair used to fetch the quoted price (e.g. `XLM_USD`).
    /// `None` when `quoted_price_stroops` is `None`.
    pub price_pair: Option<Symbol>,
}

/// Simplified task record used by the create_task/assign_task/complete_task/fail_task
/// high-level lifecycle API. Stored under DataKey::TaskRecord.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskRecord {
    /// Auto-generated task ID (sequential counter-based, stored as BytesN<32>).
    pub task_id: u64,
    /// Submitter who created the task.
    pub submitter: Address,
    /// SHA-256 hash of the task description.
    pub description_hash: BytesN<32>,
    /// Budget in XLM stroops (1 XLM = 10_000_000 stroops).
    pub budget_xlm: i128,
    /// Coordinator-assigned agent (set during assign_task).
    pub assigned_agent: Option<Address>,
    /// SHA-256 hash of the result (set during complete_task).
    pub result_hash: Option<BytesN<32>>,
    /// Failure reason (set during fail_task).
    pub fail_reason: Option<soroban_sdk::String>,
    /// Current task status.
    pub status: TaskLifecycleStatus,
    /// Ledger timestamp at creation.
    pub created_at: u64,
    /// Ledger timestamp of last status update.
    pub updated_at: u64,
}

/// Three-state lifecycle for the create_task/assign_task/complete_task API.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TaskLifecycleStatus {
    Created = 0,
    Assigned = 1,
    Completed = 2,
    Failed = 3,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Version,
    /// Task metadata for the lower-level store_task_metadata API.
    Task(BytesN<32>),
    /// Optional OracleManager contract address used to resolve quoted prices.
    OracleManager,
    /// Coordinator address — the only address permitted to call assign_task.
    Coordinator,
    /// High-level task record for the create_task/assign_task/complete_task API.
    TaskRecord(u64),
    /// Auto-incrementing counter for task IDs.
    TaskCounter,
    /// Index: total number of tasks (for pagination).
    TaskCount,
}

/// Emitted exactly once per successful `store_task_metadata` call, under
/// topics `(task_meta, created)`. See `docs/TASK_LIFECYCLE_EVENTS.md`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskCreatedEvent {
    pub version: u32,
    pub task_id: BytesN<32>,
    pub prompt_hash: BytesN<32>,
    pub assigned_agents: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64,
    /// Oracle-quoted price in stroops at the moment of task creation.
    /// `None` means no oracle was configured.
    pub quoted_price_stroops: Option<i128>,
}

/// Emitted for a successful non-terminal status transition (currently only
/// `Pending -> Running`), under topics `(task_meta, updated)`. Terminal
/// transitions (`-> Completed` / `-> Failed`) emit [`TaskFinalizedEvent`]
/// instead — never both — so each transition emits exactly one lifecycle
/// event. See `docs/TASK_LIFECYCLE_EVENTS.md`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskUpdatedEvent {
    pub version: u32,
    pub task_id: BytesN<32>,
    pub agent: Address,
    pub old_status: TaskStatus,
    pub new_status: TaskStatus,
    pub updated_at: u64,
}

/// Emitted for a successful transition into a terminal status (`Completed`
/// or `Failed`), under topics `(task_meta, finalized)`. `final_status` is
/// always one of those two values. See `docs/TASK_LIFECYCLE_EVENTS.md`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskFinalizedEvent {
    pub version: u32,
    pub task_id: BytesN<32>,
    pub agent: Address,
    pub old_status: TaskStatus,
    pub final_status: TaskStatus,
    pub finalized_at: u64,
}

/// Emitted when the admin sets a new OracleManager address.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct OracleManagerSetEvent {
    /// The new OracleManager contract address; `None` means it was cleared.
    pub oracle_manager: Option<Address>,
}

/// Emitted on create_task (high-level API), topics `(task_store, task_created)`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TaskStoreCreatedEvent {
    pub version: u32,
    pub task_id: u64,
    pub submitter: Address,
    pub description_hash: BytesN<32>,
    pub budget_xlm: i128,
    pub created_at: u64,
}

/// Emitted on assign_task, topics `(task_store, task_assigned)`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TaskStoreAssignedEvent {
    pub version: u32,
    pub task_id: u64,
    pub agent_id: Address,
    pub assigned_at: u64,
}

/// Emitted on complete_task, topics `(task_store, task_completed)`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TaskStoreCompletedEvent {
    pub version: u32,
    pub task_id: u64,
    pub agent_id: Address,
    pub result_hash: BytesN<32>,
    pub completed_at: u64,
}

/// Emitted on fail_task, topics `(task_store, task_failed)`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct TaskStoreFailedEvent {
    pub version: u32,
    pub task_id: u64,
    pub actor: Address,
    pub reason: soroban_sdk::String,
    pub failed_at: u64,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    AlreadyExists = 2,
    NoAssignedAgents = 3,
    DuplicateAgent = 4,
    InvalidDag = 5,
    InvalidTtl = 6,
    NotAssignedAgent = 7,
    InvalidStatusTransition = 8,
    Expired = 9,
    AlreadyInitialized = 10,
    NotInitialized = 11,
    Unauthorized = 12,
    UpgradeFailed = 13,
    InvalidBudget = 14,
    NotCoordinator = 15,
    TaskNotAssigned = 16,
    TaskAlreadyFinalized = 17,
    MissingPricePair = 18,
    OraclePriceUnavailable = 19,
}
