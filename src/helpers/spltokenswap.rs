// Taken from the solana-farm-sdk found here https://docs.rs/solana-farm-sdk/1.1.3/src/solana_farm_sdk/program/protocol/orca.rs.html
use solana_program::{
    account_info::AccountInfo,
    program::invoke,
};
use spl_token_swap::state::SwapVersion;
use rust_decimal::Decimal;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::{
        spl::{get_tokens, get_token_balance, get_mint_decimals, verify_balanced_pool},
        math::{ceiling_division, get_no_fee_amount},
    },
    helpers::constants::orca_swap,
};

use super::spl::calculate_pool_price;

pub const ORCA_FEE: f64 = 0.003;
pub const ORCA_FEE_NUMERATOR: u64 = 3;
pub const ORCA_FEE_DENOMINATOR: u64 = 1000;

declare_check_assert_macros!(SourceFileId::SplTokenSwap);

#[allow(clippy::too_many_arguments)]
pub fn swap<'a>(
    program_id: &AccountInfo<'a>,
    token_program_id: &AccountInfo<'a>,
    amm_id: &AccountInfo<'a>,
    amm_authority_id: &AccountInfo<'a>,
    user_transfer_authority: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    swap_base_vault: &AccountInfo<'a>,
    swap_quote_vault: &AccountInfo<'a>,
    pool_mint: &AccountInfo<'a>,
    fees: &AccountInfo<'a>,
    authority_signer_seeds:&[&[&[u8]]],
    token_a_amount_in: u64,
    token_b_amount_in: u64,
) -> LucraResult {
    let (amount_in, min_amount_out) = get_pool_swap_amounts(
        swap_base_vault,
        swap_quote_vault,
        token_a_amount_in,
        token_b_amount_in,
    )?;

    let accs = [
        user_transfer_authority.clone(),
        source.clone(),
        destination.clone(),
        program_id.clone(),
        swap_base_vault.clone(),
        swap_quote_vault.clone(),
        pool_mint.clone(),
        token_program_id.clone(),
        amm_id.clone(),
        amm_authority_id.clone(),
        fees.clone(),
    ];

    swap_with_seeds(
        &accs,
        authority_signer_seeds,
        amount_in,
        min_amount_out,
    )
}

fn swap_with_seeds(
    accounts: &[AccountInfo],
    _seeds: &[&[&[u8]]],
    amount_in: u64,
    min_amount_out: u64,
) -> LucraResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        token_a_custody_account,
        token_b_custody_account,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        _spl_token_id,
        amm_id,
        amm_authority,
        fees_account
        ] = accounts
    {
        if pool_program_id.key != &orca_swap::id() {
            return Err(throw_err!(LucraErrorCode::InvalidAccountInput));
        }

        let data = spl_token_swap::instruction::Swap {
            amount_in,
            minimum_amount_out: min_amount_out,
        };

        let instruction = spl_token_swap::instruction::swap(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            authority_account.key,
            token_a_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            token_b_custody_account.key,
            lp_token_mint.key,
            fees_account.key,
            None,
            data,
        )?;

        invoke(&instruction, accounts).map_err(|_| throw_err!(LucraErrorCode::TransactionFailed))
    } else {
        Err(throw_err!(LucraErrorCode::Default))
    }
}

fn get_pool_token_balances<'a, 'b>(
    pool_token_a_account: &'a AccountInfo<'b>,
    pool_token_b_account: &'a AccountInfo<'b>,
) -> LucraResult<(u64, u64)> {
    Ok((
        get_token_balance(pool_token_a_account)?,
        get_token_balance(pool_token_b_account)?,
    ))
}

