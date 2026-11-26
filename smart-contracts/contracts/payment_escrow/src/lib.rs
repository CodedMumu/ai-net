#![no_std]

//! # Payment Escrow Contract
//!
//! Holds user funds locked against task completion.
//!
//! ## Emergency Pause
//!
//! The multisig_governance contract address is stored in Instance storage at
//! initialization. Only that address may call `pause()` or `unpause()`.
//!
//! When paused:
//! - `lock()` returns `Error::ContractPaused` — no new funds can be locked.
//! - `release()` and `refund()` continue to work — existing fund holders are
//!   never blocked from recovering their funds.
//!
//! ## Authorization
//!
//! Every on-chain mutation performs strict `require_auth()` per AGENTS.md.

mod errors;

pub use errors::Error;

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

// ── Storage Keys ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Singleton: address of the multisig_governance contract.
    Governance,
    /// Singleton: emergency pause flag.
    Paused,
    /// Per-task escrow record keyed by task_id symbol.
    Escrow(Symbol),
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// Status of an escrow record.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    /// Funds are locked and awaiting settlement.
    Locked,
    /// Funds have been released to the agent.
    Released,
    /// Funds have been refunded to the submitter.
    Refunded,
}

/// A single escrow record.
#[contracttype]
#[derive(Clone)]
pub struct EscrowRecord {
    pub task_id: Symbol,
    pub submitter: Address,
    pub agent: Address,
    pub amount: i128,
    pub status: EscrowStatus,
}

// ── Events ────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct PauseActivatedEvent {
    pub caller: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct PauseLiftedEvent {
    pub caller: Address,
}

#[contracttype]
#[derive(Clone)]
pub struct FundsLockedEvent {
    pub task_id: Symbol,
    pub submitter: Address,
    pub agent: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct FundsReleasedEvent {
    pub task_id: Symbol,
    pub agent: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct FundsRefundedEvent {
    pub task_id: Symbol,
    pub submitter: Address,
    pub amount: i128,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_governance(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Governance)
        .ok_or(Error::NotInitialized)
}

fn require_governance(env: &Env) -> Result<Address, Error> {
    let gov = read_governance(env)?;
    gov.require_auth();
    Ok(gov)
}

fn require_not_paused(env: &Env) -> Result<(), Error> {
    let paused: bool = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    if paused {
        return Err(Error::ContractPaused);
    }
    Ok(())
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct PaymentEscrowContract;

#[contractimpl]
impl PaymentEscrowContract {
    /// Initialize the contract.
    ///
    /// `governance` is the multisig_governance contract address. Only this
    /// address may call `pause()` and `unpause()`.
    pub fn initialize(env: Env, governance: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Governance) {
            return Err(Error::AlreadyInitialized);
        }
        governance.require_auth();
        env.storage()
            .instance()
            .set(&DataKey::Governance, &governance);
        env.storage().instance().set(&DataKey::Paused, &false);
        Ok(())
    }

    /// Pause the contract. Only callable by the multisig_governance address.
    ///
    /// Emits `PauseActivated`.
    pub fn pause(env: Env) -> Result<(), Error> {
        let caller = require_governance(&env)?;
        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("paused")),
            PauseActivatedEvent {
                caller: caller.clone(),
            },
        );
        Ok(())
    }

