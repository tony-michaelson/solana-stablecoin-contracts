use std::cell::RefMut;

use anchor_lang::prelude::ProgramAccount;
use arrayref::array_ref;
use marinade_finance;
use oracles::state::Oracle;
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
};
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::{
        constants::{
            SOL_MATA_ORCA_AMM,
            orca_swap,
            SOL_MATA_ORACLE,
            serum_v3,
            raydium_v4,
            SOL_MATA_RAYDIUM_AMM,
        },
        raydium::swap as raydium_swap,
        spltokenswap::swap as orca_swap,
        spl::*,
        oracle::{verify_orca_has_more_volume, verify_raydium_has_more_volume},
        solana::transfer,
        marinade::liquid_unstake,
    },
    state::{
        AmmTypes,
        MataLoan,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::HarvestPenalty);

#[inline(never)]
pub fn process_harvest_penalty(program_id: &Pubkey, amm_type: AmmTypes, accounts: &[AccountInfo]) -> LucraResult {
    match amm_type {
        AmmTypes::None => Err(throw_err!(LucraErrorCode::NotImplemented)),
        AmmTypes::Orca => process_harvest_penalty_orca(program_id, accounts),
        AmmTypes::Raydium => process_harvest_penalty_raydium(program_id, accounts),
    }
}

