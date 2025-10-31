use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::next_account_info;
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::instruction::VaultIx;
use crate::states::VaultAccount;

pub struct Processor;

impl Processor {
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = VaultIx::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match instruction {
            VaultIx::Deposit { amount } => Self::process_deposit(program_id, accounts, amount),
            VaultIx::Withdraw { amount } => Self::process_withdraw(program_id, accounts, amount),
        }
    }

    fn process_deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let user_account = next_account_info(account_info_iter)?;
        let vault_account = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (vault_pda, bump) = Pubkey::find_program_address(
            &[VaultAccount::SEED, user_account.key.as_ref()],
            program_id,
        );

        if !vault_pda.eq(vault_account.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        if vault_account.data_is_empty() {
            let rent = Rent::get()?;
            let space = VaultAccount::LEN;
            let rent_lamports = rent.minimum_balance(space);

            invoke_signed(
                &system_instruction::create_account(
                    user_account.key,
                    vault_account.key,
                    rent_lamports,
                    space as u64,
                    program_id,
                ),
                &[
                    user_account.clone(),
                    vault_account.clone(),
                    system_program.clone(),
                ],
                &[&[VaultAccount::SEED, user_account.key.as_ref(), &[bump]]],
            )?;

            let vault_data = VaultAccount {
                owner: *user_account.key,
                balance: 0,
                bump,
            };

            vault_data.serialize(&mut &mut vault_account.data.borrow_mut()[..])?;
        }

        invoke_signed(
            &system_instruction::transfer(user_account.key, vault_account.key, amount),
            &[user_account.clone(), vault_account.clone()],
            &[],
        )?;

        let mut vault_data = VaultAccount::try_from_slice(&vault_account.data.borrow())?;
        vault_data.balance = vault_data
            .balance
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        vault_data.serialize(&mut &mut vault_account.data.borrow_mut()[..])?;

        Ok(())
    }

    fn process_withdraw(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let user_account = next_account_info(account_info_iter)?;
        let vault_account = next_account_info(account_info_iter)?;

        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (vault_pda, _bump) = Pubkey::find_program_address(
            &[VaultAccount::SEED, user_account.key.as_ref()],
            program_id,
        );

        if !vault_pda.eq(vault_account.key) {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut vault_data = VaultAccount::try_from_slice(&vault_account.data.borrow())?;

        if vault_data.owner != *user_account.key {
            return Err(ProgramError::IllegalOwner);
        }

        if vault_data.balance < amount {
            return Err(ProgramError::InsufficientFunds);
        }

        **vault_account.try_borrow_mut_lamports()? -= amount;
        **user_account.try_borrow_mut_lamports()? += amount;

        vault_data.balance = vault_data.balance.checked_sub(amount).unwrap();
        vault_data.serialize(&mut &mut vault_account.data.borrow_mut()[..])?;

        msg!(
            "Withdrew {} lamports. New balance: {}",
            amount,
            vault_data.balance
        );

        Ok(())
    }
}