    /// Unpause the contract. Only callable by the multisig_governance address.
    ///
    /// Emits `PauseLifted`.
    pub fn unpause(env: Env) -> Result<(), Error> {
        let caller = require_governance(&env)?;
        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("unpaused")),
            PauseLiftedEvent {
                caller: caller.clone(),
            },
        );
        Ok(())
    }

    /// Query the current pause state.
    pub fn get_pause_state(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Lock funds in escrow for a task.
    ///
    /// Returns `Error::ContractPaused` when the contract is paused.
    /// `submitter` must call `require_auth()`.
    pub fn lock(
        env: Env,
        task_id: Symbol,
        submitter: Address,
        agent: Address,
        amount: i128,
    ) -> Result<(), Error> {
        require_not_paused(&env)?;
        submitter.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let key = DataKey::Escrow(task_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::EscrowAlreadySettled);
        }

        let record = EscrowRecord {
            task_id: task_id.clone(),
            submitter: submitter.clone(),
            agent: agent.clone(),
            amount,
            status: EscrowStatus::Locked,
        };
        env.storage().persistent().set(&key, &record);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("locked")),
            FundsLockedEvent {
                task_id,
                submitter,
                agent,
                amount,
            },
        );

        Ok(())
    }

    /// Release locked funds to the agent on task completion.
    ///
    /// Intentionally NOT gated by `require_not_paused` — existing fund holders
    /// must be able to settle even during an emergency pause.
    pub fn release(env: Env, task_id: Symbol, caller: Address) -> Result<(), Error> {
        caller.require_auth();

        let key = DataKey::Escrow(task_id.clone());
        let mut record: EscrowRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::EscrowNotFound)?;

        if record.status != EscrowStatus::Locked {
            return Err(Error::EscrowAlreadySettled);
        }

        // Only governance or the submitter may release.
        let governance = read_governance(&env)?;
        if caller != record.submitter && caller != governance {
            return Err(Error::Unauthorized);
        }

        record.status = EscrowStatus::Released;
        env.storage().persistent().set(&key, &record);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("released")),
            FundsReleasedEvent {
                task_id,
                agent: record.agent,
                amount: record.amount,
            },
        );

        Ok(())
    }

    /// Refund locked funds to the submitter.
    ///
    /// Intentionally NOT gated by `require_not_paused` — existing fund holders
    /// must be able to recover funds even during an emergency pause.
    pub fn refund(env: Env, task_id: Symbol, caller: Address) -> Result<(), Error> {
        caller.require_auth();

        let key = DataKey::Escrow(task_id.clone());
        let mut record: EscrowRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::EscrowNotFound)?;

        if record.status != EscrowStatus::Locked {
            return Err(Error::EscrowAlreadySettled);
        }

        // Only governance or the submitter may refund.
        let governance = read_governance(&env)?;
        if caller != record.submitter && caller != governance {
            return Err(Error::Unauthorized);
        }

        record.status = EscrowStatus::Refunded;
        env.storage().persistent().set(&key, &record);

        env.events().publish(
            (symbol_short!("escrow"), symbol_short!("refunded")),
            FundsRefundedEvent {
                task_id,
                submitter: record.submitter,
                amount: record.amount,
            },
        );

        Ok(())
    }

    /// Get an escrow record by task_id.
    pub fn get_escrow(env: Env, task_id: Symbol) -> Option<EscrowRecord> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(task_id))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        symbol_short, Env, Symbol,
    };

    struct Fixture {
        env: Env,
        client: PaymentEscrowContractClient<'static>,
        governance: Address,
        submitter: Address,
        agent: Address,
        task_id: Symbol,
    }

    fn fixture() -> Fixture {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PaymentEscrowContract, ());
        let client = PaymentEscrowContractClient::new(&env, &contract_id);

        let governance = Address::generate(&env);
        let submitter = Address::generate(&env);
        let agent = Address::generate(&env);
        let task_id = Symbol::new(&env, "task_001");

        client.initialize(&governance);

        Fixture {
            env,
            client,
            governance,
            submitter,
            agent,
            task_id,
        }
    }

    // ── Pause / Unpause ───────────────────────────────────────────────────────

    #[test]
    fn pause_sets_paused_flag() {
        let f = fixture();
        assert!(!f.client.get_pause_state());
        f.client.pause();
        assert!(f.client.get_pause_state());
    }

    #[test]
    fn unpause_clears_paused_flag() {
        let f = fixture();
        f.client.pause();
        f.client.unpause();
        assert!(!f.client.get_pause_state());
    }

    #[test]
    fn pause_emits_pause_activated_event() {
        let f = fixture();
        f.client.pause();
        let events = f.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("escrow"), symbol_short!("paused")).into_val(&f.env)
        );
    }

    #[test]
    fn unpause_emits_pause_lifted_event() {
        let f = fixture();
        f.client.pause();
        // Clear events from pause call
        let _ = f.env.events().all();
        f.client.unpause();
        let events = f.env.events().all();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events.get(0).unwrap().1,
            (symbol_short!("escrow"), symbol_short!("unpaused")).into_val(&f.env)
        );
    }

    // ── Lock blocked when paused ──────────────────────────────────────────────

    #[test]
    fn lock_while_paused_returns_contract_paused() {
        let f = fixture();
        f.client.pause();
        let result = f.client.try_lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));
    }

    #[test]
    fn lock_after_unpause_succeeds() {
        let f = fixture();
        f.client.pause();
        f.client.unpause();
        f.client.lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        let record = f.client.get_escrow(&f.task_id).unwrap();
        assert_eq!(record.status, EscrowStatus::Locked);
    }

    // ── Release / Refund succeed during pause ─────────────────────────────────

    #[test]
    fn release_while_paused_succeeds() {
        let f = fixture();
        f.client.lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        f.client.pause();
        // release should succeed even during pause
        f.client.release(&f.task_id, &f.submitter);
        let record = f.client.get_escrow(&f.task_id).unwrap();
        assert_eq!(record.status, EscrowStatus::Released);
    }

    #[test]
    fn refund_while_paused_succeeds() {
        let f = fixture();
        f.client.lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        f.client.pause();
        // refund should succeed even during pause
        f.client.refund(&f.task_id, &f.submitter);
        let record = f.client.get_escrow(&f.task_id).unwrap();
        assert_eq!(record.status, EscrowStatus::Refunded);
    }

    // ── Authorization ─────────────────────────────────────────────────────────

    #[test]
    fn get_pause_state_returns_false_after_init() {
        let f = fixture();
        assert!(!f.client.get_pause_state());
    }

    #[test]
    fn get_pause_state_returns_true_after_pause() {
        let f = fixture();
        f.client.pause();
        assert!(f.client.get_pause_state());
    }

    #[test]
    fn get_pause_state_returns_false_after_unpause() {
        let f = fixture();
        f.client.pause();
        f.client.unpause();
        assert!(!f.client.get_pause_state());
    }

    // ── Lock normal flow ──────────────────────────────────────────────────────

    #[test]
    fn lock_stores_escrow_record() {
        let f = fixture();
        f.client.lock(&f.task_id, &f.submitter, &f.agent, &5_000_000i128);
        let record = f.client.get_escrow(&f.task_id).unwrap();
        assert_eq!(record.amount, 5_000_000i128);
        assert_eq!(record.status, EscrowStatus::Locked);
        assert_eq!(record.submitter, f.submitter);
        assert_eq!(record.agent, f.agent);
    }

    #[test]
    fn double_lock_same_task_is_rejected() {
        let f = fixture();
        f.client.lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        let result = f.client.try_lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        assert_eq!(result, Err(Ok(Error::EscrowAlreadySettled)));
    }

    #[test]
    fn zero_amount_lock_is_rejected() {
        let f = fixture();
        let result = f.client.try_lock(&f.task_id, &f.submitter, &f.agent, &0i128);
        assert_eq!(result, Err(Ok(Error::InvalidAmount)));
    }

    // ── Full pause cycle ──────────────────────────────────────────────────────

    #[test]
    fn full_pause_cycle_lock_blocked_then_allowed() {
        let f = fixture();

        // 1. Pause
        f.client.pause();

        // 2. Lock is blocked
        let result = f.client.try_lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        assert_eq!(result, Err(Ok(Error::ContractPaused)));

        // 3. Unpause
        f.client.unpause();

        // 4. Lock succeeds
        f.client.lock(&f.task_id, &f.submitter, &f.agent, &1_000_000i128);
        let record = f.client.get_escrow(&f.task_id).unwrap();
        assert_eq!(record.status, EscrowStatus::Locked);
    }
}
