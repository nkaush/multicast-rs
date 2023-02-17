mod multicast;
mod message;
mod bank;

pub use std::collections::HashMap;

pub type HostName = String;
pub type Port = u16;

pub type Config = HashMap<String, (HostName, Port)>;