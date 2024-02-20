use std::cell::RefMut;

use anchor_lang::prelude::*;
use arrayref::array_ref;
use marinade_finance;
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
    helpers::marinade::liquid_unstake,
    state::{
        MataLoan,
        LoanType,
        staking::StakingAccount,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::CloseMataLoan);

const CLOSE_OUT_MATA_LOAN_SIZE: usize = 16;

#[inline(never)]
pub fn process_close_out_mata_loan(program_id: &Pubkey, unstake_msol: bool, accounts: &[AccountInfo]) -> LucraResult {
    if accounts.len() == CLOSE_OUT_MATA_LOAN_SIZE {
        close_out_mata_loan(program_id, unstake_msol, accounts)
    } else {
        close_out_mata_loan_with_locked_stake(program_id, unstake_msol, accounts)
    }
}

#[inline(never)]
fn close_out_mata_loan(program_id: &Pubkey, unstake_msol: bool, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = CLOSE_OUT_MATA_LOAN_SIZE;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,            // write
        marinade_state_ai,          // write
        loan_ai,                    // write
        user_account_ai,            // write
        user_msol_account_ai,       // write
        mata_mint_ai,               // write
        user_mata_account_ai,       // write
        msol_vault_authority_ai,    // read
        msol_vault_ai,              // write
        msol_mint_ai,               // write
        liq_pool_sol_leg_pda_ai,    // write
        liq_pool_msol_leg_ai,       // write
        treasury_msol_account_ai,   // write
        system_program_ai,          // read
        token_program_ai,           // read
        marinade_program_ai,        // read
    ] = accounts;

    check_eq!(user_account_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_mata_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check!(system_state.loans_enabled, LucraErrorCode::LoansNotEnabled)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.loan_type, LoanType::Default, LucraErrorCode::InvalidLoanType)?;

    close_loan(
        program_id,
        &mut system_state,
        &mut loan,
        mata_mint_ai,
        msol_vault_ai,
        msol_vault_authority_ai,
        
        marinade_program_ai,
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        treasury_msol_account_ai,
        
        user_account_ai,
        user_msol_account_ai,
        user_mata_account_ai,

        system_program_ai,
        token_program_ai,
        unstake_msol,
    )
}

#[inline(never)]
fn close_out_mata_loan_with_locked_stake(program_id: &Pubkey, unstake_msol: bool, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 17;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,            // write
        marinade_state_ai,          // write
        loan_ai,                    // write
        user_account_ai,            // write
        user_msol_account_ai,       // write
        mata_mint_ai,               // write
        user_mata_account_ai,       // write
        msol_vault_authority_ai,    // read
        msol_vault_ai,              // write
        staking_account_ai,         // write
        msol_mint_ai,               // write
        liq_pool_sol_leg_pda_ai,    // write
        liq_pool_msol_leg_ai,       // write
        treasury_msol_account_ai,   // write
        system_program_ai,          // read
        token_program_ai,           // read
        marinade_program_ai,        // read
    ] = accounts;

    check_eq!(user_account_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_account_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_mata_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check!(system_state.loans_enabled, LucraErrorCode::LoansNotEnabled)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.loan_type, LoanType::LucraBacked, LucraErrorCode::InvalidLoanType)?;
    let mut staking_account: RefMut<StakingAccount> = StakingAccount::load_mut_checked(staking_account_ai, program_id)?;

    check_eq!(&staking_account.owner, user_account_ai.key, LucraErrorCode::InvalidAccountInput)?;

    staking_account.remove_locked_total(loan.staking_collateral_amount);

    close_loan(
        program_id,
        &mut system_state,
        &mut loan,
        mata_mint_ai,
        msol_vault_ai,
        msol_vault_authority_ai,
        
        marinade_program_ai,
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        treasury_msol_account_ai,
        
        user_account_ai,
        user_msol_account_ai,
        user_mata_account_ai,

        system_program_ai,
        token_program_ai,
        unstake_msol,
    )?;

    Ok(())
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn close_loan<'a>(
    program_id: &Pubkey,
    system_state: &mut RefMut<SystemState>,
    loan: &mut RefMut<MataLoan>,
    mata_mint_ai: &AccountInfo<'a>,
    msol_vault_ai: &AccountInfo<'a>,
    msol_vault_authority_ai: &AccountInfo<'a>,
    
    marinade_program_ai: &AccountInfo<'a>,
    marinade_state_ai: &AccountInfo<'a>,
    msol_mint_ai: &AccountInfo<'a>,
    liq_pool_sol_leg_pda_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_ai: &AccountInfo<'a>,
    treasury_msol_account_ai: &AccountInfo<'a>,

    user_account_ai: &AccountInfo<'a>,
    user_msol_account_ai: &AccountInfo<'a>,
    user_mata_account_ai: &AccountInfo<'a>,

    system_program_ai: &AccountInfo<'a>,
    token_program_ai: &AccountInfo<'a>,
    unstake_msol: bool,
) -> LucraResult {
    let clock = &Clock::get()?;

    let user_mata_account = Account::unpack(&user_mata_account_ai.data.borrow())?;
    let user_msol_account = Account::unpack(&user_msol_account_ai.data.borrow())?;
    
    check!(loan.loan_creation_date + system_state.epoch < clock.unix_timestamp, LucraErrorCode::Timelock)?;
    check_eq!(loan.repaid, false, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&loan.owner, user_account_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&loan.loan_mint, mata_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&user_mata_account.mint, mata_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&user_mata_account.owner, user_account_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&user_msol_account.owner, user_account_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.mata_mint.address, mata_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.msol_vault.address, msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check!(user_mata_account.amount >= loan.loan_amount, LucraErrorCode::InvalidAmount)?;

    system_state.burn_mata(
        mata_mint_ai,
        user_mata_account_ai,
        loan.loan_amount,
        user_account_ai,
        token_program_ai,
    )?;

    let sol_to_return = loan.calc_remaining_sol();

    // Convert the lamports to msol
    let state = ProgramAccount::<marinade_finance::state::State>::try_from(marinade_program_ai.clone().key, &marinade_state_ai.clone()).unwrap();
    let msol_lamports = state.calc_msol_from_lamports(sol_to_return).unwrap();

    system_state.transfer_from_msol_vault(
        program_id,
        msol_vault_ai,
        user_msol_account_ai,
        msol_vault_authority_ai,
        token_program_ai,
        msol_lamports,
    )?;

    if unstake_msol {
        // Unstake collateral to user's sol account 
        liquid_unstake(
            marinade_state_ai,
            msol_mint_ai,
            liq_pool_sol_leg_pda_ai,
            liq_pool_msol_leg_ai,
            treasury_msol_account_ai,
            user_msol_account_ai,
            user_account_ai,
            user_account_ai,
            &[&[&[]]],
            token_program_ai,
            system_program_ai,
            marinade_program_ai,
            msol_lamports,
        )?;
    }

    loan.repaid();
    
    system_state.remove_collateral(sol_to_return);

    Ok(())
}