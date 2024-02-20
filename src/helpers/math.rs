use rust_decimal::{Decimal, prelude::ToPrimitive};
use crate::{
    error::{
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
};

declare_check_assert_macros!(SourceFileId::Math);

pub fn calculate_annual_interest_rate(rate: u32, starting_amount: u64, time: Decimal) -> LucraResult<u64> {
    // A = P(1 + rt)
    // A = Total Accrued Amount
    // P = Principal
    // r = Rate of Interest per year
    // t = time period

    let rate = Decimal::new(rate.into(), 2);
    let t = time;
    let p = Decimal::from(starting_amount);
    let a = t
        .checked_mul(rate)
        .unwrap()
        .checked_mul(p)
        .unwrap()
        .floor()
        .to_u64()
        .unwrap();

    Ok(a)
}

pub fn ceiling_division(dividend: Decimal, divisor: Decimal) -> LucraResult<(Decimal, Decimal)> {
    let quotient = dividend
        .checked_div(divisor)
        .ok_or(math_err!())?;

    if quotient == Decimal::ZERO {
        return Ok((Decimal::ZERO, divisor));
    }
  
    Ok((quotient, divisor))
}

pub fn get_no_fee_amount(
    amount: Decimal,
    fee_numerator: Decimal,
    fee_denominator: Decimal,
) -> LucraResult<u64> {
    if amount == Decimal::ZERO {
        return Ok(0)
    }

    let value = fee_numerator
        .checked_div(fee_denominator)
        .ok_or(math_err!())?
        .checked_mul(amount)
        .ok_or(math_err!())?;

    let max = std::cmp::max(value, Decimal::from(1_u64));

    amount
        .checked_sub(max)
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_annual_interest_rate() {
        let inflation_rate = 10; // 10%
        let amount = 10_000;
        let time = Decimal::from(1_u64).checked_div(52.into()).unwrap();
        let expected = 19;

        let actual = calculate_annual_interest_rate(inflation_rate, amount, time).unwrap();
        assert_eq!(expected, actual);
    }
}