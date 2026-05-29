use pinocchio::{
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_token::instructions::TransferChecked;

use crate::{
    instruction::{ReserveInstruction, SetCircuitBreakerData},
    state::{is_uninitialized_reserve, Reserve},
};

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ReserveInstruction::try_from(instruction_data)?;
    let current_slot = Clock::get()?.slot;

    match instruction {
        ReserveInstruction::InitializeReserve(data) => {
            process_initialize_reserve(program_id, accounts, data, current_slot)
        }
        ReserveInstruction::DepositLiquidity(data) => process_deposit_liquidity(
            program_id,
            accounts,
            data.amount,
            data.decimals,
            current_slot,
        ),
        ReserveInstruction::BorrowLiquidity(data) => process_borrow_liquidity(
            program_id,
            accounts,
            data.amount,
            data.decimals,
            current_slot,
        ),
        ReserveInstruction::SetCircuitBreaker(data) => {
            process_set_circuit_breaker(program_id, accounts, data, current_slot)
        }
    }
}

fn process_initialize_reserve(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: crate::instruction::InitializeReserveData,
    current_slot: u64,
) -> ProgramResult {
    let [reserve_account, authority, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    validate_reserve_account_for_write(reserve_account, program_id)?;
    validate_signer(authority)?;

    let mut reserve_data = reserve_account.try_borrow_mut()?;
    if !is_uninitialized_reserve(&reserve_data)? {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let reserve = Reserve::new(
        authority.address().to_bytes(),
        data.max_deposit_per_slot,
        data.max_borrow_per_slot,
        data.max_utilization_bps,
        current_slot,
    )?;
    reserve.pack(&mut reserve_data)
}

fn process_deposit_liquidity(
    program_id: &Address,
    accounts: &mut [AccountView],
    amount: u64,
    decimals: u8,
    current_slot: u64,
) -> ProgramResult {
    let [reserve_account, depositor, user_source_token, reserve_vault_token, mint, ..] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    validate_reserve_account_for_write(reserve_account, program_id)?;
    validate_signer(depositor)?;
    validate_token_account_pair(user_source_token, reserve_vault_token)?;

    let mut reserve_data = reserve_account.try_borrow_mut()?;
    let mut reserve = Reserve::unpack(&reserve_data)?;
    reserve.deposit_liquidity(amount, current_slot)?;
    TransferChecked::new(
        user_source_token,
        mint,
        reserve_vault_token,
        depositor,
        amount,
        decimals,
    )
    .invoke()?;
    reserve.pack(&mut reserve_data)
}

fn process_borrow_liquidity(
    program_id: &Address,
    accounts: &mut [AccountView],
    amount: u64,
    decimals: u8,
    current_slot: u64,
) -> ProgramResult {
    let [reserve_account, borrower, reserve_vault_token, borrower_destination_token, mint, vault_authority, ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    validate_reserve_account_for_write(reserve_account, program_id)?;
    validate_signer(borrower)?;
    validate_signer(vault_authority)?;
    validate_token_account_pair(reserve_vault_token, borrower_destination_token)?;

    let mut reserve_data = reserve_account.try_borrow_mut()?;
    let mut reserve = Reserve::unpack(&reserve_data)?;
    reserve.assert_authority(vault_authority.address().as_array())?;
    reserve.borrow_liquidity(amount, current_slot)?;
    TransferChecked::new(
        reserve_vault_token,
        mint,
        borrower_destination_token,
        vault_authority,
        amount,
        decimals,
    )
    .invoke()?;
    reserve.pack(&mut reserve_data)
}

fn process_set_circuit_breaker(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: SetCircuitBreakerData,
    current_slot: u64,
) -> ProgramResult {
    let [reserve_account, authority, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    validate_reserve_account_for_write(reserve_account, program_id)?;
    validate_signer(authority)?;

    let mut reserve_data = reserve_account.try_borrow_mut()?;
    let mut reserve = Reserve::unpack(&reserve_data)?;
    reserve.assert_authority(authority.address().as_array())?;
    reserve.set_breaker_config(
        data.max_deposit_per_slot,
        data.max_borrow_per_slot,
        data.max_utilization_bps,
        data.flags,
        data.halt_slots,
        current_slot,
    )?;
    reserve.pack(&mut reserve_data)
}

fn validate_reserve_account_for_write(
    reserve_account: &AccountView,
    program_id: &Address,
) -> ProgramResult {
    if !reserve_account.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if !reserve_account.owned_by(program_id) {
        return Err(ProgramError::InvalidAccountOwner);
    }
    Ok(())
}

fn validate_signer(account: &AccountView) -> ProgramResult {
    if !account.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    Ok(())
}

fn validate_token_account_pair(first: &AccountView, second: &AccountView) -> ProgramResult {
    if !first.is_writable() || !second.is_writable() {
        return Err(ProgramError::InvalidArgument);
    }
    if first.address() == second.address() {
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
