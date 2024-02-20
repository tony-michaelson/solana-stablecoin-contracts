#![allow(unused_imports)]

use crate::processor;
use solana_program::{
    account_info::AccountInfo, 
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process(program_id, accounts, instruction_data).map_err(|e| {
        msg!("{}", e);
        e.into()
    })
}