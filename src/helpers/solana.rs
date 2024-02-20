use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
};

pub fn transfer<'a>(
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    lamports: u64,
    authority_signer_seeds: &[&[&[u8]]],
    system_program: &AccountInfo<'a>,
) -> ProgramResult {
    let transfer_instruction = &solana_program::system_instruction::transfer(
        source.key,
        destination.key,
        lamports
    );
    let accs = [
        source.clone(),
        destination.clone(),
        system_program.clone(),
    ];
    solana_program::program::invoke_signed(transfer_instruction, &accs, authority_signer_seeds)
}