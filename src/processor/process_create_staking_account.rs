use std::cell::{Ref, RefMut};
use std::mem::size_of;

use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::{rent::Rent, Sysvar},
    pubkey::Pubkey,
};
use legends_loadable_trait::Loadable;
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    state::{
        DataType,
        MetaData,
        staking::StakingAccount,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::CreateStakingAccount);

#[inline(never)]
pub fn process_create_staking_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 4;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,                // read
        staking_state_ai,               // read
        staking_account_ai,             // write
        owner_ai,                       // read
    ] = accounts;
        
    let rent = &Rent::get()?;

    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(staking_account_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(owner_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;

    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check!(system_state.staking_enabled, LucraErrorCode::StakingNotEnabled)?;
    check_eq!(&system_state.staking_state, staking_state_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let mut staking_account: RefMut<StakingAccount> = StakingAccount::load_mut(staking_account_ai)?;
    check!(!staking_account.meta_data.is_initialized, LucraErrorCode::Default)?;

    check!(
        rent.is_exempt(staking_account_ai.lamports(), size_of::<StakingAccount>()),
        LucraErrorCode::NotRentExempt
    )?;

    staking_account.meta_data = MetaData::new(DataType::StakingAccount, 0, true);
    staking_account.owner = *owner_ai.key;
    staking_account.total = 0;
    staking_account.locked_total = 0;

    Ok(())
}