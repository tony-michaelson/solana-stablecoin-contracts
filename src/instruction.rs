use bincode::serialize;
use serde::{Deserialize, Serialize};
use solana_program::{
    instruction::{AccountMeta, Instruction as SolInstruction},
    pubkey::Pubkey,
};
use crate::{
    helpers::constants::{
        CREATOR_AUTHORITY, DAO_AUTHORITY, orca_swap,
        raydium_v4, serum_v3,
    },
    id,
    state::{
        AmmTypes,
        CurrencyTypes,
        SystemState,
        staking::{
            StakingTimeframe,
            StakingState,
        }, ArbState,
    },
};

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    /// Initializes the program state (system + staking + arb)
    /// 
    /// Accounts expected by this instruction (16):
    ///
    /// 0: `[]` marinade_state_ai
    /// 1: `[]` creator_authority_ai - Single wallet that can sign for initial initialize transaction
    /// 2: `[]` mata_mint_ai
    /// 3: `[]` lucra_mint_ai
    /// 4: `[]` reward_mint_ai
    /// 5: `[]` staked_lucra_mint_ai
    /// 6: `[writable]` system_state_ai
    /// 7: `[writable]` arb_state_ai
    /// 8: `[]` msol_vault_ai
    /// 9: `[]` arb_coffer_ai
    /// 10: `[]` rewards_vault_ai,
    /// 11: `[writable]` staking_state_ai
    /// 12: `[]` arb_fund_ai
    /// 13: `[]` wsol_holding_vault_ai
    /// 14: `[]` mata_holding_vault_ai
    /// 15: `[]` lucra_holding_vault_ai
    Initialize {
        min_deposit: u64,
        collateral_requirement: u32,
        epoch: i64,
        loans_enabled: bool,
        staking_enabled: bool,
        arbitrage_enabled: bool,
        peg_check_enabled: bool,
        max_amount_of_lucra_to_mint: u64,
        daily_arb_limit: u64,
        maximum_outstanding_mata: u64,
        lcp: u8,
    },

    /// DAO instruction for updating the state
    /// 
    /// Accounts expected by this instruction (3)
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[]` dao_authority_ai
    UpdateState {
        min_deposit: u64,
        collateral_requirement: u32,
        loans_enabled: bool,
        staking_enabled: bool,
        arbitrage_enabled: bool,
        peg_check_enabled: bool,
        max_amount_of_lucra_to_mint: u64,
        daily_arb_limit: u64,
        maximum_outstanding_mata: u64,
        minimum_harvest_amount: u64,
        reward_fee: u32,
        lcp: u8,
    },

    /// Creates a mata loan
    /// 
    /// Accounts expected by this instruction (22 or 24):
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` loan_ai
    /// 3: `[writable]` msol_vault_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[]` mata_mint_authority_ai
    /// 6: `[writable]` user_account_ai
    /// 7: `[writable]` user_mata_account_ai
    /// 8: `[writable]` user_msol_account_ai
    /// 9: `[]` sol_usdc_oracle_ai
    /// 10: `[]` sol_usdt_oracle_ai
    /// 11: `[]` sol_mata_oracle_ai
    /// 12: `[writable]` msol_mint_ai
    /// 13: `[writable]` liq_pool_sol_leg_pda_ai
    /// 14: `[writable]` liq_pool_msol_leg_ai
    /// 15: `[]` liq_pool_msol_leg_authority_ai
    /// 16: `[writable]` reserve_pda_ai
    /// 17: `[]` msol_mint_authority_ai
    /// 18: `[]` fees_ai
    /// 19: `[]` system_program_ai
    /// 20: `[]` token_program_ai
    /// 21: `[]` marinade_program_ai
    /// 
    /// or
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` loan_ai
    /// 3: `[writable]` msol_vault_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[]` mata_mint_authority_ai
    /// 6: `[writable]` user_account_ai
    /// 7: `[writable]` user_mata_account_ai
    /// 8: `[writable]` user_msol_account_ai
    /// 9: `[writable]` user_staking_account_ai
    /// 10: `[]` sol_usdc_oracle_ai
    /// 11: `[]` sol_usdt_oracle_ai
    /// 12: `[]` sol_mata_oracle_ai
    /// 13: `[]` lucra_sol_oracle_ai
    /// 14: `[writable]` msol_mint_ai
    /// 15: `[writable]` liq_pool_sol_leg_pda_ai
    /// 16: `[writable]` liq_pool_msol_leg_ai
    /// 17: `[]` liq_pool_msol_leg_authority_ai
    /// 18: `[writable]` reserve_pda_ai
    /// 19: `[]` msol_mint_authority_ai
    /// 20: `[]` fees_ai
    /// 21: `[]` system_program_ai
    /// 22: `[]` token_program_ai
    /// 23: `[]` marinade_program_ai
    CreateMataLoan {
        lamports: u64,
    },

    /// Closes a Mata `loan`
    /// 
    /// Accounts expected by this instruction (16 or 17):
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` loan_ai
    /// 3: `[writable]` user_account_ai
    /// 4: `[writable]` user_msol_account_ai
    /// 5: `[writable]` mata_mint_ai
    /// 6: `[writable]` user_mata_account_ai
    /// 7: `[]` msol_vault_authority_ai
    /// 8: `[writable]` msol_vault_ai
    /// 9: `[writable]` msol_mint_ai
    /// 10: `[writable]` liq_pool_sol_leg_pda_ai
    /// 11: `[writable]` liq_pool_msol_leg_ai
    /// 12: `[writable]` treasury_msol_account_ai
    /// 13: `[]` system_program_ai
    /// 14: `[]` token_program_ai
    /// 15: `[]` marinade_program_ai
    /// 
    /// or
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` loan_ai
    /// 3: `[writable]` user_account_ai
    /// 4: `[writable]` user_msol_account_ai
    /// 5: `[writable]` mata_mint_ai
    /// 6: `[writable]` user_mata_account_ai
    /// 7: `[]` msol_vault_authority_ai
    /// 8: `[writable]` msol_vault_ai
    /// 9: `[writable]` staking_account_ai
    /// 10: `[writable]` msol_mint_ai
    /// 11: `[writable]` liq_pool_sol_leg_pda_ai
    /// 12: `[writable]` liq_pool_msol_leg_ai
    /// 13: `[writable]` treasury_msol_account_ai
    /// 14: `[]` system_program_ai
    /// 15: `[]` token_program_ai
    /// 16: `[]` marinade_program_ai
    CloseOutMataLoan {
        unstake_msol: bool,
    },

    /// Creates a new staking account
    /// 
    /// Accounts expected by this instruction (4):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[]` staking_state_ai
    /// 2: `[writable]` staking_account_ai
    /// 3: `[]` owner_ai
    CreateStakingAccount {},

    /// Creates a new stake balance account
    /// 
    /// Accounts expected by this instruction (7):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[]` staking_state_ai
    /// 2: `[writable]` stake_balance_ai
    /// 3: `[]` owner_ai
    /// 4: `[]` deposit_vault_ai
    /// 5: `[]` stake_vault_ai
    /// 6: `[]` pending_vault_ai
    CreateStakeBalance { nonce: u8, staking_timeframe: u8 },

    /// Deposits lucra into the deposit account
    /// 
    /// Accounts expected by this instruction (6):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[]` stake_balance_ai
    /// 2: `[writable]` from_account_ai
    /// 3: `[writable]` deposit_vault_ai
    /// 4: `[]` owner_ai
    /// 5: `[]` token_program_ai 
    DepositStake { lucra: u64 },

    /// Stakes an amount of deposited tokens. Special care should be taken with this instruction.
    /// Users could have outstanding rewards that they would lose access too if this instruction is invoked before they are claimed.
    /// 
    /// Accounts expected by this instruction (12):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` staking_state_ai
    /// 2: `[writable]` staking_account_ai
    /// 3: `[writable]` stake_balance_ai
    /// 4: `[writable]` deposit_vault_ai
    /// 5: `[writable]` stake_vault_ai
    /// 6: `[]` owner_ai
    /// 7: `[]` transfer_authority_ai
    /// 8: `[writable]` staked_lucra_mint_ai
    /// 9: `[writable]` user_staked_lucra_account_ai
    /// 10: `[]` mint_authority_ai - mint authority for staked lucra
    /// 11: `[writable]` token_program_ai
    Stake { lucra: u64 },

    /// Starts the unstake process for an amount of locked stake.
    /// 
    /// Accounts expected by this instruction (15):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[]` staking_state_ai
    /// 2: `[writable]` staking_account_ai
    /// 3: `[writable]` stake_balance_ai
    /// 4: `[writable]` staked_lucra_mint_ai
    /// 5: `[writable]` user_staked_lucra_account_ai
    /// 6: `[]` owner_ai
    /// 7: `[writable]` stake_vault_ai
    /// 8: `[writable]` pending_vault_ai
    /// 9: `[]` vault_authority_ai
    /// 10: `[writable]` pending_withdrawal_ai
    /// 11: `[]` sol_usdt_oracle_ai
    /// 12: `[]` sol_usdc_oracle_ai
    /// 13: `[]` lucra_sol_oracle_ai
    /// 14: `[]` token_program_ai
    StartUnstake { lucra: u64 },

    /// Ends the unstake process by putting the coins in the deposit account
    /// 
    /// Accounts expected by this instruction (9)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` pending_withdrawal_ai
    /// 2: `[]` stake_balance_ai
    /// 3: `[writable]` pending_vault_ai
    /// 4: `[writable]` deposit_vault_ai
    /// 5: `[]` owner_ai
    /// 6: `[]` transfer_authority_ai 
    /// 7: `[writable]` sol_account_ai 
    /// 8: `[]` token_program_ai 
    EndUnstake {},

    /// Withdraws the deposited lucra from a staking account back to the user's wallet
    /// 
    /// Accounts expected by this instruction (9):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` stake_balance_ai
    /// 2: `[writable]` deposit_vault_ai
    /// 3: `[]` stake_vault_ai
    /// 4: `[]` pending_vault_ai
    /// 5: `[writable]` to_account_ai
    /// 6: `[writable]` owner_ai
    /// 7: `[]` transfer_authority_ai
    /// 8: `[]` token_program_ai 
    WithdrawStake { lucra: u64 },

    /// Claims a user's portion of the staking reward
    /// 
    /// Accounts expected by this instruction (13):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[]` staking_state_ai
    /// 2: `[writable]` stake_balance_ai
    /// 3: `[]` reward_ai
    /// 4: `[]` user_staked_lucra_account_ai
    /// 5: `[writable]` lucra_vault_ai
    /// 6: `[writable]` lucra_account_ai
    /// 7: `[writable]` rewards_vault_ai
    /// 8: `[writable]` msol_account_ai
    /// 9: `[]` rewards_vault_transfer_authority_ai
    /// 10: `[writable]` lucra_mint_ai
    /// 11: `[]` lucra_mint_authority_ai
    /// 12: `[]` token_program_ai
    ClaimReward {},

    /// Puts a staking reward onchain
    /// 
    /// Accounts expected by this instruction (13):
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` staking_state_ai
    /// 2: `[]` marinade_state_ai
    /// 3: `[writable]` reward_ai
    /// 4: `[writable]` stake_mint_ai
    /// 5: `[writable]` msol_vault_ai
    /// 6: `[writable]` rewards_vault_ai
    /// 7: `[writable]` arb_coffer_ai
    /// 8: `[]` msol_vault_transfer_authority_ai
    /// 9: `[writable]` user_reward_account_ai
    /// 10: `[writable]` reward_mint_ai
    /// 11: `[]` reward_mint_authority_ai
    /// 12: `[]` token_program_ai
    DropReward {},

    /// DAO instruction for transfer funds from the msol vault
    /// 
    /// Accounts expected by this instruction (6)
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[]` dao_authority_ai
    /// 2: `[writable]` from_vault_ai
    /// 3: `[writable]` to_account_ai
    /// 4: `[]` transfer_authority_ai
    /// 5: `[]` token_program_ai
    TransferFunds {
        lamports: u64,
    },

    /// Creates a price history account
    /// 
    /// Accounts expected by this instruction (2)
    /// 
    /// 0: `[]` creator_authority_ai
    /// 1: `[writable]` price_history_account_ai
    CreatePriceHistory {},

    /// Updates a price history account. A price history account will be updated every
    /// hour and the prices will be averaged for that day.
    /// 
    /// Accounts expected by this instruction (9)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` price_history_ai
    /// 2: `[]` sol_usdc_oracle_ai
    /// 3: `[]` sol_usdt_oracle_ai
    /// 4: `[]` lucra_sol_oracle_ai
    /// 5: `[writable]` user_reward_account_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` token_program_ai
    UpdatePriceHistory {},

    /// Redeems reward tokens for Lucra
    /// 
    /// Accounts expected by this instruction (9)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` user_reward_account_ai
    /// 2: `[writable]` user_lucra_account_ai
    /// 3: `[]` user_authority_ai
    /// 4: `[writable]` reward_mint_ai
    /// 5: `[writable]` lucra_mint_ai
    /// 6: `[]` lucra_mint_authority_ai
    /// 7: `[]` lucra_sol_oracle_ai
    /// 8: `[]` token_program_ai
    RedeemRewardTokens {
        reward_tokens: u64,
    },

    /// Adds collateral to a loan. This does not give back more mata
    /// 
    /// Accounts expected by this instruction (20 or 16)
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` loan_ai
    /// 3: `[writable]` msol_vault_ai
    /// 4: `[writable]` owner_ai
    /// 5: `[writable]` user_msol_account_ai
    /// 6: `[writable]` user_staking_account_ai
    /// 7: `[]` sol_usdc_oracle_ai
    /// 8: `[]` sol_usdt_oracle_ai
    /// 9: `[]` lucra_sol_oracle_ai
    /// 10: `[writable]` msol_mint_ai
    /// 11: `[writable]` liq_pool_sol_leg_pda_ai
    /// 12: `[writable]` liq_pool_msol_leg_ai
    /// 13: `[]` liq_pool_msol_leg_authority_ai
    /// 14: `[writable]` reserve_pda_ai
    /// 15: `[]` msol_mint_authority_ai
    /// 16: `[]` fees_ai
    /// 17: `[]` system_program_ai
    /// 18: `[]` token_program_ai
    /// 19: `[]` marinade_program_ai
    /// 
    /// or
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` loan_ai
    /// 3: `[writable]` msol_vault_ai
    /// 4: `[writable]` owner_ai
    /// 5: `[writable]` user_msol_account_ai
    /// 6: `[writable]` msol_mint_ai
    /// 7: `[writable]` liq_pool_sol_leg_pda_ai
    /// 8: `[writable]` liq_pool_msol_leg_ai
    /// 9: `[]` liq_pool_msol_leg_authority_ai
    /// 10: `[writable]` reserve_pda_ai
    /// 11: `[]` msol_mint_authority_ai
    /// 12: `[]` fees_ai
    /// 13: `[]` system_program_ai
    /// 14: `[]` token_program_ai
    /// 15: `[]` marinade_program_ai
    AddCollateral {
        lamports: u64,
    },

    /// Determines the penalty on an outstanding loan
    /// 
    /// Accounts expected by this instruction (10)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` loan_ai
    /// 2: `[]` sol_usdc_oracle_ai
    /// 3: `[]` sol_usdt_oracle_ai
    /// 4: `[]` sol_mata_oracle_ai
    /// 5: `[]` price_history_ai
    /// 6: `[writable]` user_reward_account_ai
    /// 7: `[writable]` reward_mint_ai
    /// 8: `[]` reward_mint_authority_ai
    /// 9: `[]` token_program_ai
    DeterminePenalty {},

    /// Harvests the penalty from a loan and rewards a fee to the user for performing the transaction
    /// 
    /// Accounts expected by this instruction (25 or 33)
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` msol_vault_ai
    /// 3: `[]` msol_vault_authority_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[writable]` loan_ai
    /// 6: `[]` sol_mata_oracle_ai
    /// 7: `[writable]` user_account_ai
    /// 8: `[writable]` user_wsol_account_ai
    /// 9: `[writable]` user_mata_account_ai
    /// 10: `[writable]` user_msol_account_ai
    /// 11: `[writable]` msol_mint_ai
    /// 12: `[writable]` liq_pool_sol_leg_pda_ai
    /// 13: `[writable]` liq_pool_msol_leg_ai
    /// 14: `[writable]` treasury_msol_account_ai
    /// 15: `[]` system_program_ai
    /// 16: `[]` marinade_program_ai
    /// 17: `[writable]` sm_amm_ai
    /// 18: `[]` sm_amm_authority_ai
    /// 19: `[writable]` sm_pool_base_vault_ai
    /// 20: `[writable]` sm_pool_quote_vault_ai
    /// 21: `[writable]` sm_pool_mint_ai
    /// 22: `[writable]` sm_pool_fees_ai
    /// 23: `[]` token_swap_program_ai
    /// 24: `[]` token_program_ai
    /// 
    /// or 
    /// 
    /// 0: `[writable]` system_state_ai
    /// 1: `[writable]` marinade_state_ai
    /// 2: `[writable]` msol_vault_ai
    /// 3: `[]` msol_vault_authority_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[writable]` loan_ai
    /// 6: `[]` sol_mata_oracle_ai
    /// 7: `[writable]` user_msol_account_ai
    /// 8: `[writable]` msol_mint_ai
    /// 9: `[writable]` liq_pool_sol_leg_pda_ai
    /// 10: `[writable]` liq_pool_msol_leg_ai
    /// 11: `[writable]` treasury_msol_account_ai
    /// 12: `[]` system_program_ai
    /// 13: `[]` marinade_program_ai
    /// 14: `[writable]` user_account_ai
    /// 15: `[writable]` user_wsol_account_ai
    /// 16: `[write]` user_mata_account_ai
    /// 17: `[]` pool_program_ai
    /// 18: `[writable]` _pool_wsol_account_ai
    /// 19: `[writable]` _pool_mata_account_ai
    /// 20: `[]` token_program_ai
    /// 21: `[writable]` amm_program_ai
    /// 22: `[]` _amm_authority_ai
    /// 23: `[writable]` _amm_open_orders_ai
    /// 24: `[writable]` _amm_target_ai
    /// 25: `[writable]` _serum_sol_mata_market_ai
    /// 26: `[]` serum_program_ai
    /// 27: `[writable]` _serum_bids_ai
    /// 28: `[writable]` _serum_asks_ai
    /// 29: `[writable]` _serum_event_queue_ai
    /// 30: `[writable]` _serum_base_vault_ai
    /// 31: `[writable]` _serum_quote_vault_ai
    /// 32: `[]` _serum_vault_signer_ai
    HarvestPenalty { amm_type: u8 },

    /// Sell the funds generated during the minting process for sol
    /// 
    /// Accounts expected by this instruction (32 or 31 or 24 or 23)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[writable]` mata_holding_vault_ai
    /// 4: `[]` mata_holding_vault_authority_ai
    /// 5: `[writable]` mata_mint_ai
    /// 6: `[]` wsol_mint_ai
    /// 7: `[writable]` reward_mint_ai
    /// 8: `[]` reward_mint_authority_ai
    /// 9: `[]` sol_usdc_oracle_ai
    /// 10: `[]` sol_usdt_oracle_ai
    /// 11: `[]` sol_mata_oracle_ai
    /// 12: `[writable]` user_reward_account_ai
    /// 13: `[writable]` user_account_ai
    /// 14: `[writable]` user_wsol_account_ai
    /// 15: `[writable]` user_mata_account_ai
    /// 16: `[]` pool_program_ai
    /// 17: `[writable]` pool_wsol_account_ai
    /// 18: `[writable]` pool_mata_account_ai
    /// 19: `[]` token_program_ai
    /// 20: `[writable]` amm_program_ai
    /// 21: `[]` _amm_authority_ai
    /// 22: `[writable]` amm_open_orders_ai
    /// 23: `[writable]` _amm_target_ai
    /// 24: `[writable]` serum_sol_mata_market_ai
    /// 25: `[]` serum_program_ai
    /// 26: `[writable]` _serum_bids_ai
    /// 27: `[writable]` _serum_asks_ai
    /// 28: `[writable]` _serum_event_queue_ai
    /// 29: `[writable]` _serum_base_vault_ai
    /// 30: `[writable]` _serum_quote_vault_ai
    /// 31: `[]` _serum_vault_signer_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[writable]` lucra_holding_vault_ai
    /// 4: `[]` lucra_holding_vault_authority_ai
    /// 5: `[writable]` reward_mint_ai
    /// 6: `[]` reward_mint_authority_ai
    /// 7: `[]` sol_usdc_oracle_ai
    /// 8: `[]` sol_usdt_oracle_ai
    /// 9: `[]` lucra_sol_oracle_ai
    /// 10: `[]` sol_mata_oracle_ai
    /// 11: `[writable]` user_reward_account_ai
    /// 12: `[writable]` user_account_ai
    /// 13: `[writable]` user_lucra_account_ai
    /// 14: `[writable]` user_wsol_account_ai
    /// 15: `[]` pool_program_ai
    /// 16: `[writable]` pool_lucra_account_ai
    /// 17: `[writable]` pool_wsol_account_ai
    /// 18: `[]` token_program_ai
    /// 19: `[writable]` amm_program_ai
    /// 20: `[]` _amm_authority_ai
    /// 21: `[writable]` amm_open_orders_ai
    /// 22: `[writable]` _amm_target_ai
    /// 23: `[writable]` serum_lucra_sol_market_ai
    /// 24: `[]` serum_program_ai
    /// 25: `[writable]` _serum_bids_ai
    /// 26: `[writable]` _serum_asks_ai
    /// 27: `[writable]` _serum_event_queue_ai
    /// 28: `[writable]` _serum_base_vault_ai
    /// 29: `[writable]` _serum_quote_vault_ai
    /// 30: `[]` _serum_vault_signer_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[writable]` mata_holding_vault_ai
    /// 4: `[]` mata_holding_vault_authority_ai
    /// 5: `[writable]` mata_mint_ai
    /// 6: `[]` wsol_mint_ai
    /// 7: `[writable]` reward_mint_ai
    /// 8: `[]` reward_mint_authority_ai
    /// 9: `[]` sol_usdc_oracle_ai
    /// 10: `[]` sol_usdt_oracle_ai
    /// 11: `[]` sol_mata_oracle_ai
    /// 12: `[writable]` user_reward_account_ai
    /// 13: `[]` user_account_ai
    /// 14: `[writable]` user_wsol_account_ai
    /// 15: `[writable]` user_mata_account_ai
    /// 16: `[writable]` sm_amm_ai
    /// 17: `[]` sm_amm_authority_ai
    /// 18: `[writable]` sm_pool_base_vault_ai
    /// 19: `[writable]` sm_pool_quote_vault_ai
    /// 20: `[writable]` sm_pool_mint_ai
    /// 21: `[writable]` sm_pool_fees_ai
    /// 22: `[]` token_swap_program_ai
    /// 23: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[writable]` lucra_holding_vault_ai
    /// 4: `[]` lucra_holding_vault_authority_ai
    /// 5: `[writable]` reward_mint_ai
    /// 6: `[]` reward_mint_authority_ai
    /// 7: `[]` sol_usdc_oracle_ai
    /// 8: `[]` sol_usdt_oracle_ai
    /// 9: `[]` lucra_sol_oracle_ai
    /// 10: `[]` sol_mata_oracle_ai
    /// 11: `[writable]` user_reward_account_ai
    /// 12: `[]` user_account_ai
    /// 13: `[writable]` user_lucra_account_ai
    /// 14: `[writable]` user_wsol_account_ai
    /// 15: `[writable]` ls_amm_ai
    /// 16: `[]` ls_amm_authority_ai
    /// 17: `[writable]` ls_pool_base_vault_ai
    /// 18: `[writable]` ls_pool_quote_vault_ai
    /// 19: `[writable]` ls_pool_mint_ai
    /// 20: `[writable]` ls_pool_fees_ai
    /// 21: `[]` token_swap_program_ai
    /// 22: `[]` token_program_ai
    SellFundsForArb {
        fund_source: u8,
        amm_type: u8,
        lamports: u64,
    },

    /// Buy and burn a token using the sol bought during selling
    /// 
    /// Accounts expected by this instruction (29 or 31 or 21 or 23)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[]` arb_fund_authority_ai
    /// 4: `[writable]` lucra_mint_ai
    /// 5: `[writable]` reward_mint_ai
    /// 6: `[]` reward_mint_authority_ai
    /// 7: `[]` lucra_sol_oracle_ai
    /// 8: `[writable]` wsol_holding_vault_ai
    /// 9: `[writable]` user_reward_account_ai
    /// 10: `[writable]` user_account_ai
    /// 11: `[writable]` user_lucra_account_ai
    /// 12: `[writable]` user_wsol_account_ai
    /// 13: `[]` pool_program_ai
    /// 14: `[writable]` _pool_lucra_account_ai
    /// 15: `[writable]` _pool_wsol_account_ai
    /// 16: `[]` token_program_ai
    /// 17: `[writable]` _amm_program_ai
    /// 18: `[]` _amm_authority_ai
    /// 19: `[writable]` _amm_open_orders_ai
    /// 20: `[writable]` _amm_target_ai
    /// 21: `[writable]` serum_lucra_sol_market_ai
    /// 22: `[]` serum_program_ai
    /// 23: `[writable]` _serum_bids_ai
    /// 24: `[writable]` _serum_asks_ai
    /// 25: `[writable]` _serum_event_queue_ai
    /// 26: `[writable]` _serum_base_vault_ai
    /// 27: `[writable]` _serum_quote_vault_ai
    /// 28: `[]` _serum_vault_signer_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[]` arb_fun_authority_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[]` wsol_mint_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` sol_usdc_oracle_ai
    /// 9: `[]` sol_usdt_oracle_ai
    /// 10: `[]` sol_mata_oracle_ai
    /// 11: `[writable]` user_reward_account_ai
    /// 12: `[writable]` user_account_ai
    /// 13: `[writable]` user_wsol_account_ai
    /// 14: `[writable]` user_mata_account_ai
    /// 15: `[]` pool_program_ai
    /// 16: `[writable]` pool_wsol_account_ai
    /// 17: `[writable]` pool_mata_account_ai
    /// 18: `[]` token_program_ai
    /// 19: `[writable]` amm_program_ai
    /// 20: `[]` _amm_authority_ai
    /// 21: `[writable]` amm_open_orders_ai
    /// 22: `[writable]` _amm_target_ai
    /// 23: `[writable]` serum_sol_mata_market_ai
    /// 24: `[]` serum_program_ai
    /// 25: `[writable]` _serum_bids_ai
    /// 26: `[writable]` _serum_asks_ai
    /// 27: `[writable]` _serum_event_queue_ai
    /// 28: `[writable]` _serum_base_vault_ai
    /// 29: `[writable]` _serum_quote_vault_ai
    /// 30: `[]` _serum_vault_signer_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[]` arb_fund_authority_ai
    /// 4: `[writable]` wsol_holding_vault_ai
    /// 5: `[writable]` lucra_mint_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` lucra_sol_oracle_ai
    /// 9: `[]` user_account_ai
    /// 10: `[writable]` user_lucra_account_ai
    /// 11: `[writable]` user_wsol_account_ai
    /// 12: `[writable]` user_reward_account_ai
    /// 13: `[writable]` ls_amm_ai
    /// 14: `[]` ls_amm_authority_ai
    /// 15: `[writable]` ls_pool_base_vault_ai
    /// 16: `[writable]` ls_pool_quote_vault_ai
    /// 17: `[writable]` ls_pool_mint_ai
    /// 18: `[writable]` ls_pool_fees_ai
    /// 19: `[]` token_swap_program_ai
    /// 20: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_fund_ai
    /// 3: `[]` arb_fund_authority_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[]` wsol_mint_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` sol_usdc_oracle_ai
    /// 9: `[]` sol_usdt_oracle_ai
    /// 10: `[]` sol_mata_oracle_ai
    /// 11: `[]` user_account_ai
    /// 12: `[writable]` user_wsol_account_ai
    /// 13: `[writable]` user_mata_account_ai 
    /// 14: `[writable]` user_reward_account_ai 
    /// 15: `[writable]` sm_amm_ai
    /// 16: `[]` sm_amm_authority_ai
    /// 17: `[writable]` sm_pool_base_vault_ai
    /// 18: `[writable]` sm_pool_quote_vault_ai
    /// 19: `[writable]` sm_pool_mint_ai
    /// 20: `[writable]` sm_pool_fees_ai
    /// 21: `[]` token_swap_program_ai
    /// 22: `[]` token_program_ai
    BuyBurnForArb {
        fund_source: u8,
        amm_type: u8,
        lamports: u64,
    },

    /// Stakes the wsol in the holding account and sends it to the arb coffer
    /// 
    /// Accounts expected by this instruction (23)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[]` arb_state_ai
    /// 2: `[writable]` arb_coffer_ai
    /// 3: `[writable]` wsol_holding_vault_ai
    /// 4: `[]` wsol_holding_vault_authority_ai
    /// 5: `[writable]` reward_mint_ai
    /// 6: `[]` reward_mint_authority_ai
    /// 7: `[writable]` user_account_ai
    /// 8: `[writable]` user_wsol_account_ai
    /// 9: `[writable]` user_msol_account_ai
    /// 10: `[writable]` user_reward_account_ai
    /// 11: `[writable]` temp_wsol_account_ai
    /// 12: `[writable]` marinade_state_ai
    /// 13: `[writable]` msol_mint_ai
    /// 14: `[writable]` liq_pool_sol_leg_pda_ai
    /// 15: `[writable]` liq_pool_msol_leg_ai
    /// 16: `[]` liq_pool_msol_leg_authority_ai
    /// 17: `[writable]` reserve_pda_ai
    /// 18: `[]` msol_mint_authority_ai
    /// 19: `[]` fees_ai
    /// 20: `[]` system_program_ai
    /// 21: `[]` marinade_program_ai
    /// 22: `[]` token_program_ai
    CleanUpArb {},

    /// Mints the funds to start the arbitrage process
    /// 
    /// Accounts expected by this instruction (19 or 26 or 18 or 26 or 30 or 26 or 27)
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[]` arb_fund_ai
    /// 3: `[writable]` mata_holding_vault_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[]` mata_mint_authority_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` sol_usdc_oracle_ai
    /// 9: `[]` sol_usdt_oracle_ai
    /// 10: `[]` lucra_sol_oracle_ai
    /// 11: `[]` sol_mata_oracle_ai
    /// 12: `[]` sm_base_vault_ai
    /// 13: `[]` sm_base_mint_ai
    /// 14: `[]` sm_quote_vault_ai
    /// 15: `[]` sm_amm_open_orders_ai
    /// 16: `[]` sm_amm_program_ai
    /// 17: `[writable]` user_reward_account_ai
    /// 18: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[]` arb_fund_ai
    /// 3: `[]` arb_coffer_ai
    /// 4: `[writable]` lucra_holding_vault_ai
    /// 5: `[writable]` lucra_mint_ai
    /// 6: `[]` lucra_mint_authority_ai
    /// 7: `[writable]` reward_mint_ai
    /// 8: `[]` reward_mint_authority_ai
    /// 9: `[]` sol_usdc_oracle_ai
    /// 10: `[]` sol_usdt_oracle_ai
    /// 11: `[]` lucra_sol_oracle_ai
    /// 12: `[]` sol_mata_oracle_ai
    /// 13: `[]` sm_raydium_base_vault_ai
    /// 14: `[]` sm_raydium_base_mint_ai
    /// 15: `[]` sm_raydium_quote_vault_ai
    /// 16: `[]` sm_raydium_quote_mint_ai
    /// 17: `[]` sm_raydium_amm_ai
    /// 18: `[]` sm_raydium_amm_open_orders_ai
    /// 19: `[]` sm_orca_base_vault_ai
    /// 20: `[]` sm_orca_quote_vault_ai
    /// 21: `[]` sm_orca_amm_ai
    /// 22: `[writable]` user_reward_account_ai
    /// 23: `[]` marinade_state_ai
    /// 24: `[]` marinade_program_ai
    /// 25: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[]` arb_fund_ai
    /// 3: `[writable]` mata_holding_vault_ai
    /// 4: `[writable]` mata_mint_ai
    /// 5: `[]` mata_mint_authority_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` sol_usdc_oracle_ai
    /// 9: `[]` sol_usdt_oracle_ai
    /// 10: `[]` lucra_sol_oracle_ai
    /// 11: `[]` sol_mata_oracle_ai
    /// 12: `[]` sm_amm_ai
    /// 13: `[]` sm_base_vault_ai
    /// 14: `[]` sm_base_mint_ai
    /// 15: `[]` sm_quote_vault_ai
    /// 16: `[writable]` user_reward_account_ai
    /// 17: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[]` arb_fund_ai
    /// 3: `[]` arb_coffer_ai
    /// 4: `[writable]` lucra_holding_vault_ai
    /// 5: `[writable]` lucra_mint_ai
    /// 6: `[]` lucra_mint_authority_ai
    /// 7: `[writable]` reward_mint_ai
    /// 8: `[]` reward_mint_authority_ai
    /// 9: `[]` sol_usdc_oracle_ai
    /// 10: `[]` sol_usdt_oracle_ai
    /// 11: `[]` lucra_sol_oracle_ai
    /// 12: `[]` sol_mata_oracle_ai
    /// 13: `[]` sm_orca_amm_ai
    /// 14: `[]` sm_orca_base_vault_ai
    /// 15: `[]` sm_orca_base_mint_ai
    /// 16: `[]` sm_orca_quote_vault_ai
    /// 17: `[]` sm_orca_quote_mint_ai
    /// 18: `[]` sm_raydium_open_orders_ai
    /// 19: `[]` sm_raydium_base_vault_ai
    /// 20: `[]` sm_raydium_quote_vault_ai
    /// 21: `[]` sm_raydium_amm_ai
    /// 22: `[writable]` user_reward_account_ai
    /// 23: `[]` marinade_state_ai
    /// 24: `[]` marinade_program_ai
    /// 25: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_coffer_ai
    /// 3: `[]` arb_coffer_authority_ai
    /// 4: `[writable]` arb_fund_ai
    /// 5: `[]` mata_mint_ai
    /// 6: `[]` wsol_mint_ai
    /// 7: `[writable]` reward_mint_ai
    /// 8: `[]` reward_mint_authority_ai
    /// 9: `[]` sol_usdc_oracle_ai
    /// 10: `[]` sol_usdt_oracle_ai
    /// 11: `[]` sol_mata_oracle_ai
    /// 12: `[]` sm_raydium_base_vault_ai
    /// 13: `[]` sm_raydium_quote_vault_ai
    /// 14: `[]` sm_raydium_amm_open_orders_ai
    /// 15: `[]` sm_raydium_amm_ai
    /// 16: `[]` sm_orca_base_vault_ai
    /// 17: `[]` sm_orca_quote_vault_ai
    /// 18: `[]` sm_orca_amm_ai
    /// 19: `[writable]` user_account_ai
    /// 20: `[writable]` user_msol_account_ai
    /// 21: `[writable]` user_reward_account_ai
    /// 22: `[writable]` msol_mint_ai
    /// 23: `[writable]` liq_pool_sol_leg_pda_ai
    /// 24: `[writable]` liq_pool_msol_leg_ai
    /// 25: `[writable]` treasury_msol_account_ai
    /// 26: `[writable]` marinade_state_ai
    /// 27: `[]` marinade_program_ai
    /// 28: `[]` system_program_ai
    /// 29: `[]` token_program_ai
    /// 
    /// or
    /// 
    /// 0: `[]` system_state_ai,
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_coffer_ai
    /// 3: `[]` arb_coffer_authority_ai
    /// 4: `[writable]` arb_fund_ai
    /// 5: `[]` mata_mint_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` sol_usdc_oracle_ai
    /// 9: `[]` sol_usdt_oracle_ai
    /// 10: `[]` sol_mata_oracle_ai
    /// 11: `[]` sm_base_vault_ai
    /// 12: `[]` sm_base_mint_ai
    /// 13: `[]` sm_quote_vault_ai
    /// 14: `[]` sm_amm_ai
    /// 15: `[writable]` user_account_ai
    /// 16: `[writable]` user_msol_account_ai
    /// 17: `[writable]` user_reward_account_ai
    /// 18: `[writable]` msol_mint_ai
    /// 19: `[writable]` liq_pool_sol_leg_pda_ai
    /// 20: `[writable]` liq_pool_msol_leg_ai
    /// 21: `[writable]` treasury_msol_account_ai
    /// 22: `[writable]` marinade_state_ai
    /// 23: `[]` marinade_program_ai
    /// 24: `[]` system_program_ai
    /// 25: `[]` token_program_ai
    /// 
    /// or 
    /// 
    /// 0: `[]` system_state_ai
    /// 1: `[writable]` arb_state_ai
    /// 2: `[writable]` arb_coffer_ai
    /// 3: `[]` arb_coffer_authority_ai
    /// 4: `[writable]` arb_fund_ai
    /// 5: `[]` mata_mint_ai
    /// 6: `[writable]` reward_mint_ai
    /// 7: `[]` reward_mint_authority_ai
    /// 8: `[]` sol_usdc_oracle_ai
    /// 9: `[]` sol_usdt_oracle_ai
    /// 10: `[]` sol_mata_oracle_ai
    /// 11: `[]` sm_base_vault_ai
    /// 12: `[]` sm_base_mint_ai
    /// 13: `[]` sm_quote_vault_ai
    /// 14: `[]` sm_open_orders_ai
    /// 15: `[]` sm_amm_ai
    /// 16: `[writable]` user_account_ai
    /// 17: `[writable]` user_msol_account_ai
    /// 18: `[writable]` user_reward_account_ai
    /// 19: `[writable]` msol_mint_ai
    /// 20: `[writable]` liq_pool_sol_leg_pda_ai
    /// 21: `[writable]` liq_pool_msol_leg_ai
    /// 22: `[writable]` treasury_msol_account_ai
    /// 23: `[writable]` marinade_state_ai
    /// 24: `[]` marinade_program_ai
    /// 25: `[]` system_program_ai
    /// 26: `[]` token_program_ai
    MintFundsForArb {
        fund_source: u8,
        amm_type: u8,
        lamports: u64,
    },
}

