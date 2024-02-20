use bytemuck::Contiguous;
use num_enum::IntoPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

pub type LucraResult<T = ()> = Result<T, LucraError>;

#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum SourceFileId {
    Account = 0,
    AddCollateral,
    ArbState,
    BeginCreateMataLoan,
    BuyBurnForArb,
    ClaimReward,
    CleanUpArb,
    CloseMataLoan,
    CreateOracle,
    CreatePriceHistory,
    CreateStakingAccount,
    CreateStakeBalance,
    CofferArb,
    Decimal,
    DepositStake,
    DeterminePenalty,
    DropReward,
    EndUnstake,
    HarvestPenalty,
    Initialize,
    Loans,
    LucraMataArb,
    MataLucraArb,
    Math,
    MintFundsForArb,
    Oracle,
    OracleHelper,
    PendingFunds,
    PendingWithdrawal,
    PriceHistory,
    Rate,
    Raydium,
    RedeemRewardTokens,
    Reward,
    SellFundsForArb,
    Spl,
    SplTokenSwap,
    Stake,
    StakeBalance,
    Staking,
    StakingState,
    StartUnstake,
    SystemState,
    TransferFunds,
    UpdatePrice,
    UpdatePriceHistory,
    UpdateState,
    WithdrawStake,
}

impl std::fmt::Display for SourceFileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceFileId::Account => write!(f, "src/helpers/account.rs"),
            SourceFileId::Math => write!(f, "src/helpers/math.rs"),
            SourceFileId::OracleHelper => write!(f, "src/helpers/oracle.rs"),
            SourceFileId::Spl => write!(f, "src/helpers/spl.rs"),
            SourceFileId::SplTokenSwap => write!(f, "src/helpers/spl_token_swap.rs"),
            SourceFileId::Raydium => write!(f, "src/helpers/raydium.rs"),

            SourceFileId::Loans => write!(f, "src/state/loans/mataloan.rs"),
            SourceFileId::Oracle => write!(f, "src/state/oracle.rs"),
            SourceFileId::PendingFunds => write!(f, "src/state/pendingfunds.rs"),
            SourceFileId::PriceHistory => write!(f, "src/state/pricehistory.rs"),
            SourceFileId::ArbState => write!(f, "src/state/arbitrage/arb_state.rs"),
            SourceFileId::PendingWithdrawal => write!(f, "src/state/staking/pendingwithdrawal.rs"),
            SourceFileId::Reward => write!(f, "src/state/staking/rewards/reward.rs"),
            SourceFileId::StakeBalance => write!(f, "src/state/staking/stakebalance.rs"),
            SourceFileId::Staking => write!(f, "src/state/staking/stakeaccount.rs"),
            SourceFileId::StakingState => write!(f, "src/state/staking/staking_state.rs"),
            SourceFileId::SystemState => write!(f, "src/state/system_state.rs"),
            
            SourceFileId::Decimal => write!(f, "src/math/decimal.rs"),
            SourceFileId::Rate => write!(f, "src/math/rate.rs"),
            
            SourceFileId::AddCollateral => write!(f, "src/processor/process_add_collateral.rs"),
            SourceFileId::BeginCreateMataLoan => write!(f, "src/processor/process_begin_create_mata_loan.rs"),
            SourceFileId::BuyBurnForArb => write!(f, "src/process/process_buy_burn_for_arb.rs"),
            SourceFileId::LucraMataArb => write!(f, "src/processor/process_lucra_mata_arb.rs"),
            SourceFileId::MataLucraArb => write!(f, "src/processor/process_mata_lucra_arb.rs"),
            SourceFileId::ClaimReward => write!(f, "src/processor/process_claim_reward.rs"),
            SourceFileId::CleanUpArb => write!(f, "src/processor/process_clean_up_arb.rs"),
            SourceFileId::CloseMataLoan => write!(f, "src/processor/process_close_mata_loan.rs"),
            SourceFileId::CreateOracle => write!(f, "src/processor/process_create_oracle.rs"),
            SourceFileId::CreatePriceHistory => write!(f, "src/processor/process_create_price_history.rs"),
            SourceFileId::CreateStakeBalance => write!(f, "src/processor/process_create_stake_balance.rs"),
            SourceFileId::CreateStakingAccount => write!(f, "src/processor/process_create_staking_account.rs"),
            SourceFileId::DepositStake => write!(f, "src/processor/process_deposit_stake.rs"),
            SourceFileId::DeterminePenalty => write!(f, "src/processor/process_determine_penalty.rs"),
            SourceFileId::DropReward => write!(f, "src/processor/process_drop_reward.rs"),
            SourceFileId::EndUnstake => write!(f, "src/processor/process_end_unstake.rs"),
            SourceFileId::HarvestPenalty => write!(f, "src/processor/process_harvest_penalty.rs"),
            SourceFileId::Initialize => write!(f, "src/processor/process_initialize.rs"),
            SourceFileId::CofferArb => write!(f, "src/processor/process_coffer_arb.rs"),
            SourceFileId::MintFundsForArb => write!(f, "src/processor/process_mint_funds_for_arb.rs"),
            SourceFileId::RedeemRewardTokens => write!(f, "src/process/process_redeem_reward_tokens.rs"),
            SourceFileId::SellFundsForArb => write!(f, "src/processor/process_sell_funds_for_arb.rs"),
            SourceFileId::Stake => write!(f, "src/processor/process_stake.rs"),
            SourceFileId::StartUnstake => write!(f, "src/processor/process_start_unstake.rs"),
            SourceFileId::TransferFunds => write!(f, "src/processor/process_transfer_funds.rs"),
            SourceFileId::UpdatePrice => write!(f, "src/processor/process_update_price.rs"),
            SourceFileId::UpdatePriceHistory => write!(f, "src/processor/process_update_price_history.rs"),
            SourceFileId::UpdateState => write!(f, "src/processor/process_update_state.rs"),
            SourceFileId::WithdrawStake => write!(f, "src/processor/process_withdraw_stake.rs"),
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LucraError {
    #[error(transparent)]
    ProgramError(#[from] ProgramError),
    #[error("{lucra_error_code}; {source_file_id}:{line}")]
    LucraErrorCode { lucra_error_code: LucraErrorCode, line: u32, source_file_id: SourceFileId },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum LucraErrorCode {
    // 0
    #[error("LucraErrorCode::InvalidAmount")]
    InvalidAmount,

    #[error("LucraErrorCode::InvalidAccountOwner")]
    InvalidAccountOwner,

    #[error("LucraErrorCode::InvalidAccountInput")]
    InvalidAccountInput,

    #[error("LucraErrorCode::NotRentExempt")]
    NotRentExempt,

    #[error("LucraErrorCode::Timelock")]
    Timelock,
    
    // 5
    #[error("LucraErrorCode::InvalidNonce")]
    InvalidNonce,

    #[error("LucraErrorCode::OutstandingLoans")]
    OutstandingLoans,

    #[error("LucraErrorCode::AlreadyProcessed")]
    AlreadyProcessed,

    #[error("LucraErrorCode::NotStakedDuringDrop")]
    NotStakedDuringDrop,

    #[error("LucraErrorCode::InsufficientTimePassed")]
    InsufficientTimePassed,
    
    // 10
    #[error("LucraErrorCode::CalculationFailed")]
    CalculationFailure,

    #[error("LucraErrorCode::MathError")]
    MathError,

    #[error("LucraErrorCode::EarlyRewardDrop")]
    EarlyRewardDrop,

    #[error("LucraErrorCode::AccountNotMutable")]
    AccountNotMutable,

    #[error("LucraErrorCode::AccountNotSigner")]
    AccountNotSigner,
    
    // 15
    #[error("LucraErrorCode::StakingAccountNotUnlocked")]
    StakingAccountNotUnlocked,

    #[error("LucraErrorCode::ArbitrageNotEnabled")]
    ArbitrageNotEnabled,

    #[error("LucraErrorCode::StakingNotEnabled")]
    StakingNotEnabled,

    #[error("LucraErrorCode::LoansNotEnabled")]
    LoansNotEnabled,

    #[error("LucraErrorCode::InvalidLoanType")]
    InvalidLoanType,

    // 20
    #[error("LucraErrorCode::RewardsOutstanding")]
    RewardsOutstanding,

    #[error("LucraErrorCode::InvalidOracleConfig")]
    InvalidOracleConfig,

    #[error("LucraErrorCode::ClaimOutOfOrder")]
    ClaimOutOfOrder,

    #[error("LucraErrorCode::OracleStatusNotValid")]
    OracleStatusNotValid,

    #[error("LucraErrorCode::OracleStale")]
    OracleStale,

    // 25
    #[error("LucraErrorCode::PriceDataInvalid")]
    PriceDataInvalid,

    #[error("LucraErrorCode::PriceAlreadyUpdatedForSlot")]
    PriceAlreadyUpdatedForSlot,

    #[error("LucraErrorCode::BrokenPeg")]
    BrokenPeg,

    #[error("LucraErrorCode::PegNotBroken")]
    PegNotBroken,

    #[error("LucraErrorCode::NoPenaltyToHarvest")]
    NoPenaltyToHarvest,

    // 30
    #[error("LucraErrorCode::EmptyPool")]
    EmptyPool,

    #[error("LucraErrorCode::TransactionFailed")]
    TransactionFailed,

    #[error("LucraErrorCode::NotImplemented")]
    NotImplemented,

    #[error("LucraErrorCode::InvalidStateTransition")]
    InvalidStateTransition,

    #[error("LucraErrorCode::InvalidState")]
    InvalidState,

    // 35
    #[error("LucraErrorCode::Default Check the source code for more info")]
    Default = u32::MAX_VALUE,

}

impl From<LucraError> for ProgramError {
    fn from(e: LucraError) -> ProgramError {
        match e {
            LucraError::ProgramError(pe) => pe,
            LucraError::LucraErrorCode { lucra_error_code, line: _, source_file_id: _ } => {
                ProgramError::Custom(lucra_error_code.into())
            }
        }
    }
}

#[inline]
pub fn check_assert(
    cond: bool,
    lucra_error_code: LucraErrorCode,
    line: u32,
    source_file_id: SourceFileId,
) -> LucraResult<()> {
    if cond {
        Ok(())
    } else {
        Err(LucraError::LucraErrorCode { lucra_error_code, line, source_file_id })
    }
}

#[macro_export]
macro_rules! declare_check_assert_macros {
    ($source_file_id:expr) => {
        #[allow(unused_macros)]
        macro_rules! check {
            ($cond:expr, $err:expr) => {
                check_assert($cond, $err, line!(), $source_file_id)
            };
        }

        #[allow(unused_macros)]
        macro_rules! check_eq {
            ($x:expr, $y:expr, $err:expr) => {
                check_assert($x == $y, $err, line!(), $source_file_id)
            };
        }

        #[allow(unused_macros)]
        macro_rules! throw {
            () => {
                LucraError::LucraErrorCode {
                    lucra_error_code: LucraErrorCode::Default,
                    line: line!(),
                    source_file_id: $source_file_id,
                }
            };
        }

        #[allow(unused_macros)]
        macro_rules! throw_err {
            ($err:expr) => {
                LucraError::LucraErrorCode {
                    lucra_error_code: $err,
                    line: line!(),
                    source_file_id: $source_file_id,
                }
            };
        }

        #[allow(unused_macros)]
        macro_rules! math_err {
            () => {
                LucraError::LucraErrorCode {
                    lucra_error_code: LucraErrorCode::MathError,
                    line: line!(),
                    source_file_id: $source_file_id,
                }
            };
        }
    };
}