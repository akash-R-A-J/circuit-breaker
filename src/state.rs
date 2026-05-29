use pinocchio::error::ProgramError;

use crate::error::ReserveError;
use crate::instruction::validate_breaker_config;

pub const RESERVE_DISCRIMINATOR: u8 = b'R';
pub const RESERVE_VERSION: u8 = 1;
pub const RESERVE_LEN: usize = 112;
pub const FLAG_MANUAL_HALT: u8 = 1 << 0;
pub const FLAG_DEPOSITS_DISABLED: u8 = 1 << 1;
pub const FLAG_BORROWS_DISABLED: u8 = 1 << 2;
pub const KNOWN_FLAGS: u8 = FLAG_MANUAL_HALT | FLAG_DEPOSITS_DISABLED | FLAG_BORROWS_DISABLED;

const AUTHORITY_OFFSET: usize = 2;
const LIQUIDITY_AVAILABLE_OFFSET: usize = 34;
const BORROWED_AMOUNT_OFFSET: usize = 42;
const DEPOSIT_WINDOW_SLOT_OFFSET: usize = 50;
const BORROW_WINDOW_SLOT_OFFSET: usize = 58;
const DEPOSIT_WINDOW_AMOUNT_OFFSET: usize = 66;
const BORROW_WINDOW_AMOUNT_OFFSET: usize = 74;
const MAX_DEPOSIT_PER_SLOT_OFFSET: usize = 82;
const MAX_BORROW_PER_SLOT_OFFSET: usize = 90;
const MAX_UTILIZATION_BPS_OFFSET: usize = 98;
const HALT_UNTIL_SLOT_OFFSET: usize = 100;
const FLAGS_OFFSET: usize = 108;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reserve {
    pub authority: [u8; 32],
    pub liquidity_available: u64,
    pub borrowed_amount: u64,
    pub deposit_window_slot: u64,
    pub borrow_window_slot: u64,
    pub deposit_window_amount: u64,
    pub borrow_window_amount: u64,
    pub max_deposit_per_slot: u64,
    pub max_borrow_per_slot: u64,
    pub max_utilization_bps: u16,
    pub halt_until_slot: u64,
    pub flags: u8,
}

