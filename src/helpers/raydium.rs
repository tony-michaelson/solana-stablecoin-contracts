use std::convert::identity;
use anchor_lang::AnchorDeserialize;
use arrayref::{array_ref, array_mut_ref, mut_array_refs};
use safe_transmute::{self, to_bytes::transmute_to_bytes};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    msg,
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
        constants::{raydium_v4, serum_v3},
        spl::{get_mint_decimals, get_token_balance, verify_balanced_pool, calculate_pool_price},
    },
};

declare_check_assert_macros!(SourceFileId::Raydium);

pub const RAYDIUM_FEE: f64 = 0.0025;

pub fn swap(
    accounts: &[AccountInfo],
    token_a_amount_in: u64,
    token_b_amount_in: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_token_a_account,
        user_token_b_account,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        spl_token_id,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        serum_program_id,
        serum_bids,
        serum_asks,
        serum_event_queue,
        serum_coin_vault_account,
        serum_pc_vault_account,
        serum_vault_signer
        ] = accounts
    {
        check_eq!(pool_program_id.key, &raydium_v4::id(), LucraErrorCode::InvalidAccountInput)?;

        let (amount_in, min_amount_out) = get_pool_swap_amounts(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
            token_a_amount_in,
            token_b_amount_in,
        )?;

        let mut raydium_accounts = Vec::with_capacity(18);
        raydium_accounts.push(AccountMeta::new_readonly(*spl_token_id.key, false));
        raydium_accounts.push(AccountMeta::new(*amm_id.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*amm_authority.key, false));
        raydium_accounts.push(AccountMeta::new(*amm_open_orders.key, false));
        raydium_accounts.push(AccountMeta::new(*amm_target.key, false));
        raydium_accounts.push(AccountMeta::new(*pool_coin_token_account.key, false));
        raydium_accounts.push(AccountMeta::new(*pool_pc_token_account.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*serum_program_id.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_market.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_bids.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_asks.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_event_queue.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_coin_vault_account.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_pc_vault_account.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*serum_vault_signer.key, false));
        if token_a_amount_in == 0 {
            raydium_accounts.push(AccountMeta::new(*user_token_b_account.key, false));
            raydium_accounts.push(AccountMeta::new(*user_token_a_account.key, false));
        } else {
            raydium_accounts.push(AccountMeta::new(*user_token_a_account.key, false));
            raydium_accounts.push(AccountMeta::new(*user_token_b_account.key, false));
        }
        raydium_accounts.push(AccountMeta::new_readonly(*user_account.key, true));

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumSwap {
                instruction: 9,
                amount_in,
                min_amount_out,
            }
            .to_vec()?,
        };
        invoke(&instruction, accounts)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    Ok(())
}

pub fn get_pool_swap_amounts<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    coin_token_amount_in: u64,
    pc_token_amount_in: u64,
) -> Result<(u64, u64), ProgramError> {
    if (coin_token_amount_in == 0 && pc_token_amount_in == 0)
        || (coin_token_amount_in > 0 && pc_token_amount_in > 0)
    {
        return Err(ProgramError::InvalidArgument);
    }
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
        amm_open_orders,
        amm_id,
    )?;
    if coin_balance == 0 || pc_balance == 0 {
        return Err(ProgramError::Custom(412));
    }
    if coin_token_amount_in == 0 {
        // pc to coin
        let amount_in_no_fee = (pc_token_amount_in as f64 * (1.0 - RAYDIUM_FEE)) as u64;
        let estimated_coin_amount = Decimal::from_f64_retain(coin_balance as f64 * amount_in_no_fee as f64)
            .ok_or(math_err!())?
            .checked_div(Decimal::from_f64_retain(pc_balance as f64 + amount_in_no_fee as f64).ok_or(math_err!())?)
            .ok_or(math_err!())?
            .to_u64()
            .ok_or(math_err!())?;
        Ok((
            pc_token_amount_in,
            if estimated_coin_amount > 1 {
                estimated_coin_amount - 1
            } else {
                0
            },
        ))
    } else {
        // coin to pc
        let amount_in_no_fee = (coin_token_amount_in as f64 * (1.0 - RAYDIUM_FEE)) as u64;
        let estimated_pc_amount = Decimal::from_f64_retain(pc_balance as f64 * amount_in_no_fee as f64)
            .ok_or(math_err!())?
            .checked_div(Decimal::from_f64_retain(coin_balance as f64 + amount_in_no_fee as f64).ok_or(math_err!())?)
            .ok_or(math_err!())?
            .to_u64()
            .ok_or(math_err!())?;
        Ok((
            coin_token_amount_in,
            if estimated_pc_amount > 1 {
                estimated_pc_amount - 1
            } else {
                0
            },
        ))
    }
}

