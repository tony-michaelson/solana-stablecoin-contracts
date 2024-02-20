use std::{
    cell::{Ref, RefMut},
    ops::Deref,
};

use arrayref::array_ref;
use rust_decimal_macros::dec;
use solana_program::{
    account_info::AccountInfo,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::{
    state::Account,
};
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
    helpers::math::calculate_annual_interest_rate,
    state::{
        staking::{
            StakeBalance,
            StakingState,
            Reward,
        },
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::ClaimReward);

#[inline(never)]
pub fn process_claim_reward(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 13;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                        // read
        staking_state_ai,                       // read
        stake_balance_ai,                       // write
        reward_ai,                              // read
        user_staked_lucra_account_ai,           // read
        lucra_vault_ai,                         // write
        lucra_account_ai,                       // write
        rewards_vault_ai,                       // write
        msol_account_ai,                        // write
        rewards_vault_transfer_authority_ai,    // read
        lucra_mint_ai,                          // write
        lucra_mint_authority_ai,                // read
        token_program_ai,                       // read
    ] = accounts;

    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(reward_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_staked_lucra_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(rewards_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    
    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check_eq!(&system_state.rewards_vault.address, rewards_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.staking_state, staking_state_ai.key, LucraErrorCode::InvalidAccountInput)?;
    let staking_state: Ref<StakingState> = StakingState::load_checked(staking_state_ai, program_id)?;

    let mut stake_balance: RefMut<StakeBalance> = StakeBalance::load_mut(stake_balance_ai)?;
    check!(!stake_balance.closed, LucraErrorCode::InvalidAccountInput)?;
    let staked_lucra_account = Account::unpack(&user_staked_lucra_account_ai.data.borrow())?;
    check_eq!(staked_lucra_account.owner, stake_balance.owner, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(staked_lucra_account.mint, staking_state.stake_mint.address, LucraErrorCode::InvalidAccountInput)?;

    let reward: Ref<Reward> = Reward::load(reward_ai)?;

    let msol_account = Account::unpack(&msol_account_ai.data.borrow())?;
    let lucra_account = Account::unpack(&lucra_account_ai.data.borrow())?;

    check_eq!(&system_state.lucra_mint.address, lucra_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check!(stake_balance.last_stake_timestamp != 0, LucraErrorCode::NotStakedDuringDrop)?;
    check!(stake_balance.last_stake_timestamp <= reward.start_timestamp, LucraErrorCode::NotStakedDuringDrop)?;
    check!(stake_balance.reward_cursor <= reward.reward_cursor, LucraErrorCode::AlreadyProcessed)?;
    check!(stake_balance.reward_cursor == reward.reward_cursor, LucraErrorCode::ClaimOutOfOrder)?;

    check_eq!(&stake_balance.balances.stake_vault, lucra_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.owner, &msol_account.owner, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_balance.owner, &lucra_account.owner, LucraErrorCode::InvalidAccountInput)?;

    transfer_reward(
        program_id,
        system_state.deref(),
        &staked_lucra_account,
        &reward,
        rewards_vault_ai,
        msol_account_ai,
        rewards_vault_transfer_authority_ai,
        token_program_ai,
    )?;

    let staking_timeframe = stake_balance.staking_timeframe;    
    let inflation_amount = calculate_inflation(staking_timeframe.annual_inflation_rate(), staked_lucra_account.amount)?;
    system_state.mint_lucra(
        program_id,
        lucra_mint_ai,
        lucra_account_ai,
        inflation_amount,
        lucra_mint_authority_ai,
        token_program_ai,
    )?;

    stake_balance.increment_reward_cursor(reward.reward_cursor);

    Ok(())
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn transfer_reward<'a>(
    program_id: &Pubkey,
    system_state: &SystemState,
    stake_vault: &Account,
    reward: &Reward,
    rewards_vault_ai: &AccountInfo<'a>,
    msol_account_ai: &AccountInfo<'a>,
    transfer_authority_ai: &AccountInfo<'a>,
    token_program_ai: &AccountInfo<'a>,
) -> LucraResult {
    let reward_amount = Decimal::from(stake_vault.amount)
        .checked_mul(reward.total.into())
        .ok_or(math_err!())?
        .checked_div(reward.pool_token_supply.into())
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;

    system_state.transfer_from_reward_vault(
        program_id,
        rewards_vault_ai,
        msol_account_ai,
        transfer_authority_ai,
        token_program_ai,
        reward_amount,
    )?;

    Ok(())
}

/// Inflation is x% annually of whatever lucra you have staked.
/// It uses the weighted lucra value so that people who choose to lock up get better rewards.
pub fn calculate_inflation(
    inflation_rate: u8,
    stake_amount: u64,
) -> LucraResult<u64> {
    let time = dec!(1).checked_div(dec!(52)).unwrap();
    calculate_annual_interest_rate(inflation_rate as u32, stake_amount, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_inflation() {
        // Should calculate a week's worth of inflation
        let inflation_rate = 10; // 10%
        let amount = 10_000;
        let expected = 19;

        let actual = calculate_inflation(inflation_rate, amount).unwrap();
        assert_eq!(expected, actual);
    }
}