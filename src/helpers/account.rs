use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{fees::Fees, Sysvar},
};
use spl_token_swap::state::SwapVersion;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    id,
};
declare_check_assert_macros!(SourceFileId::Account);

pub fn find_program_address(state: &Pubkey, seed: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&state.to_bytes()[..32], seed],
        &id(),
    )
}

pub fn get_raydium_open_orders(raydium_amm: &AccountInfo) -> LucraResult<Pubkey> {
    let offset = if raydium_amm.data_len() == 752 {
        496
    } else {
        0
    };

    if offset == 0 {
        return Err(throw_err!(LucraErrorCode::InvalidAccountInput));
    }
    let raydium_amm_data = raydium_amm.try_borrow_data()?;
    let pubkey = Pubkey::new_from_array(*array_ref![raydium_amm_data, offset, 32]);
    
    Ok(pubkey)
}

pub fn get_raydium_pool_accounts(raydium_amm: &AccountInfo) -> LucraResult<(Pubkey, Pubkey)> {
    let (base_offset, quote_offset) = if raydium_amm.data_len() == 752 {
        (336, 368)
    } else {
        (0, 0)
    };

    if base_offset == 0 || quote_offset == 0  {
        return Err(throw_err!(LucraErrorCode::InvalidAccountInput));
    }
    let raydium_amm_data = raydium_amm.try_borrow_data()?;
    let base_pubkey = Pubkey::new_from_array(*array_ref![raydium_amm_data, base_offset, 32]);
    let quote_pubkey = Pubkey::new_from_array(*array_ref![raydium_amm_data, quote_offset, 32]);

    Ok((base_pubkey, quote_pubkey))
}

pub fn get_orca_pool_accounts(orca_pool: &AccountInfo) -> LucraResult<(Pubkey, Pubkey)> {
    let pool = SwapVersion::unpack(&orca_pool.data.borrow())?;

    Ok((*pool.token_a_account(), *pool.token_b_account()))
}

pub fn close_account(account: &AccountInfo) -> u64 {
    let starting_lamports = account.lamports();
    **account.lamports.borrow_mut() = 0;

    starting_lamports
}

pub fn add_lamports(account: &AccountInfo, lamports: u64) {
    let starting_lamports = account.lamports();
    **account.lamports.borrow_mut() = starting_lamports
        .checked_add(lamports)
        .ok_or(math_err!())
        .unwrap();
}

pub fn verify_account_will_still_have_lamports(
    fees_ai: &AccountInfo,
    starting_lamports: u64,
    lamports_to_take: u64,
) -> LucraResult {
    let fees = Fees::from_account_info(fees_ai).unwrap();
    let lps = fees.fee_calculator.lamports_per_signature;
    check!(lamports_to_take + (lps * 3) <= starting_lamports, LucraErrorCode::InvalidAmount) // Don't let users accidentally close their accounts...
}