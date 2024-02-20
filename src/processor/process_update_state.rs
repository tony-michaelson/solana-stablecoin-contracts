use std::cell::RefMut;

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
};
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::DAO_AUTHORITY,
    state::{
        ArbState,
        UpdateStateParams,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::UpdateState);

#[inline(never)]
pub fn process_update_state(program_id: &Pubkey, state_params: &UpdateStateParams, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 3;
    let accounts = array_ref!(accounts, 0, NUM_FIXED);
    let [
        system_state_ai,    // write
        arb_state_ai,       // write
        dao_authority_ai,   // read
    ] = accounts;

    check_eq!(dao_authority_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    check_eq!(dao_authority_ai.key, &DAO_AUTHORITY, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(arb_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;

    let mut system_state: RefMut<SystemState> = SystemState::load_mut_checked(system_state_ai, program_id)?;
    let mut arb_state: RefMut<ArbState> = ArbState::load_mut_checked(arb_state_ai, program_id)?;
    
    system_state.min_deposit = state_params.min_deposit;
    system_state.collateral_requirement = state_params.collateral_requirement;
    system_state.loans_enabled = state_params.loans_enabled;
    system_state.staking_enabled = state_params.staking_enabled;
    system_state.arbitrage_enabled = state_params.arbitrage_enabled;
    system_state.peg_check_enabled = state_params.peg_check_enabled;
    system_state.maximum_outstanding_mata = state_params.maximum_outstanding_mata;
    system_state.minimum_harvest_amount = state_params.minimum_harvest_amount;
    system_state.reward_fee = state_params.reward_fee;
    system_state.lcp = state_params.lcp;
    
    arb_state.daily_limit = state_params.daily_arb_limit;
    arb_state.max_amount_of_lucra_to_mint = state_params.max_amount_of_lucra_to_mint;

    Ok(())
}