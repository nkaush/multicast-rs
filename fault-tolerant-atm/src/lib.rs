pub mod bank;
pub mod cli;

pub use multicast::{TotalOrderedMulticast, parse_config};
pub use bank::{Bank, Transaction, TransactionType};
pub use cli::Cli;

pub fn get_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("Clock may have gone backwards")
        .as_secs_f64()
}
