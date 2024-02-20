use std::cell::{Ref, RefMut};

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{clock::Clock, Sysvar},
    pubkey::Pubkey,
    program_pack::Pack,
};
use spl_token::state::Account;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    state::{
        staking::{
            StakeBalance,
            StakingAccount,
            StakingState,
        },
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::Stake);

#[inline(never)]
pub fn process_stake(program_id: &Pubkey, lucra: u64, accounts: &[AccountInfo]) -> LucraResult {
    check!(lucra > 0, LucraErrorCode::InvalidAmount)?;

    const NUM_FIXED: usize = 12;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // read
        staking_state_ai,               // read
        staking_account_ai,             // write
        stake_balance_ai,               // write
        deposit_vault_ai,               // write
        stake_vault_ai,                 // write
        owner_ai,                       // read
        transfer_authority_ai,          // read
        staked_lucra_mint_ai,           // write
        user_staked_lucra_account_ai,   // write
        mint_authority_ai,              // read
        token_program_ai,               // read
    ] = accounts;

    let clock = &Clock::get()?;

    check_eq!(staking_account_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(deposit_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_staked_lucra_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?; 

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check_eq!(&system_state.staking_state, staking_state_ai.key, LucraErrorCode::InvalidAccountInput)?;
    let staking_state: RefMut<StakingState> = StakingState::load_mut_checked(staking_state_ai, program_id)?;
    let mut staking_account: RefMut<StakingAccount> = StakingAccount::load_mut_checked(staking_account_ai, program_id)?;
    let mut stake_balance: RefMut<StakeBalance> = StakeBalance::load_mut_checked(stake_balance_ai, program_id)?;

    let staked_lucra_account = Account::unpack(&user_staked_lucra_account_ai.data.borrow())?;

    check_eq!(staked_lucra_account.mint, staking_state.stake_mint.address, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&staked_lucra_account.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&staking_account.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(staking_account.owner, stake_balance.owner, LucraErrorCode::InvalidAccountInput)?;
    check!(staking_state.reward_cursor == stake_balance.reward_cursor, LucraErrorCode::RewardsOutstanding)?;
    check_eq!(&stake_balance.balances.deposit_vault, deposit_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.balances.stake_vault, stake_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&staking_state.stake_mint.address, staked_lucra_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;

    stake_balance.transfer_from_deposit_to_stake(
        program_id,
        &system_state.key,
        deposit_vault_ai,
        stake_vault_ai,
        transfer_authority_ai,
        token_program_ai,
        lucra,
    )?;

    let staking_timeframe_weight = stake_balance.staking_timeframe.weight();
    let weighted_spl_token_amount = lucra
        .checked_mul(staking_timeframe_weight)
        .ok_or(math_err!())?;
    staking_state.mint_stake(
        program_id,
        staked_lucra_mint_ai,
        user_staked_lucra_account_ai,
        weighted_spl_token_amount,
        mint_authority_ai,
        token_program_ai,
    )?;

    stake_balance.last_stake_timestamp = clock.unix_timestamp;
    stake_balance.increment_reward_cursor(staking_state.reward_cursor);
    staking_account.add_total(lucra);

    Ok(())
}