pub mod multicast;
pub mod bank;
pub mod cli;

pub use multicast::TotalOrderedMulticast;
pub use bank::{Bank, UserInput};
pub use cli::Cli;

pub type Port = u16;
pub type HostName = String;
pub type NodeId = String;
pub type ConnectionList = Vec<NodeId>;

use std::collections::HashMap;
pub type Config = HashMap<NodeId, (HostName, Port, ConnectionList)>;