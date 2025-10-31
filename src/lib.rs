#![allow(unexpected_cfgs)]

pub mod instruction;
pub mod processor;
pub mod states;

use processor::Processor;
use solana_program::entrypoint;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &solana_program::pubkey::Pubkey,
    accounts: &[solana_program::account_info::AccountInfo],
    instruction_data: &[u8],
) -> solana_program::entrypoint::ProgramResult {
    Processor::process_instruction(program_id, accounts, instruction_data)
}