impl Reserve {
    pub fn new(
        authority: [u8; 32],
        max_deposit_per_slot: u64,
        max_borrow_per_slot: u64,
        max_utilization_bps: u16,
        current_slot: u64,
    ) -> Result<Self, ProgramError> {
        validate_breaker_config(
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
        )?;
        Ok(Self {
            authority,
            liquidity_available: 0,
            borrowed_amount: 0,
            deposit_window_slot: current_slot,
            borrow_window_slot: current_slot,
            deposit_window_amount: 0,
            borrow_window_amount: 0,
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
            halt_until_slot: current_slot,
            flags: 0,
        })
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() != RESERVE_LEN {
            return Err(ReserveError::InvalidReserveAccount.into());
        }
        if input[0] != RESERVE_DISCRIMINATOR || input[1] != RESERVE_VERSION {
            return Err(ReserveError::InvalidReserveAccount.into());
        }

        let mut authority = [0u8; 32];
        authority.copy_from_slice(&input[AUTHORITY_OFFSET..AUTHORITY_OFFSET + 32]);

        let reserve = Self {
            authority,
            liquidity_available: read_u64(input, LIQUIDITY_AVAILABLE_OFFSET)?,
            borrowed_amount: read_u64(input, BORROWED_AMOUNT_OFFSET)?,
            deposit_window_slot: read_u64(input, DEPOSIT_WINDOW_SLOT_OFFSET)?,
            borrow_window_slot: read_u64(input, BORROW_WINDOW_SLOT_OFFSET)?,
            deposit_window_amount: read_u64(input, DEPOSIT_WINDOW_AMOUNT_OFFSET)?,
            borrow_window_amount: read_u64(input, BORROW_WINDOW_AMOUNT_OFFSET)?,
            max_deposit_per_slot: read_u64(input, MAX_DEPOSIT_PER_SLOT_OFFSET)?,
            max_borrow_per_slot: read_u64(input, MAX_BORROW_PER_SLOT_OFFSET)?,
            max_utilization_bps: read_u16(input, MAX_UTILIZATION_BPS_OFFSET)?,
            halt_until_slot: read_u64(input, HALT_UNTIL_SLOT_OFFSET)?,
            flags: input[FLAGS_OFFSET],
        };

        reserve.validate()?;
        Ok(reserve)
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<(), ProgramError> {
        if output.len() != RESERVE_LEN {
            return Err(ReserveError::InvalidReserveAccount.into());
        }
        self.validate()?;

        output.fill(0);
        output[0] = RESERVE_DISCRIMINATOR;
        output[1] = RESERVE_VERSION;
        output[AUTHORITY_OFFSET..AUTHORITY_OFFSET + 32].copy_from_slice(&self.authority);
        write_u64(output, LIQUIDITY_AVAILABLE_OFFSET, self.liquidity_available)?;
        write_u64(output, BORROWED_AMOUNT_OFFSET, self.borrowed_amount)?;
        write_u64(output, DEPOSIT_WINDOW_SLOT_OFFSET, self.deposit_window_slot)?;
        write_u64(output, BORROW_WINDOW_SLOT_OFFSET, self.borrow_window_slot)?;
        write_u64(
            output,
            DEPOSIT_WINDOW_AMOUNT_OFFSET,
            self.deposit_window_amount,
        )?;
        write_u64(
            output,
            BORROW_WINDOW_AMOUNT_OFFSET,
            self.borrow_window_amount,
        )?;
        write_u64(
            output,
            MAX_DEPOSIT_PER_SLOT_OFFSET,
            self.max_deposit_per_slot,
        )?;
        write_u64(output, MAX_BORROW_PER_SLOT_OFFSET, self.max_borrow_per_slot)?;
        write_u16(output, MAX_UTILIZATION_BPS_OFFSET, self.max_utilization_bps)?;
        write_u64(output, HALT_UNTIL_SLOT_OFFSET, self.halt_until_slot)?;
        output[FLAGS_OFFSET] = self.flags;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ProgramError> {
        validate_breaker_config(
            self.max_deposit_per_slot,
            self.max_borrow_per_slot,
            self.max_utilization_bps,
        )?;
        if self.flags & !KNOWN_FLAGS != 0 {
            return Err(ReserveError::InvalidCircuitBreakerConfig.into());
        }
        Ok(())
    }

    pub fn set_breaker_config(
        &mut self,
        max_deposit_per_slot: u64,
        max_borrow_per_slot: u64,
        max_utilization_bps: u16,
        flags: u8,
        halt_slots: u64,
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        validate_breaker_config(
            max_deposit_per_slot,
            max_borrow_per_slot,
            max_utilization_bps,
        )?;
        if flags & !KNOWN_FLAGS != 0 {
            return Err(ReserveError::InvalidCircuitBreakerConfig.into());
        }
        self.max_deposit_per_slot = max_deposit_per_slot;
        self.max_borrow_per_slot = max_borrow_per_slot;
        self.max_utilization_bps = max_utilization_bps;
        self.flags = flags;
        self.halt_until_slot = current_slot
            .checked_add(halt_slots)
            .ok_or(ReserveError::MathOverflow)?;
        Ok(())
    }

    pub fn deposit_liquidity(
        &mut self,
        amount: u64,
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        self.assert_not_halted(current_slot)?;
        if self.flags & FLAG_DEPOSITS_DISABLED != 0 {
            return Err(ReserveError::DepositsDisabled.into());
        }
        self.advance_deposit_window(current_slot)?;
        let next_window_amount = self
            .deposit_window_amount
            .checked_add(amount)
            .ok_or(ReserveError::MathOverflow)?;
        if next_window_amount > self.max_deposit_per_slot {
            return Err(ReserveError::DepositWindowExceeded.into());
        }
        self.liquidity_available = self
            .liquidity_available
            .checked_add(amount)
            .ok_or(ReserveError::MathOverflow)?;
        self.deposit_window_amount = next_window_amount;
        Ok(())
    }

    pub fn borrow_liquidity(&mut self, amount: u64, current_slot: u64) -> Result<(), ProgramError> {
        self.assert_not_halted(current_slot)?;
        if self.flags & FLAG_BORROWS_DISABLED != 0 {
            return Err(ReserveError::BorrowsDisabled.into());
        }
        self.advance_borrow_window(current_slot)?;
        let next_window_amount = self
            .borrow_window_amount
            .checked_add(amount)
            .ok_or(ReserveError::MathOverflow)?;
        if next_window_amount > self.max_borrow_per_slot {
            return Err(ReserveError::BorrowWindowExceeded.into());
        }
        let next_liquidity_available = self
            .liquidity_available
            .checked_sub(amount)
            .ok_or(ProgramError::InsufficientFunds)?;
        let next_borrowed_amount = self
            .borrowed_amount
            .checked_add(amount)
            .ok_or(ReserveError::MathOverflow)?;
        assert_utilization_allowed(
            next_liquidity_available,
            next_borrowed_amount,
            self.max_utilization_bps,
        )?;

        self.liquidity_available = next_liquidity_available;
        self.borrowed_amount = next_borrowed_amount;
        self.borrow_window_amount = next_window_amount;
        Ok(())
    }

    pub fn assert_authority(&self, authority: &[u8; 32]) -> Result<(), ProgramError> {
        if &self.authority != authority {
            return Err(ReserveError::Unauthorized.into());
        }
        Ok(())
    }

    fn assert_not_halted(&self, current_slot: u64) -> Result<(), ProgramError> {
        if self.flags & FLAG_MANUAL_HALT != 0 || current_slot < self.halt_until_slot {
            return Err(ReserveError::CircuitBreakerHalted.into());
        }
        Ok(())
    }

    fn advance_deposit_window(&mut self, current_slot: u64) -> Result<(), ProgramError> {
        if current_slot < self.deposit_window_slot {
            return Err(ProgramError::InvalidArgument);
        }
        if current_slot != self.deposit_window_slot {
            self.deposit_window_slot = current_slot;
            self.deposit_window_amount = 0;
        }
        Ok(())
    }

    fn advance_borrow_window(&mut self, current_slot: u64) -> Result<(), ProgramError> {
        if current_slot < self.borrow_window_slot {
            return Err(ProgramError::InvalidArgument);
        }
        if current_slot != self.borrow_window_slot {
            self.borrow_window_slot = current_slot;
            self.borrow_window_amount = 0;
        }
        Ok(())
    }
}

pub fn assert_utilization_allowed(
    liquidity_available: u64,
    borrowed_amount: u64,
    max_utilization_bps: u16,
) -> Result<(), ProgramError> {
    let total_assets = liquidity_available
        .checked_add(borrowed_amount)
        .ok_or(ReserveError::MathOverflow)?;
    if total_assets == 0 {
        return Ok(());
    }

    let utilization_bps = (borrowed_amount as u128)
        .checked_mul(10_000)
        .ok_or(ReserveError::MathOverflow)?
        .checked_div(total_assets as u128)
        .ok_or(ReserveError::MathOverflow)?;
    if utilization_bps > max_utilization_bps as u128 {
        return Err(ReserveError::UtilizationExceeded.into());
    }
    Ok(())
}

pub fn is_uninitialized_reserve(data: &[u8]) -> Result<bool, ProgramError> {
    if data.len() != RESERVE_LEN {
        return Err(ReserveError::InvalidReserveAccount.into());
    }
    Ok(data[0] == 0)
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, ProgramError> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .ok_or(ReserveError::InvalidReserveAccount)?
            .try_into()
            .map_err(|_| ReserveError::InvalidReserveAccount)?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, ProgramError> {
    Ok(u64::from_le_bytes(
        data.get(offset..offset + 8)
            .ok_or(ReserveError::InvalidReserveAccount)?
            .try_into()
            .map_err(|_| ReserveError::InvalidReserveAccount)?,
    ))
}

fn write_u16(data: &mut [u8], offset: usize, value: u16) -> Result<(), ProgramError> {
    let target = data
        .get_mut(offset..offset + 2)
        .ok_or(ReserveError::InvalidReserveAccount)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_u64(data: &mut [u8], offset: usize, value: u64) -> Result<(), ProgramError> {
    let target = data
        .get_mut(offset..offset + 8)
        .ok_or(ReserveError::InvalidReserveAccount)?;
    target.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

/** tests **/

#[cfg(test)]
mod tests {
    use super::*;

    fn reserve() -> Reserve {
        Reserve::new([7; 32], 1_000, 500, 8_000, 10).unwrap()
    }

    #[test]
    fn pack_round_trips_without_unsafe_casts() {
        let mut bytes = [0u8; RESERVE_LEN];
        let mut state = reserve();
        state.deposit_liquidity(600, 10).unwrap();
        state.borrow_liquidity(200, 10).unwrap();
        state.pack(&mut bytes).unwrap();

        let decoded = Reserve::unpack(&bytes).unwrap();
        assert_eq!(decoded, state);
        assert_eq!(bytes[0], RESERVE_DISCRIMINATOR);
        assert_eq!(bytes[1], RESERVE_VERSION);
    }

    #[test]
    fn deposit_window_rejects_excess_volume() {
        let mut state = reserve();
        state.deposit_liquidity(700, 10).unwrap();

        let before = state;
        assert_eq!(
            state.deposit_liquidity(301, 10),
            Err(ReserveError::DepositWindowExceeded.into())
        );
        assert_eq!(state, before);
    }

    #[test]
    fn deposit_window_resets_on_next_slot() {
        let mut state = reserve();
        state.deposit_liquidity(1_000, 10).unwrap();
        state.deposit_liquidity(1_000, 11).unwrap();

        assert_eq!(state.deposit_window_slot, 11);
        assert_eq!(state.deposit_window_amount, 1_000);
        assert_eq!(state.liquidity_available, 2_000);
    }

    #[test]
    fn borrow_window_rejects_excess_volume() {
        let mut state = reserve();
        state.deposit_liquidity(1_000, 10).unwrap();
        state.borrow_liquidity(300, 10).unwrap();

        let before = state;
        assert_eq!(
            state.borrow_liquidity(201, 10),
            Err(ReserveError::BorrowWindowExceeded.into())
        );
        assert_eq!(state, before);
    }

    #[test]
    fn borrow_rejects_utilization_above_cap() {
        let mut state = Reserve::new([7; 32], 2_000, 2_000, 6_000, 10).unwrap();
        state.deposit_liquidity(1_000, 10).unwrap();

        let before = state;
        assert_eq!(
            state.borrow_liquidity(700, 10),
            Err(ReserveError::UtilizationExceeded.into())
        );
        assert_eq!(state, before);
    }

    #[test]
    fn manual_halt_blocks_user_paths() {
        let mut state = reserve();
        state.deposit_liquidity(100, 10).unwrap();
        state
            .set_breaker_config(1_000, 500, 8_000, FLAG_MANUAL_HALT, 0, 10)
            .unwrap();

        assert_eq!(
            state.deposit_liquidity(100, 10),
            Err(ReserveError::CircuitBreakerHalted.into())
        );
        assert_eq!(
            state.borrow_liquidity(1, 10),
            Err(ReserveError::CircuitBreakerHalted.into())
        );
    }

    #[test]
    fn timed_halt_expires_by_slot() {
        let mut state = reserve();
        state
            .set_breaker_config(1_000, 500, 8_000, 0, 5, 10)
            .unwrap();

        assert_eq!(
            state.deposit_liquidity(100, 14),
            Err(ReserveError::CircuitBreakerHalted.into())
        );
        state.deposit_liquidity(100, 15).unwrap();
    }
}
