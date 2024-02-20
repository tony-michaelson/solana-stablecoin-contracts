mod process_create_mata_loan;
mod process_close_out_mata_loan;
mod process_initialize;
mod process_create_staking_account;
mod process_create_stake_balance;
mod process_deposit_stake;
mod process_stake;
mod process_start_unstake;
mod process_end_unstake;
mod process_withdraw_stake;
mod process_claim_reward;
mod process_drop_reward;
mod process_update_state;
mod process_transfer_funds;
mod process_redeem_reward_tokens;
mod process_add_collateral;
mod process_determine_penalty;
mod process_harvest_penalty;
mod process_create_price_history;
mod process_update_price_history;
mod process_sell_funds_for_arb;
mod process_buy_burn_for_arb;
mod process_clean_up_arb;
mod process_mint_funds_for_arb;

use crate::instruction::Instruction;

use process_create_mata_loan::*;
use process_close_out_mata_loan::*;
use process_initialize::*;
use process_create_staking_account::*;
use process_create_stake_balance::*;
use process_deposit_stake::*;
use process_stake::*;
use process_start_unstake::*;
use process_end_unstake::*;
use process_withdraw_stake::*;
use process_claim_reward::*;
use process_drop_reward::*;
use process_update_state::*;
use process_transfer_funds::*;
use process_redeem_reward_tokens::*;
use process_add_collateral::*;
use process_determine_penalty::*;
use process_harvest_penalty::*;
use process_create_price_history::*;
use process_update_price_history::*;
use process_sell_funds_for_arb::*;
use process_buy_burn_for_arb::*;
use process_clean_up_arb::*;
use process_mint_funds_for_arb::*;

