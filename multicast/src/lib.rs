mod connection_pool;
mod total_order;
mod protocol;
mod reliable;
mod config;
mod member;
mod basic;

use member::{MulticastMemberHandle, MemberStateMessage};
pub use config::{Config, NodeId, parse_config};
use tokio::sync::mpsc::UnboundedReceiver;
use std::collections::HashMap;

type MulticastGroup = HashMap<NodeId, MulticastMemberHandle>;
type IncomingChannel<M> = UnboundedReceiver<MemberStateMessage<M>>;

pub use total_order::TotalOrderedMulticast;
pub use reliable::ReliableMulticast;
pub use basic::BasicMulticast;

use async_trait::async_trait;
use serde::Serialize;

pub enum MulticastError {
    RebroadcastError(Vec<NodeId>),
    BroadcastError(Vec<NodeId>),
    InvalidRecipient(NodeId),
    ClientDisconnected(NodeId),
    AllClientsDisconnected
}

#[async_trait]
pub trait Multicast<M> where M: Serialize {
    fn connect(node_id: NodeId, configuration: Config) -> Self;

    fn connect_timeout(node_id: NodeId, configuration: Config, timeout_secs: u64) -> Self;

    fn broadcast(&mut self, msg: M) -> Result<(), MulticastError>;

    fn send_to(&mut self, msg: M, recipient: NodeId) -> Result<(), MulticastError>;

    async fn deliver(&mut self) -> Result<M, MulticastError> where M: Send;
}