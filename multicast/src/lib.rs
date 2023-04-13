mod connection_pool;
mod total_order;
mod protocol;
mod reliable;
mod config;
mod member;
mod basic;
mod pipe;

use member::{MulticastMemberHandle, MemberStateMessage};
pub use config::{Config, NodeId, parse_config};
use tokio::sync::mpsc::UnboundedReceiver;
use std::collections::HashMap;

type MulticastGroup = HashMap<NodeId, MulticastMemberHandle>;
type IncomingChannel<M> = UnboundedReceiver<MemberStateMessage<M>>;

pub use total_order::TotalOrderedMulticast;
pub use reliable::ReliableMulticast;
pub use basic::BasicMulticast;

use serde::{Serialize, de::DeserializeOwned};
use async_trait::async_trait;

#[derive(Debug)]
pub enum MulticastError {
    BroadcastError(Vec<NodeId>),
    InvalidRecipient(NodeId),
    ClientDisconnected(NodeId),
    AllClientsDisconnected,
    InternalError
}

#[async_trait]
pub trait Multicast<M> where M: Send + Serialize {
    async fn connect(node_id: NodeId, configuration: Config, timeout_secs: u64) 
        -> Self where M: 'static + Serialize + Send + DeserializeOwned;

    async fn broadcast(&mut self, msg: M) -> Result<(), MulticastError>;

    async fn send_to(&mut self, msg: M, recipient: NodeId) -> Result<(), MulticastError>;

    async fn deliver(&mut self) -> Result<M, MulticastError>;
}