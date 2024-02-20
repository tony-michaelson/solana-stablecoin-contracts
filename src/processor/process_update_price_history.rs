use std::cell::{Ref, RefMut};

use arrayref::array_ref;
use rust_decimal_macros::dec;
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
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
    helpers::constants::{
        PRICE_HISTORY_ID,
        SOL_USDC_ORACLE,
        SOL_USDT_ORACLE,
        LUCRA_SOL_ORACLE,
        UNIX_HOUR,
    },
    helpers::oracle::{get_lucra_price, get_sol_price},
    state::{
        PriceHistory,
        SystemState,
    },
};

declare_check_assert_macros!(SourceFileId::UpdatePriceHistory);

#[inline(never)]
pub fn process_update_price_history(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 9;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        system_state_ai,            // read

        price_history_ai,           // write

        sol_usdc_oracle_ai,         // read
        sol_usdt_oracle_ai,         // read
        lucra_sol_oracle_ai,        // read

        user_reward_account_ai,     // write
        reward_mint_ai,             // write
        reward_mint_authority_ai,   // read
        token_program_ai,           // read
    ] = accounts;

    let clock = &Clock::get()?;

    // Verify the accounts are owned by the right programs
    check_eq!(system_state_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(price_history_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdc_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(lucra_sol_oracle_ai.owner, program_id, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(user_reward_account_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;
    check_eq!(reward_mint_ai.owner, &spl_token::id(), LucraErrorCode::InvalidAccountOwner)?;

    // Verify the oracles
    check_eq!(sol_usdc_oracle_ai.key, &SOL_USDC_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(sol_usdt_oracle_ai.key, &SOL_USDT_ORACLE, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(lucra_sol_oracle_ai.key, &LUCRA_SOL_ORACLE, LucraErrorCode::InvalidAccountInput)?;

    check_eq!(price_history_ai.key, &PRICE_HISTORY_ID, LucraErrorCode::InvalidAccountInput)?;
    
    let system_state: Ref<SystemState> = SystemState::load_checked(system_state_ai, program_id)?;
    check_eq!(&system_state.reward_mint.address, reward_mint_ai.key, LucraErrorCode::InvalidAccountInput)?;
    
    // Check to see if the price can be updated (should be atleast 1 hour difference)
    let mut price_history: Box<RefMut<PriceHistory>> = PriceHistory::load_mut_checked(price_history_ai, program_id)?;
    let price_history_update_counter_before = price_history.update_counter;
    check!(price_history.last_update_timestamp + UNIX_HOUR <= clock.unix_timestamp, LucraErrorCode::InsufficientTimePassed)?;
    
    let sol_price = get_sol_price(sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;
    let lucra_price = get_lucra_price(lucra_sol_oracle_ai, sol_usdc_oracle_ai, sol_usdt_oracle_ai, clock)?;

    // where does the current unix timestamp sit between the intervals?
    let interval_start = price_history.interval_start;
    if clock.unix_timestamp >= interval_start 
        && clock.unix_timestamp <= price_history.interval_end() {
            // We are between the intervals and should record a price
            let current_interval_historic_price = price_history.find_price_by_timestamp(interval_start);
            
            if let Some(historic_price) = current_interval_historic_price {
                // We have a price for the interval
                // Update the price by averaging the new price in
                let decimals = 6_u8;
                let exponent = Decimal::from(10_i64.pow(decimals.into()));
                historic_price.sol_price = sol_price
                    .checked_mul(exponent)
                    .ok_or(math_err!())?
                    .checked_add(historic_price.sol_price.into())
                    .ok_or(math_err!())?
                    .checked_div(dec!(2))
                    .ok_or(math_err!())?
                    .floor()
                    .to_u64()
                    .ok_or(math_err!())?;

                historic_price.lucra_price = lucra_price
                    .checked_mul(exponent)
                    .ok_or(math_err!())?
                    .checked_add(historic_price.lucra_price.into())
                    .ok_or(math_err!())?
                    .checked_div(dec!(2))
                    .ok_or(math_err!())?
                    .floor()
                    .to_u64()
                    .ok_or(math_err!())?;          
            } else {
                // We didn't have a price to update for that interval. 
                // Create a new one for the current interval
                // make sure the counter is reset
                price_history.reset_counter();

                // find a price for the new day
                let decimals = 6_u8;
                let exponent = Decimal::from(10_u64.pow(decimals.into()));
                let sol_price = sol_price
                    .checked_mul(exponent)
                    .ok_or(math_err!())?
                    .floor()
                    .to_u64()
                    .ok_or(math_err!())?;
                let lucra_price = lucra_price
                    .checked_mul(exponent)
                    .ok_or(math_err!())?
                    .floor()
                    .to_u64()
                    .ok_or(math_err!())?;
                
                price_history.replace_oldest_price(
                    interval_start,
                    sol_price,
                    decimals,
                    lucra_price,
                    decimals,
                );
            }
    } else if clock.unix_timestamp > price_history.interval_end() {
        // we are on a new interval
        // verify the previous intervals price was updated enough times
        // start a new price

        if price_history_update_counter_before < 12 {
            let current_interval_price = price_history.find_current_interval_price();
            
            if let Some(value) = current_interval_price {
                value.zero_out_prices();
            }
        }

        price_history.reset_counter();
        price_history.update_interval(clock.unix_timestamp);

        let interval_start = price_history.interval_start;
        let decimals = 6_u8;
        let exponent = Decimal::from(10_i64.pow(decimals.into()));
        let sol_price = sol_price
            .checked_mul(exponent)
            .ok_or(math_err!())?
            .floor()
            .to_u64()
            .ok_or(math_err!())?;
        let lucra_price = lucra_price
            .checked_mul(exponent)
            .ok_or(math_err!())?
            .floor()
            .to_u64()
            .ok_or(math_err!())?;

        price_history.replace_oldest_price(
            interval_start,
            sol_price,
            decimals,
            lucra_price,
            decimals,
        );
    } else {
        // we are before the start of the interval. This is a bad state
        return Err(throw_err!(LucraErrorCode::Default));
    }

    // increment the counter and update the timestamp
    price_history.increment_counter();
    price_history.last_update_timestamp = clock.unix_timestamp;

    // Pay the user for their efforts
    system_state.mint_reward(
        program_id,
        reward_mint_ai,
        user_reward_account_ai,
        1,
        reward_mint_authority_ai,
        token_program_ai,
    )?;

    Ok(())
}