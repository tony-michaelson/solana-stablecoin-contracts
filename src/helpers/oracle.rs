use arrayref::array_ref;
use solana_program::{
    account_info::AccountInfo,
    sysvar::clock::Clock,
    pubkey::Pubkey,
};
use rust_decimal::{Decimal, MathematicalOps};
use oracles::state::Oracle;
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::ORACLE_PRICE_MAX_SLOTS,
};

declare_check_assert_macros!(SourceFileId::OracleHelper);

pub fn verify_orca_has_more_volume(oracle: &Oracle, raydium_market: &Pubkey, orca_market: &Pubkey) -> LucraResult {
    let raydium_vol = get_volume(oracle, raydium_market)?;
    let orca_vol = get_volume(oracle, orca_market)?;

    check!(orca_vol >= raydium_vol, LucraErrorCode::InvalidAmount)?;
    Ok(())
}

pub fn verify_raydium_has_more_volume(oracle: &Oracle, raydium_market: &Pubkey, orca_market: &Pubkey) -> LucraResult {
    let raydium_vol = get_volume(oracle, raydium_market)?;
    let orca_vol = get_volume(oracle, orca_market)?;

    check!(raydium_vol >= orca_vol, LucraErrorCode::InvalidAmount)?;
    Ok(())
}

pub fn check_raydium_has_more_volume(oracle: &Oracle, raydium_market: &Pubkey, orca_market: &Pubkey) -> LucraResult<bool> {
    let raydium_vol = get_volume(oracle, raydium_market)?;
    let orca_vol = get_volume(oracle, orca_market)?;

    Ok(raydium_vol >= orca_vol)
}

pub fn get_volume(oracle: &Oracle, market: &Pubkey) -> LucraResult<u64> {
    let price_source = oracle.find_price_source_by_market(market).unwrap();
    let volume = price_source.agg_price.vol;

    Ok(volume)
}

pub fn get_sol_price(sol_usdc_oracle_ai: &AccountInfo, sol_usdt_oracle_ai: &AccountInfo, clock: &Clock) -> LucraResult<Decimal> {
    let sol_usdc_price = get_oracle_price(sol_usdc_oracle_ai, clock)?;
    let sol_usdt_price = get_oracle_price(sol_usdt_oracle_ai, clock)?;

    Ok(if sol_usdc_price > sol_usdt_price { sol_usdt_price } else { sol_usdc_price })
}

pub fn get_lucra_price(lucra_sol_oracle_ai: &AccountInfo, sol_usdc_oracle_ai: &AccountInfo, sol_usdt_oracle_ai: &AccountInfo, clock: &Clock) -> LucraResult<Decimal> {
    let lucra_sol_price = get_oracle_price(lucra_sol_oracle_ai, clock)?;
    let sol_usd_price = get_sol_price(sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;

    let usd_sol = Decimal::from(1_u64)
        .checked_div(sol_usd_price)
        .ok_or(math_err!())?;
    let lucra_usd_price = Decimal::from(1_u64)
        .checked_div(usd_sol)
        .ok_or(math_err!())?
        .checked_mul(lucra_sol_price)
        .ok_or(math_err!())?;

    Ok(lucra_usd_price)
}

pub fn get_mata_price(sol_mata_oracle_ai: &AccountInfo, sol_usdc_oracle_ai: &AccountInfo, sol_usdt_oracle_ai: &AccountInfo, clock: &Clock) -> LucraResult<Decimal> {
    let sol_mata_price = get_oracle_price(sol_mata_oracle_ai, clock)?;
    let sol_usd_price = get_sol_price(sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;

    let sol_mata = Decimal::from(1_u64)
        .checked_div(sol_mata_price)
        .ok_or(math_err!())?;
    let mata_usd_price = sol_mata
        .checked_mul(sol_usd_price)
        .ok_or(math_err!())?;

    Ok(mata_usd_price)
}

pub fn get_oracle_price(oracle_ai: &AccountInfo, clock: &Clock) -> LucraResult<Decimal> {
    let price_data = oracle_ai.try_borrow_data()?;
    let price = u64::from_le_bytes(*array_ref![price_data, 11_097, 8]);
    let valid_slot = u64::from_le_bytes(*array_ref![price_data, 11_105, 8]);
    let expo = u8::from_le_bytes(*array_ref![price_data, 72, 1]);
    let status = u8::from_le_bytes(*array_ref![price_data, 11_222, 1]);

    calc_oracle_price(price, expo, valid_slot, status, clock.slot)
}

fn calc_oracle_price(price: u64, expo: u8, valid_slot: u64, status: u8, current_slot: u64) -> LucraResult<Decimal> {
    if status != 1 {
        return Err(throw_err!(LucraErrorCode::OracleStatusNotValid));
    }

    if valid_slot + ORACLE_PRICE_MAX_SLOTS < current_slot {
        return Err(throw_err!(LucraErrorCode::OracleStale));
    }

    get_price(price, expo)
}

pub fn get_price(price: u64, expo: u8) -> LucraResult<Decimal> {
    Ok(
        Decimal::from(price)
            .checked_div(
                Decimal::TEN.powi(expo as i64)
            )
            .unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_oracle_price() {
        let result = calc_oracle_price(28_050_000, 6, 1_357_892, 1, 1_357_892).unwrap();
        let expected = Decimal::from(28_050_000_u64).checked_div(1_000_000.into()).unwrap(); // 28.05
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calc_oracle_price_should_fail_when_oracle_status_invalid() {
        let result = calc_oracle_price(28_050_000, 6, 1_357_892, 0, 1_357_892);
        assert!(matches!(
            result.unwrap_err(),
            LucraError::LucraErrorCode {
                lucra_error_code: LucraErrorCode::OracleStatusNotValid,
                line: 101,
                source_file_id: SourceFileId::OracleHelper,
            }
        ));
    }

    #[test]
    fn test_calc_oracle_price_should_fail_when_slot_is_stale() {
        let result = calc_oracle_price(28_050_000, 6, 1_357_892, 1, 1_358_000);
        assert!(matches!(
            result.unwrap_err(),
            LucraError::LucraErrorCode {
                lucra_error_code: LucraErrorCode::OracleStale,
                line: 105,
                source_file_id: SourceFileId::OracleHelper,
            }
        ));
    }
}