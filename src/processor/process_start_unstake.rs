use std::{
    cell::{Ref, RefMut},
    mem::size_of,
};

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
    pubkey::Pubkey,
    program_pack::Pack,
};
use spl_token::state::Account;
use legends_loadable_trait::Loadable;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::{SOL_USDC_ORACLE, SOL_USDT_ORACLE, LUCRA_SOL_ORACLE},
    helpers::{oracle::*, spl::spl_token_burn},
    state::{
        DataType,
        MetaData,
        staking::{
            PendingWithdrawal,
            StakeBalance,
            StakingAccount,
            StakingState,
        },
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::StartUnstake);

#[inline(never)]
pub fn process_start_unstake(program_id: &Pubkey, lucra: u64, accounts: &[AccountInfo]) -> LucraResult {
    check!(lucra > 0, LucraErrorCode::InvalidAmount)?;

    const NUM_FIXED: usize = 15;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // read
        staking_state_ai,               // read
        staking_account_ai,             // write
        stake_balance_ai,               // write
        staked_lucra_mint_ai,           // write
        user_staked_lucra_account_ai,   // write
        owner_ai,                       // read
        stake_vault_ai,                 // write
        pending_vault_ai,               // write
        vault_authority_ai,             // read
        pending_withdrawal_ai,          // write

        sol_usdc_oracle_ai,             // read
        sol_usdt_oracle_ai,             // read
        lucra_sol_oracle_ai,            // read

        token_program_ai,               // read
    ] = accounts;

    let clock = &Clock::get()?;
    let rent = &Rent::get()?;
    
    check_eq!(staking_account_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(pending_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_staked_lucra_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdt_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdc_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_sol_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(sol_usdc_oracle_ai.key, &SOL_USDC_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.key, &SOL_USDT_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(lucra_sol_oracle_ai.key, &LUCRA_SOL_ORACLE, LucraErrorCode::InvalidAccountInput)?;

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check_eq!(&system_state.staking_state, staking_state_ai.key, LucraErrorCode::InvalidAccountInput)?;
    let staking_state: Ref<StakingState> = StakingState::load_checked(staking_state_ai, program_id)?;
    let mut staking_account: RefMut<StakingAccount> = StakingAccount::load_mut_checked(staking_account_ai, program_id)?;
    let mut stake_balance: RefMut<StakeBalance> = StakeBalance::load_mut_checked(stake_balance_ai, program_id)?;
    check!(!stake_balance.closed, LucraErrorCode::InvalidAccountInput)?;
    let staking_timeframe = stake_balance.staking_timeframe;
    let stake_vault = Account::unpack(&stake_vault_ai.data.borrow())?;

    let staked_lucra_account = Account::unpack(&user_staked_lucra_account_ai.data.borrow())?;
    check_eq!(&staked_lucra_account.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(staked_lucra_account.mint, staking_state.stake_mint.address, LucraErrorCode::InvalidAccountInput)?;

    check_eq!(&staking_account.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check!(
        rent.is_exempt(pending_withdrawal_ai.lamports(), size_of::<PendingWithdrawal>()),
        LucraErrorCode::NotRentExempt
    )?;
    let mut pending_withdrawal: RefMut<PendingWithdrawal> = PendingWithdrawal::load_mut(pending_withdrawal_ai)?;
    check!(!pending_withdrawal.meta_data.is_initialized, LucraErrorCode::Default)?;

    let lucra_market_price = get_lucra_price(lucra_sol_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let value_of_lucra_to_be_unlocked = Decimal::from(lucra)
        .checked_mul(lucra_market_price)
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;
    let value_locked_up = Decimal::from(stake_vault.amount)
        .checked_mul(lucra_market_price)
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;

    check!(staking_state.reward_cursor == stake_balance.reward_cursor, LucraErrorCode::RewardsOutstanding)?;
    check_eq!(&staking_state.stake_mint.address, staked_lucra_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check!(value_locked_up - staking_account.locked_total >= value_of_lucra_to_be_unlocked, LucraErrorCode::OutstandingLoans)?;
    check!(stake_balance.last_stake_timestamp + (system_state.epoch * staking_timeframe.timeframe_multiplier()) <= clock.unix_timestamp, LucraErrorCode::StakingAccountNotUnlocked)?;
    check_eq!(&stake_balance.balances.pending_vault, pending_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.balances.stake_vault, stake_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let staked_lucra = lucra
        .checked_mul(staking_timeframe.weight())
        .ok_or(math_err!())?;
    check!(staked_lucra <= staked_lucra_account.amount, LucraErrorCode::InvalidAmount)?;
    spl_token_burn(
        staked_lucra_mint_ai,
        user_staked_lucra_account_ai,
        staked_lucra,
        owner_ai,
        &[],
        token_program_ai,
    )?;

    stake_balance.transfer_from_stake_to_pending(
        program_id,
        system_state_ai.key,
        stake_vault_ai,
        pending_vault_ai,
        vault_authority_ai,
        token_program_ai,
        lucra,
    )?;

    let end_timestamp = clock
        .unix_timestamp
        .checked_add(system_state.epoch)
        .ok_or(math_err!())?;

    pending_withdrawal.meta_data = MetaData::new(DataType::PendingWithdrawal, 0, true);
    pending_withdrawal.stake_balance = *stake_balance_ai.key;
    pending_withdrawal.start_timestamp = clock.unix_timestamp;
    pending_withdrawal.end_timestamp = end_timestamp;
    pending_withdrawal.lucra = lucra;
    pending_withdrawal.open();

    stake_balance.update_last_stake_timestamp(clock.unix_timestamp);

    staking_account.remove_total(lucra);

    Ok(())
}