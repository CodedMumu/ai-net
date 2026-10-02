# Task Store Contract — Lifecycle Events

This document details the Soroban events emitted by the `task_store` smart
contract. These events give off-chain indexers and the UI a consistent,
versioned signal for every task lifecycle transition, without needing to
poll `get_task_metadata`/`get_task_status`.

## Standardised Event Format

All ai-net contract events follow a uniform envelope:

```
{ contract_id, event_type, version, payload }
```

| Field         | Type   | Description                                                     |
|---------------|--------|-----------------------------------------------------------------|
| `contract_id` | string | Soroban contract address (C…)                                   |
| `event_type`  | string | `"<namespace>:<event_name>"` (e.g. `"task_meta:created"`)       |
| `version`     | u32    | Payload schema version; increment on breaking changes           |
| `payload`     | struct | Event-specific payload (see per-event tables below)             |

All contracts publish events as `(topic_pair, data_struct)`:
- **Topic 1** — contract namespace symbol (e.g. `task_meta`, `registry`)
- **Topic 2** — event name symbol (e.g. `created`, `agent_reg`)
- **Data** — a `#[contracttype]` struct carrying the versioned payload

## Event Schema by Contract

### task_store contract

| event_type             | Topics                          | Version |
|------------------------|---------------------------------|---------|
| `task_meta:created`    | `(task_meta, created)`          | 1       |
| `task_meta:updated`    | `(task_meta, updated)`          | 1       |
| `task_meta:finalized`  | `(task_meta, finalized)`        | 1       |

### agent_registry contract

| event_type                | Topics                          | Version |
|---------------------------|---------------------------------|---------|
| `registry:init`           | `(registry, init)`              | 1       |
| `registry:agent_reg`      | `(registry, agent_reg)`         | 1       |
| `registry:agent_drg`      | `(registry, agent_drg)`         | 1       |
| `registry:price_upd`      | `(registry, price_upd)`         | 1       |
| `registry:bond_lck`       | `(registry, bond_lck)`          | 1       |
| `registry:bond_slsh`      | `(registry, bond_slsh)`         | 1       |
| `registry:bond_ret`       | `(registry, bond_ret)`          | 1       |
| `registry:err_rptd`       | `(registry, err_rptd)`          | 1       |
| `registry:err_rslvd`      | `(registry, err_rslvd)`         | 1       |
| `registry:adm_chngd`      | `(registry, adm_chngd)`         | 1       |
| `registry:op_prop`        | `(registry, op_prop)`           | 1       |
| `registry:op_appr`        | `(registry, op_appr)`           | 1       |
| `registry:op_exec`        | `(registry, op_exec)`           | 1       |
| `registry:op_canc`        | `(registry, op_canc)`           | 1       |

### platform_config contract

| event_type              | Topics                          | Version |
|-------------------------|---------------------------------|---------|
| `plt_cfg:cfg_upd`       | `(plt_cfg, cfg_upd)`            | 1       |

### multisig_governance contract

| event_type              | Topics                          | Version |
|-------------------------|---------------------------------|---------|
| `gov_ms:proposed`       | `(gov_ms, proposed)`            | 1       |
| `gov_ms:approved`       | `(gov_ms, approved)`            | 1       |
| `gov_ms:executed`       | `(gov_ms, executed)`            | 1       |
| `gov_ms:cancelled`      | `(gov_ms, cancelled)`           | 1       |

### reputation contract

| event_type              | Topics                          | Version |
|-------------------------|---------------------------------|---------|
| `reputat:outcome`       | `(reputat, outcome)`            | 1       |
| `reputat:decayed`       | `(reputat, decayed)`            | 1       |

---

## Event Versioning & Compatibility

Every payload carries a `version: u32` field (currently `1`, the
`TASK_LIFECYCLE_EVENT_VERSION` constant in
`contracts/task_store/src/types.rs`). The schema is **append-only**:

- Adding a new field to an existing payload does **not** require a version
  bump — consumers should tolerate unknown/new fields.
- Removing, renaming, or changing the type/meaning of an existing field
  **does** require a version bump, so existing consumers can detect the
  change (by branching on `version`) instead of silently misreading data.
