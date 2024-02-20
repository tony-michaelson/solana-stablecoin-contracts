use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    program_pack::Pack,
    msg,
};
use spl_token::state::{Mint, Account};
use rust_decimal::Decimal;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
};

declare_check_assert_macros!(SourceFileId::Spl);

pub fn spl_token_mint_to<'a>(
    mint: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    amount: u64,
    authority: &AccountInfo<'a>,
    authority_signer_seeds: &[&[&[u8]]],
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let mint_to_instruction = &spl_token::instruction::mint_to(
        token_program.key, 
        mint.key, 
        destination.key, 
        authority.key, 
        &[], 
        amount
    )?; 
    let accs = [
        mint.clone(),
        destination.clone(),
        authority.clone(),
        token_program.clone()
    ];
        
    solana_program::program::invoke_signed(mint_to_instruction, &accs, authority_signer_seeds)
}

pub fn spl_token_burn<'a>(
    mint: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    amount: u64,
    authority: &AccountInfo<'a>,
    authority_signer_seeds: &[&[&[u8]]],
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let burn_instruction = &spl_token::instruction::burn(
        token_program.key,
        source.key,
        mint.key,
        authority.key,
        &[],
        amount,
    )?;
    let accs = [
        source.clone(), 
        mint.clone(), 
        authority.clone(), 
        token_program.clone()
    ];

    solana_program::program::invoke_signed(burn_instruction, &accs, authority_signer_seeds)
}

pub fn spl_token_transfer<'a>(
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    amount: u64,
    authority: &AccountInfo<'a>,
    authority_signer_seeds: &[&[&[u8]]],
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let transfer_instruction = &spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;
    let accs = [
        source.clone(),
        destination.clone(),
        authority.clone(),
        token_program.clone()
    ];

    solana_program::program::invoke_signed(transfer_instruction, &accs, authority_signer_seeds)
}

pub fn spl_close_account<'a>(
    account: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    owner: &AccountInfo<'a>,
    authority_signer_seeds: &[&[&[u8]]],
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let close_account_instruction = &spl_token::instruction::close_account(
        token_program.key,
        account.key,
        destination.key,
        owner.key,
        &[],
    )?;
    let accs = [
        account.clone(),
        destination.clone(),
        owner.clone(),
        token_program.clone()
    ];
    
    solana_program::program::invoke_signed(close_account_instruction, &accs, authority_signer_seeds)
}

pub fn sync_native<'a>(
    account: &AccountInfo<'a>,
    _authority_signer_seeds: &[&[&[u8]]],
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let sync_native_instruction = &spl_token::instruction::sync_native(
        token_program.key,
        account.key,
    )?;
    let accs = [
        account.clone(),
        token_program.clone(),
    ];

    solana_program::program::invoke(sync_native_instruction, &accs)
}

pub fn get_tokens(
    token_account: &AccountInfo,
    mint_account: &AccountInfo,
) -> LucraResult<Decimal> {
    let balance = get_token_balance(token_account)?;
    let decimals = get_mint_decimals(mint_account)?;

    let lamports_per_token = Decimal::from(10i64
        .checked_pow(decimals.into())
        .ok_or(math_err!())?
    );

    Decimal::from(balance)
        .checked_div(lamports_per_token)
        .ok_or(math_err!())
}

pub fn get_mint_decimals(mint_account: &AccountInfo) -> LucraResult<u8> {
    let data = mint_account.try_borrow_data()?;
    check_eq!(data.len(), Mint::LEN, LucraErrorCode::InvalidAccountInput)?;
    let decimals = array_ref![data, 44, 1];

    Ok(u8::from_le_bytes(*decimals))
}

pub fn get_token_balance(token_account: &AccountInfo) -> LucraResult<u64> {
    let data = token_account.try_borrow_data()?;
    check_eq!(data.len(), Account::LEN, LucraErrorCode::InvalidAccountInput)?;
    let amount = array_ref![data, 64, 8];

    Ok(u64::from_le_bytes(*amount))
}

pub fn get_token_account_mint(token_account: &AccountInfo) -> LucraResult<Pubkey> {
    let data = token_account.try_borrow_data()?;
    check_eq!(data.len(), Account::LEN, LucraErrorCode::InvalidAccountInput)?;
    let mint = array_ref![data, 0, 32];

    Ok(Pubkey::new_from_array(*mint))
}

pub fn calculate_pool_price(
    base_amount: Decimal,
    quote_amount: Decimal,
) -> LucraResult<Decimal> {
    quote_amount
        .checked_div(base_amount)
        .ok_or(math_err!())
}

pub fn verify_balanced_pool(
    new_price: Decimal,
    desired_price: Decimal,
    tolerance: Decimal,
) -> LucraResult {
    msg!("new_price: {:?}", new_price);
    let upper_bound = desired_price.checked_add(tolerance).ok_or(math_err!())?;
    msg!("upper_bound: {:?}", upper_bound);
    let lower_bound = desired_price.checked_sub(tolerance).ok_or(math_err!())?;
    msg!("lower_bound: {:?}", lower_bound);

    if new_price <= upper_bound && 
        new_price >= lower_bound {
        Ok(())
    } else {
        Err(throw_err!(LucraErrorCode::MathError))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_verify_balanced_pool() {
        let base_amount = Decimal::from(1_000_000_u64);
        let quote_amount = Decimal::from(10_000_000_u64);
        let desired_price = Decimal::from(10_u64);
        let tolerance = Decimal::new(5, 3);

        let new_price = calculate_pool_price(base_amount, quote_amount).unwrap();

        let result = verify_balanced_pool(
            new_price,
            desired_price,
            tolerance,
        ).is_ok();
        assert!(result);

        let base_amount = Decimal::from(1_000_000_u64);
        let quote_amount = Decimal::from(15_000_000_u64);

        let new_price = calculate_pool_price(base_amount, quote_amount).unwrap();

        let result = verify_balanced_pool(
            new_price,
            desired_price,
            tolerance,
        ).is_ok();        
        assert!(!result);
    }
}