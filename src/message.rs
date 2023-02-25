use serde::{Serialize, Deserialize};
use core::cmp::Ordering;
use crate::NodeId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessageType {
    PriorityRequest(PriorityRequestType),
    PriorityProposal(PriorityProposalType),
    PriorityMessage(PriorityMessageType)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkMessage {
    /// If `sequence_num` is some, then this message must be reliably delivered.
    /// Otherwise, this message is a one-off and may be dropped.
    pub sequence_num: Option<usize>,
    pub msg_type: NetworkMessageType,
    pub forwarded_for: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityRequestType {
    pub local_id: MessageId,
    pub message: UserInput
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityProposalType {
    pub requester_local_id: MessageId,
    pub priority: MessagePriority
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityMessageType {
    pub local_id: MessageId,
    pub priority: MessagePriority,
}

#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq)]
pub struct MessageId {
    pub original_sender: NodeId,
    pub local_id: usize
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserInput {
    Deposit(String, usize),
    Transfer(String, String, usize)
}