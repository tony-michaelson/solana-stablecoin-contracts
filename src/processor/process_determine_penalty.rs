use std::cell::{Ref, RefMut};

use arrayref::array_ref;
use rust_decimal_macros::dec;
use solana_program::{
    account_info::AccountInfo,
    clock::UnixTimestamp,
    sysvar::{clock::Clock, Sysvar},
    pubkey::Pubkey,
    native_token::LAMPORTS_PER_SOL,
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use time::{OffsetDateTime, Time};
use crate::{
    error::{
        check_assert,
        LucraError,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::{
        LAMPORTS_PER_LUCRA,
        PRICE_HISTORY_ID,
        SOL_MATA_ORACLE,
        SOL_USDT_ORACLE,
        SOL_USDC_ORACLE,
    },
    helpers::math::*,
    helpers::oracle::*,
    state::{
        MataLoan,
        PriceHistory, 
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::DeterminePenalty);

/// Anyone can run this contract in order to determine penalty that needs to be harvested on a loan
#[inline(never)]
pub fn process_determine_penalty(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 10;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,            // read
        loan_ai,                    // write

        sol_usdc_oracle_ai,         // read
        sol_usdt_oracle_ai,         // read
        sol_mata_oracle_ai,         // read
        price_history_ai,           // read

        user_reward_account_ai,     // write
        reward_mint_ai,             // write
        reward_mint_authority_ai,   // read
        token_program_ai,           // read
    ] = accounts;

    let clock = &Clock::get()?;

    // Verify the accounts are owned by the right programs
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(loan_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdc_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_usdt_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(sol_mata_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(price_history_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(user_reward_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(reward_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    
    // Verify accounts against expectations
    check_eq!(sol_usdc_oracle_ai.key, &SOL_USDC_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.key, &SOL_USDT_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_mata_oracle_ai.key, &SOL_MATA_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(price_history_ai.key, &PRICE_HISTORY_ID, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(token_program_ai.key, &spl_token::id(), LucraErrorCode::InvalidAccountInput)?;
 
    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check_eq!(&system_state.reward_mint.address, reward_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;

    let mut loan: RefMut<MataLoan> = MataLoan::load_mut_checked(loan_ai, program_id)?;
    check_eq!(loan.repaid, false, LucraErrorCode::InvalidAccountInput)?;
    check!(loan.penalty_harvested < loan.sol_collateral_amount, LucraErrorCode::InvalidAmount)?;

    let mata_market_price = get_mata_price(sol_mata_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;

    let penalty_multiplier = calculate_penalty_multiplier(mata_market_price)?;
    let penalty_to_charge = accumulate_penalty_rate_charge(price_history_ai, &loan, penalty_multiplier, program_id)?;

    loan.add_penalty_to_harvest(penalty_to_charge);
    loan.update_last_day_penalty_was_checked(clock.unix_timestamp);

    // Pay the user for running the contract
    system_state.mint_reward(
        program_id, 
        reward_mint_ai, 
        user_reward_account_ai, 
        1, 
        reward_mint_authority_ai, 
        token_program_ai
    )?;

    Ok(())
}

#[inline(never)]
fn calculate_penalty_multiplier(mata_price: Decimal) -> LucraResult<u64> {
    // penalty multiplier is based off of how much the mata price has deviated from the peg
    // for every 5 cents = 2x

    // if mata is above the peg then there is no penalty multiplier
    if mata_price >= Decimal::from(1_u64) {
        return Ok(1)
    }

    let five_cents = Decimal::new(5, 2);
    let remainder = mata_price
        .checked_div(five_cents)
        .ok_or(math_err!())?
        .floor()
        .to_u64()
        .ok_or(math_err!())?;
    let expected_remainder = 20;

    let multiplier = (expected_remainder - remainder) * 2;

    Ok(multiplier)
}

// Will find the penalty owed for days that have passed.
// Does not update the penalty_to_harvest field.
#[inline(never)]
fn accumulate_penalty_rate_charge<'a>(price_history_ai: &AccountInfo<'a>, loan: &RefMut<MataLoan>, penalty_multiplier: u64, program_id: &Pubkey) -> LucraResult<u64> {
    let price_history: Box<Ref<PriceHistory>> = PriceHistory::load_checked(price_history_ai, program_id)?;
    let clock = &Clock::get()?;

    _accumulate_penalty_rate_charge(&price_history, loan, penalty_multiplier, clock.unix_timestamp)
}

#[inline(never)]
fn _accumulate_penalty_rate_charge(price_history: &Ref<PriceHistory>, loan: &RefMut<MataLoan>, penalty_multiplier: u64, timestamp: UnixTimestamp) -> LucraResult<u64> {
    let time = Time::from_hms(0, 0, 0).unwrap();
    let today =  OffsetDateTime::from_unix_timestamp(timestamp)
        .unwrap()
        .replace_time(time)
        .unix_timestamp();
    let date_last_harvested = OffsetDateTime::from_unix_timestamp(loan.last_day_penalty_was_checked)
        .unwrap()
        .replace_time(time)
        .unix_timestamp();

    let mut collateral_value;
    let mut annual_penalty_rate;
    let mut penalty_rate = 0_u64;
    let one_day = dec!(1).checked_div(356.into()).unwrap();
    
    for history in price_history.prices.iter() {
        // if the price is 0 then the day was invalid. Skip charging any penalty for that day
        if history.sol_price == 0 || history.lucra_price == 0 {
            continue;
        }

        // If the loan was created after the date for the price we can filter it out that day
        if history.date < loan.loan_creation_date {
            continue;
        }

        // Don't process anything for todays date
        if history.date == today {
            continue;
        }

        // only run on days that haven't been harvested
        if history.date > date_last_harvested {
            // Find value of collateral for given day
            collateral_value = calculate_collateral_value(history.sol_price, history.sol_decimals, loan.sol_collateral_amount, history.lucra_price, history.lucra_decimals, loan.staking_collateral_amount).unwrap();
            // Find the penalty rate for the collateral
            annual_penalty_rate = loan.calc_penalty_rate_percentage(collateral_value)?;
            // Calculate how much penalty to charge
            penalty_rate += calculate_annual_interest_rate(annual_penalty_rate, loan.sol_collateral_amount, one_day)? * penalty_multiplier;
        }
    }

    // cap penalty at loan sol collateral amount
    if penalty_rate > loan.sol_collateral_amount {
        penalty_rate = loan.sol_collateral_amount;
    }

    if penalty_rate + loan.penalty_harvested > loan.sol_collateral_amount {
        penalty_rate = loan.sol_collateral_amount - loan.penalty_harvested;
    }

    Ok(penalty_rate)
}

#[inline(never)]
fn calculate_collateral_value(sol_price: u64, sol_decimals: u8, sol_collateral_amount: u64, lucra_price: u64, lucra_decimals: u8, staking_collateral_amount: u64) -> LucraResult<Decimal> {
    // Calculate the value of the collateral
    let sol_market_price = get_price(sol_price, sol_decimals)?;
    let lucra_market_price = get_price(lucra_price, lucra_decimals)?;
    let sol_side = Decimal::from(sol_collateral_amount)
        .checked_mul(sol_market_price)
        .ok_or(math_err!())?
        .checked_div(LAMPORTS_PER_SOL.into())
        .ok_or(math_err!())?;
    let lucra_side = Decimal::from(staking_collateral_amount)
        .checked_mul(lucra_market_price)
        .ok_or(math_err!())?
        .checked_div(LAMPORTS_PER_LUCRA)
        .ok_or(math_err!())?;

    sol_side
        .checked_add(lucra_side)
        .ok_or(math_err!())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use crate::state::HistoricPrice;

    #[test]
    fn test_accumulate_penalty_rate() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        prices[0] = HistoricPrice {
            sol_price: 20_000_000, // 20 dollars
            sol_decimals: 6,
            lucra_price: 1_000_000, // 1 dollar
            lucra_decimals: 6,
            date: 1,
            padding: [0; 6],
        };
        prices[1] = HistoricPrice {
            sol_price: 20_000_000, // 20 dollars
            sol_decimals: 6,
            lucra_price: 1_000_000, // 1 dollar
            lucra_decimals: 6,
            date: 2,
            padding: [0; 6],
        };
        prices[2] = HistoricPrice {
            sol_price: 20_000_000, // 20 dollars
            sol_decimals: 6,
            lucra_price: 1_000_000, // 1 dollar
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[3] = HistoricPrice {
            sol_price: 20_000_000, // 20 dollars
            sol_decimals: 6,
            lucra_price: 1_000_000, // 1 dollar
            lucra_decimals: 6,
            date: 4,
            padding: [0; 6],
        };

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 20_000_000,
            loan_amount: 133_333_333,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 0;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_max_bad() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        prices[0] = HistoricPrice {
            sol_price: 5_000_000,
            sol_decimals: 6,
            lucra_price: 30_000,
            lucra_decimals: 6,
            date: 1,
            padding: [0; 6],
        };
        prices[1] = HistoricPrice {
            sol_price: 5_000_000,
            sol_decimals: 6,
            lucra_price: 30_000,
            lucra_decimals: 6,
            date: 2,
            padding: [0; 6],
        };
        prices[2] = HistoricPrice {
            sol_price: 5_000_000,
            sol_decimals: 6,
            lucra_price: 30_000,
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[3] = HistoricPrice {
            sol_price: 5_000_000,
            sol_decimals: 6,
            lucra_price: 30_000,
            lucra_decimals: 6,
            date: 4,
            padding: [0; 6],
        };

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 50_000_000_000,   // Sol price at time of loan
            loan_amount: 233_333_333,   // In mata
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        // mata and lucra price have tanked hard enough that there is less than a 25% of the collateral left.

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 4_044_943_820;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_mixed_bag() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        prices[0] = HistoricPrice {
            sol_price: 20_000_000,
            sol_decimals: 6,
            lucra_price: 1_000_000,
            lucra_decimals: 6,
            date: 1,
            padding: [0; 6],
        };
        prices[1] = HistoricPrice {
            sol_price: 500_000,
            sol_decimals: 6,
            lucra_price: 500_000,
            lucra_decimals: 6,
            date: 2,
            padding: [0; 6],
        };
        prices[2] = HistoricPrice {
            sol_price: 55_000_000,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[3] = HistoricPrice {
            sol_price: 25_000_000,
            sol_decimals: 6,
            lucra_price: 500_000,
            lucra_decimals: 6,
            date: 4,
            padding: [0; 6],
        };

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 50_000_000,
            loan_amount: 233_333_333,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 460_674_156;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_skips_days() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        prices[0] = HistoricPrice {
            sol_price: 25_000_000,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 1,
            padding: [0; 6],
        };
        prices[1] = HistoricPrice {
            sol_price: 10_000_000,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 2,
            padding: [0; 6],
        };
        prices[2] = HistoricPrice {
            sol_price: 50_000_000,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[3] = HistoricPrice {
            sol_price: 0,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[4] = HistoricPrice {
            sol_price: 0,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[5] = HistoricPrice {
            sol_price: 1_000_000,
            sol_decimals: 6,
            lucra_price: 500_000,
            lucra_decimals: 6,
            date: 4,
            padding: [0; 6],
        };

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 50_000_000,
            loan_amount: 233_333_333,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 688_202_246;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_skips_days_before_loan_was_created() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        prices[0] = HistoricPrice {
            sol_price: 20_000_000,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 1,
            padding: [0; 6],
        };
        prices[1] = HistoricPrice {
            sol_price: 50_000,
            sol_decimals: 6,
            lucra_price: 50_000,
            lucra_decimals: 6,
            date: 2,
            padding: [0; 6],
        };
        prices[2] = HistoricPrice {
            sol_price: 200_000_000,
            sol_decimals: 6,
            lucra_price: 100_000,
            lucra_decimals: 6,
            date: 3,
            padding: [0; 6],
        };
        prices[3] = HistoricPrice {
            sol_price: 1_000_000,
            sol_decimals: 6,
            lucra_price: 500_000,
            lucra_decimals: 6,
            date: 4,
            padding: [0; 6],
        };

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 50_000_000,
            loan_amount: 233_333_333,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 1_573_033_707;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_cannot_be_over_sol_collateral_amount() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        for i in 0..20 {
            prices[i] = HistoricPrice {
                sol_price: 50_000,
                sol_decimals: 6,
                lucra_price: 50_000,
                lucra_decimals: 6,
                date: i as i64,
                padding: [0; 6],
            };
        }

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 20_000_000,
            loan_amount: 133_333_333,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 10_000_000_000;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_cannot_overflow_the_sol_collateral_amount() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        for i in 0..6 {
            prices[i] = HistoricPrice {
                sol_price: 50_000,
                sol_decimals: 6,
                lucra_price: 50_000,
                lucra_decimals: 6,
                date: i as i64,
                padding: [0; 6],
            };
        }

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 20_000_000,
            loan_amount: 133_333_333,
            penalty_harvested: 5_000_000_000,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 0).unwrap();
        let expected = 5_000_000_000;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_doesnt_process_todays_date() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        for i in 0..7 {
            prices[i] = HistoricPrice {
                sol_price: 50_000,
                sol_decimals: 6,
                lucra_price: 50_000,
                lucra_decimals: 6,
                date: i as i64,
                padding: [0; 6],
            };
        }

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 20_000_000,
            loan_amount: 133_333_333,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 1, 7).unwrap();
        let expected = 6_067_415_730;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_accumulate_penalty_rate_factors_price_multiplier() {
        let mut prices = [
            HistoricPrice {
                ..HistoricPrice::default()
            }; 30
        ];
        for i in 0..7 {
            prices[i] = HistoricPrice {
                sol_price: 5_000_000,
                sol_decimals: 6,
                lucra_price: 35_000,
                lucra_decimals: 6,
                date: i as i64,
                padding: [0; 6],
            };
        }

        let price_history = PriceHistory {
            prices,
            ..PriceHistory::default()
        };
        let c = RefCell::new(price_history);
        let b1 = c.borrow();
        let b2 = Ref::map(b1, |data| data);
        let price_history = Box::from(b2);

        let loan = MataLoan {
            sol_collateral_amount: 10 * LAMPORTS_PER_SOL,
            staking_collateral_amount: 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(),
            market_price: 50_000_000,
            loan_amount: 233_333_333,
            collateral_rate: 300,
            ..MataLoan::default()
        };
        let c = RefCell::new(loan);
        let b1 = c.borrow_mut();
        let b2 = RefMut::map(b1, |data| data);

        let actual = _accumulate_penalty_rate_charge(&price_history, &b2, 2, 7).unwrap();
        let expected = 10_000_000_000;

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_collateral_value() {
        let sol_price = 20_000_000; // 20 dollars
        let sol_decimals = 6;
        let lucra_price = 1_000_000; // 1 dollar
        let lucra_decimals = 6;

        let sol_collateral_amount = 10 * LAMPORTS_PER_SOL; // 200 dollars of sol
        let staking_collateral_amount = 0;

        let actual = calculate_collateral_value(sol_price, sol_decimals, sol_collateral_amount, lucra_price, lucra_decimals, staking_collateral_amount).unwrap();
        let expected = Decimal::from(200_u64);
        
        assert_eq!(actual, expected);
        
        let staking_collateral_amount = 200 * LAMPORTS_PER_LUCRA.to_u64().unwrap(); // 200 dollars of lucra
        let actual = calculate_collateral_value(sol_price, sol_decimals, sol_collateral_amount, lucra_price, lucra_decimals, staking_collateral_amount).unwrap();
        let expected = Decimal::from(400_u64);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_calculate_penalty_multiplier() {
        let mata_price = Decimal::from(1_u64);

        let actual = calculate_penalty_multiplier(mata_price).unwrap();
        let expected = 1;

        assert_eq!(actual, expected);

        let mata_price = Decimal::new(95, 2);
        let actual = calculate_penalty_multiplier(mata_price).unwrap();
        let expected = 2;

        assert_eq!(actual, expected);

        let mata_price = Decimal::new(90, 2);
        let actual = calculate_penalty_multiplier(mata_price).unwrap();
        let expected = 4;

        assert_eq!(actual, expected);

        let mata_price = Decimal::new(60, 2);
        let actual = calculate_penalty_multiplier(mata_price).unwrap();
        let expected = 16;

        assert_eq!(actual, expected);
    }
}