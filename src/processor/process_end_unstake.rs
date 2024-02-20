use std::cell::{Ref, RefMut};

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{clock::Clock, Sysvar},
    pubkey::Pubkey,
};
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::account::*,
    state::staking::{
        PendingWithdrawal,
        StakeBalance,
    },
    state::SystemState,
};

declare_check_assert_macros!(SourceFileId::EndUnstake);

#[inline(never)]
pub fn process_end_unstake(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 9;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,        // read
        pending_withdrawal_ai,  // write
        stake_balance_ai,       // read
        pending_vault_ai,       // write
        deposit_vault_ai,       // write
        owner_ai,               // read
        transfer_authority_ai,  // read
        sol_account_ai,         // write
        token_program_ai,       // read
    ] = accounts;

    let clock = &Clock::get()?;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(pending_withdrawal_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(deposit_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(pending_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;

    let mut pending_withdrawal: RefMut<PendingWithdrawal> = PendingWithdrawal::load_mut_checked(pending_withdrawal_ai, program_id)?;
    check!(!pending_withdrawal.closed(), LucraErrorCode::InvalidAccountInput)?;
    let stake_balance: Ref<StakeBalance> = StakeBalance::load_checked(stake_balance_ai, program_id)?;

    check_eq!(&stake_balance.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.owner, sol_account_ai.key, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(&pending_withdrawal.stake_balance, stake_balance_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check!(pending_withdrawal.end_timestamp <= clock.unix_timestamp, LucraErrorCode::Timelock)?;

    check_eq!(&stake_balance.balances.deposit_vault, deposit_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.balances.pending_vault, pending_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    stake_balance.transfer_from_pending_to_deposit(
        program_id,
        system_state_ai.key,
        pending_vault_ai,
        deposit_vault_ai,
        transfer_authority_ai,
        token_program_ai,
        pending_withdrawal.lucra,
    )?;

    pending_withdrawal.close();
    let lamports = close_account(pending_withdrawal_ai);
    add_lamports(sol_account_ai, lamports);

    Ok(())
}