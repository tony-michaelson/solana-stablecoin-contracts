use anchor_lang::{
    prelude::*,
    InstructionData,
    solana_program::instruction::Instruction,
};
use marinade_onchain_helper::cpi_context_accounts::{MarinadeLiquidUnstake, MarinadeDeposit};
use solana_program::{
    account_info::AccountInfo,
};

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    marinade_state: &AccountInfo<'a>,
    msol_mint: &AccountInfo<'a>,
    liq_pool_sol_leg_pda: &AccountInfo<'a>,
    liq_pool_msol_leg: &AccountInfo<'a>,
    liq_pool_msol_leg_authority: &AccountInfo<'a>,
    reserve_pda: &AccountInfo<'a>,
    user_sol_account: &AccountInfo<'a>,
    user_msol_account: &AccountInfo<'a>,
    msol_mint_authority: &AccountInfo<'a>,
    authority_signer_seeds: &[&[&[u8]]],
    system_program: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    marinade_program: &AccountInfo<'a>,
    lamports: u64,
) -> ProgramResult {
    let cpi_accounts = MarinadeDeposit {
        state: marinade_state.clone(),
        msol_mint: msol_mint.clone(),
        liq_pool_sol_leg_pda: liq_pool_sol_leg_pda.clone(),
        liq_pool_msol_leg: liq_pool_msol_leg.clone(),
        liq_pool_msol_leg_authority: liq_pool_msol_leg_authority.clone(),
        reserve_pda: reserve_pda.clone(),
        transfer_from: user_sol_account.clone(),
        mint_to: user_msol_account.clone(),
        msol_mint_authority: msol_mint_authority.clone(),
        system_program: system_program.clone(),
        token_program: token_program.clone(),
    };
    let cpi_program = marinade_program.clone();
    let cpi_signers = authority_signer_seeds;
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, cpi_signers);
    let data = marinade_finance::instruction::Deposit { lamports };
    let ix = Instruction {
        program_id: *cpi_ctx.program.key,
        accounts: cpi_ctx.accounts.to_account_metas(None),
        data: data.data(),
    };
    solana_program::program::invoke(
        &ix,
        &cpi_ctx.to_account_infos(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn liquid_unstake<'a>(
    marinade_state: &AccountInfo<'a>,
    msol_mint: &AccountInfo<'a>,
    liq_pool_sol_leg_pda: &AccountInfo<'a>,
    liq_pool_msol_leg: &AccountInfo<'a>,
    treasury_msol_account: &AccountInfo<'a>,
    get_msol_from: &AccountInfo<'a>,
    get_msol_from_authority: &AccountInfo<'a>,
    transfer_sol_to: &AccountInfo<'a>,
    authority_signer_seeds: &[&[&[u8]]],
    token_program: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
    marinade_program: &AccountInfo<'a>,
    msol_amount: u64,
) -> ProgramResult {
    let cpi_accounts = MarinadeLiquidUnstake {
        state: marinade_state.clone(),
        msol_mint: msol_mint.clone(),
        liq_pool_sol_leg_pda: liq_pool_sol_leg_pda.clone(),
        liq_pool_msol_leg: liq_pool_msol_leg.clone(),
        treasury_msol_account: treasury_msol_account.clone(),
        get_msol_from: get_msol_from.clone(),
        get_msol_from_authority: get_msol_from_authority.clone(),
        transfer_sol_to: transfer_sol_to.clone(),
        system_program: system_program.clone(),
        token_program: token_program.clone(),
    };
    let cpi_program = marinade_program.clone();
    let cpi_signers = authority_signer_seeds;
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, cpi_signers);
    let data = marinade_finance::instruction::LiquidUnstake { msol_amount };
    let ix = Instruction {
        program_id: *cpi_ctx.program.key,
        accounts: cpi_ctx.accounts.to_account_metas(None),
        data: data.data(),
    };
    solana_program::program::invoke(
        &ix,
        &cpi_ctx.to_account_infos(),
    )
}