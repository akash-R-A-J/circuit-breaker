use pinocchio::error::ProgramError;

use crate::error::ReserveError;

pub const TAG_INITIALIZE_RESERVE: u8 = 0;
pub const TAG_DEPOSIT_LIQUIDITY: u8 = 1;
pub const TAG_BORROW_LIQUIDITY: u8 = 2;
pub const TAG_SET_CIRCUIT_BREAKER: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReserveInstruction {
    InitializeReserve(InitializeReserveData),
    DepositLiquidity(TransferData),
    BorrowLiquidity(TransferData),
    SetCircuitBreaker(SetCircuitBreakerData),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitializeReserveData {
    pub max_deposit_per_slot: u64,
    pub max_borrow_per_slot: u64,
    pub max_utilization_bps: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferData {
    pub amount: u64,
    pub decimals: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetCircuitBreakerData {
    pub max_deposit_per_slot: u64,
    pub max_borrow_per_slot: u64,
    pub max_utilization_bps: u16,
    pub flags: u8,
    pub halt_slots: u64,
}

impl TryFrom<&[u8]> for ReserveInstruction {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match *tag {
            TAG_INITIALIZE_RESERVE => Ok(Self::InitializeReserve(InitializeReserveData::try_from(
                rest,
            )?)),
            TAG_DEPOSIT_LIQUIDITY => Ok(Self::DepositLiquidity(TransferData::try_from(rest)?)),
            TAG_BORROW_LIQUIDITY => Ok(Self::BorrowLiquidity(TransferData::try_from(rest)?)),
            TAG_SET_CIRCUIT_BREAKER => Ok(Self::SetCircuitBreaker(
                SetCircuitBreakerData::try_from(rest)?,
            )),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

impl TryFrom<&[u8]> for InitializeReserveData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        require_exact_len(data, 18)?;
        let max_deposit_per_slot = read_u64(data, 0)?;
        let max_borrow_per_slot = read_u64(data, 8)?;
        let max_utilization_bps = read_u16(data, 16)?;
        validate_breaker_config(
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
        )?;

        Ok(Self {
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
        })
    }
}

impl TryFrom<&[u8]> for TransferData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        require_exact_len(data, 9)?;
        let amount = read_u64(data, 0)?;
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self {
            amount,
            decimals: data[8],
        })
    }
}

impl TryFrom<&[u8]> for SetCircuitBreakerData {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        require_exact_len(data, 27)?;
        let max_deposit_per_slot = read_u64(data, 0)?;
        let max_borrow_per_slot = read_u64(data, 8)?;
        let max_utilization_bps = read_u16(data, 16)?;
        let flags = data[18];
        let halt_slots = read_u64(data, 19)?;
        validate_breaker_config(
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
        )?;

        Ok(Self {
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
            flags,
            halt_slots,
        })
    }
}

pub fn validate_breaker_config(
    max_deposit_per_slot: u64,
    max_borrow_per_slot: u64,
    max_utilization_bps: u16,
) -> Result<(), ProgramError> {
    if max_deposit_per_slot == 0 || max_borrow_per_slot == 0 || max_utilization_bps > 10_000 {
        return Err(ReserveError::InvalidCircuitBreakerConfig.into());
    }
    Ok(())
}

fn require_exact_len(data: &[u8], expected: usize) -> Result<(), ProgramError> {
    if data.len() != expected {
        return Err(ProgramError::InvalidInstructionData);
    }
    Ok(())
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, ProgramError> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .ok_or(ProgramError::InvalidInstructionData)?
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, ProgramError> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ProgramError::InvalidInstructionData)?
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    ))
}
