use serde::{Serialize, Deserialize};
use core::cmp::Ordering;
use super::NodeId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessageType<M> {
    PriorityRequest(PriorityRequestType<M>),
    PriorityProposal(PriorityProposalType),
    PriorityMessage(PriorityMessageType)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkMessage<M> {
    /// If `sequence_num` is some, then this message must be reliably delivered.
    /// Otherwise, this message is a one-off and may be dropped.
    pub sequence_num: Option<usize>,
    pub msg_type: NetworkMessageType<M>,
    pub forwarded_for: Option<NodeId>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityRequestType<M> {
    pub local_id: MessageId,
    pub message: M
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityProposalType {
    pub requester_local_id: MessageId,
    pub priority: MessagePriority
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityMessageType {
    pub local_id: MessageId,
    pub priority: MessagePriority,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct MessageId {
    pub original_sender: NodeId,
    pub local_id: usize
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MessagePriority {
    pub priority: usize,
    pub proposer: NodeId
}

impl Ord for MessagePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.proposer.cmp(&other.proposer),
            x => x
        }
    }
}

impl PartialOrd for MessagePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => Some(self.proposer.cmp(&other.proposer)),
            x => Some(x)
        }
    }
}

impl PartialEq for MessagePriority {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.proposer == other.proposer
    }
}

impl Eq for MessagePriority {}