#[allow(clippy::too_many_arguments)]
pub fn initialize(
    marinade_state: &Pubkey,
    state: &Pubkey,
    staking_state: &Pubkey,
    arb_state: &Pubkey,
    mata_mint: &Pubkey,
    lucra_mint: &Pubkey,
    reward_mint: &Pubkey,
    staked_lucra_mint: &Pubkey,
    msol_vault: &Pubkey,
    arb_coffer: &Pubkey,
    rewards_vault: &Pubkey,
    arb_fund: &Pubkey,
    wsol_holding_vault: &Pubkey,
    mata_holding_vault: &Pubkey,
    lucra_holding_vault: &Pubkey,
    min_deposit: u64,
    collateral_requirement: u32,
    epoch: i64,
    loans_enabled: bool,
    staking_enabled: bool,
    arbitrage_enabled: bool,
    peg_check_enabled: bool,
    max_amount_of_lucra_to_mint: u64,
    daily_arb_limit: u64,
    maximum_outstanding_mata: u64,
    lcp: u8,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*marinade_state, false),
        AccountMeta::new_readonly(CREATOR_AUTHORITY, true),
        AccountMeta::new_readonly(*mata_mint, false),
        AccountMeta::new_readonly(*lucra_mint, false),
        AccountMeta::new_readonly(*reward_mint, false),
        AccountMeta::new_readonly(*staked_lucra_mint, false),
        AccountMeta::new(*state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new_readonly(*msol_vault, false),
        AccountMeta::new_readonly(*arb_coffer, false),
        AccountMeta::new_readonly(*rewards_vault, false),
        AccountMeta::new(*staking_state, false),
        AccountMeta::new_readonly(*arb_fund, false),
        AccountMeta::new_readonly(*wsol_holding_vault, false),
        AccountMeta::new_readonly(*mata_holding_vault, false),
        AccountMeta::new_readonly(*lucra_holding_vault, false),
    ];
    let data = Instruction::Initialize { 
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

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_state(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    min_deposit: u64,
    collateral_requirement: u32,
    loans_enabled: bool,
    staking_enabled: bool,
    arbitrage_enabled: bool,
    peg_check_enabled: bool,
    max_amount_of_lucra_to_mint: u64,
    daily_arb_limit: u64,
    maximum_outstanding_mata: u64,
    minimum_harvest_amount: u64,
    reward_fee: u32,
    lcp: u8,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new_readonly(DAO_AUTHORITY, true),
    ];
    let data = Instruction::UpdateState { 
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

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_mata_loan(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    loan: &Pubkey,
    msol_vault: &Pubkey,
    mata_mint: &Pubkey,
    mata_mint_authority: &Pubkey,
    transfer_from: &Pubkey,
    user_mata_account: &Pubkey,
    user_msol_account: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_address: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    liq_pool_msol_leg_authority: &Pubkey,
    reserve_address: &Pubkey,
    msol_mint_authority: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),

        AccountMeta::new(*loan, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new_readonly(*mata_mint_authority, false),
        AccountMeta::new(*transfer_from, true),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new(*user_msol_account, false),

        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
    
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_address, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new_readonly(*liq_pool_msol_leg_authority, false),
        AccountMeta::new(*reserve_address, false),
        AccountMeta::new_readonly(*msol_mint_authority, false),
    
        AccountMeta::new_readonly(solana_program::sysvar::fees::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
    ];
    let data = Instruction::CreateMataLoan { lamports };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_mata_loan_with_locked_stake(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    loan: &Pubkey,
    msol_vault: &Pubkey,
    mata_mint: &Pubkey,
    mata_mint_authority: &Pubkey,
    transfer_from: &Pubkey,
    user_mata_account: &Pubkey,
    user_msol_account: &Pubkey,
    staking_account: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_address: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    liq_pool_msol_leg_authority: &Pubkey,
    reserve_address: &Pubkey,
    msol_mint_authority: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),

        AccountMeta::new(*loan, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new_readonly(*mata_mint_authority, false),
        AccountMeta::new(*transfer_from, true),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*staking_account, false),

        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
    
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_address, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new_readonly(*liq_pool_msol_leg_authority, false),
        AccountMeta::new(*reserve_address, false),
        AccountMeta::new_readonly(*msol_mint_authority, false),
    
        AccountMeta::new_readonly(solana_program::sysvar::fees::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
    ];
    let data = Instruction::CreateMataLoan { lamports };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn close_mata_loan(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    loan: &Pubkey,
    user_account: &Pubkey,
    user_msol_account: &Pubkey,
    mata_mint: &Pubkey,
    user_mata_account: &Pubkey,
    msol_vault: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_address: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    unstake_msol: bool,
) -> SolInstruction {
    let msol_vault_authority = SystemState::find_msol_vault_authority(system_state).0;
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new(*loan, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new_readonly(msol_vault_authority, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_address, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
    ];
    let data = Instruction::CloseOutMataLoan {
        unstake_msol,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn close_mata_loan_with_locked_stake(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    loan: &Pubkey,
    user_account: &Pubkey,
    user_msol_account: &Pubkey,
    mata_mint: &Pubkey,
    user_mata_account: &Pubkey,
    user_staking_account: &Pubkey,
    msol_vault: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_address: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    unstake_msol: bool,
) -> SolInstruction {
    let msol_vault_authority = SystemState::find_msol_vault_authority(system_state).0;
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new(*loan, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new_readonly(msol_vault_authority, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new(*user_staking_account, false),
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_address, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
    ];
    let data = Instruction::CloseOutMataLoan {
        unstake_msol
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_staking_account(
    system_state: &Pubkey,
    staking_state: &Pubkey,
    staking_account: &Pubkey,
    owner: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new_readonly(*staking_state, false),
        AccountMeta::new(*staking_account, false),
        AccountMeta::new_readonly(*owner, true),
    ];
    let data = Instruction::CreateStakingAccount { };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_stake_balance(
    system_state: &Pubkey,
    staking_state: &Pubkey,
    stake_balance: &Pubkey,
    owner: &Pubkey,
    deposit_vault: &Pubkey,
    stake_vault: &Pubkey,
    pending_vault: &Pubkey,
    nonce: u8,
    staking_timeframe: StakingTimeframe,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new_readonly(*staking_state, false),
        AccountMeta::new(*stake_balance, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(*deposit_vault, false),
        AccountMeta::new_readonly(*stake_vault, false),
        AccountMeta::new_readonly(*pending_vault, false),
    ];
    let data = Instruction::CreateStakeBalance { nonce, staking_timeframe: staking_timeframe as u8 };
    
    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_stake(
    system_state: &Pubkey,
    stake_balance: &Pubkey,
    from_account: &Pubkey,
    to_account: &Pubkey,
    owner: &Pubkey,
    lucra: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new_readonly(*stake_balance, false),
        AccountMeta::new(*from_account, false),
        AccountMeta::new(*to_account, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::DepositStake { lucra };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn stake(
    system_state: &Pubkey,
    staking_state: &Pubkey,
    staking_account: &Pubkey,
    stake_balance: &Pubkey,
    deposit_vault: &Pubkey,
    stake_vault: &Pubkey,
    owner: &Pubkey,
    staked_lucra_account: &Pubkey,
    transfer_authority: &Pubkey,
    staked_lucra_mint: &Pubkey,
    lucra: u64,
) -> SolInstruction {
    let staked_lucra_mint_authority = StakingState::find_stake_mint_authority(staking_state).0;
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*staking_state, false),
        AccountMeta::new(*staking_account, false),
        AccountMeta::new(*stake_balance, false),
        AccountMeta::new(*deposit_vault, false),
        AccountMeta::new(*stake_vault, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(*transfer_authority, false),
        AccountMeta::new(*staked_lucra_mint, false),
        AccountMeta::new(*staked_lucra_account, false),
        AccountMeta::new_readonly(staked_lucra_mint_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::Stake { lucra };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn start_unstake(
    system_state: &Pubkey,
    staking_state: &Pubkey,
    staking_account: &Pubkey,
    stake_balance: &Pubkey,
    stake_vault: &Pubkey,
    pending_vault: &Pubkey,
    pending_withdrawal: &Pubkey,
    owner: &Pubkey,
    staked_lucra_account: &Pubkey,
    transfer_authority: &Pubkey,
    staked_lucra_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    lucra: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new_readonly(*staking_state, false),
        AccountMeta::new(*staking_account, false),
        AccountMeta::new(*stake_balance, false),
        AccountMeta::new(*staked_lucra_mint, false),
        AccountMeta::new(*staked_lucra_account, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new(*stake_vault, false),
        AccountMeta::new(*pending_vault, false),
        AccountMeta::new_readonly(*transfer_authority, false),
        AccountMeta::new(*pending_withdrawal, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::StartUnstake { lucra };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn end_unstake(
    system_state: &Pubkey,
    pending_withdrawal: &Pubkey,
    stake_balance: &Pubkey,
    pending_vault: &Pubkey,
    deposit_vault: &Pubkey,
    owner: &Pubkey,
    transfer_authority: &Pubkey,
    user_sol_account: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*pending_withdrawal, false),
        AccountMeta::new_readonly(*stake_balance, false),
        AccountMeta::new(*pending_vault, false),
        AccountMeta::new(*deposit_vault, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(*transfer_authority, false),
        AccountMeta::new(*user_sol_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::EndUnstake { };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_stake(
    system_state: &Pubkey,
    stake_balance: &Pubkey,
    to_account: &Pubkey,
    deposit_vault: &Pubkey,
    stake_vault: &Pubkey,
    pending_vault: &Pubkey,
    owner: &Pubkey,
    transfer_authority: &Pubkey,
    lucra: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*stake_balance, false),
        AccountMeta::new(*deposit_vault, false),
        AccountMeta::new_readonly(*stake_vault, false),
        AccountMeta::new_readonly(*pending_vault, false),
        AccountMeta::new(*to_account, false),
        AccountMeta::new(*owner, true),
        AccountMeta::new_readonly(*transfer_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::WithdrawStake { lucra };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn claim_reward(
    system_state: &Pubkey,
    staking_state: &Pubkey,
    stake_balance: &Pubkey,
    reward: &Pubkey,
    staked_lucra_account: &Pubkey,
    lucra_vault: &Pubkey,
    lucra_account: &Pubkey,
    rewards_vault: &Pubkey,
    msol_account: &Pubkey,
    lucra_mint: &Pubkey,
) -> SolInstruction {
    let rewards_vault_authority = SystemState::find_rewards_vault_authority(system_state).0;
    let lucra_mint_authority = SystemState::find_lucra_mint_authority(system_state).0;
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new_readonly(*staking_state, false),
        AccountMeta::new(*stake_balance, false),
        AccountMeta::new_readonly(*reward, false),
        AccountMeta::new(*staked_lucra_account, false),
        AccountMeta::new(*lucra_vault, false),
        AccountMeta::new(*lucra_account, false),
        AccountMeta::new(*rewards_vault, false),
        AccountMeta::new(*msol_account, false),
        AccountMeta::new_readonly(rewards_vault_authority, false),
        AccountMeta::new(*lucra_mint, false),
        AccountMeta::new_readonly(lucra_mint_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::ClaimReward { };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn drop_reward(
    system_state: &Pubkey,
    staking_state: &Pubkey,
    marinade_state: &Pubkey,
    reward: &Pubkey,
    staked_lucra_mint: &Pubkey,
    msol_vault: &Pubkey,
    rewards_vault: &Pubkey,
    arb_coffer: &Pubkey,
    user_reward_account: &Pubkey,
    reward_mint: &Pubkey,
    reward_mint_authority: &Pubkey,
) -> SolInstruction {
    let msol_vault_transfer_authority = SystemState::find_msol_vault_authority(system_state).0;
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*staking_state, false),
        AccountMeta::new_readonly(*marinade_state, false),
        AccountMeta::new(*reward, false),
        AccountMeta::new_readonly(*staked_lucra_mint, false),
        AccountMeta::new_readonly(*msol_vault, false),
        AccountMeta::new(*rewards_vault, false),
        AccountMeta::new(*arb_coffer, false),
        AccountMeta::new_readonly(msol_vault_transfer_authority, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(*reward_mint_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::DropReward { };
    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_funds(
    system_state: &Pubkey,
    from_account: &Pubkey,
    to_account: &Pubkey,
    transfer_authority: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new_readonly(DAO_AUTHORITY, true),
        AccountMeta::new(*from_account, false),
        AccountMeta::new(*to_account, false),
        AccountMeta::new_readonly(*transfer_authority, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::TransferFunds {
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_price_history(
    price_history: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(CREATOR_AUTHORITY, true),
        AccountMeta::new(*price_history, false),
    ];
    let data = Instruction::CreatePriceHistory {};

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_price_history(
    system_state: &Pubkey,
    price_history: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    user_reward_account: &Pubkey,
    reward_mint: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*price_history, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::UpdatePriceHistory {};

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn redeem_reward_tokens(
    system_state: &Pubkey,
    user_reward_account: &Pubkey,
    user_lucra_reward_account: &Pubkey,
    user_authority: &Pubkey,
    reward_mint: &Pubkey,
    lucra_mint: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    reward_tokens: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*user_lucra_reward_account, false),
        AccountMeta::new_readonly(*user_authority, true),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new(*lucra_mint, false),
        AccountMeta::new_readonly(SystemState::find_lucra_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::RedeemRewardTokens { reward_tokens };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn add_collateral(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    loan: &Pubkey,
    msol_vault: &Pubkey,
    owner: &Pubkey,
    user_msol_account: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_address: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    liq_pool_msol_leg_authority: &Pubkey,
    reserve_address: &Pubkey,
    msol_mint_authority: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),

        AccountMeta::new(*loan, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new(*owner, true),
        AccountMeta::new(*user_msol_account, false),
    
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_address, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new_readonly(*liq_pool_msol_leg_authority, false),
        AccountMeta::new(*reserve_address, false),
        AccountMeta::new_readonly(*msol_mint_authority, false),
    
        AccountMeta::new_readonly(solana_program::sysvar::fees::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
    ];
    let data = Instruction::AddCollateral { lamports };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn add_collateral_with_locked_stake(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    loan: &Pubkey,
    msol_vault: &Pubkey,
    owner: &Pubkey,
    user_msol_account: &Pubkey,
    staking_account: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_address: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    liq_pool_msol_leg_authority: &Pubkey,
    reserve_address: &Pubkey,
    msol_mint_authority: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),

        AccountMeta::new(*loan, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new(*owner, true),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*staking_account, false),

        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
    
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_address, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new_readonly(*liq_pool_msol_leg_authority, false),
        AccountMeta::new(*reserve_address, false),
        AccountMeta::new_readonly(*msol_mint_authority, false),
    
        AccountMeta::new_readonly(solana_program::sysvar::fees::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
    ];
    let data = Instruction::AddCollateral { lamports };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn determine_penalty(
    system_state: &Pubkey,
    loan: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    price_history: &Pubkey,
    user_reward_account: &Pubkey,
    reward_mint: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*loan, false),

        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*price_history, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::DeterminePenalty { };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn harvest_penalty_with_orca(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    msol_vault: &Pubkey,
    mata_mint: &Pubkey,
    loan: &Pubkey,
    user_account: &Pubkey,
    user_wsol_account: &Pubkey,
    user_mata_account: &Pubkey,
    user_msol_account: &Pubkey,
    sol_mata_oracle: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_pda: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    sm_amm: &Pubkey,
    sm_amm_authority: &Pubkey,
    sm_pool_base_vault: &Pubkey,
    sm_pool_quote_vault: &Pubkey,
    sm_pool_mint: &Pubkey,
    sm_pool_fees: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new_readonly(SystemState::find_msol_vault_authority(system_state).0, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*loan, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),

        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new(*user_msol_account, false),
        
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_pda, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        
        AccountMeta::new(*sm_amm, false),
        AccountMeta::new_readonly(*sm_amm_authority, false),
        AccountMeta::new(*sm_pool_base_vault, false),
        AccountMeta::new(*sm_pool_quote_vault, false),
        AccountMeta::new(*sm_pool_mint, false),
        AccountMeta::new(*sm_pool_fees, false),
        
        AccountMeta::new_readonly(orca_swap::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    let data = Instruction::HarvestPenalty { amm_type: AmmTypes::Orca as u8 };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn harvest_penalty_with_raydium(
    system_state: &Pubkey,
    marinade_state: &Pubkey,
    msol_vault: &Pubkey,
    mata_mint: &Pubkey,
    loan: &Pubkey,
    sol_mata_oracle: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_pda: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    user_msol_account: &Pubkey,
    user_account: &Pubkey,
    user_lucra_account: &Pubkey,
    user_wsol_account: &Pubkey,
    pool_lucra_account: &Pubkey,
    pool_wsol_account: &Pubkey,
    amm_program: &Pubkey,
    amm_authority: &Pubkey,
    amm_open_orders: &Pubkey,
    amm_target: &Pubkey,
    serum_lucra_sol_market: &Pubkey,
    serum_bids: &Pubkey,
    serum_asks: &Pubkey,
    serum_event_queue: &Pubkey,
    serum_base_vault: &Pubkey,
    serum_quote_vault: &Pubkey,
    serum_vault_signer: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new(*system_state, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new(*msol_vault, false),
        AccountMeta::new_readonly(SystemState::find_msol_vault_authority(system_state).0, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*loan, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),

        AccountMeta::new(*user_msol_account, false),

        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_pda, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),

        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_lucra_account, false),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new_readonly(raydium_v4::id(), false),
        AccountMeta::new(*pool_lucra_account, false),
        AccountMeta::new(*pool_wsol_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new(*amm_program, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_open_orders, false),
        AccountMeta::new(*amm_target, false),
        AccountMeta::new(*serum_lucra_sol_market, false),
        AccountMeta::new_readonly(serum_v3::id(), false),
        AccountMeta::new(*serum_bids, false),
        AccountMeta::new(*serum_asks, false),
        AccountMeta::new(*serum_event_queue, false),
        AccountMeta::new(*serum_base_vault, false),
        AccountMeta::new(*serum_quote_vault, false),
        AccountMeta::new_readonly(*serum_vault_signer, false),
    ];

    let data = Instruction::HarvestPenalty { amm_type: AmmTypes::Raydium as u8 };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sell_lucra_for_arb_funds_using_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    lucra_holding_vault: &Pubkey,
    reward_mint: &Pubkey,

    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,

    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_lucra_account: &Pubkey,
    user_wsol_account: &Pubkey,
    pool_lucra_account: &Pubkey,
    pool_wsol_account: &Pubkey,
    amm_program: &Pubkey,
    amm_authority: &Pubkey,
    amm_open_orders: &Pubkey,
    amm_target: &Pubkey,
    serum_lucra_sol_market: &Pubkey,
    serum_bids: &Pubkey,
    serum_asks: &Pubkey,
    serum_event_queue: &Pubkey,
    serum_base_vault: &Pubkey,
    serum_quote_vault: &Pubkey,
    serum_vault_signer: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*lucra_holding_vault, false),
        AccountMeta::new_readonly(ArbState::find_lucra_holding_vault_authority(arb_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_lucra_account, false),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new_readonly(raydium_v4::id(), false),
        AccountMeta::new(*pool_lucra_account, false),
        AccountMeta::new(*pool_wsol_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new(*amm_program, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_open_orders, false),
        AccountMeta::new(*amm_target, false),
        AccountMeta::new(*serum_lucra_sol_market, false),
        AccountMeta::new_readonly(serum_v3::id(), false),
        AccountMeta::new(*serum_bids, false),
        AccountMeta::new(*serum_asks, false),
        AccountMeta::new(*serum_event_queue, false),
        AccountMeta::new(*serum_base_vault, false),
        AccountMeta::new(*serum_quote_vault, false),
        AccountMeta::new_readonly(*serum_vault_signer, false),
    ];
    let data = Instruction::SellFundsForArb { 
        fund_source: CurrencyTypes::Lucra as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports: 0,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sell_mata_for_arb_funds_using_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    mata_holding_vault: &Pubkey,
    mata_mint: &Pubkey,
    wsol_mint: &Pubkey,
    reward_mint: &Pubkey,

    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,

    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_wsol_account: &Pubkey,
    user_mata_account: &Pubkey,
    pool_wsol_account: &Pubkey,
    pool_mata_account: &Pubkey,
    amm_program: &Pubkey,
    amm_authority: &Pubkey,
    amm_open_orders: &Pubkey,
    amm_target: &Pubkey,
    serum_sol_mata_market: &Pubkey,
    serum_bids: &Pubkey,
    serum_asks: &Pubkey,
    serum_event_queue: &Pubkey,
    serum_base_vault: &Pubkey,
    serum_quote_vault: &Pubkey,
    serum_vault_signer: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*mata_holding_vault, false),
        AccountMeta::new_readonly(ArbState::find_mata_holding_vault_authority(arb_state).0, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*wsol_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new_readonly(raydium_v4::id(), false),
        AccountMeta::new(*pool_wsol_account, false),
        AccountMeta::new(*pool_mata_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new(*amm_program, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_open_orders, false),
        AccountMeta::new(*amm_target, false),
        AccountMeta::new(*serum_sol_mata_market, false),
        AccountMeta::new_readonly(serum_v3::id(), false),
        AccountMeta::new(*serum_bids, false),
        AccountMeta::new(*serum_asks, false),
        AccountMeta::new(*serum_event_queue, false),
        AccountMeta::new(*serum_base_vault, false),
        AccountMeta::new(*serum_quote_vault, false),
        AccountMeta::new_readonly(*serum_vault_signer, false),
    ];
    let data = Instruction::SellFundsForArb { 
        fund_source: CurrencyTypes::Mata as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports: 0,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sell_lucra_for_arb_funds_using_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    lucra_holding_vault: &Pubkey,
    reward_mint: &Pubkey,

    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,

    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_lucra_account: &Pubkey,
    user_wsol_account: &Pubkey,
    ls_amm: &Pubkey,
    ls_amm_authority: &Pubkey,
    ls_pool_base_vault: &Pubkey,
    ls_pool_quote_vault: &Pubkey,
    ls_pool_mint: &Pubkey,
    ls_pool_fees: &Pubkey,
    sm_amm: &Pubkey,
    sm_pool_base_vault: &Pubkey,
    sm_pool_quote_vault: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*lucra_holding_vault, false),
        AccountMeta::new_readonly(ArbState::find_lucra_holding_vault_authority(arb_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(*user_account, true),
        AccountMeta::new(*user_lucra_account, false),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new_readonly(*ls_amm, false),
        AccountMeta::new_readonly(*ls_amm_authority, false),
        AccountMeta::new(*ls_pool_base_vault, false),
        AccountMeta::new(*ls_pool_quote_vault, false),
        AccountMeta::new(*ls_pool_mint, false),
        AccountMeta::new(*ls_pool_fees, false),
        AccountMeta::new_readonly(*sm_amm, false),
        AccountMeta::new_readonly(*sm_pool_base_vault, false),
        AccountMeta::new_readonly(*sm_pool_quote_vault, false),
        AccountMeta::new_readonly(orca_swap::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::SellFundsForArb { 
        fund_source: CurrencyTypes::Lucra as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sell_mata_for_arb_funds_using_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    mata_holding_vault: &Pubkey,
    mata_mint: &Pubkey,
    wsol_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_wsol_account: &Pubkey,
    user_mata_account: &Pubkey,
    sm_amm: &Pubkey,
    sm_amm_authority: &Pubkey,
    sm_pool_base_vault: &Pubkey,
    sm_pool_quote_vault: &Pubkey,
    sm_pool_mint: &Pubkey,
    sm_pool_fees: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*mata_holding_vault, false),
        AccountMeta::new_readonly(ArbState::find_mata_holding_vault_authority(arb_state).0, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*wsol_mint, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(*user_account, true),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new_readonly(*sm_amm, false),
        AccountMeta::new_readonly(*sm_amm_authority, false),
        AccountMeta::new(*sm_pool_base_vault, false),
        AccountMeta::new(*sm_pool_quote_vault, false),
        AccountMeta::new(*sm_pool_mint, false),
        AccountMeta::new(*sm_pool_fees, false),
        AccountMeta::new_readonly(orca_swap::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::SellFundsForArb { 
        fund_source: CurrencyTypes::Mata as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mint_mata_for_arb_funds_checking_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    mata_holding_vault: &Pubkey,
    mata_mint: &Pubkey,
    reward_mint: &Pubkey,

    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,

    sm_base_vault: &Pubkey,
    sm_base_mint: &Pubkey,
    sm_quote_vault: &Pubkey,
    sm_amm_open_orders: &Pubkey,
    sm_amm_program: &Pubkey,

    user_reward_account: &Pubkey,

    mata: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new_readonly(*arb_fund, false),
        AccountMeta::new(*mata_holding_vault, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new_readonly(SystemState::find_mata_mint_authority(system_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_base_vault, false),
        AccountMeta::new_readonly(*sm_base_mint, false),
        AccountMeta::new_readonly(*sm_quote_vault, false),
        AccountMeta::new_readonly(*sm_amm_open_orders, false),
        AccountMeta::new_readonly(*sm_amm_program, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Mata as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports: mata,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mint_lucra_for_arb_funds_checking_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    arb_coffer: &Pubkey,
    lucra_holding_vault: &Pubkey,
    lucra_mint: &Pubkey,
    reward_mint: &Pubkey,

    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    
    sm_raydium_base_vault: &Pubkey,
    sm_raydium_base_mint: &Pubkey,
    sm_raydium_quote_vault: &Pubkey,
    sm_raydium_quote_mint: &Pubkey,
    sm_raydium_amm_open_orders: &Pubkey,
    sm_raydium_amm: &Pubkey,

    sm_orca_base_vault: &Pubkey,
    sm_orca_quote_vault: &Pubkey,
    sm_orca_amm: &Pubkey,

    user_reward_account: &Pubkey,

    marinade_state: &Pubkey,

    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new_readonly(*arb_fund, false),
        AccountMeta::new_readonly(*arb_coffer, false),
        AccountMeta::new(*lucra_holding_vault, false),
        AccountMeta::new(*lucra_mint, false),
        AccountMeta::new_readonly(SystemState::find_lucra_mint_authority(system_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_raydium_base_vault, false),
        AccountMeta::new_readonly(*sm_raydium_base_mint, false),
        AccountMeta::new_readonly(*sm_raydium_quote_vault, false),
        AccountMeta::new_readonly(*sm_raydium_quote_mint, false),
        AccountMeta::new_readonly(*sm_raydium_amm_open_orders, false),
        AccountMeta::new_readonly(*sm_raydium_amm, false),
        AccountMeta::new_readonly(*sm_orca_base_vault, false),
        AccountMeta::new_readonly(*sm_orca_quote_vault, false),
        AccountMeta::new_readonly(*sm_orca_amm, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(*marinade_state, false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Lucra as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mint_mata_for_arb_funds_checking_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    mata_holding_vault: &Pubkey,
    mata_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    sm_amm: &Pubkey,
    sm_base_vault: &Pubkey,
    sm_base_mint: &Pubkey,
    sm_quote_vault: &Pubkey,
    user_reward_account: &Pubkey,
    mata: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new_readonly(*arb_fund, false),
        AccountMeta::new(*mata_holding_vault, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new_readonly(SystemState::find_mata_mint_authority(system_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_amm, false),
        AccountMeta::new_readonly(*sm_base_vault, false),
        AccountMeta::new_readonly(*sm_base_mint, false),
        AccountMeta::new_readonly(*sm_quote_vault, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Mata as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports: mata,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mint_lucra_for_arb_funds_checking_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    arb_coffer: &Pubkey,
    lucra_holding_vault: &Pubkey,
    lucra_mint: &Pubkey,
    reward_mint: &Pubkey,

    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    
    sm_orca_amm: &Pubkey,
    sm_orca_base_vault: &Pubkey,
    sm_orca_base_mint: &Pubkey,
    sm_orca_quote_vault: &Pubkey,
    sm_orca_quote_mint: &Pubkey,
    
    sm_raydium_open_orders: &Pubkey,
    sm_raydium_base_vault: &Pubkey,
    sm_raydium_quote_vault: &Pubkey,
    sm_raydium_amm: &Pubkey,

    user_reward_account: &Pubkey,

    marinade_state: &Pubkey,

    lamports: u64
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new_readonly(*arb_fund, false),
        AccountMeta::new_readonly(*arb_coffer, false),
        AccountMeta::new(*lucra_holding_vault, false),
        AccountMeta::new(*lucra_mint, false),
        AccountMeta::new_readonly(SystemState::find_lucra_mint_authority(system_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_orca_amm, false),
        AccountMeta::new_readonly(*sm_orca_base_vault, false),
        AccountMeta::new_readonly(*sm_orca_base_mint, false),
        AccountMeta::new_readonly(*sm_orca_quote_vault, false),
        AccountMeta::new_readonly(*sm_orca_quote_mint, false),
        AccountMeta::new_readonly(*sm_raydium_open_orders, false),
        AccountMeta::new_readonly(*sm_raydium_base_vault, false),
        AccountMeta::new_readonly(*sm_raydium_quote_vault, false),
        AccountMeta::new_readonly(*sm_raydium_amm, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(*marinade_state, false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),

    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Lucra as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_msol_for_arb_funds(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_coffer: &Pubkey,
    arb_fund: &Pubkey,
    mata_mint: &Pubkey,
    wsol_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    sm_raydium_base_vault: &Pubkey,
    sm_raydium_quote_vault: &Pubkey,
    sm_raydium_amm_open_orders: &Pubkey,
    sm_raydium_amm: &Pubkey,
    sm_orca_base_vault: &Pubkey,
    sm_orca_quote_vault: &Pubkey,
    sm_orca_amm: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_msol_account: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_pda: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    marinade_state: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_coffer, false),
        AccountMeta::new_readonly(SystemState::find_arb_coffer_authority(system_state).0, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*wsol_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_raydium_base_vault, false),
        AccountMeta::new_readonly(*sm_raydium_quote_vault, false),
        AccountMeta::new_readonly(*sm_raydium_amm_open_orders, false),
        AccountMeta::new_readonly(*sm_raydium_amm, false),
        AccountMeta::new_readonly(*sm_orca_base_vault, false),
        AccountMeta::new_readonly(*sm_orca_quote_vault, false),
        AccountMeta::new_readonly(*sm_orca_amm, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_pda, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Msol as u8, 
        amm_type: AmmTypes::None as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_msol_for_arb_funds_checking_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_coffer: &Pubkey,
    arb_fund: &Pubkey,
    mata_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    sm_base_vault: &Pubkey,
    sm_base_mint: &Pubkey,
    sm_quote_vault: &Pubkey,
    sm_amm: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_msol_account: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_pda: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    marinade_state: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_coffer, false),
        AccountMeta::new_readonly(SystemState::find_arb_coffer_authority(system_state).0, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_base_vault, false),
        AccountMeta::new_readonly(*sm_base_mint, false),
        AccountMeta::new_readonly(*sm_quote_vault, false),
        AccountMeta::new_readonly(*sm_amm, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_pda, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Msol as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_msol_for_arb_funds_checking_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_coffer: &Pubkey,
    arb_fund: &Pubkey,
    mata_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    sm_base_vault: &Pubkey,
    sm_base_mint: &Pubkey,
    sm_quote_vault: &Pubkey,
    sm_amm_open_orders: &Pubkey,
    sm_amm: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_msol_account: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_pda: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    treasury_msol_account: &Pubkey,
    marinade_state: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_coffer, false),
        AccountMeta::new_readonly(SystemState::find_arb_coffer_authority(system_state).0, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new_readonly(*sm_base_vault, false),
        AccountMeta::new_readonly(*sm_base_mint, false),
        AccountMeta::new_readonly(*sm_quote_vault, false),
        AccountMeta::new_readonly(*sm_amm_open_orders, false),
        AccountMeta::new_readonly(*sm_amm, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_pda, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new(*treasury_msol_account, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::MintFundsForArb { 
        fund_source: CurrencyTypes::Msol as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spend_arb_funds_for_lucra_using_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    lucra_mint: &Pubkey,
    reward_mint: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    wsol_holding_vault: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_lucra_account: &Pubkey,
    user_wsol_account: &Pubkey,
    pool_lucra_account: &Pubkey,
    pool_wsol_account: &Pubkey,
    amm_program: &Pubkey,
    amm_authority: &Pubkey,
    amm_open_orders: &Pubkey,
    amm_target: &Pubkey,
    serum_lucra_sol_market: &Pubkey,
    serum_bids: &Pubkey,
    serum_asks: &Pubkey,
    serum_event_queue: &Pubkey,
    serum_base_vault: &Pubkey,
    serum_quote_vault: &Pubkey,
    serum_vault_signer: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new_readonly(ArbState::find_arb_fund_authority(arb_state).0, false),
        AccountMeta::new(*lucra_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new(*wsol_holding_vault, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_lucra_account, false),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new_readonly(raydium_v4::id(), false),
        AccountMeta::new(*pool_lucra_account, false),
        AccountMeta::new(*pool_wsol_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new(*amm_program, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_open_orders, false),
        AccountMeta::new(*amm_target, false),
        AccountMeta::new(*serum_lucra_sol_market, false),
        AccountMeta::new_readonly(serum_v3::id(), false),
        AccountMeta::new(*serum_bids, false),
        AccountMeta::new(*serum_asks, false),
        AccountMeta::new(*serum_event_queue, false),
        AccountMeta::new(*serum_base_vault, false),
        AccountMeta::new(*serum_quote_vault, false),
        AccountMeta::new_readonly(*serum_vault_signer, false),
    ];
    let data = Instruction::BuyBurnForArb {
        fund_source: CurrencyTypes::Lucra as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports: 0,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spend_arb_funds_for_mata_using_raydium(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    mata_mint: &Pubkey,
    wsol_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_wsol_account: &Pubkey,
    user_mata_account: &Pubkey,
    pool_wsol_account: &Pubkey,
    pool_mata_account: &Pubkey,
    amm_program: &Pubkey,
    amm_authority: &Pubkey,
    amm_open_orders: &Pubkey,
    amm_target: &Pubkey,
    serum_sol_mata_market: &Pubkey,
    serum_bids: &Pubkey,
    serum_asks: &Pubkey,
    serum_event_queue: &Pubkey,
    serum_base_vault: &Pubkey,
    serum_quote_vault: &Pubkey,
    serum_vault_signer: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new_readonly(ArbState::find_arb_fund_authority(arb_state).0, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*wsol_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new_readonly(raydium_v4::id(), false),
        AccountMeta::new(*pool_wsol_account, false),
        AccountMeta::new(*pool_mata_account, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new(*amm_program, false),
        AccountMeta::new_readonly(*amm_authority, false),
        AccountMeta::new(*amm_open_orders, false),
        AccountMeta::new(*amm_target, false),
        AccountMeta::new(*serum_sol_mata_market, false),
        AccountMeta::new_readonly(serum_v3::id(), false),
        AccountMeta::new(*serum_bids, false),
        AccountMeta::new(*serum_asks, false),
        AccountMeta::new(*serum_event_queue, false),
        AccountMeta::new(*serum_base_vault, false),
        AccountMeta::new(*serum_quote_vault, false),
        AccountMeta::new_readonly(*serum_vault_signer, false),
    ];
    let data = Instruction::BuyBurnForArb { 
        fund_source: CurrencyTypes::Mata as u8, 
        amm_type: AmmTypes::Raydium as u8,
        lamports
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spend_arb_funds_for_lucra_using_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    wsol_holding_vault: &Pubkey,
    lucra_mint: &Pubkey,
    reward_mint: &Pubkey,
    lucra_sol_oracle: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_lucra_account: &Pubkey,
    user_wsol_account: &Pubkey,
    ls_amm: &Pubkey,
    ls_amm_authority: &Pubkey,
    ls_pool_base_vault: &Pubkey,
    ls_pool_quote_vault: &Pubkey,
    ls_pool_mint: &Pubkey,
    ls_pool_fees: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new_readonly(ArbState::find_arb_fund_authority(arb_state).0, false),
        AccountMeta::new(*wsol_holding_vault, false),
        AccountMeta::new(*lucra_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*lucra_sol_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(*user_account, true),
        AccountMeta::new(*user_lucra_account, false),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new_readonly(*ls_amm, false),
        AccountMeta::new_readonly(*ls_amm_authority, false),
        AccountMeta::new(*ls_pool_base_vault, false),
        AccountMeta::new(*ls_pool_quote_vault, false),
        AccountMeta::new(*ls_pool_mint, false),
        AccountMeta::new(*ls_pool_fees, false),
        AccountMeta::new_readonly(orca_swap::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::BuyBurnForArb { 
        fund_source: CurrencyTypes::Lucra as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports: 0,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spend_arb_funds_for_mata_using_orca(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_fund: &Pubkey,
    mata_mint: &Pubkey,
    wsol_mint: &Pubkey,
    reward_mint: &Pubkey,
    sol_usdc_oracle: &Pubkey,
    sol_usdt_oracle: &Pubkey,
    sol_mata_oracle: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_wsol_account: &Pubkey,
    user_mata_account: &Pubkey, 
    sm_amm: &Pubkey,
    sm_amm_authority: &Pubkey,
    sm_pool_base_vault: &Pubkey,
    sm_pool_quote_vault: &Pubkey,
    sm_pool_mint: &Pubkey,
    sm_pool_fees: &Pubkey,
    lamports: u64,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_fund, false),
        AccountMeta::new_readonly(ArbState::find_arb_fund_authority(arb_state).0, false),
        AccountMeta::new(*mata_mint, false),
        AccountMeta::new(*wsol_mint, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new_readonly(*sol_usdc_oracle, false),
        AccountMeta::new_readonly(*sol_usdt_oracle, false),
        AccountMeta::new_readonly(*sol_mata_oracle, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new_readonly(*user_account, true),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new(*user_mata_account, false),
        AccountMeta::new_readonly(*sm_amm, false),
        AccountMeta::new_readonly(*sm_amm_authority, false),
        AccountMeta::new(*sm_pool_base_vault, false),
        AccountMeta::new(*sm_pool_quote_vault, false),
        AccountMeta::new(*sm_pool_mint, false),
        AccountMeta::new(*sm_pool_fees, false),
        AccountMeta::new_readonly(orca_swap::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::BuyBurnForArb { 
        fund_source: CurrencyTypes::Mata as u8, 
        amm_type: AmmTypes::Orca as u8,
        lamports,
    };

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
} 

#[allow(clippy::too_many_arguments)]
pub fn clean_up_arb(
    system_state: &Pubkey,
    arb_state: &Pubkey,
    arb_coffer: &Pubkey,
    wsol_holding_vault: &Pubkey,
    reward_mint: &Pubkey,
    temp_wsol_account: &Pubkey,
    user_account: &Pubkey,
    user_reward_account: &Pubkey,
    user_wsol_account: &Pubkey,
    user_msol_account: &Pubkey,
    marinade_state: &Pubkey,
    msol_mint: &Pubkey,
    liq_pool_sol_leg_pda: &Pubkey,
    liq_pool_msol_leg: &Pubkey,
    liq_pool_msol_leg_authority: &Pubkey,
    reserve_pda: &Pubkey,
    msol_mint_authority: &Pubkey,
) -> SolInstruction {
    let accounts = vec![
        AccountMeta::new_readonly(*system_state, false),
        AccountMeta::new(*arb_state, false),
        AccountMeta::new(*arb_coffer, false),
        AccountMeta::new(*wsol_holding_vault, false),
        AccountMeta::new_readonly(ArbState::find_wsol_holding_vault_authority(arb_state).0, false),
        AccountMeta::new(*reward_mint, false),
        AccountMeta::new_readonly(SystemState::find_reward_mint_authority(system_state).0, false),
        AccountMeta::new(*user_account, true),
        AccountMeta::new(*user_wsol_account, false),
        AccountMeta::new(*user_msol_account, false),
        AccountMeta::new(*user_reward_account, false),
        AccountMeta::new(*temp_wsol_account, false),
        AccountMeta::new(*marinade_state, false),
        AccountMeta::new(*msol_mint, false),
        AccountMeta::new(*liq_pool_sol_leg_pda, false),
        AccountMeta::new(*liq_pool_msol_leg, false),
        AccountMeta::new_readonly(*liq_pool_msol_leg_authority, false),
        AccountMeta::new(*reserve_pda, false),
        AccountMeta::new_readonly(*msol_mint_authority, false),
        AccountMeta::new_readonly(solana_program::sysvar::fees::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(marinade_finance::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let data = Instruction::CleanUpArb {};

    SolInstruction {
        program_id: id(),
        accounts,
        data: serialize(&data).unwrap(),
    }
}