- The topic pair for a given lifecycle stage (e.g. `(task_meta, created)`)
  is stable; a schema-breaking change bumps `version` in the payload, it
  does not introduce a new topic.

## Invariant: exactly one event per transition

Every successful call to `store_task_metadata` emits exactly one `created`
event. Every successful call to `update_task_status` emits exactly one
event — `updated` for a non-terminal transition, or `finalized` for a
transition into a terminal status — never both, and never zero. A call
that is rejected (unauthorized agent, invalid transition, expired task)
emits no lifecycle event at all, since it errors out before any state
change or publish.

---

### 1. Task Created

Emitted once, when `store_task_metadata` succeeds.

- **Topic 1**: `Symbol::new(env, "task_meta")`
- **Topic 2**: `Symbol::new(env, "created")`
- **Data (Structure)**: `TaskCreatedEvent`
  ```rust
  pub struct TaskCreatedEvent {
      pub version: u32,
      pub task_id: BytesN<32>,
      pub prompt_hash: BytesN<32>,
      pub assigned_agents: Vec<Address>,
      pub created_at: u64,
      pub expires_at: u64,
  }
  ```

### 2. Task Updated

Emitted when `update_task_status` succeeds with a **non-terminal**
transition. Today the only non-terminal transition is `Pending -> Running`;
`Pending -> Failed` is terminal and emits `finalized` instead (see below).

- **Topic 1**: `Symbol::new(env, "task_meta")`
- **Topic 2**: `Symbol::new(env, "updated")`
- **Data (Structure)**: `TaskUpdatedEvent`
  ```rust
  pub struct TaskUpdatedEvent {
      pub version: u32,
      pub task_id: BytesN<32>,
      pub agent: Address,
      pub old_status: TaskStatus,
      pub new_status: TaskStatus,
      pub updated_at: u64,
  }
  ```

### 3. Task Finalized

Emitted when `update_task_status` succeeds with a transition **into a
terminal status** — `-> Completed` or `-> Failed`. `final_status` is
always one of those two values; `old_status` records what it transitioned
from.

- **Topic 1**: `Symbol::new(env, "task_meta")`
- **Topic 2**: `Symbol::new(env, "finalized")`
- **Data (Structure)**: `TaskFinalizedEvent`
  ```rust
  pub struct TaskFinalizedEvent {
      pub version: u32,
      pub task_id: BytesN<32>,
      pub agent: Address,
      pub old_status: TaskStatus,
      pub final_status: TaskStatus,
      pub finalized_at: u64,
  }
  ```

## Status transition → event map

| Transition | Event |
|---|---|
| (none) → `store_task_metadata` succeeds | `created` |
| `Pending` → `Running` | `updated` |
| `Running` → `Completed` | `finalized` |
| `Pending` → `Failed` | `finalized` |
| `Running` → `Failed` | `finalized` |
| Any other transition (rejected — `InvalidStatusTransition`) | *(no event)* |

## Reading events with the JS/TS SDK

The contract's generated bindings (`smart-contracts/src/`) expose the
event payload types once regenerated from the built Wasm. Off-chain code
should filter by topic pair before decoding, e.g.:

```ts
if (topics[0] === "task_meta" && topics[1] === "finalized") {
  const event = scValToNative(data) as TaskFinalizedEvent;
  // event.version, event.final_status, ...
}
```

## Backend Event Indexer

The backend runs a `ContractEventIndexer` service
(`backend/src/services/eventIndexer.ts`) that polls the Stellar Horizon
API every 3 seconds and stores indexed events in `events.db`.

### Query API

```
GET /api/events?contract=<id>&type=<event_type>&from=<ISO>&to=<ISO>&limit=50&offset=0
```

| Parameter  | Type   | Description                                              |
|------------|--------|----------------------------------------------------------|
| `contract` | string | Filter by Soroban contract ID                            |
| `type`     | string | Filter by event type (e.g. `"task_meta:created"`)         |
| `from`     | string | ISO-8601 lower bound on `occurred_at` (inclusive)        |
| `to`       | string | ISO-8601 upper bound on `occurred_at` (inclusive)        |
| `limit`    | int    | Max records per page (1–200, default 50)                 |
| `offset`   | int    | Pagination offset (default 0)                            |