use bincode::deserialize;
use std::{
    convert::TryFrom,
};
use solana_program::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
};
use crate::{
    error::{
        LucraResult,
    },
    state::{
        AmmTypes,
        staking::StakingTimeframe,
        StateParams,
        UpdateStateParams,
        CurrencyTypes,
    },
};

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> LucraResult {
    let instruction = deserialize::<Instruction>(instruction_data).unwrap();

    match instruction {
        Instruction::CreateMataLoan {
            lamports,
        } => {
            msg!("Instruction: Create Loan");
            process_create_mata_loan(
                program_id,
                lamports,
                accounts,
            )
        }
        Instruction::CloseOutMataLoan {
            unstake_msol,
        } => {
            msg!("Instruction: Close Loan");
            process_close_out_mata_loan(
                program_id,
                unstake_msol,
                accounts,
            )
        }
        Instruction::Initialize {
            min_deposit,
            collateral_requirement,
            epoch,
            loans_enabled,
            staking_enabled,
            arbitrage_enabled,
            peg_check_enabled,
            max_amount_of_lucra_to_mint,
            daily_arb_limit,
            maximum_outstanding_mata,
            lcp,
        } => {
            msg!("Instruction: Initialize");
            let state_params = StateParams {
                min_deposit,
                collateral_requirement,
                epoch,
                loans_enabled,
                staking_enabled,
                arbitrage_enabled,
                peg_check_enabled,
                max_amount_of_lucra_to_mint,
                daily_arb_limit,
                maximum_outstanding_mata,
                lcp,
            };
            process_initialize(program_id, &state_params, accounts)
        }
        Instruction::CreateStakingAccount { } => {
            msg!("Instruction: Create Staking Account");
            process_create_staking_account(program_id, accounts)
        }
        Instruction::CreateStakeBalance {
            nonce,
            staking_timeframe,
        } => {
            msg!("Instruction: Create Stake Balance");
            let staking_timeframe = StakingTimeframe::try_from(staking_timeframe).unwrap();
            process_create_stake_balance(program_id, nonce, staking_timeframe, accounts)
        }
        Instruction::DepositStake {
            lucra,
        } => {
            msg!("Instruction: Deposit Stake");
            process_deposit_stake(program_id, lucra, accounts)
        }
        Instruction::Stake {
            lucra,
        } => {
            msg!("Instruction: Stake");
            process_stake(program_id, lucra, accounts)
        }
        Instruction::StartUnstake {
            lucra,
        } => {
            msg!("Instruction: Start Unstake");
            process_start_unstake(program_id, lucra, accounts)
        }
        Instruction::EndUnstake { } => {
            msg!("Instruction: End Unstake");
            process_end_unstake(program_id, accounts)
        }
        Instruction::WithdrawStake {
            lucra,
        } => {
            msg!("Instruction: Withdraw Stake");
            process_withdraw_stake(program_id, lucra, accounts)
        }
        Instruction::ClaimReward { } => {
            msg!("Instruction: Claim Reward");
            process_claim_reward(program_id, accounts)
        }
        Instruction::DropReward { } => {
            msg!("Instruction: Drop Reward");
            process_drop_reward(program_id, accounts)
        }
        Instruction::UpdateState {
            min_deposit,
            collateral_requirement,
            loans_enabled,
            staking_enabled,
            arbitrage_enabled,
            peg_check_enabled,
            max_amount_of_lucra_to_mint,
            daily_arb_limit,
            maximum_outstanding_mata,
            minimum_harvest_amount,
            reward_fee,
            lcp,
        } => {
            msg!("Instruction: Update State");
            let state_params = UpdateStateParams {
                min_deposit,
                collateral_requirement,
                loans_enabled,
                staking_enabled,
                arbitrage_enabled,
                peg_check_enabled,
                max_amount_of_lucra_to_mint,
                daily_arb_limit,
                maximum_outstanding_mata,
                minimum_harvest_amount,
                reward_fee,
                lcp,
            };
            process_update_state(program_id, &state_params, accounts)
        }
        Instruction::TransferFunds {
            lamports,
        } => {
            msg!("Instruction: Transfer Funds");
            process_transfer_funds(program_id, lamports, accounts)
        }
        Instruction::CreatePriceHistory { } => {
            msg!("Instruction: Create Price History");
            process_create_price_history(program_id, accounts)
        }
        Instruction::UpdatePriceHistory { } => {
            msg!("Instruction: Update Price History");
            process_update_price_history(program_id, accounts)
        }
        Instruction::RedeemRewardTokens {
            reward_tokens,
        } => {
            msg!("Instruction: Redeem Oracle Reward");
            process_redeem_reward_tokens(program_id, reward_tokens, accounts)
        }
        Instruction::AddCollateral {
            lamports,
        } => {
            msg!("Instruction: Add Collateral");
            process_add_collateral(program_id, lamports, accounts)
        }
        Instruction::DeterminePenalty { } => {
            msg!("Instruction: Determine Penalty");
            process_determine_penalty(program_id, accounts)
        }
        Instruction::HarvestPenalty { 
            amm_type
        } => {
            msg!("Instruction: Harvest Penalty");
            let amm_type = AmmTypes::try_from(amm_type).unwrap();
            process_harvest_penalty(program_id, amm_type, accounts)
        }
        Instruction::SellFundsForArb { 
            fund_source,
            amm_type,
            lamports,
        } => {
            msg!("Instruction: Sell Funds for Arb");
            let fund_source = CurrencyTypes::try_from(fund_source).unwrap();
            let amm_type = AmmTypes::try_from(amm_type).unwrap();
            process_sell_funds_for_arb(program_id, fund_source, amm_type, lamports, accounts)
        }
        Instruction::BuyBurnForArb { 
            fund_source,
            amm_type,
            lamports,
        } => {
            msg!("Instruction: Buy Burn for Arb");
            let fund_source = CurrencyTypes::try_from(fund_source).unwrap();
            let amm_type = AmmTypes::try_from(amm_type).unwrap();
            process_buy_burn_for_arb(program_id, fund_source, amm_type, lamports, accounts)
        }
        Instruction::CleanUpArb {} => {
            msg!("Instruction: Clean Up Arb");
            process_clean_up_arb(program_id, accounts)
        }
        Instruction::MintFundsForArb { 
            fund_source,
            amm_type,
            lamports,
        } => {
            msg!("Instruction: Mint Funds for Arb");
            let fund_source = CurrencyTypes::try_from(fund_source).unwrap();
            let amm_type = AmmTypes::try_from(amm_type).unwrap();
            process_mint_funds_for_arb(program_id, fund_source, amm_type, lamports, accounts)
        }
    }
}