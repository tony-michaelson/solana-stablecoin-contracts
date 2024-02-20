use std::{
    cell::RefMut,
    mem::size_of,
};

use anchor_lang::AccountDeserialize;
use arrayref::array_ref;
use marinade_finance::state::State as MarinadeState;
use solana_program::{account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey, sysvar::{clock::Clock, rent::Rent, Sysvar}};
use spl_token::state::{Mint, Account};
use legends_loadable_trait::Loadable;
use time::{OffsetDateTime, Time};
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::spl::*,
    helpers::constants::{CREATOR_AUTHORITY, SOL_FEE_PLUS_INTEREST},
    state::{
        ArbState, 
        Limit, 
        StateEnum,
        DataType,
        MetaData,
        SystemState,
        StateParams,
        staking::StakingState,
    },
};

declare_check_assert_macros!(SourceFileId::Initialize);

#[inline(never)]
pub fn process_initialize(program_id: &Pubkey, state_params: &StateParams, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 16;
    let accounts = array_ref!(accounts, 0, NUM_FIXED);
    let [
        marinade_state_ai,              // read
        creator_authority_ai,           // read
        mata_mint_ai,                   // read
        lucra_mint_ai,                  // read
        reward_mint_ai,                 // read
        staked_lucra_mint_ai,           // read
        system_state_ai,                // write
        arb_state_ai,                   // write
        msol_vault_ai,                  // read
        arb_coffer_ai,                  // read
        rewards_vault_ai,               // read
        staking_state_ai,               // write
        arb_fund_ai,                    // read
        wsol_holding_vault_ai,          // read
        mata_holding_vault_ai,          // read
        lucra_holding_vault_ai,         // read
    ] = accounts;

    let clock = &Clock::get()?;
    let rent = &Rent::get()?;

    check_eq!(creator_authority_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;
    check_eq!(creator_authority_ai.key, &CREATOR_AUTHORITY, LucraErrorCode::InvalidAccountInput)?;

    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(arb_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    
    check!(rent.is_exempt(system_state_ai.lamports(), size_of::<SystemState>()), LucraErrorCode::Default)?;
    let mut state: RefMut<SystemState> = SystemState::load_mut(system_state_ai)?;
    check!(!state.meta_data.is_initialized, LucraErrorCode::Default)?;
    
    check!(rent.is_exempt(staking_state_ai.lamports(), size_of::<StakingState>()), LucraErrorCode::Default)?;
    let mut staking_state: RefMut<StakingState> = StakingState::load_mut(staking_state_ai)?;
    check!(!staking_state.meta_data.is_initialized, LucraErrorCode::Default)?;

    check!(rent.is_exempt(arb_state_ai.lamports(), size_of::<ArbState>()), LucraErrorCode::Default)?;
    let mut arb_state: RefMut<ArbState> = ArbState::load_mut(arb_state_ai)?;
    check!(!arb_state.meta_data.is_initialized, LucraErrorCode::Default)?;

    msg!("1");
    let mata_mint_authority_bump_seed = verify_mata_mint(system_state_ai, mata_mint_ai)?;
    msg!("2");
    let lucra_mint_authority_bump_seed = verify_lucra_mint(system_state_ai, lucra_mint_ai)?;
    msg!("3");
    let msol_vault_authority_bump_seed = verify_msol_vault(system_state_ai, marinade_state_ai, msol_vault_ai)?;
    msg!("4");
    let arb_coffer_authority_bump_seed = verify_arb_coffer(system_state_ai, arb_coffer_ai)?;
    msg!("5");
    let reward_mint_authority_bump_seed = verify_reward_mint(system_state_ai, reward_mint_ai)?;
    msg!("6");
    let staked_lucra_mint_authority_bump_seed = verify_staked_lucra_mint(staking_state_ai, staked_lucra_mint_ai)?;
    msg!("7");
    let rewards_vault_authority_bump_seed = verify_rewards_vault(system_state_ai, rewards_vault_ai)?;
    msg!("8");
    let arb_fund_authority_bump_seed = verify_arb_fund(arb_state_ai, arb_fund_ai)?;
    msg!("9");
    let wsol_holding_vault_authority_bump_seed = verify_wsol_holding_vault(arb_state_ai, wsol_holding_vault_ai)?;
    msg!("10");
    let mata_holding_vault_authority_bump_seed = verify_mata_holding_vault(arb_state_ai, mata_holding_vault_ai, mata_mint_ai.key)?;
    msg!("11");
    let lucra_holding_vault_authority_bump_seed = verify_lucra_holding_vault(arb_state_ai, lucra_holding_vault_ai, lucra_mint_ai.key)?;

    // Initialize System State
    state.meta_data = MetaData::new(DataType::SystemState, 0, true);
    state.key = *system_state_ai.key;
    state.staking_state = *staking_state_ai.key;
    state.arb_state = *arb_state_ai.key;
    state.min_deposit = state_params.min_deposit;
    state.collateral_requirement = state_params.collateral_requirement;
    state.maximum_outstanding_mata = state_params.maximum_outstanding_mata;
    state.minimum_harvest_amount = SOL_FEE_PLUS_INTEREST as u64 * 100;
    state.total_outstanding_mata = 0;
    state.reward_fee = SOL_FEE_PLUS_INTEREST;                // Reward fee = 1 sol fee plus 10%
    state.epoch = state_params.epoch;
    state.lucra_mint.address = *lucra_mint_ai.key;
    state.lucra_mint.authority_bump_seed = lucra_mint_authority_bump_seed;
    state.mata_mint.address = *mata_mint_ai.key;
    state.mata_mint.authority_bump_seed = mata_mint_authority_bump_seed;
    state.reward_mint.address = *reward_mint_ai.key;
    state.reward_mint.authority_bump_seed = reward_mint_authority_bump_seed;
    state.msol_vault.address = *msol_vault_ai.key;
    state.msol_vault.authority_bump_seed = msol_vault_authority_bump_seed;
    state.arb_coffer.address = *arb_coffer_ai.key;
    state.arb_coffer.authority_bump_seed = arb_coffer_authority_bump_seed;
    state.rewards_vault.address = *rewards_vault_ai.key;
    state.rewards_vault.authority_bump_seed = rewards_vault_authority_bump_seed;
    state.total_sol_collateral = 0;
    state.staking_enabled = state_params.staking_enabled;
    state.loans_enabled = state_params.loans_enabled;
    state.arbitrage_enabled = state_params.arbitrage_enabled;
    state.peg_check_enabled = state_params.peg_check_enabled;
    state.peg_broken = false;
    state.lcp = state_params.lcp;
    
    // Initialize Staking State
    staking_state.meta_data = MetaData::new(DataType::StakingState, 0, true);
    staking_state.key = *staking_state_ai.key;
    staking_state.current_reward_pubkey = Pubkey::default();
    staking_state.stake_mint.address = *staked_lucra_mint_ai.key;
    staking_state.stake_mint.authority_bump_seed = staked_lucra_mint_authority_bump_seed;
    staking_state.last_reward = 0;
    staking_state.last_drop_timestamp = 0;
    staking_state.reward_cursor = 0;
    
    // Initialize Arb State
    arb_state.meta_data = MetaData::new(DataType::ArbState, 0, true);
    arb_state.key = *arb_state_ai.key;
    arb_state.daily_limit = state_params.daily_arb_limit;
    arb_state.max_amount_of_lucra_to_mint = state_params.max_amount_of_lucra_to_mint;
    arb_state.buying_lucra = false;
    let start_of_day = OffsetDateTime::from_unix_timestamp(clock.unix_timestamp)
        .unwrap()
        .replace_time(Time::from_hms(0, 0, 0).unwrap())
        .unix_timestamp();
    arb_state.start_of_day_timestamp = start_of_day;
    arb_state.agg_limit = state_params.daily_arb_limit;
    arb_state.rolling_limits = [Limit {
            date: 0,
            limit: 0,
        }; 30];
    arb_state.rolling_limits[0].date = start_of_day;
    arb_state.rolling_limits[0].limit = state_params.daily_arb_limit;
    arb_state.arb_fund.address = *arb_fund_ai.key;
    arb_state.arb_fund.authority_bump_seed = arb_fund_authority_bump_seed;
    arb_state.wsol_holding_vault.address = *wsol_holding_vault_ai.key;
    arb_state.wsol_holding_vault.authority_bump_seed = wsol_holding_vault_authority_bump_seed;
    arb_state.mata_holding_vault.address = *mata_holding_vault_ai.key;
    arb_state.mata_holding_vault.authority_bump_seed = mata_holding_vault_authority_bump_seed;
    arb_state.lucra_holding_vault.address = *lucra_holding_vault_ai.key;
    arb_state.lucra_holding_vault.authority_bump_seed = lucra_holding_vault_authority_bump_seed;
    arb_state.state = StateEnum::Minting;

    Ok(())
}

fn verify_mata_mint(
    system_state_ai: &AccountInfo,
    mata_mint_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (mata_mint_authority_address, mata_mint_authority_bump_seed) = SystemState::find_mata_mint_authority(system_state_ai.key);
    verify_mint(&mata_mint_authority_address, mata_mint_ai)?;

    Ok(mata_mint_authority_bump_seed)
}

fn verify_lucra_mint(
    system_state_ai: &AccountInfo,
    lucra_mint_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (lucra_mint_authority_address, lucra_mint_authority_bump_seed) = SystemState::find_lucra_mint_authority(system_state_ai.key);
    verify_mint(&lucra_mint_authority_address, lucra_mint_ai)?;

    Ok(lucra_mint_authority_bump_seed)
}

fn verify_reward_mint(
    system_state_ai: &AccountInfo,
    reward_mint_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (reward_mint_authority_address, reward_mint_authority_bump_seed) = SystemState::find_reward_mint_authority(system_state_ai.key);
    verify_mint(&reward_mint_authority_address, reward_mint_ai)?;

    let reward_mint = Mint::unpack(&reward_mint_ai.data.borrow())?;
    check!(reward_mint.supply == 0, LucraErrorCode::InvalidAccountInput)?;

    Ok(reward_mint_authority_bump_seed)
}

fn verify_staked_lucra_mint(
    staking_state_ai: &AccountInfo,
    staked_lucra_mint_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (staked_lucra_mint_authority_address, staked_lucra_mint_authority_bump_seed) = StakingState::find_stake_mint_authority(staking_state_ai.key);
    verify_mint(&staked_lucra_mint_authority_address, staked_lucra_mint_ai)?;

    let staked_lucra_mint = Mint::unpack(&staked_lucra_mint_ai.data.borrow())?;
    check!(staked_lucra_mint.supply == 0, LucraErrorCode::InvalidAccountInput)?;

    Ok(staked_lucra_mint_authority_bump_seed)
}

fn verify_mint(
    address_to_verify: &Pubkey,
    mint_ai: &AccountInfo,
) -> LucraResult {
    let mint = Mint::unpack(&mint_ai.data.borrow())?;
    check_eq!(mint_ai.owner, &spl_token::ID, LucraErrorCode::InvalidAccountOwner)?;
    check!(mint.mint_authority.contains(address_to_verify), LucraErrorCode::InvalidAccountInput)?;
    check!(mint.freeze_authority.is_none(), LucraErrorCode::InvalidAccountInput)?;

    Ok(())
}

fn verify_msol_vault(
    system_state_ai: &AccountInfo,
    marinade_state_ai: &AccountInfo,
    msol_vault_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (msol_vault_authority_address, msol_vault_authority_bump_seed) = SystemState::find_msol_vault_authority(system_state_ai.key);
    verify_vault(&msol_vault_authority_address, msol_vault_ai)?;
    
    let msol_vault = Account::unpack(&msol_vault_ai.data.borrow())?;
    let mut marinade_data: &[u8] = &marinade_state_ai.try_borrow_data().unwrap();
    let marinade_state = MarinadeState::try_deserialize(&mut marinade_data).unwrap();
    check_eq!(&msol_vault.mint, &marinade_state.msol_mint, LucraErrorCode::InvalidAccountInput)?;
    check!(msol_vault.delegate.is_none(), LucraErrorCode::InvalidAccountInput)?;
    check!(msol_vault.close_authority.is_none(), LucraErrorCode::InvalidAccountInput)?;

    Ok(msol_vault_authority_bump_seed)
}

fn verify_arb_coffer(
    system_state_ai: &AccountInfo,
    arb_coffer_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (arb_coffer_authority_address, arb_coffer_authority_bump_seed) = SystemState::find_arb_coffer_authority(system_state_ai.key);
    verify_vault(&arb_coffer_authority_address, arb_coffer_ai)?;

    Ok(arb_coffer_authority_bump_seed)
}

fn verify_rewards_vault(
    system_state_ai: &AccountInfo,
    rewards_vault_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (rewards_vault_authority_address, rewards_vault_authority_bump_seed) = SystemState::find_rewards_vault_authority(system_state_ai.key);
    verify_vault(&rewards_vault_authority_address, rewards_vault_ai)?;

    let rewards_vault_balance = get_token_balance(rewards_vault_ai)?;
    check!(rewards_vault_balance == 0, LucraErrorCode::InvalidAmount)?;

    Ok(rewards_vault_authority_bump_seed)
}

fn verify_arb_fund(
    arb_state_ai: &AccountInfo,
    arb_fund_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (arb_fund_authority_address, arb_fund_authority_bump_seed) = ArbState::find_arb_fund_authority(arb_state_ai.key);
    msg!("arb_state_ai: {:?}", arb_state_ai.key.to_string());
    msg!("arb_fund_authority_address: {:?}", arb_fund_authority_address);
    msg!("arb_fund_ai: {:?}", arb_fund_ai.key.to_string());
    verify_vault(&arb_fund_authority_address, arb_fund_ai)?;

    let arb_fund_balance = get_token_balance(arb_fund_ai)?;
    check!(arb_fund_balance == 0, LucraErrorCode::InvalidAmount)?;

    Ok(arb_fund_authority_bump_seed)
}

fn verify_wsol_holding_vault(
    arb_state_ai: &AccountInfo,
    wsol_holding_vault_ai: &AccountInfo,
) -> LucraResult<u8> {
    let (wsol_holding_vault_authority_address, wsol_holding_vault_authority_bump_seed) = ArbState::find_wsol_holding_vault_authority(arb_state_ai.key);
    verify_vault(&wsol_holding_vault_authority_address, wsol_holding_vault_ai)?;

    let wsol_holding_vault_balance = get_token_balance(wsol_holding_vault_ai)?;
    check!(wsol_holding_vault_balance == 0, LucraErrorCode::InvalidAmount)?;

    Ok(wsol_holding_vault_authority_bump_seed)
}

fn verify_mata_holding_vault(
    arb_state_ai: &AccountInfo,
    mata_holding_vault_ai: &AccountInfo,
    mata_mint: &Pubkey,
) -> LucraResult<u8> {
    let (mata_holding_vault_authority_address, mata_holding_vault_authority_bump_seed) = ArbState::find_mata_holding_vault_authority(arb_state_ai.key);
    verify_vault_and_mint(&mata_holding_vault_authority_address, mata_holding_vault_ai, mata_mint)?;

    let mata_holding_vault_balance = get_token_balance(mata_holding_vault_ai)?;
    check!(mata_holding_vault_balance == 0, LucraErrorCode::InvalidAmount)?;

    Ok(mata_holding_vault_authority_bump_seed)
}

fn verify_lucra_holding_vault(
    arb_state_ai: &AccountInfo,
    lucra_holding_vault_ai: &AccountInfo,
    lucra_mint: &Pubkey,
) -> LucraResult<u8> {
    let (lucra_holding_vault_authority_address, lucra_holding_vault_authority_bump_seed) = ArbState::find_lucra_holding_vault_authority(arb_state_ai.key);
    verify_vault_and_mint(&lucra_holding_vault_authority_address, lucra_holding_vault_ai, lucra_mint)?;

    let lucra_holding_vault_balance = get_token_balance(lucra_holding_vault_ai)?;
    check!(lucra_holding_vault_balance == 0, LucraErrorCode::InvalidAmount)?;

    Ok(lucra_holding_vault_authority_bump_seed)
}

fn verify_vault(
    address_to_verify: &Pubkey,
    vault_ai: &AccountInfo,
) -> LucraResult {
    let vault = Account::unpack(&vault_ai.data.borrow())?;
    check_eq!(vault_ai.owner, &spl_token::ID, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(&vault.owner, address_to_verify, LucraErrorCode::InvalidAccountOwner)?;

    Ok(())
}

fn verify_vault_and_mint(
    address_to_verify: &Pubkey,
    vault_ai: &AccountInfo,
    mint: &Pubkey,
) -> LucraResult {
    let vault = Account::unpack(&vault_ai.data.borrow())?;
    check_eq!(vault_ai.owner, &spl_token::ID, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(&vault.owner, address_to_verify, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(&vault.mint, mint, LucraErrorCode::InvalidAccountOwner)?;

    Ok(())
}