### Indexer Status

```
GET /api/events/status
```

Returns `{ running, lastIndexedLedger, eventsIndexed, lastPollAt, lastError }`.


## Versioning & Compatibility

Every payload carries a `version: u32` field (currently `1`, the
`TASK_LIFECYCLE_EVENT_VERSION` constant in
`contracts/task_store/src/types.rs`). The schema is **append-only**:

- Adding a new field to an existing payload does **not** require a version
  bump — consumers should tolerate unknown/new fields.
- Removing, renaming, or changing the type/meaning of an existing field
  **does** require a version bump, so existing consumers can detect the
  change (by branching on `version`) instead of silently misreading data.
- The topic pair for a given lifecycle stage (e.g. `(task_meta, created)`)
  is stable; a schema-breaking change bumps `version` in the payload, it
  does not introduce a new topic.

## Invariant: exactly one event per transition

Every successful call to `store_task_metadata` emits exactly one `created`
event. Every successful call to `update_task_status` emits exactly one
event — `updated` for a non-terminal transition, or `finalized` for a
transition into a terminal status — never both, and never zero. A call
that is rejected (unauthorized agent, invalid transition, expired task)
emits no lifecycle event at all, since it errors out before any state
change or publish.

---

### 1. Task Created

Emitted once, when `store_task_metadata` succeeds.

- **Topic 1**: `Symbol::new(env, "task_meta")`
- **Topic 2**: `Symbol::new(env, "created")`
- **Data (Structure)**: `TaskCreatedEvent`
  ```rust
  pub struct TaskCreatedEvent {
      pub version: u32,
      pub task_id: BytesN<32>,
      pub prompt_hash: BytesN<32>,
      pub assigned_agents: Vec<Address>,
      pub created_at: u64,
      pub expires_at: u64,
  }
  ```

### 2. Task Updated

Emitted when `update_task_status` succeeds with a **non-terminal**
transition. Today the only non-terminal transition is `Pending -> Running`;
`Pending -> Failed` is terminal and emits `finalized` instead (see below).

- **Topic 1**: `Symbol::new(env, "task_meta")`
- **Topic 2**: `Symbol::new(env, "updated")`
- **Data (Structure)**: `TaskUpdatedEvent`
  ```rust
  pub struct TaskUpdatedEvent {
      pub version: u32,
      pub task_id: BytesN<32>,
      pub agent: Address,
      pub old_status: TaskStatus,
      pub new_status: TaskStatus,
      pub updated_at: u64,
  }
  ```

### 3. Task Finalized

Emitted when `update_task_status` succeeds with a transition **into a
terminal status** — `-> Completed` or `-> Failed`. `final_status` is
always one of those two values; `old_status` records what it transitioned
from.

- **Topic 1**: `Symbol::new(env, "task_meta")`
- **Topic 2**: `Symbol::new(env, "finalized")`
- **Data (Structure)**: `TaskFinalizedEvent`
  ```rust
  pub struct TaskFinalizedEvent {
      pub version: u32,
      pub task_id: BytesN<32>,
      pub agent: Address,
      pub old_status: TaskStatus,
      pub final_status: TaskStatus,
      pub finalized_at: u64,
  }
  ```

## Status transition → event map

| Transition | Event |
|---|---|
| (none) → `store_task_metadata` succeeds | `created` |
| `Pending` → `Running` | `updated` |
| `Running` → `Completed` | `finalized` |
| `Pending` → `Failed` | `finalized` |
| `Running` → `Failed` | `finalized` |
| Any other transition (rejected — `InvalidStatusTransition`) | *(no event)* |

## Reading events with the JS/TS SDK

The contract's generated bindings (`smart-contracts/src/`) expose the
event payload types once regenerated from the built Wasm. Off-chain code
should filter by topic pair before decoding, e.g.:

```ts
if (topics[0] === "task_meta" && topics[1] === "finalized") {
  const event = scValToNative(data) as TaskFinalizedEvent;
  // event.version, event.final_status, ...
}
```
