use serde::{Serialize, Deserialize};
use core::cmp::Ordering;
// use std::cmp::Ordering;

#[derive(Serialize, Deserialize)]
pub enum NetworkMessage {
    PriorityRequest(PriorityRequestType),
    PriorityProposal(PriorityProposalType),
    PriorityMessage(PriorityMessageType),
    NameMessage(String)
}

#[derive(Serialize, Deserialize)]
pub struct PriorityRequestType {
    sender: String,
    local_id: usize
}

#[derive(Serialize, Deserialize)]
pub struct PriorityProposalType {
    proposer: String,
    local_id: usize,
    priority: usize
}

/// TODO: https://doc.rust-lang.org/stable/std/cmp/trait.Ord.html#how-can-i-implement-ord
#[derive( PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityMessageType {
    priority: usize,
    proposer: String,
    message: String
}

impl Ord for PriorityMessageType {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.proposer.cmp(&other.proposer),
            x => x
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum UserInput {
    Deposit(String, usize),
    Transfer(String, String, usize)
}