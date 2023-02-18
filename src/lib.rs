mod multicast;
mod message;
mod bank;
mod cli;

pub use std::collections::HashMap;

pub type HostName = String;
pub type Port = u16;

pub type Config = HashMap<String, (HostName, Port, Vec<String>)>;