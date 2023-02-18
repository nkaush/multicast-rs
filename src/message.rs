pub struct FromMulticast {

}

pub struct ToMulticast {

}

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

/// TODO: implement the Ord trait
#[derive(Ord, PartialOrd, PartialEq, Eq)]
pub struct PriorityMessageType {
    proposer: String,
    priority: usize,
    message: String
}

#[derive(Debug)]
pub enum UserInput {
    Deposit(String, usize),
    Transfer(String, String, usize)
}