use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReserveError {
    InvalidReserveAccount = 6_000,
    InvalidCircuitBreakerConfig = 6_001,
    CircuitBreakerHalted = 6_002,
    DepositsDisabled = 6_003,
    BorrowsDisabled = 6_004,
    DepositWindowExceeded = 6_005,
    BorrowWindowExceeded = 6_006,
    UtilizationExceeded = 6_007,
    MathOverflow = 6_008,
    Unauthorized = 6_009,
}

impl From<ReserveError> for ProgramError {
    fn from(error: ReserveError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
