use std::cell::Ref;

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
};
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::DAO_AUTHORITY,
    state::SystemState,
};

declare_check_assert_macros!(SourceFileId::TransferFunds);

#[inline(never)]
pub fn process_transfer_funds(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 6;
    let accounts = array_ref!(accounts, 0, NUM_FIXED);
    let [
        system_state_ai,        // write
        dao_authority_ai,       // read
        from_vault_ai,          // write
        to_account_ai,          // write
        transfer_authority_ai,  // read
        token_program_ai,       // read
    ] = accounts;

    check_eq!(dao_authority_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    check_eq!(dao_authority_ai.key, &DAO_AUTHORITY, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(token_program_ai.key, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(from_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(to_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;

    system_state.transfer_from_msol_vault(
        program_id,
        from_vault_ai,
        to_account_ai,
        transfer_authority_ai,
        token_program_ai,
        lamports,
    )
}