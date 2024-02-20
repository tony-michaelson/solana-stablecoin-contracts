use std::cell::RefMut;
use std::mem::size_of;

use arrayref::array_ref;
use legends_loadable_trait::Loadable;
use solana_program::{
    account_info::AccountInfo,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar, clock::Clock},
};
use crate::{
    error::{
        check_assert,
        LucraErrorCode,
        LucraResult,
        SourceFileId,
    },
    helpers::constants::{CREATOR_AUTHORITY, PRICE_HISTORY_ID},
    state::{
        DataType,
        MetaData,
        HistoricPrice,
        PriceHistory,
    },
};

declare_check_assert_macros!(SourceFileId::CreatePriceHistory);

// We only support tracking the lucra/sol price and sol/usd price.
#[inline(never)]
pub fn process_create_price_history(program_id: &Pubkey, accounts: &[AccountInfo]) -> LucraResult {
    const NUM_FIXED: usize = 2;
    let accounts = array_ref![accounts, 0, NUM_FIXED];
    let [
        creator_authority_ai,               // read
        price_history_ai,                   // write
    ] = accounts;

    let rent = &Rent::get()?;
    let clock = &Clock::get()?;

    check_eq!(creator_authority_ai.is_signer, true, LucraErrorCode::AccountNotSigner)?;
    check_eq!(creator_authority_ai.key, &CREATOR_AUTHORITY, LucraErrorCode::InvalidAccountInput)?;
    check_eq!(price_history_ai.key, &PRICE_HISTORY_ID, LucraErrorCode::InvalidAccountInput)?;

    check_eq!(price_history_ai.owner, program_id, LucraErrorCode::InvalidAccountOwner)?;

    check!(rent.is_exempt(price_history_ai.lamports(), size_of::<PriceHistory>()), LucraErrorCode::Default)?;
    let mut price_history: RefMut<PriceHistory> = PriceHistory::load_mut(price_history_ai)?;
    check!(!price_history.meta_data.is_initialized, LucraErrorCode::Default)?;

    price_history.meta_data = MetaData::new(DataType::PriceHistory, 0 , true);
    price_history.prices = [ HistoricPrice {
        lucra_price: 0,
        lucra_decimals: 0,
        date: 0,
        sol_price: 0,
        sol_decimals: 0,
        padding: [0; 6],
    }; 30];
    price_history.last_update_timestamp = 0;
    price_history.update_counter = 0;
    price_history.interval_start = clock.unix_timestamp;

    Ok(())
}