#[macro_use]
pub mod error;

pub mod instruction;
pub mod processor;
pub mod helpers;
pub mod state;

pub mod entrypoint;

pub use solana_program;

solana_program::declare_id!("LCu6pNvyoBkwCHYL6PbMLintScmZFrkDdbq1D7KZ4ay");