fn get_pool_swap_amounts<'a, 'b>(
    pool_token_a_account: &'a AccountInfo<'b>,
    pool_token_b_account: &'a AccountInfo<'b>,
    token_a_amount_in: u64,
    token_b_amount_in: u64,
) -> LucraResult<(u64, u64)> {
    if (token_a_amount_in == 0 && token_b_amount_in == 0)
        || (token_a_amount_in > 0 && token_b_amount_in > 0)
    {
        return Err(throw_err!(LucraErrorCode::InvalidAmount));
    }
    let (token_a_balance, token_b_balance) =
        get_pool_token_balances(pool_token_a_account, pool_token_b_account)?;
    if token_a_balance == 0 || token_b_balance == 0 {
        return Err(throw_err!(LucraErrorCode::EmptyPool));
    }
    let token_a_balance = token_a_balance as u128;
    let token_b_balance = token_b_balance as u128;
    if token_a_amount_in == 0 {
        // b to a
        let amount_in_no_fee =
            get_no_fee_amount(token_b_amount_in.into(), ORCA_FEE_NUMERATOR.into(), ORCA_FEE_DENOMINATOR.into())?
                as u128;
        let num = Decimal::from(token_a_balance)
            .checked_mul(amount_in_no_fee.into())
            .ok_or(math_err!())?;
        let den = Decimal::from(token_b_balance)
            .checked_add(amount_in_no_fee.into())
            .ok_or(math_err!())?;
        let estimated_token_a_amount = num.checked_div(den).ok_or(math_err!())?;

        Ok((
            token_b_amount_in,
            get_no_fee_amount(estimated_token_a_amount, 3_i64.into(), 100_i64.into())?,
        ))
    } else {
        // a to b
        let amount_in_no_fee =
            get_no_fee_amount(
                token_a_amount_in.into(),
                ORCA_FEE_NUMERATOR.into(),
                ORCA_FEE_DENOMINATOR.into()
            )? as u128;

        let num = Decimal::from(token_b_balance)
            .checked_mul(amount_in_no_fee.into())
            .ok_or(math_err!())?;
        let den = Decimal::from(token_a_balance)
            .checked_add(amount_in_no_fee.into())
            .ok_or(math_err!())?;
        let estimated_token_b_amount = num.checked_div(den).ok_or(math_err!())?;
        
        Ok((
            token_a_amount_in,
            get_no_fee_amount(estimated_token_b_amount, 3_i64.into(), 100_i64.into())?,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn verify_orca_pools_will_be_balanced<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_coin_token_mint: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    pool_pc_token_mint: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    desired_price: Decimal,
    coin_amount: u64,
    pc_amount: u64,
) -> LucraResult {
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
    )?;
    let (
        trade_fee_numerator, trade_fee_denominator,
        owner_trade_fee_numerator, owner_trade_fee_denominator,
        _host_fee_numerator, _host_fee_denominator,
    ) = get_fees(amm_id)?;

    let coin_decimals = Decimal::from(10_i64.pow(get_mint_decimals(pool_coin_token_mint)?.into()));
    let pc_decimals = Decimal::from(10_i64.pow(get_mint_decimals(pool_pc_token_mint)?.into()));

    let coin_balance = Decimal::from(coin_balance)
        .checked_div(coin_decimals)
        .ok_or(math_err!())?;

    let pc_balance = Decimal::from(pc_balance)
        .checked_div(pc_decimals)
        .ok_or(math_err!())?;

    let coin_amount = Decimal::from(coin_amount)
        .checked_div(coin_decimals)
        .ok_or(math_err!())?;

    let pc_amount = Decimal::from(pc_amount)
        .checked_div(pc_decimals)
        .ok_or(math_err!())?;

    _verify_orca_pools_will_be_balanced(
        coin_balance,
        pc_balance,
        Decimal::from(trade_fee_numerator),
        Decimal::from(trade_fee_denominator),
        Decimal::from(owner_trade_fee_numerator),
        Decimal::from(owner_trade_fee_denominator),
        coin_amount,
        pc_amount,
        desired_price,
    )
}

#[allow(clippy::too_many_arguments)]
fn _verify_orca_pools_will_be_balanced(
    coin_balance: Decimal,
    pc_balance: Decimal,
    trade_fee_numerator: Decimal,
    trade_fee_denominator: Decimal,
    owner_fee_numerator: Decimal,
    owner_fee_denominator: Decimal,
    coin_amount: Decimal,
    pc_amount: Decimal,
    desired_price: Decimal,
) -> LucraResult {
    let invariant = coin_balance
        .checked_mul(pc_balance)
        .ok_or(math_err!())?;
    let swap_amount = if coin_amount > Decimal::ZERO { coin_amount } else { pc_amount };
    let trade_fee_amount = swap_amount
        .checked_mul(trade_fee_numerator)
        .ok_or(math_err!())?
        .checked_div(trade_fee_denominator)
        .ok_or(math_err!())?;
    let owner_fee_amount = swap_amount
        .checked_mul(owner_fee_numerator)
        .ok_or(math_err!())?
        .checked_div(owner_fee_denominator)
        .ok_or(math_err!())?;

    let fees = trade_fee_amount
        .checked_add(owner_fee_amount)
        .ok_or(math_err!())?;
    let swap_amount_less_fees = swap_amount
        .checked_sub(fees)
        .ok_or(math_err!())?;

    let source_amount = if coin_amount > Decimal::ZERO { coin_balance } else { pc_balance };    
    let (new_destination_amount, new_source_amount) = ceiling_division(invariant, source_amount.checked_add(swap_amount_less_fees).ok_or(math_err!())?)?;

    let new_source_amount = new_source_amount
        .checked_add(fees)
        .ok_or(math_err!())?;

    let new_base_amount = if coin_amount > Decimal::ZERO { new_source_amount } else { new_destination_amount };
    let new_quote_amount = if coin_amount > Decimal::ZERO { new_destination_amount } else { new_source_amount };

    let new_price = calculate_pool_price(new_base_amount, new_quote_amount)?;

    let tolerance = Decimal::new(1, 3);
    verify_balanced_pool(
        new_price,
        desired_price,
        tolerance,
    )
}

// Verify that the pools are balanced to an acceptable tolerance
#[allow(clippy::too_many_arguments)]
pub fn verify_orca_pools_are_balanced(
    coin_vault: &AccountInfo,
    coin_mint: &AccountInfo,
    pc_vault: &AccountInfo,
    pc_mint: &AccountInfo,
    desired_price: Decimal,
) -> LucraResult {
    let coin_balance = get_tokens(coin_vault, coin_mint)?;
    let pc_balance = get_tokens(pc_vault, pc_mint)?;

    let new_price = calculate_pool_price(coin_balance, pc_balance)?;

    let tolerance = Decimal::new(1, 3);
    verify_balanced_pool(
        new_price,
        desired_price,
        tolerance,
    )
}

pub fn get_fees(
    amm_id: &AccountInfo,
) -> LucraResult<(u64, u64, u64, u64, u64, u64)> {
    let amm = SwapVersion::unpack(&amm_id.data.borrow())?;
    let fees = amm.fees();

    Ok((
        fees.trade_fee_numerator,
        fees.trade_fee_denominator,
        fees.owner_trade_fee_numerator,
        fees.owner_trade_fee_denominator,
        fees.host_fee_numerator,
        fees.host_fee_denominator,
    ))
}

pub fn verify_orca_pool_has_proper_mints<'a, 'b>(
    orca_pool_ai: &'a AccountInfo<'b>,
    base_mint_ai: &'a AccountInfo<'b>,
    quote_mint_ai: &'a AccountInfo<'b>,
) -> LucraResult {
    let amm_id_data = spl_token_swap::state::SwapVersion::unpack(&orca_pool_ai.try_borrow_data()?)?;

    check_eq!(amm_id_data.token_a_mint(), base_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(amm_id_data.token_b_mint(), quote_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_orca_pools_will_be_balanced() {
        let coin_balance = Decimal::new(139_396_44, 2);
        let pc_balance = Decimal::new(5_532_953_37, 2);
        let trade_fee_numerator = Decimal::from(25);
        let trade_fee_denominator = Decimal::from(10_000);
        let owner_trade_fee_numerator = Decimal::from(25);
        let owner_trade_fee_denominator = Decimal::from(10_000);
        let coin_amount = Decimal::from(100_u64);
        let pc_amount = Decimal::ZERO;
        let desired_price = Decimal::new(39_635_327, 6);

        let result = _verify_orca_pools_will_be_balanced(
            coin_balance,
            pc_balance,
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            coin_amount,
            pc_amount,
            desired_price
        );
        assert_eq!(result, Ok(()));

        let desired_price = Decimal::new(39_693_649, 6);
        let result = _verify_orca_pools_will_be_balanced(
            coin_balance, 
            pc_balance, 
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            pc_amount,
            coin_amount,
            desired_price
        );
        assert_eq!(result, Ok(()))
    }
}