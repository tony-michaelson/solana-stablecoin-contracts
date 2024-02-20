use std::cell::{Ref, RefMut};
use std::mem::size_of;

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{rent::Rent, Sysvar},
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;
use legends_loadable_trait::Loadable;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    state::{
        DataType,
        MetaData,
        staking::{
            StakeBalance,
            StakingTimeframe,
            StakingState,
        },
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::CreateStakeBalance);

#[inline(never)]
pub fn process_create_stake_balance(program_id: &Pubkey, nonce: u8, staking_timeframe: StakingTimeframe, accounts: &[AccountInfo]) -> LucraResult {    
    const NUM_FIXED: usize = 7;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // read
        staking_state_ai,               // read
        stake_balance_ai,               // write
        owner_ai,                       // read
        deposit_vault_ai,               // read
        stake_vault_ai,                 // read
        pending_vault_ai,               // read
    ] = accounts;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_balance_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(pending_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(deposit_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(stake_vault_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check_eq!(&system_state.staking_state, staking_state_ai.key, LucraErrorCode::InvalidAccountInput)?;
    let staking_state: Ref<StakingState> = StakingState::load_checked(staking_state_ai, program_id)?;
    let mut stake_balance: RefMut<StakeBalance> = StakeBalance::load_mut(stake_balance_ai)?;
    check!(!stake_balance.meta_data.is_initialized, LucraErrorCode::Default)?;

    let deposit_vault = Account::unpack(&deposit_vault_ai.data.borrow())?;
    let stake_vault = Account::unpack(&stake_vault_ai.data.borrow())?;
    let pending_vault = Account::unpack(&pending_vault_ai.data.borrow())?;
    
    let rent = &Rent::get()?;

    check!(
        rent.is_exempt(stake_balance_ai.lamports(), size_of::<StakeBalance>()),
        LucraErrorCode::NotRentExempt
    )?;

    let authority_signer_seeds = &[
        owner_ai.key.as_ref(),
        system_state_ai.key.as_ref(),
        &[nonce],
    ];
    let vault_owner_pda = Pubkey::create_program_address(authority_signer_seeds, program_id).map_err(|_| throw_err!(LucraErrorCode::InvalidNonce))?;
    check_eq!(&deposit_vault.owner, &vault_owner_pda, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(&stake_vault.owner, &vault_owner_pda, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(&pending_vault.owner, &vault_owner_pda, LucraErrorCode::InvalidAccountOwner)?;

    check_eq!(&deposit_vault.mint, &system_state.lucra_mint.address, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&stake_vault.mint, &system_state.lucra_mint.address, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&pending_vault.mint, &system_state.lucra_mint.address, LucraErrorCode::InvalidAccountInput)?;
   
    stake_balance.meta_data = MetaData::new(DataType::StakeBalance, 0, true);
    stake_balance.owner = *owner_ai.key;
    stake_balance.reward_cursor = staking_state.reward_cursor;
    stake_balance.staking_timeframe = staking_timeframe;
    stake_balance.balances.deposit_vault = *deposit_vault_ai.key;
    stake_balance.balances.stake_vault = *stake_vault_ai.key;
    stake_balance.balances.pending_vault = *pending_vault_ai.key;
    stake_balance.signer_bump_seed = nonce;
    stake_balance.closed = false;

    Ok(())
}