pub fn get_pool_token_balances<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
) -> Result<(u64, u64), ProgramError> {
    // get token balances
    let mut token_a_balance = get_token_balance(pool_coin_token_account)?;
    let mut token_b_balance = get_token_balance(pool_pc_token_account)?;

    // adjust with open orders
    if amm_open_orders.data_len() == 3228 {
        let open_orders_data = amm_open_orders.try_borrow_data()?;
        let base_token_total = array_ref![open_orders_data, 85, 8];
        let quote_token_total = array_ref![open_orders_data, 101, 8];

        token_a_balance += u64::from_le_bytes(*base_token_total);
        token_b_balance += u64::from_le_bytes(*quote_token_total);
    }

    // adjust with amm take pnl
    let (pnl_coin_offset, pnl_pc_offset) = if amm_id.data_len() == 752 {
        (192, 200)
    } else {
        (0, 0)
    };
    if pnl_coin_offset > 0 {
        let amm_id_data = amm_id.try_borrow_data()?;
        let need_take_pnl_coin = u64::from_le_bytes(*array_ref![amm_id_data, pnl_coin_offset, 8]);
        let need_take_pnl_pc = u64::from_le_bytes(*array_ref![amm_id_data, pnl_pc_offset, 8]);

        // safe to use unchecked sub
        token_a_balance -= if need_take_pnl_coin < token_a_balance {
            need_take_pnl_coin
        } else {
            token_a_balance
        };
        // safe to use unchecked sub
        token_b_balance -= if need_take_pnl_pc < token_b_balance {
            need_take_pnl_pc
        } else {
            token_b_balance
        };
    }

    Ok((token_a_balance, token_b_balance))
}

