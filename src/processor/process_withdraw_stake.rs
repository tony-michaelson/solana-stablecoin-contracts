use std::cell::{Ref, RefMut};

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::{
    state::{Account},
};
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::account::{close_account, add_lamports},
    helpers::spl::*,
    state::{
        staking::StakeBalance,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::WithdrawStake);

#[inline(never)]
pub fn process_withdraw_stake(program_id: &Pubkey, lucra: u64, accounts: &[AccountInfo]) -> LucraResult {
    check!(lucra > 0, LucraErrorCode::InvalidAmount)?;

    const NUM_FIXED: usize = 9;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,        // read
        stake_balance_ai,       // write
        deposit_vault_ai,       // write
        stake_vault_ai,         // read
        pending_vault_ai,       // read
        to_account_ai,          // write
        owner_ai,               // write
        transfer_authority_ai,  // read
        token_program_ai,       // read
    ] = accounts;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(deposit_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(pending_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(to_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    let mut stake_balance: RefMut<StakeBalance> = StakeBalance::load_mut_checked(stake_balance_ai, program_id)?;
    check!(!stake_balance.closed, LucraErrorCode::InvalidAccountInput)?;
    let to_account = Account::unpack(&to_account_ai.data.borrow())?;

    check_eq!(&to_account.mint, &system_state.lucra_mint.address, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&to_account.owner, &stake_balance.owner, LucraErrorCode::InvalidAccountInput)?;

    check_eq!(&stake_balance.balances.deposit_vault, deposit_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.balances.stake_vault, stake_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.balances.pending_vault, pending_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    stake_balance.transfer_from_deposit(
        program_id,
        system_state_ai.key,
        deposit_vault_ai,
        to_account_ai,
        transfer_authority_ai,
        token_program_ai,
        lucra,
    )?;

    let deposit_vault_balance = get_token_balance(deposit_vault_ai)?;
    let stake_vault_balance = get_token_balance(stake_vault_ai)?;
    let pending_vault_balance = get_token_balance(pending_vault_ai)?;

    if deposit_vault_balance == 0
        && stake_vault_balance == 0
        && pending_vault_balance == 0
    {
        stake_balance.closed = true;
        // Close the stake balance account
        let lamports = close_account(stake_balance_ai);
        add_lamports(owner_ai, lamports);
    }

    Ok(())
}