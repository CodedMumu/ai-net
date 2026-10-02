#![no_std]

//! # Payment Escrow Contract
//!
//! Holds XLM funds in escrow until a task is completed, disputed, or expired.
//!
//! ## State machine
//!
//! ```text
//!  create_escrow
//!       │
//!       ▼
//!    Active ──────────────────► Released  (coordinator calls release_escrow)
//!       │
//!       ├──────────────────────► Disputed  (coordinator or agent calls dispute_escrow)
//!       │                           │
//!       │                    (dispute_resolution
//!       │                     contract invoked after
//!       │                     48-ledger hold window)
//!       │
//!       └──────────────────────► Expired   (anyone calls expire_escrow after timeout_ledger)
//! ```
//!
//! ## Re-entrancy protection
//!
//! State is written to storage **before** any external calls or event emissions
//! so an unexpected re-entry cannot observe stale state.
//!
//! ## Storage model
//!
//! Each escrow is stored in **Temporary** storage keyed by `DataKey::Escrow(task_id)`.
//! Soroban auto-expires the entry when the TTL lapses, providing a natural
//! cleanup path for tasks that are abandoned without an explicit `expire_escrow`
//! call. The TTL is set to `timeout_ledger` at creation time.

mod errors;
mod types;

pub use errors::Error;
pub use types::*;

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

/// Number of ledgers the escrow is held in Disputed state before the
/// dispute_resolution contract may be invoked.  At ~5 s/ledger this is ≈4 min.
pub const DISPUTE_HOLD_LEDGERS: u32 = 48;

/// Minimum ledgers the temporary entry must still live after a TTL bump.
pub const TTL_THRESHOLD: u32 = 50_000;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Contract administrator.
    Admin,
    /// Escrow record keyed by task_id.
    Escrow(Symbol),
}

#[contract]
pub struct PaymentEscrowContract;

fn require_admin(env: &Env) -> Result<Address, Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Unauthorized)?;
    admin.require_auth();
    Ok(admin)
}

