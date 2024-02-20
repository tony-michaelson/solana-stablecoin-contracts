use std::{
    cell::RefMut,
    mem::size_of,
};

use arrayref::array_ref;
use legends_loadable_trait::Loadable;
use spl_token::state::Account;
use solana_program::{
    account_info::AccountInfo,
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    program_pack::Pack,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
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
        account::{verify_account_will_still_have_lamports, add_lamports, close_account},
        constants::{LAMPORTS_PER_MATA, SOL_USDC_ORACLE, SOL_USDT_ORACLE, LUCRA_SOL_ORACLE, SOL_MATA_ORACLE },
        spl::*,
        oracle::*,
        marinade::deposit,
    },
    state::{
        DataType,
        MetaData,
        MataLoan,
        LoanType,
        SystemState,
        staking::StakingAccount,
    },
};

declare_check_assert_macros!(SourceFileId::BeginCreateMataLoan);

const CREATE_MATA_LOAN_SIZE: usize = 22;

#[inline(never)]
pub fn process_create_mata_loan(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    if accounts.len() == CREATE_MATA_LOAN_SIZE {
        create_mata_loan(program_id, lamports, accounts)
    } else {
        create_mata_loan_with_locked_stake(program_id, lamports, accounts)
    }
}

#[inline(never)]
fn create_mata_loan(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = CREATE_MATA_LOAN_SIZE;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // write
        marinade_state_ai,              // write
        loan_ai,                        // write
        msol_vault_ai,                  // write
        mata_mint_ai,                   // write
        mata_mint_authority_ai,         // write

        user_account_ai,                // write
        user_mata_account_ai,           // write
        user_msol_account_ai,           // write

        sol_usdc_oracle_ai,             // read
        sol_usdt_oracle_ai,             // read
        sol_mata_oracle_ai,             // read

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

    // Verify proper signers
    check_eq!(user_account_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    // Verify accounts are owned by the right programs
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdt_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdc_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_mata_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(sol_usdc_oracle_ai.key, &SOL_USDC_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.key, &SOL_USDT_ORACLE, LucraErrorCode::InvalidAccountInput)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check!(system_state.loans_enabled, LucraErrorCode::LoansNotEnabled)?;

    if system_state.peg_check_enabled {
        check_eq!(sol_mata_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
        check_eq!(sol_mata_oracle_ai.key, &SOL_MATA_ORACLE, LucraErrorCode::InvalidAccountInput)?;

        let mata_market_price = get_mata_price(sol_mata_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
        system_state.update_peg(mata_market_price)?;
    }

    let sol_market_price = get_sol_price(sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let lamport_dollar_value = sol_market_price
        .checked_mul(lamports.into())
        .ok_or(math_err!())?
        .checked_div(LAMPORTS_PER_SOL.into())
        .ok_or(math_err!())?;
    let loan_amount = get_loan_amount(lamport_dollar_value, system_state.collateral_requirement)?;

    create_loan(
        program_id,
        &mut system_state,
        msol_vault_ai,
        mata_mint_ai,
        mata_mint_authority_ai,
        marinade_program_ai,
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        liq_pool_msol_leg_authority_ai,
        reserve_pda_ai,
        msol_mint_authority_ai,
        user_account_ai,
        user_mata_account_ai,
        user_msol_account_ai,
        loan_ai,
        fees_ai,
        system_program_ai,
        token_program_ai,
        lamports,
        loan_amount,
        0,
        sol_market_price.floor().to_u64().ok_or(math_err!())?,
        LoanType::Default,
    )?;

    Ok(())
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn create_mata_loan_with_locked_stake(program_id: &Pubkey, lamports: u64, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 24;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // write
        marinade_state_ai,              // write
        loan_ai,                        // write
        msol_vault_ai,                  // write
        mata_mint_ai,                   // write
        mata_mint_authority_ai,         // write
        user_account_ai,                // write
        user_mata_account_ai,           // write
        user_msol_account_ai,           // write
        user_staking_account_ai,        // write

        sol_usdc_oracle_ai,             // read
        sol_usdt_oracle_ai,             // read
        sol_mata_oracle_ai,             // read
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

    // Verify Signers
    check_eq!(user_account_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    // Verify accounts are owned by the right programs
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_staking_account_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_mata_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdt_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdc_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(lucra_sol_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(sol_usdc_oracle_ai.key, &SOL_USDC_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.key, &SOL_USDT_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(lucra_sol_oracle_ai.key, &LUCRA_SOL_ORACLE, LucraErrorCode::InvalidAccountInput)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check!(system_state.loans_enabled, LucraErrorCode::LoansNotEnabled)?;

    let mut user_staking_account: RefMut<StakingAccount> = StakingAccount::load_mut_checked(user_staking_account_ai, program_id)?;
    check_eq!(&user_staking_account.owner, user_account_ai.key, LucraErrorCode::InvalidAccountInput)?;

    if system_state.peg_check_enabled {
        check_eq!(sol_mata_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
        check_eq!(sol_mata_oracle_ai.key, &SOL_MATA_ORACLE, LucraErrorCode::InvalidAccountInput)?;

        let mata_market_price = get_mata_price(sol_mata_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
        system_state.update_peg(mata_market_price)?;
    }

    let sol_market_price = get_sol_price(sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let lamport_dollar_value = sol_market_price
        .checked_mul(lamports.into())
        .ok_or(math_err!())?
        .checked_div(LAMPORTS_PER_SOL.into())
        .ok_or(math_err!())?;
    let staking_value_required = get_required_stake_value(system_state.lcp, lamports, sol_market_price)?;
    let total_value_supplied = lamport_dollar_value
        .checked_add(staking_value_required)
        .ok_or(math_err!())?;

    let loan_amount = get_loan_amount(total_value_supplied, system_state.collateral_requirement)?;

    let lucra_market_price = get_lucra_price(lucra_sol_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let value_left_to_lock: u64 = Decimal::from(user_staking_account.total)
        .checked_mul(lucra_market_price)
        .ok_or(math_err!())?
        .checked_sub(user_staking_account.locked_total.into())
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;

    check!(staking_value_required.floor().to_u64().unwrap() <= value_left_to_lock, LucraErrorCode::InvalidAmount)?;

    create_loan(
        program_id,
        &mut system_state,
        msol_vault_ai,
        mata_mint_ai,
        mata_mint_authority_ai,
        marinade_program_ai,
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        liq_pool_msol_leg_authority_ai,
        reserve_pda_ai,
        msol_mint_authority_ai,
        user_account_ai,
        user_mata_account_ai,
        user_msol_account_ai,
        loan_ai,
        fees_ai,
        system_program_ai,
        token_program_ai,
        lamports,
        loan_amount,
        staking_value_required.floor().to_u64().ok_or(math_err!())?,
        sol_market_price.floor().to_u64().ok_or(math_err!())?,
        LoanType::LucraBacked,
    )?;

    user_staking_account.add_locked_total(staking_value_required.floor().to_u64().ok_or(math_err!())?);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_loan<'a>(
    program_id: &Pubkey,
    system_state: &mut SystemState,
    msol_vault_ai: &AccountInfo<'a>,
    mata_mint_ai: &AccountInfo<'a>,
    mata_mint_authority_ai: &AccountInfo<'a>,

    marinade_program_ai: &AccountInfo<'a>,
    marinade_state_ai: &AccountInfo<'a>,
    msol_mint_ai: &AccountInfo<'a>,
    liq_pool_sol_leg_pda_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_authority_ai: &AccountInfo<'a>,
    reserve_pda_ai: &AccountInfo<'a>,
    msol_mint_authority_ai: &AccountInfo<'a>,
    
    user_account_ai: &AccountInfo<'a>,
    user_mata_account_ai: &AccountInfo<'a>,
    user_msol_account_ai: &AccountInfo<'a>,
    loan_ai: &AccountInfo<'a>,

    fees_ai: &AccountInfo<'a>,
    system_program_ai: &AccountInfo<'a>,
    token_program_ai: &AccountInfo<'a>,
    
    lamports: u64,
    loan_amount: u64,
    staking_collateral_amount: u64,
    sol_market_price: u64,
    loan_type: LoanType,
) -> LucraResult {
    if !(system_state.peg_check_enabled && system_state.peg_broken) {
        _create_loan(
            program_id,
            system_state,
            msol_vault_ai,
            mata_mint_ai,
            mata_mint_authority_ai,
            marinade_program_ai,
            marinade_state_ai,
            msol_mint_ai,
            liq_pool_sol_leg_pda_ai,
            liq_pool_msol_leg_ai,
            liq_pool_msol_leg_authority_ai,
            reserve_pda_ai,
            msol_mint_authority_ai,
            user_account_ai,
            user_mata_account_ai,
            user_msol_account_ai,
            loan_ai,
            fees_ai,
            system_program_ai,
            token_program_ai,
            lamports,
            loan_amount,
            staking_collateral_amount,
            sol_market_price,
            loan_type,
        )?;    
    } else {
        // No data was initialized so there is nothing that can be done if an attacker opens the account
        // back up after we close it.
        let lamports = close_account(loan_ai);
        add_lamports(user_account_ai, lamports);
    }

    Ok(())
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub fn _create_loan<'a>(
    program_id: &Pubkey,
    system_state: &mut SystemState,
    msol_vault_ai: &AccountInfo<'a>,
    mata_mint_ai: &AccountInfo<'a>,
    mata_mint_authority_ai: &AccountInfo<'a>,

    marinade_program_ai: &AccountInfo<'a>,
    marinade_state_ai: &AccountInfo<'a>,
    msol_mint_ai: &AccountInfo<'a>,
    liq_pool_sol_leg_pda_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_ai: &AccountInfo<'a>,
    liq_pool_msol_leg_authority_ai: &AccountInfo<'a>,
    reserve_pda_ai: &AccountInfo<'a>,
    msol_mint_authority_ai: &AccountInfo<'a>,
    
    user_account_ai: &AccountInfo<'a>,
    user_mata_account_ai: &AccountInfo<'a>,
    user_msol_account_ai: &AccountInfo<'a>,
    loan_ai: &AccountInfo<'a>,

    fees_ai: &AccountInfo<'a>,
    system_program_ai: &AccountInfo<'a>,
    token_program_ai: &AccountInfo<'a>,
    
    lamports: u64,
    loan_amount: u64,
    staking_collateral_amount: u64,
    sol_market_price: u64,
    loan_type: LoanType,
) -> LucraResult {
    let clock = &Clock::get()?;
    let rent = &Rent::get()?;

    system_state.add_outstanding_mata(loan_amount)?;

    // Verify Loan account is created but not initialized
    check!(
        rent.is_exempt(loan_ai.lamports(), size_of::<MataLoan>()),
        LucraErrorCode::NotRentExempt
    )?;
    let mut loan: RefMut<MataLoan> = MataLoan::load_mut(loan_ai)?;
    check!(!loan.meta_data.is_initialized, LucraErrorCode::Default)?;

    let user_mata_account = Account::unpack(&user_mata_account_ai.data.borrow())?;
    check_eq!(user_mata_account.mint, system_state.mata_mint.address, LucraErrorCode::InvalidAccountInput)?;

    verify_account_will_still_have_lamports(fees_ai, user_account_ai.lamports(), lamports)?;    
    check!(lamports > system_state.min_deposit, LucraErrorCode::InvalidAmount)?;
    check!(user_account_ai.key != msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.mata_mint.address, mata_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.msol_vault.address, msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;
    
    let msol_balance_before = get_token_balance(user_msol_account_ai)?;
    deposit(
        marinade_state_ai,
        msol_mint_ai,
        liq_pool_sol_leg_pda_ai,
        liq_pool_msol_leg_ai,
        liq_pool_msol_leg_authority_ai,
        reserve_pda_ai,
        user_account_ai,
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

    spl_token_transfer(
        user_msol_account_ai,
        msol_vault_ai,
        msol_received,
        user_account_ai,
        &[],
        token_program_ai
    )?;

    system_state.mint_mata(
        program_id,
        mata_mint_ai,
        user_mata_account_ai,
        loan_amount,
        mata_mint_authority_ai,
        token_program_ai,
    )?;

    loan.meta_data = MetaData::new(DataType::Loan, 0, true);
    loan.repaid = false;
    loan.loan_type = loan_type;
    loan.owner = *user_account_ai.key;
    loan.collateral_rate = system_state.collateral_requirement;
    loan.sol_collateral_amount = lamports;
    loan.staking_collateral_amount = staking_collateral_amount;
    loan.market_price = sol_market_price;
    loan.loan_amount = loan_amount;
    loan.penalty_harvested = 0;
    loan.penalty_to_harvest = 0;
    loan.loan_mint = *mata_mint_ai.key;
    loan.loan_creation_date = clock.unix_timestamp;
    loan.last_day_penalty_was_checked = clock.unix_timestamp;

    system_state.add_collateral(lamports);

    Ok(())
}

#[inline(never)]
fn get_required_stake_value(lcp: u8, lamports: u64, sol_price: Decimal) -> LucraResult<Decimal> {
    let lcp = Decimal::new(lcp.into(), 2);
    Decimal::from(lamports)
        .checked_mul(lcp)
        .ok_or(math_err!())?
        .checked_mul(sol_price)
        .ok_or(math_err!())?
        .checked_div(LAMPORTS_PER_SOL.into())
        .ok_or(math_err!())
}

// Supplied_collateral is in dollars
#[inline(never)]
fn get_loan_amount(supplied_collateral: Decimal, collateral_requirement: u32) -> LucraResult<u64> {
    let collateral_requirement = Decimal::new(collateral_requirement as i64, 2);
    let loan_amount = supplied_collateral
        .checked_div(collateral_requirement)
        .ok_or(math_err!())?
        .checked_mul(LAMPORTS_PER_MATA)
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;

    Ok(loan_amount)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_required_stake() {
        let lcp = 100;
        let lamports = 10 * LAMPORTS_PER_SOL; // 10 sol
        let sol_price = Decimal::new(10, 0);
        
        let expected = sol_price.checked_mul(lamports.into()).unwrap().checked_div(LAMPORTS_PER_SOL.into()).unwrap();
        let actual = get_required_stake_value(lcp, lamports, sol_price).unwrap();

        assert_eq!(actual, expected);

        let lcp = 50;
        let expected = Decimal::from(50);
        let actual = get_required_stake_value(lcp, lamports, sol_price).unwrap();

        assert_eq!(actual, expected);

        let lcp = 150;
        let expected = Decimal::from(150);
        let actual = get_required_stake_value(lcp, lamports, sol_price).unwrap();

        assert_eq!(actual, expected);
    }
}