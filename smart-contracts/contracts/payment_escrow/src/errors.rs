//! # Error Types for Payment Escrow

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has not been initialized.
    NotInitialized = 1,
    /// Caller is not authorized to perform this action.
    Unauthorized = 2,
    /// The contract has already been initialized.
    AlreadyInitialized = 3,
    /// The contract is currently paused; new locks are rejected.
    ContractPaused = 4,
    /// The escrow record for this task does not exist.
    EscrowNotFound = 5,
    /// The escrow has already been released or refunded.
    EscrowAlreadySettled = 6,
    /// The lock amount must be greater than zero.
    InvalidAmount = 7,
}
