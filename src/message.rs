use serde::{Serialize, Deserialize};
use core::cmp::Ordering;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetworkMessage {
    PriorityRequest(PriorityRequestType),
    PriorityProposal(PriorityProposalType),
    PriorityMessage(PriorityMessageType)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityRequestType {
    pub sender: String,
    pub local_id: usize
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityProposalType {
    pub proposer: String,
    pub local_id: usize,
    pub priority: usize
}

/// TODO: https://doc.rust-lang.org/stable/std/cmp/trait.Ord.html#how-can-i-implement-ord
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityMessageType {
    pub priority: usize,
    pub proposer: String,
    pub message: UserInput
}

impl Ord for PriorityMessageType {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.proposer.cmp(&other.proposer),
            x => x
        }
    }
}

impl PartialOrd for PriorityMessageType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => Some(self.proposer.cmp(&other.proposer)),
            x => Some(x)
        }
    }
}

impl PartialEq for PriorityMessageType {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.proposer == other.proposer
    }
}

impl Eq for PriorityMessageType {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserInput {
    Deposit(String, usize),
    Transfer(String, String, usize)
}