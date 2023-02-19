pub mod multicast;
pub mod message;
pub mod bank;
pub mod cli;

pub use multicast::*;
pub use message::*;
pub use bank::*;
pub use cli::*;

pub use std::collections::HashMap;

pub type ConnectionList = Vec<String>;
pub type HostName = String;
pub type Port = u16;

pub type Config = HashMap<String, (HostName, Port, ConnectionList)>;