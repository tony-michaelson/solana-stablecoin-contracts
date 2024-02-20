use std::cell::{Ref, RefMut};
use std::mem::size_of;

use anchor_lang::AccountDeserialize;
use arrayref::array_ref;
use marinade_finance::state::State as MarinadeState;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::{
    state::{Account, Mint},
};
use legends_loadable_trait::Loadable;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::spl::get_token_account_mint,
    state::{
        DataType,
        MetaData,
        staking::{Reward, StakingState},
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::DropReward);

#[inline(never)]
pub fn process_drop_reward(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 13;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                    // read
        staking_state_ai,                   // write
        marinade_state_ai,                  // read
        reward_ai,                          // write
        stake_mint_ai,                      // read
        msol_vault_ai,                      // read
        
        rewards_vault_ai,                   // write
        arb_coffer_ai,                      // write
        msol_vault_transfer_authority_ai,   // read

        user_reward_account_ai,             // write
        reward_mint_ai,                     // write
        reward_mint_authority_ai,           // read
        token_program_ai,                   // read
    ] = accounts;

    let rent = &Rent::get().unwrap();
    let clock = &Clock::get().unwrap();

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(reward_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(rewards_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(arb_coffer_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_reward_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(reward_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    let mut marinade_data: &[u8] = &marinade_state_ai.try_borrow_data().unwrap();
    let marinade_state = MarinadeState::try_deserialize(&mut marinade_data).unwrap();

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check_eq!(&system_state.arb_coffer.address, arb_coffer_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.staking_state, staking_state_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.reward_mint.address, reward_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.rewards_vault.address, rewards_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    let mut staking_state: RefMut<StakingState> = StakingState::load_mut_checked(staking_state_ai, program_id)?;
    if staking_state.last_drop_timestamp != 0 {
        check!(staking_state.last_drop_timestamp + system_state.epoch <= clock.unix_timestamp, LucraErrorCode::EarlyRewardDrop)?;
    }

    // Check that the last reward is not the new one
    if staking_state.current_reward_pubkey != Pubkey::default() {
        check!(&staking_state.current_reward_pubkey != reward_ai.key, LucraErrorCode::InvalidAccountInput)?;
    }

    check!(
        rent.is_exempt(reward_ai.lamports(), size_of::<Reward>()),
        LucraErrorCode::NotRentExempt
    )?;
    let mut reward: RefMut<Reward> = Reward::load_mut(reward_ai)?;
    check!(!reward.meta_data.is_initialized, LucraErrorCode::Default)?;

    let stake_mint = Mint::unpack(&stake_mint_ai.data.borrow())?;
    let msol_vault = Account::unpack(&msol_vault_ai.data.borrow())?;
    let user_token_account_mint = get_token_account_mint(user_reward_account_ai)?;

    check_eq!(&user_token_account_mint, reward_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&staking_state.stake_mint.address, stake_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.msol_vault.address, msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let msol_lamport_value = marinade_state.calc_msol_from_lamports(system_state.total_sol_collateral).unwrap();
    let total = msol_vault.amount
        .checked_sub(msol_lamport_value)
        .ok_or(math_err!())?;
    let half_of_total = total
        .checked_div(2)
        .ok_or(math_err!())?;

    reward.meta_data = MetaData::new(DataType::Reward, 0, true);
    reward.previous_reward = staking_state.current_reward_pubkey;
    reward.pool_token_supply = stake_mint.supply;
    reward.reward_cursor = staking_state.reward_cursor;
    reward.total = half_of_total;
    reward.start_timestamp = clock.unix_timestamp;

    staking_state.last_reward = half_of_total;
    staking_state.current_reward_pubkey = *reward_ai.key;
    staking_state.last_drop_timestamp = clock.unix_timestamp;
    staking_state.increment_reward_cursor();

    // Transfer half to the arb_coffer
    system_state.transfer_from_msol_vault(
        program_id, 
        msol_vault_ai, 
        arb_coffer_ai, 
        msol_vault_transfer_authority_ai, 
        token_program_ai, 
        half_of_total
    )?;

    // Transfer other half to the rewards vault
    system_state.transfer_from_msol_vault(
        program_id,
        msol_vault_ai,
        rewards_vault_ai,
        msol_vault_transfer_authority_ai,
        token_program_ai,
        half_of_total,
    )?;

    // Pay the user for their efforts
    system_state.mint_reward(
        program_id,
        reward_mint_ai,
        user_reward_account_ai,
        1,
        reward_mint_authority_ai,
        token_program_ai,
    )?;

    Ok(())
}
