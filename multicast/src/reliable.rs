use super::{
    member::MemberStateMessageType, IncomingChannel, Multicast, MulticastError,
    config::{Config, NodeId}, basic::BasicMulticast, MulticastGroup
};
use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use async_trait::async_trait;
use log::trace;

/// A reliable multicast implementation that guarantees delivery to all 
/// members of the group if a message is delivered to at least one member.
pub struct ReliableMulticast<M> {
    /// The underlying basic multicast protocol
    basic: BasicMulticast<ReliableNetworkMessage<M>>,
    prior_seq: HashMap<NodeId, usize>,
    next_seq_num: usize
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReliableNetworkMessage<M> {
    /// The message to reliably multicast to all other members of the group.
    pub msg: M,
    /// If `sequence_num` is `Some(_)`, where `_` is the sequence number, then
    /// this message must be reliably delivered. Otherwise, this message is a 
    /// one-off message to a single recipient and may be dropped if the sender
    /// crashes before transmitting all bytes over the network.
    pub sequence_num: Option<usize>,
    /// If this message was forwarded on behalf of some original sender, then 
    /// `forwarded_for` is `Some(_)` where `_` is the `NodeId` of the original 
    /// sender. Otherwise, this message originated from the connection it was 
    /// received on.
    pub forwarded_for: Option<NodeId>
}

impl<M> ReliableMulticast<M> {
    pub(crate) fn new(group: MulticastGroup, from_members: IncomingChannel<ReliableNetworkMessage<M>>) -> Self {
        Self { 
            basic: BasicMulticast::new(group, from_members),
            prior_seq: HashMap::new(),
            next_seq_num: 0
        }
    }

    fn get_next_seq_num(&mut self) -> Option<usize> {
        let id = self.next_seq_num;
        self.next_seq_num += 1;
        Some(id)
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.basic.remove_member(member_id);
    }

    pub fn members(&self) -> &HashSet<NodeId> {
        &self.basic.members()
    }
}

#[async_trait]
impl<M> Multicast<M> for ReliableMulticast<M> where M: Send + Serialize {
    async fn connect(_: usize, _: Config, _: u64) -> Self where M: 'static + DeserializeOwned { todo!() }

    async fn broadcast(&mut self, msg: M) -> Result<(), MulticastError> { 
        let sequence_num = self.get_next_seq_num();
        self.basic.broadcast(ReliableNetworkMessage {
            msg,
            forwarded_for: None,
            sequence_num
        }).await
    }

    async fn send_to(&mut self, msg: M, recipient: NodeId) -> Result<(), MulticastError> { 
        self.basic.send_to(
            ReliableNetworkMessage { 
                msg, sequence_num: None, forwarded_for: None
            }, 
            recipient
        ).await
    }

    async fn deliver(&mut self) -> Result<M, MulticastError> { 
        let member_state = match self.basic.raw_deliver().await {
            Some(s) => s,
            None => return Err(MulticastError::AllClientsDisconnected)
        };

        match member_state.msg {
            MemberStateMessageType::Message(mut msg) => {
                if msg.sequence_num.is_none() {
                    trace!("network message from node {} ... one off message", member_state.member_id);
                    
                    return Ok(msg.msg);
                }
    
                let msg_seq_num = msg.sequence_num.unwrap();
    
                let mut except = vec![member_state.member_id];
                let original_sender = match msg.forwarded_for {
                    Some(original_sender) => {
                        except.push(original_sender);
                        original_sender
                    },
                    None => member_state.member_id
                };
    
                if let Some(last_seq) = self.prior_seq.get(&original_sender) {
                    if last_seq >= &msg_seq_num {
                        trace!("network message from node {} ... skipping ... last_seq={} and msg.sequence_num={}", member_state.member_id, last_seq, msg_seq_num);
                        
                        // got a duplicated message, skip and wait --> recurse!
                        return self.deliver().await; 
                    }
                }
    
                trace!("network message from node {} ... got ReliableNetworkMessage {{ msg: (...), sequence_num: {:?}, forwarded_for: {:?}}}", member_state.member_id, msg.sequence_num, msg.forwarded_for);
    
                self.prior_seq.insert(original_sender, msg_seq_num);
                msg.forwarded_for = Some(original_sender);
                self.basic
                    .broadcast_except(msg, except)
                    .map(|m| m.msg)
            },
            MemberStateMessageType::NetworkError => Err(MulticastError::ClientDisconnected(member_state.member_id))
        }
    }
}