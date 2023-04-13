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
use std::fmt;

pub struct MulticastError {

}

#[async_trait]
pub trait Multicast<M> {
    fn connect(node_id: NodeId, configuration: Config) -> Self;

    fn connect_timeout(node_id: NodeId, configuration: Config, timeout_secs: u64) -> Self;

    fn broadcast(&mut self, msg: M) where M: fmt::Debug + Serialize;

    fn send_to(&mut self, msg: &M, recipient: &NodeId) where M: fmt::Debug + Serialize;

    async fn deliver(&mut self) -> Result<M, MulticastError>;
}