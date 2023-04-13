use serde::{Serialize, Deserialize};
use core::cmp::Ordering;
use super::NodeId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TotalOrderNetworkMessage<M> {
    PriorityRequest(PriorityRequestArgs<M>),
    PriorityProposal(PriorityProposalArgs),
    PriorityMessage(PriorityMessageArgs)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityRequestArgs<M> {
    pub local_id: MessageId,
    pub message: M
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityProposalArgs {
    pub requester_local_id: MessageId,
    pub priority: MessagePriority
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityMessageArgs {
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