pub fn verify_raydium_amm_has_proper_mints<'a, 'b>(
    raydium_amm_ai: &'a AccountInfo<'b>,
    base_mint_ai: &'a AccountInfo<'b>,
    quote_mint_ai: &'a AccountInfo<'b>,
) -> LucraResult {
    let amm_id_data = raydium_amm_ai.try_borrow_data()?;
    let coin_mint = Pubkey::try_from_slice(array_ref![amm_id_data, 400, 32]).unwrap();
    let pc_mint = Pubkey::try_from_slice(array_ref![amm_id_data, 432, 32]).unwrap();

    check_eq!(&coin_mint, base_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(&pc_mint, quote_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;

    Ok(())
}

pub fn verify_serum_market_has_proper_mints<'a, 'b>(
    market: &'a AccountInfo<'b>,
    base_mint: &'a AccountInfo<'b>,
    quote_mint: &'a AccountInfo<'b>,
) -> LucraResult {
    let market = serum_dex::state::Market::load(market, &serum_v3::id(), false).unwrap();

    check_eq!(transmute_to_bytes(&identity(market.coin_mint)), base_mint.key.to_bytes(), LucraErrorCode::InvalidAccountInput)?;
    check_eq!(transmute_to_bytes(&identity(market.pc_mint)), quote_mint.key.to_bytes(), LucraErrorCode::InvalidAccountInput)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn calculate_new_price(
    coin_balance: Decimal,
    pc_balance: Decimal,
    trade_fee_numerator: Decimal,
    trade_fee_denominator: Decimal,
    coin_amount: Decimal,
    pc_amount: Decimal,
) -> LucraResult<Decimal> {
    let (new_base_amount, new_quote_amount) = calculate_new_amounts(
        coin_balance,
        pc_balance,
        trade_fee_numerator,
        trade_fee_denominator,
        coin_amount,
        pc_amount,
    )?;

    calculate_pool_price(new_base_amount, new_quote_amount)
}

#[allow(clippy::too_many_arguments)]
pub fn verify_raydium_pools_will_be_balanced<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_coin_token_mint: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    pool_pc_token_mint: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    desired_price: Decimal,
    coin_amount: u64,
    pc_amount: u64,
) -> LucraResult {
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
        amm_open_orders,
        amm_id,
    )?;
    let (
        trade_fee_numerator, trade_fee_denominator,
        _swap_fee_numerator, _swap_fee_denominator,
    ) = get_fees(amm_id)?;

    let coin_decimals = Decimal::from(10_i64.pow(get_mint_decimals(pool_coin_token_mint)?.into()));
    let pc_decimals = Decimal::from(10_i64.pow(get_mint_decimals(pool_pc_token_mint)?.into()));

    _verify_raydium_pools_will_be_balanced(
        coin_balance,
        coin_decimals,
        pc_balance,
        pc_decimals,
        coin_amount,
        pc_amount,
        trade_fee_numerator,
        trade_fee_denominator,
        desired_price,
    )
}

#[allow(clippy::too_many_arguments)]
fn _verify_raydium_pools_will_be_balanced(
    coin_balance: u64,
    coin_decimals: Decimal,
    pc_balance: u64,
    pc_decimals: Decimal,
    coin_amount: u64,
    pc_amount: u64,
    trade_fee_numerator: u64,
    trade_fee_denominator: u64,
    desired_price: Decimal,
) -> LucraResult {
    let coin_balance = Decimal::from(coin_balance)
        .checked_div(coin_decimals)
        .ok_or(math_err!())?;

    let pc_balance = Decimal::from(pc_balance)
        .checked_div(pc_decimals)
        .ok_or(math_err!())?;
    
    msg!("intial coin_balance {:?}", coin_balance);
    msg!("intial pc_balance {:?}", pc_balance);

    let coin_amount = Decimal::from(coin_amount)
        .checked_div(coin_decimals)
        .ok_or(math_err!())?;

    let pc_amount = Decimal::from(pc_amount)
        .checked_div(pc_decimals)
        .ok_or(math_err!())?;

    msg!("initial coin_amount: {:?}", coin_amount);
    msg!("initial pc_amount: {:?}", pc_amount);

    let new_price = calculate_new_price(
        coin_balance,
        pc_balance,
        Decimal::from(trade_fee_numerator),
        Decimal::from(trade_fee_denominator),
        coin_amount,
        pc_amount,
    )?;

    let tolerance = Decimal::new(1, 3);
    verify_balanced_pool(
        new_price,
        desired_price,
        tolerance,
    )
}

#[allow(clippy::too_many_arguments)]
fn calculate_new_amounts(
    coin_balance: Decimal,
    pc_balance: Decimal,
    trade_fee_numerator: Decimal,
    trade_fee_denominator: Decimal,
    coin_amount: Decimal,
    pc_amount: Decimal,
) -> LucraResult<(Decimal, Decimal)> {
    let swap_amount = if coin_amount > Decimal::ZERO { coin_amount } else { pc_amount };
    let multiplier = trade_fee_denominator.checked_sub(trade_fee_numerator).ok_or(math_err!())?;
    let swap_amount_with_fee = swap_amount
        .checked_mul(multiplier)
        .ok_or(math_err!())?
        .checked_div(trade_fee_denominator)
        .ok_or(math_err!())?;

    let source_amount = if coin_amount > Decimal::ZERO { coin_balance } else { pc_balance };
    let destination_amount = if coin_amount > Decimal::ZERO { pc_balance } else { coin_balance };

    let new_source_amount = source_amount
        .checked_add(swap_amount_with_fee)
        .ok_or(math_err!())?;

    let amount_out = destination_amount
        .checked_div(new_source_amount)
        .ok_or(math_err!())?
        .checked_mul(swap_amount_with_fee)
        .ok_or(math_err!())?;

    let new_destination_amount = destination_amount
        .checked_sub(amount_out)
        .ok_or(math_err!())?;

    let new_base_amount = if coin_amount > Decimal::ZERO { source_amount.checked_add(swap_amount).unwrap() } else { new_destination_amount };
    let new_quote_amount = if coin_amount > Decimal::ZERO { new_destination_amount } else { source_amount.checked_add(swap_amount).unwrap() };

    Ok((new_base_amount, new_quote_amount))
}

#[allow(clippy::too_many_arguments)]
pub fn verify_raydium_pools_are_balanced<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_coin_token_mint: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    pool_pc_token_mint: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    desired_price: Decimal,
) -> LucraResult {
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
        amm_open_orders,
        amm_id,
    )?;

    let coin_token_decimals = get_mint_decimals(pool_coin_token_mint)?;
    let pc_token_decimals = get_mint_decimals(pool_pc_token_mint)?;

    let coin_balance = Decimal::from(coin_balance)
        .checked_div(Decimal::from(10_i64.pow(coin_token_decimals.into())))
        .ok_or(math_err!())?;

    let pc_balance = Decimal::from(pc_balance)
        .checked_div(Decimal::from(10_i64.pow(pc_token_decimals.into())))
        .ok_or(math_err!())?;

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
) -> LucraResult<(u64, u64, u64, u64)> {
    check_eq!(amm_id.data_len(), 752, LucraErrorCode::InvalidAccountInput)?;

    let amm_id_data = amm_id.try_borrow_data()?;
    let trade_fee_numerator = u64::from_le_bytes(*array_ref![amm_id_data, 144, 8]);
    let trade_fee_denominator = u64::from_le_bytes(*array_ref![amm_id_data, 152, 8]);
    let swap_fee_numerator = u64::from_le_bytes(*array_ref![amm_id_data, 176, 8]);
    let swap_fee_denominator = u64::from_le_bytes(*array_ref![amm_id_data, 184, 8]);

    Ok((
        trade_fee_numerator, trade_fee_denominator,
        swap_fee_numerator, swap_fee_denominator,
    ))
}

#[derive(Clone, Copy, Debug)]
pub struct RaydiumSwap {
    pub instruction: u8,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

impl RaydiumSwap {
    pub const LEN: usize = 17;

