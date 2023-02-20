use serde::{Serialize, Deserialize};
use core::cmp::Ordering;

pub enum NetworkMessage {
    PriorityRequest(PriorityRequestType),
    PriorityProposalType(PriorityProposalType),
    PriorityMessageType(PriorityMessageType)
}

pub struct PriorityRequestType {
    sender: String,
    local_id: usize
}

pub struct PriorityProposalType {
    proposer: String,
    local_id: usize,
    priority: usize
}

/// TODO: https://doc.rust-lang.org/stable/std/cmp/trait.Ord.html#how-can-i-implement-ord
#[derive(Ord, PartialOrd, PartialEq, Eq)]
pub struct PriorityMessageType {
    priority: usize,
    proposer: String,
    message: String
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UserInput {
    Deposit(String, usize),
    Transfer(String, String, usize)
}