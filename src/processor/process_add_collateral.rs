use std::{
    cell::RefMut,
};

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::{
        account::{verify_account_will_still_have_lamports},
        constants::{SOL_USDC_ORACLE, SOL_USDT_ORACLE, LUCRA_SOL_ORACLE },
        spl::*,
        oracle::*,
        marinade::deposit,
    },
    state::{
        MataLoan,
        LoanType,
        SystemState,
        staking::StakingAccount,
    },
};

declare_check_assert_macros!(SourceFileId::AddCollateral);

const ADD_COLLATERAL_SIZE: usize = 16;

#[inline(never)]
pub fn process_add_collateral(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    if accounts.len() == ADD_COLLATERAL_SIZE {
        add_collateral(program_id, lamports, accounts)
    } else {
        add_collateral_with_locked_stake(program_id, lamports, accounts)
    }
}

#[inline(never)]
pub fn add_collateral(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 16;
    let accounts = array_ref!(accounts, 0, NUM_FIXED);
    let [
        system_state_ai,                // write
        marinade_state_ai,              // write
        loan_ai,                        // write
        msol_vault_ai,                  // write
        owner_ai,                       // write
        user_msol_account_ai,           // write

        msol_mint_ai,                   // write
        liq_pool_sol_leg_pda_ai,        // write
        liq_pool_msol_leg_ai,           // write
        liq_pool_msol_leg_authority_ai, // read
        reserve_pda_ai,                 // write
        msol_mint_authority_ai,         // read
        fees_ai,                        // read
        system_program_ai,              // read
        token_program_ai,               // read
        marinade_program_ai,            // read
    ] = accounts;

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(user_msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;

    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check!(system_state.loans_enabled, LucraErrorCode::LoansNotEnabled)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.loan_type, LoanType::Default, LucraErrorCode::InvalidLoanType)?;

    add_additional_collateral(
        &mut system_state,
        &mut loan,
        msol_vault_ai,
        owner_ai,
        user_msol_account_ai,
        marinade_program_ai,
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        liq_pool_msol_leg_authority_ai,
        reserve_pda_ai,
        msol_mint_authority_ai,
        fees_ai,
        system_program_ai,
        token_program_ai,
        lamports,
        0
    )
}

#[inline(never)]
pub fn add_collateral_with_locked_stake(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 20;
    let accounts = array_ref!(accounts, 0, NUM_FIXED);
    let [
        system_state_ai,                // write
        marinade_state_ai,              // write
        loan_ai,                        // write
        msol_vault_ai,                  // write
        owner_ai,                       // write
        user_msol_account_ai,           // write
        user_staking_account_ai,        // write

        sol_usdc_oracle_ai,             // read
        sol_usdt_oracle_ai,             // read
        lucra_sol_oracle_ai,            // read

        msol_mint_ai,                   // write
        liq_pool_sol_leg_pda_ai,        // write
        liq_pool_msol_leg_ai,           // write
        liq_pool_msol_leg_authority_ai, // read
        reserve_pda_ai,                 // write
        msol_mint_authority_ai,         // read
        fees_ai,                        // read
        system_program_ai,              // read
        token_program_ai,               // read
        marinade_program_ai,            // read
    ] = accounts;

    let clock = &Clock::get()?;

    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(user_staking_account_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdc_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_sol_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(user_msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;

    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(sol_usdc_oracle_ai.key, &SOL_USDC_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.key, &SOL_USDT_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(lucra_sol_oracle_ai.key, &LUCRA_SOL_ORACLE, LucraErrorCode::InvalidAccountInput)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check!(system_state.loans_enabled, LucraErrorCode::LoansNotEnabled)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;

    let mut user_staking_account: RefMut<StakingAccount> = StakingAccount::load_mut_checked(user_staking_account_ai, program_id)?;
    check_eq!(&user_staking_account.owner, owner_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let sol_market_price = get_sol_price(sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let lamport_dollar_value = sol_market_price
        .checked_mul(lamports.into())
        .ok_or(math_err!())?
        .checked_div(LAMPORTS_PER_SOL.into())
        .ok_or(math_err!())?;
    let staking_value_required = lamport_dollar_value.floor().to_u64().unwrap();
    
    let lucra_market_price = get_lucra_price(lucra_sol_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let value_left_to_lock: u64 = Decimal::from(user_staking_account.total)
        .checked_mul(lucra_market_price)
        .ok_or(math_err!())?
        .checked_sub(user_staking_account.locked_total.into())
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;

    check!(staking_value_required <= value_left_to_lock, LucraErrorCode::InvalidAmount)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.loan_type, LoanType::LucraBacked, LucraErrorCode::InvalidLoanType)?;

    add_additional_collateral(
        &mut system_state,
        &mut loan,
        msol_vault_ai,
        owner_ai,
        user_msol_account_ai,
        marinade_program_ai,
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        liq_pool_msol_leg_authority_ai,
        reserve_pda_ai,
        msol_mint_authority_ai,
        fees_ai,
        system_program_ai,
        token_program_ai,
        lamports,
        staking_value_required,
    )?;

    user_staking_account.add_locked_total(staking_value_required);

    Ok(())
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn add_additional_collateral<'a>(
    system_state: &mut RefMut<SystemState>,
    loan: &mut RefMut<MataLoan>,
    msol_vault_ai: &AccountInfo<'a>,
    owner_ai: &AccountInfo<'a>,
    user_msol_account_ai: &AccountInfo<'a>,

    marinade_program_ai: &AccountInfo<'a>,
    marinade_state_ai: &AccountInfo<'a>,
    msol_mint_ai: &AccountInfo<'a>,
    liq_pool_sol_leg_pda_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_authority_ai: &AccountInfo<'a>,
    reserve_pda_ai: &AccountInfo<'a>,
    msol_mint_authority_ai: &AccountInfo<'a>,

    fees_ai: &AccountInfo<'a>,
    system_program_ai: &AccountInfo<'a>,
    token_program_ai: &AccountInfo<'a>,

    lamports: u64,
    staking_collateral_amount: u64,
) -> LucraResult {   
    check_eq!(loan.repaid, false, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&loan.owner, owner_ai.key, LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(&system_state.msol_vault.address, msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    verify_account_will_still_have_lamports(fees_ai, owner_ai.lamports(), lamports)?;
    check!(user_msol_account_ai.key != msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let msol_balance_before = get_token_balance(user_msol_account_ai)?;
    deposit(
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        liq_pool_msol_leg_authority_ai,
        reserve_pda_ai,
        owner_ai,
        user_msol_account_ai,
        msol_mint_authority_ai,
        &[],
        system_program_ai,
        token_program_ai,
        marinade_program_ai,
        lamports,
    )?;
    let msol_balance_after = get_token_balance(user_msol_account_ai)?;
    let msol_received = msol_balance_after - msol_balance_before;

    // Transfer the msol we recieved to the vault
    spl_token_transfer(
        user_msol_account_ai,
        msol_vault_ai,
        msol_received,
        owner_ai,
        &[],
        token_program_ai
    )?;

    loan.add_sol_collateral(lamports);
    loan.add_staking_collateral(staking_collateral_amount);

    system_state.add_collateral(lamports);

    Ok(())
}