    pub fn get_size(&self) -> usize {
        RaydiumSwap::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_eq!(output.len(), RaydiumSwap::LEN, LucraErrorCode::InvalidAccountInput)?;

        let output = array_mut_ref![output, 0, RaydiumSwap::LEN];

        let (instruction_out, amount_in_out, min_amount_out_out) = mut_array_refs![output, 1, 8, 8];

        instruction_out[0] = self.instruction as u8;
        *amount_in_out = self.amount_in.to_le_bytes();
        *min_amount_out_out = self.min_amount_out.to_le_bytes();

        Ok(RaydiumSwap::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumSwap::LEN] = [0; RaydiumSwap::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_new_price() {
        let desired_price = Decimal::new(29_985_527, 6);
        let trade_fee_numerator = Decimal::from(25_u64);
        let trade_fee_denominator = Decimal::from(10_000_u64);
        let coin_balance = Decimal::from(1_000_000_u64);
        let pc_balance = Decimal::from(31_000_000_u64);
        let coin_amount = Decimal::new(16_800_000_000_000, 9);
        let pc_amount = Decimal::from(0_u64);

        let new_price = calculate_new_price(
            coin_balance,
            pc_balance,
            trade_fee_numerator,
            trade_fee_denominator,
            coin_amount,
            pc_amount,
        ).unwrap();

        let tolerance = Decimal::new(1, 3);

        assert!(verify_balanced_pool(new_price, desired_price, tolerance).is_ok());

        let desired_price = Decimal::new(29_985_527, 6);
        let trade_fee_numerator = Decimal::from(25_u64);
        let trade_fee_denominator = Decimal::from(10_000_u64);
        let coin_balance = Decimal::from(1_000_000_u64);
        let pc_balance = Decimal::from(30_000_u64);
        let coin_amount = Decimal::from(0_u64);
        let pc_amount = Decimal::from(919_600_u64);

        let new_price = calculate_new_price(
            coin_balance,
            pc_balance,
            trade_fee_numerator,
            trade_fee_denominator,
            coin_amount,
            pc_amount,
        ).unwrap();

        let tolerance = Decimal::new(1, 3);

        assert!(verify_balanced_pool(new_price, desired_price, tolerance).is_ok());
    }

    #[test]
    fn test_calculate_new_amounts() {
        let coin_balance = Decimal::new(70_890_477_809, 9);
        let pc_balance = Decimal::new(1_093_131_189, 6);
        let trade_fee_numerator = Decimal::from(25_u64);
        let trade_fee_denominator = Decimal::from(10_000_u64);
        let coin_amount = Decimal::new(46_074_775, 9);
        let pc_amount = Decimal::from(0_u64);

        let (base, quote) = calculate_new_amounts(coin_balance, pc_balance, trade_fee_numerator, trade_fee_denominator, coin_amount, pc_amount).unwrap();
        assert_eq!(base.round_dp(9), Decimal::new(70_936_552_584, 9));
        assert_eq!(quote.round_dp(6), Decimal::new(1_092_422_951, 6));

        let new_price = quote.checked_div(base).unwrap();
        assert_eq!(new_price.round_dp(9), Decimal::new(15_400_000_585, 9));
    }

    #[test]
    fn test_verify_raydium_pools_will_be_balanced() {
        let coin_balance = 139_396_44;
        let coin_decimals = Decimal::from(100_u64);
        let pc_balance = 5_532_953_37;
        let pc_decimals = Decimal::from(100_u64);
        let trade_fee_numerator = 25;
        let trade_fee_denominator = 10_000;
        let coin_amount = 10_000_u64;
        let pc_amount = 0_u64;
        let desired_price = Decimal::new(39_635_327, 6);

        let result = _verify_raydium_pools_will_be_balanced(
            coin_balance,
            coin_decimals,
            pc_balance,
            pc_decimals,
            coin_amount,
            pc_amount,
            trade_fee_numerator,
            trade_fee_denominator,
            desired_price
        );
        assert_eq!(result, Ok(()));

        let desired_price = Decimal::new(39_693_649, 6);
        let result = _verify_raydium_pools_will_be_balanced(
            coin_balance,
            coin_decimals,
            pc_balance, 
            pc_decimals,
            pc_amount,
            coin_amount,
            trade_fee_numerator,
            trade_fee_denominator,
            desired_price
        );
        assert_eq!(result, Ok(()));

        let desired_price = Decimal::new(31_034_500, 6);
        let result = _verify_raydium_pools_will_be_balanced(
            1_000_000,
            Decimal::from(1_u64),
            29_000_000,
            Decimal::from(1_u64),
            0,
            1_000_000,
            0,
            1,
            desired_price,
        );
        assert_eq!(result, Ok(()));
    }
}