use std::cell::Ref;

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::spl::*,
    state::{
        staking::StakeBalance,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::DepositStake);

#[inline(never)]
pub fn process_deposit_stake(program_id: &Pubkey, lucra: u64, accounts: &[AccountInfo]) -> LucraResult {
    check!(lucra != 0, LucraErrorCode::InvalidAmount)?;

    const NUM_FIXED: usize = 6;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,    // read
        stake_balance_ai,   // read
        from_account_ai,    // write
        deposit_vault_ai,   // write
        owner_ai,           // read
        token_program_ai,   // read
    ] = accounts;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(from_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(deposit_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    
    let stake_balance: Ref<StakeBalance> = StakeBalance::load_checked(stake_balance_ai, program_id)?;
    check!(!stake_balance.closed, LucraErrorCode::InvalidAccountInput)?;

    let from_account = Account::unpack(&from_account_ai.data.borrow())?;

    check_eq!(stake_balance.owner, from_account.owner, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&from_account.mint, &system_state.lucra_mint.address, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.balances.deposit_vault, deposit_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    spl_token_transfer(
        from_account_ai,
        deposit_vault_ai,
        lucra,
        owner_ai,
        &[],
        token_program_ai
    )?;
    
    Ok(())
}