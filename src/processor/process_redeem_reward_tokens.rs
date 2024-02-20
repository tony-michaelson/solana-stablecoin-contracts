use std::cell::Ref;
use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{clock::Clock, Sysvar},
    pubkey::Pubkey,
    program_pack::Pack,
};
use spl_token::state::Account;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::LUCRA_SOL_ORACLE,
    helpers::oracle::*,
    state::SystemState,
};

declare_check_assert_macros!(SourceFileId::RedeemRewardTokens);

#[inline(never)]
pub fn process_redeem_reward_tokens(program_id: &Pubkey, reward_tokens: u64, accounts: &[AccountInfo]) -> LucraResult {
    check!(reward_tokens != 0, LucraErrorCode::InvalidAmount)?;
    
    const NUM_FIXED: usize = 9;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // read
        
        user_reward_account_ai,         // write
        user_lucra_account_ai,          // write
        user_authority_ai,              // read

        reward_mint_ai,                 // write
        
        lucra_mint_ai,                  // write
        lucra_mint_authority_ai,        // read

        lucra_sol_oracle_ai,            // read

        token_program_ai,               // read
    ] = accounts;

    let clock = &Clock::get()?;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_sol_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_reward_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_lucra_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(reward_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(token_program_ai.key, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;

    check_eq!(lucra_sol_oracle_ai.key, &LUCRA_SOL_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(user_authority_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check_eq!(&system_state.lucra_mint.address, lucra_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    let reward_account = Account::unpack(&user_reward_account_ai.data.borrow())?;
    let lucra_account = Account::unpack(&user_lucra_account_ai.data.borrow())?;
    check_eq!(reward_mint_ai.key, &reward_account.mint, LucraErrorCode::InvalidAccountInput)?;
    check!(reward_account.amount >= reward_tokens, LucraErrorCode::InvalidAmount)?;
    check_eq!(reward_account.owner, lucra_account.owner, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(lucra_mint_ai.key, &lucra_account.mint, LucraErrorCode::InvalidAccountInput)?;

    let reward = system_state.reward_fee as u64;
    let total_reward_lamports = reward.checked_mul(reward_tokens)
        .ok_or(math_err!())?;

    let lucra_price = get_oracle_price(lucra_sol_oracle_ai, clock)?;
    let reward_to_mint = Decimal::from(total_reward_lamports)
        .checked_div(lucra_price)
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;

    system_state.mint_lucra(
        program_id,
        lucra_mint_ai,
        user_lucra_account_ai,
        reward_to_mint,
        lucra_mint_authority_ai,
        token_program_ai,
    )?;

    system_state.burn_reward(
        reward_mint_ai,
        user_reward_account_ai,
        reward_tokens,
        user_authority_ai,
        token_program_ai,
    )?;

    Ok(())
}