#[inline(never)]
pub fn process_harvest_penalty_orca(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 25;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,            // write
        marinade_state_ai,          // write
        msol_vault_ai,              // write
        msol_vault_authority_ai,    // read
        mata_mint_ai,               // write
        
        loan_ai,                    // write
        sol_mata_oracle_ai,         // read

        user_account_ai,            // write
        user_wsol_account_ai,       // write
        user_mata_account_ai,       // write
        user_msol_account_ai,       // write

        // Accounts required for msol unstake
        msol_mint_ai,               // write
        liq_pool_sol_leg_pda_ai,    // write
        liq_pool_msol_leg_ai,       // write
        treasury_msol_account_ai,   // write
        system_program_ai,          // read
        marinade_program_ai,        // read

        // Accounts required for orca swap
        sm_amm_ai,                  // write
        sm_amm_authority_ai,        // read
        sm_pool_base_vault_ai,      // write
        sm_pool_quote_vault_ai,     // write
        sm_pool_mint_ai,            // write
        sm_pool_fees_ai,            // write

        token_swap_program_ai,      // read
        token_program_ai,           // read
    ] = accounts;

    // Verify the user signed the transaction
    check_eq!(user_account_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    // Verify the accounts are owned by the right programs
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_mata_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(mata_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_wsol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_mata_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    // Verify accounts against expectations
    check_eq!(sol_mata_oracle_ai.key, &SOL_MATA_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sm_amm_ai.key, &SOL_MATA_ORCA_AMM, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(token_program_ai.key, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(token_swap_program_ai.key, &orca_swap::id(), LucraErrorCode::InvalidAccountInput)?;

    let sol_mata_oracle = Oracle::load_checked(sol_mata_oracle_ai, &oracles::id()).unwrap();
    verify_orca_has_more_volume(&sol_mata_oracle, &SOL_MATA_RAYDIUM_AMM, &SOL_MATA_ORCA_AMM)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check_eq!(&system_state.mata_mint.address, mata_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.msol_vault.address, msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.repaid, false, LucraErrorCode::InvalidAccountInput)?;

    if loan.penalty_to_harvest >= system_state.minimum_harvest_amount {
        // There is enough penalty to harvest
        let sol_received = liquid_unstake_for_harvest(
            program_id,
            &system_state,
            &loan,
            msol_vault_authority_ai,
            msol_vault_ai,
            user_account_ai,
            user_msol_account_ai,
            user_wsol_account_ai,
            marinade_state_ai,
            msol_mint_ai,
            liq_pool_sol_leg_pda_ai,
            liq_pool_msol_leg_ai,
            treasury_msol_account_ai,
            marinade_program_ai,
            system_program_ai,
            token_program_ai,
        )?;

        // Swap the wrapped sol for mata
        let user_mata_balance_before = get_token_balance(user_mata_account_ai)?;
        orca_swap(
            token_swap_program_ai,
            token_program_ai,
            sm_amm_ai,
            sm_amm_authority_ai,
            user_account_ai,
            user_wsol_account_ai,
            user_mata_account_ai,
            sm_pool_base_vault_ai,
            sm_pool_quote_vault_ai,
            sm_pool_mint_ai,
            sm_pool_fees_ai,
            &[&[&[]]],
            sol_received,               // sol in
            0,                          // mata in
        )?;
        let user_mata_balance_after = get_token_balance(user_mata_account_ai)?;
        let mata_to_burn = user_mata_balance_after - user_mata_balance_before;

        // Burn the mata
        system_state.burn_mata(
            mata_mint_ai,
            user_mata_account_ai,
            mata_to_burn,
            user_account_ai,
            token_program_ai,
        )?;

        // Update system state values
        system_state.remove_outstanding_mata(mata_to_burn);
        system_state.remove_collateral(loan.penalty_to_harvest);

        // Update loan values
        loan.update_harvested_penalty();
    } else {
        // Not enough penalty to harvest
        return Err(throw_err!(LucraErrorCode::NoPenaltyToHarvest));
    }

    Ok(())
}

#[inline(never)]
pub fn process_harvest_penalty_raydium(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 33;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // write
        marinade_state_ai,              // write
        msol_vault_ai,                  // write
        msol_vault_authority_ai,        // read
        mata_mint_ai,                   // write
        loan_ai,                        // write
        sol_mata_oracle_ai,             // read

        user_msol_account_ai,           // write

        // Accounts required for msol unstake
        msol_mint_ai,                   // write
        liq_pool_sol_leg_pda_ai,        // write
        liq_pool_msol_leg_ai,           // write
        treasury_msol_account_ai,       // write
        system_program_ai,              // read
        marinade_program_ai,            // read

        user_account_ai,                // write
        user_wsol_account_ai,           // write
        user_mata_account_ai,           // write
        pool_program_ai,                // read
        _pool_wsol_account_ai,          // write
        _pool_mata_account_ai,          // write
        token_program_ai,               // read
        amm_program_ai,                 // write
        _amm_authority_ai,              // read
        _amm_open_orders_ai,            // write
        _amm_target_ai,                 // read
        _serum_sol_mata_market_ai,      // write
        serum_program_ai,               // read
        _serum_bids_ai,                 // write
        _serum_asks_ai,                 // write
        _serum_event_queue_ai,          // write
        _serum_base_vault_ai,           // write
        _serum_quote_vault_ai,          // write
        _serum_vault_signer_ai,         // read
    ] = accounts;

    // Verify the user signed the transaction
    check_eq!(user_account_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    // Verify the accounts are owned by the right programs
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_mata_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(marinade_state_ai.owner, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(marinade_program_ai.key, &marinade_finance::id(), LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(msol_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(mata_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_wsol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_msol_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_mata_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    // Verify accounts against expectations
    check_eq!(sol_mata_oracle_ai.key, &SOL_MATA_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(token_program_ai.key, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(serum_program_ai.key, &serum_v3::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(pool_program_ai.key, &raydium_v4::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(token_program_ai.key, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(amm_program_ai.key, &SOL_MATA_RAYDIUM_AMM, LucraErrorCode::InvalidAccountInput)?;

    let sol_mata_oracle = Oracle::load_checked(sol_mata_oracle_ai, &oracles::id()).unwrap();
    verify_raydium_has_more_volume(&sol_mata_oracle, &SOL_MATA_RAYDIUM_AMM, &SOL_MATA_ORCA_AMM)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    check_eq!(&system_state.mata_mint.address, mata_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&system_state.msol_vault.address, msol_vault_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.repaid, false, LucraErrorCode::InvalidAccountInput)?;

    if loan.penalty_to_harvest >= system_state.minimum_harvest_amount {
        // There is enough penalty to harvest
        let sol_received = liquid_unstake_for_harvest(
            program_id,
            &system_state,
            &loan,
            msol_vault_authority_ai,
            msol_vault_ai,
            user_account_ai,
            user_msol_account_ai,
            user_wsol_account_ai,
            marinade_state_ai,
            msol_mint_ai,
            liq_pool_sol_leg_pda_ai,
            liq_pool_msol_leg_ai,
            treasury_msol_account_ai,
            marinade_program_ai,
            system_program_ai,
            token_program_ai,
        )?;

        // Swap the wrapped sol for mata
        let user_mata_balance_before = get_token_balance(user_mata_account_ai)?;
        let accounts = array_ref![accounts, NUM_FIXED - 19, 19];
        raydium_swap(
            accounts,
            sol_received,       // sol in
            0,                  // mata in
        )?;
        let user_mata_balance_after = get_token_balance(user_mata_account_ai)?;
        let mata_to_burn = user_mata_balance_after - user_mata_balance_before;

        // Burn the mata
        system_state.burn_mata(
            mata_mint_ai,
            user_mata_account_ai,
            mata_to_burn,
            user_account_ai,
            token_program_ai,
        )?;

        // Update system state values
        system_state.remove_outstanding_mata(mata_to_burn);
        system_state.remove_collateral(loan.penalty_to_harvest);

        // Update loan values
        loan.update_harvested_penalty();
    } else {
        // Not enough penalty to harvest
        return Err(throw_err!(LucraErrorCode::NoPenaltyToHarvest));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[inline(never)]
pub fn liquid_unstake_for_harvest<'a, 'b>(
    program_id: &'a Pubkey,
    system_state: &'a RefMut<SystemState>,
    loan: &'a RefMut<MataLoan>,
    msol_vault_authority_ai: &'a AccountInfo<'b>,
    msol_vault_ai: &'a AccountInfo<'b>,
    user_account_ai: &'a AccountInfo<'b>,
    user_msol_account_ai: &'a AccountInfo<'b>,
    user_wsol_account_ai: &'a AccountInfo<'b>,
    marinade_state_ai: &'a AccountInfo<'b>,
    msol_mint_ai: &'a AccountInfo<'b>,
    liq_pool_sol_leg_pda_ai: &'a AccountInfo<'b>,
    liq_pool_msol_leg_ai: &'a AccountInfo<'b>,
    treasury_msol_account_ai: &'a AccountInfo<'b>,
    marinade_program_ai: &'a AccountInfo<'b>,
    system_program_ai: &'a AccountInfo<'b>,
    token_program_ai: &'a AccountInfo<'b>,
) -> LucraResult<u64> {
    // Convert the lamports to msol
    let state = ProgramAccount::<marinade_finance::state::State>::try_from(marinade_program_ai.clone().key, &marinade_state_ai.clone()).unwrap();
    let msol_lamports = state.calc_msol_from_lamports(loan.penalty_to_harvest).unwrap();
    // Transfer out the msol that corresponds to that lamport value
    system_state.transfer_from_msol_vault(
        program_id,
        msol_vault_ai,
        user_msol_account_ai,
        msol_vault_authority_ai,
        token_program_ai,
        msol_lamports
    )?;

    // Unstake collateral to user's sol account
    let user_sol_balance_before = user_account_ai.lamports();
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
    let user_sol_balance_after = user_account_ai.lamports();
    let sol_received = user_sol_balance_after - user_sol_balance_before;

    // Leave a fee of the wsol as payment to the user for running the contract
    let sol_received = sol_received
        .checked_sub(system_state.reward_fee as u64)
        .ok_or(math_err!())?;

    // Transfer the sol to the wrapped sol account
    transfer(
        user_account_ai,
        user_wsol_account_ai,
        sol_received,
        &[],
        system_program_ai,
    )?;

    // Sync native the sol to get wrapped sol
    sync_native(
        user_wsol_account_ai,
        &[],
        token_program_ai,
    )?;

    Ok(sol_received)
}