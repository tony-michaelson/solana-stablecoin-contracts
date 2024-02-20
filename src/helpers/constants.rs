use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use solana_program::pubkey::Pubkey;

pub const SOL_FEE_PLUS_INTEREST: u32 = 5_500;
pub const LAMPORTS_PER_LUCRA: Decimal = dec!(1_000_000_000);
pub const LAMPORTS_PER_MATA: Decimal = dec!(1_000_000);

#[cfg(not(feature = "devnet"))]
pub const UNIX_HOUR: i64 = 3_600;

#[cfg(not(feature = "devnet"))]
pub const UNIX_DAY: i64 = 86_400;

#[cfg(not(feature = "devnet"))]
pub const ORACLE_PRICE_MAX_SLOTS: u64 = 25;

#[cfg(feature = "devnet")]
pub const UNIX_HOUR: i64 = 3_600;

#[cfg(feature = "devnet")]
pub const UNIX_DAY: i64 = 86_400;

#[cfg(feature = "devnet")]
pub const ORACLE_PRICE_MAX_SLOTS: u64 = 25;

pub mod serum_v3 {
    solana_program::declare_id!("EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj");
}

pub mod orca_swap {
    solana_program::declare_id!("3xQ8SWv2GaFXXpHZNqkXsdxq5DZciHBz6ZFoPPfbFd7U");
}

pub mod raydium_v4 {
    solana_program::declare_id!("9rpQHSyFVM1dkkHFQ2TtTzPEW7DVmEyPmN8wVniqJtuC");
}

pub mod wsol {
    solana_program::declare_id!("So11111111111111111111111111111111111111112");
}

pub const CREATOR_AUTHORITY: Pubkey = Pubkey::new_from_array([
    4, 239, 104, 212, 231, 140, 124, 88, 9, 18, 156, 231, 76, 16, 190, 140, 86, 202, 248, 45, 181,
    78, 193, 48, 7, 71, 69, 120, 72, 15, 220, 81,
]); // LGNDSCoQfZZDZDBtVLgXvmJpqzRjRBcXSxwkSZjp3wN

pub const DAO_AUTHORITY: Pubkey = Pubkey::new_from_array([
    4, 239, 104, 212, 231, 140, 124, 88, 9, 18, 156, 231, 76, 16, 190, 140, 86, 202, 248, 45, 181,
    78, 193, 48, 7, 71, 69, 120, 72, 15, 220, 81,
]); // LGNDSCoQfZZDZDBtVLgXvmJpqzRjRBcXSxwkSZjp3wN

pub const SOL_MATA_ORCA_AMM: Pubkey = Pubkey::new_from_array([
    21,142,16,169,77,155,190,65,226,123,15,54,233,81,254,87,49,86,245,126,102,20,20,226,175,192,181,200,167,48,70,31
]); // 2T9Csiqxm7cYhGKNvkhQNyGaUo1y9YiJm9xPPm58ymFL

pub const LUCRA_SOL_ORCA_AMM: Pubkey = Pubkey::new_from_array([
    249,32,104,148,195,102,46,154,193,193,121,124,19,148,133,217,160,21,59,77,118,89,121,133,133,140,29,101,107,197,17,35
]); // HmVBPAR6hBch4zbj6hnboCqgeKF2AQkDa7bc2SL54fBp

pub const SOL_MATA_RAYDIUM_AMM: Pubkey = Pubkey::new_from_array([
    137,4,110,239,17,160,233,138,215,231,130,174,13,1,219,222,138,199,28,129,50,18,231,178,93,20,251,78,129,192,136,9
]); // ADrmye5mFj9yUKQFAybHvk9y8zamdRFkc5AaiezdfG72

pub const LUCRA_SOL_RAYDIUM_AMM: Pubkey = Pubkey::new_from_array([
    10,184,37,189,87,0,196,96,80,215,191,129,50,76,196,109,70,37,16,109,198,156,183,228,238,120,91,217,140,140,144,189
]); // iqwQ4XpSxUwJ4Dj7VJCnk5dajy3fgMZuVGftCwD87Ti

pub const SOL_MATA_SERUM_MARKET: Pubkey = Pubkey::new_from_array([
    194,53,150,187,24,239,114,197,96,121,58,125,69,132,250,192,246,35,165,218,91,12,152,203,23,109,212,156,120,131,3,55
]); // E57VHAr3nRtJRK8Doib8WtfPT6uHsE6c5FSMfjXjUF1g

pub const LUCRA_SOL_SERUM_MARKET: Pubkey = Pubkey::new_from_array([
    196,87,171,20,8,224,88,66,189,179,248,191,54,237,93,76,56,142,111,17,73,193,215,17,236,25,122,5,107,198,55,224
]); // EDSShLzZzkmSDbjHnbJFU8JpKCYFPFMY8oCmWvzULwnF

pub const SOL_USDC_ORACLE: Pubkey = Pubkey::new_from_array([
    11,206,142,41,69,28,71,136,115,250,223,165,174,218,69,223,93,220,242,50,167,171,108,104,54,55,154,216,236,114,243,196
]); // o6AUA4qCJ4XHgQbLPopkw8SziZRvTKJBAXTjyMNewCP

pub const SOL_USDT_ORACLE: Pubkey = Pubkey::new_from_array([
    11,222,8,200,35,43,180,171,117,8,212,88,31,206,44,86,168,97,71,88,84,51,40,115,110,186,113,236,210,90,118,222
]); // oKrUHgwBiZzPwwDenyRYHDmKwQDHAqgTEk1U4KfRXEM

pub const LUCRA_SOL_ORACLE: Pubkey = Pubkey::new_from_array([
    12,4,152,143,232,70,234,174,41,151,211,230,71,55,82,32,28,36,66,36,154,90,192,41,43,66,201,237,183,254,253,51
]); // ouxWX8BuguuhrXfLzxRBHBRNPTSUwdgkWTwJmR6hgCi

pub const SOL_MATA_ORACLE: Pubkey = Pubkey::new_from_array([
    11,233,63,90,18,227,43,49,24,10,14,81,82,240,47,23,210,175,131,59,242,10,62,25,245,224,10,57,59,167,150,212
]); // oVmf1NDUdsKB35hjZdpeCz3TkAr3CUuvKVws5nN8kZ1

pub const PRICE_HISTORY_ID: Pubkey = Pubkey::new_from_array([
    12,45,232,22,240,116,191,56,17,41,96,193,114,210,124,190,134,195,168,57,39,123,74,22,20,32,147,63,87,53,144,40
]); // pYVZSidHAnvyQrKUuWfzzhu1FMvXWDG3VV71HgK1bgB