#[contractimpl]
impl PaymentEscrowContract {
    /// Initialize the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    /// Read the current admin address.
    pub fn admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    /// Lock funds in escrow for a task.
    ///
    /// - `coordinator`    — the address paying into escrow; must authorise.
    /// - `task_id`        — unique identifier for the task (matches task_store).
    /// - `agent`          — the agent that will perform the task.
    /// - `amount`         — XLM amount in stroops to hold.
    /// - `timeout_ledger` — ledger sequence at which the escrow auto-expires.
    ///
    /// The record is stored in Temporary storage; Soroban automatically removes
    /// it when the TTL lapses.
    pub fn create_escrow(
        env: Env,
        coordinator: Address,
        task_id: Symbol,
        agent: Address,
        amount: i128,
        timeout_ledger: u32,
    ) -> Result<(), Error> {
        coordinator.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let current_ledger = env.ledger().sequence();
        if timeout_ledger <= current_ledger {
            return Err(Error::InvalidTimeout);
        }

        let key = DataKey::Escrow(task_id.clone());
        if env.storage().temporary().has(&key) {
            return Err(Error::AlreadyExists);
        }

        // Compute TTL: how many ledgers from now until timeout.
        let ttl = timeout_ledger.saturating_sub(current_ledger);

        let record = EscrowRecord {
            task_id: task_id.clone(),
            coordinator: coordinator.clone(),
            agent: agent.clone(),
            amount,
            timeout_ledger,
            state: EscrowState::Active,
            dispute_hold_until: None,
        };

        // Write state before emitting events (re-entrancy protection).
        env.storage().temporary().set(&key, &record);
        env.storage()
            .temporary()
            .extend_ttl(&key, ttl.saturating_sub(1), ttl);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("created")),
            EscrowCreatedEvent {
                task_id,
                coordinator,
                agent,
                amount,
                timeout_ledger,
            },
        );

        Ok(())
    }

    /// Release escrowed funds to the agent after task completion.
    ///
    /// Only the coordinator may call this.  The escrow must be in `Active` state.
    pub fn release_escrow(env: Env, coordinator: Address, task_id: Symbol) -> Result<(), Error> {
        coordinator.require_auth();

        let key = DataKey::Escrow(task_id.clone());
        let mut record: EscrowRecord = env
            .storage()
            .temporary()
            .get(&key)
            .ok_or(Error::NotFound)?;

        if record.coordinator != coordinator {
            return Err(Error::Unauthorized);
        }

        match record.state {
            EscrowState::Active => {}
            EscrowState::Released => return Err(Error::AlreadyReleased),
            EscrowState::Disputed => return Err(Error::EscrowDisputed),
            EscrowState::Expired => return Err(Error::EscrowExpired),
        }

        // Update state before event (re-entrancy protection).
        record.state = EscrowState::Released;
        env.storage().temporary().set(&key, &record);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("released")),
            EscrowReleasedEvent {
                task_id,
                agent: record.agent,
                amount: record.amount,
            },
        );

        Ok(())
    }

    /// Raise a dispute on an escrow.
    ///
    /// Either the coordinator or the agent may call this.  Triggers a
    /// 48-ledger hold window after which the dispute_resolution contract
    /// should be invoked externally.
    pub fn dispute_escrow(env: Env, caller: Address, task_id: Symbol) -> Result<(), Error> {
        caller.require_auth();

        let key = DataKey::Escrow(task_id.clone());
        let mut record: EscrowRecord = env
            .storage()
            .temporary()
            .get(&key)
            .ok_or(Error::NotFound)?;

        // Only the coordinator or the agent may dispute.
        if caller != record.coordinator && caller != record.agent {
            return Err(Error::Unauthorized);
        }

        match record.state {
            EscrowState::Active => {}
            EscrowState::Released => return Err(Error::AlreadyReleased),
            EscrowState::Disputed => return Err(Error::AlreadyDisputed),
            EscrowState::Expired => return Err(Error::EscrowExpired),
        }

        let hold_until = env.ledger().sequence() + DISPUTE_HOLD_LEDGERS;

        // Update state before event (re-entrancy protection).
        record.state = EscrowState::Disputed;
        record.dispute_hold_until = Some(hold_until);
        env.storage().temporary().set(&key, &record);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("disputed")),
            EscrowDisputedEvent {
                task_id,
                caller,
                dispute_hold_until: hold_until,
            },
        );

        Ok(())
    }

    /// Expire an escrow and return funds to the coordinator.
    ///
    /// Permissionless: anyone may call this once `timeout_ledger` has been
    /// reached.  The escrow must still be in `Active` state (disputed escrows
    /// are handled by the dispute_resolution contract instead).
    pub fn expire_escrow(env: Env, task_id: Symbol) -> Result<(), Error> {
        let key = DataKey::Escrow(task_id.clone());
        let mut record: EscrowRecord = env
            .storage()
            .temporary()
            .get(&key)
            .ok_or(Error::NotFound)?;

        let current_ledger = env.ledger().sequence();

        if current_ledger < record.timeout_ledger {
            return Err(Error::NotYetExpired);
        }

        match record.state {
            EscrowState::Active => {}
            EscrowState::Released => return Err(Error::AlreadyReleased),
            EscrowState::Disputed => return Err(Error::EscrowDisputed),
            EscrowState::Expired => return Err(Error::AlreadyExpired),
        }

        // Update state before event (re-entrancy protection).
        record.state = EscrowState::Expired;
        env.storage().temporary().set(&key, &record);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("expired")),
            EscrowExpiredEvent {
                task_id,
                coordinator: record.coordinator,
                amount: record.amount,
            },
        );

        Ok(())
    }

    /// Read the escrow record for a task without modifying it.
    pub fn get_escrow(env: Env, task_id: Symbol) -> Option<EscrowRecord> {
        env.storage()
            .temporary()
            .get(&DataKey::Escrow(task_id))
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test;
