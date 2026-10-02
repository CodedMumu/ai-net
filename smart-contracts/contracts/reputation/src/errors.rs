use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidRating = 4,
    AgentNotFound = 5,
    CallerAlreadyRegistered = 6,
    CallerNotFound = 7,
    TaskAlreadyRecorded = 8,
}
