use super::{
    member::{MemberStateMessage, MemberStateMessageType}, config::NodeId,
    basic::BasicMulticast, IncomingChannel, MulticastGroup
};
use std::{collections::{HashSet, HashMap}, fmt};
use log::trace;
use serde::{Deserialize, Serialize};

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
    pub msg: M,
    /// If `sequence_num` is some, then this message must be reliably delivered.
    /// Otherwise, this message is a one-off and may be dropped.
    pub sequence_num: Option<usize>,
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

    pub fn broadcast(&mut self, msg: M) where M: fmt::Debug + Serialize {
        let net_msg = ReliableNetworkMessage {
            msg,
            forwarded_for: None,
            sequence_num: self.get_next_seq_num()
        };
        self.basic.broadcast(&net_msg, None);
    }

    pub fn send_single(&mut self, msg: M, recipient: &NodeId) where M: fmt::Debug + Serialize {
        let net_msg = ReliableNetworkMessage {
            msg,
            forwarded_for: None,
            sequence_num: None
        };
        self.basic.send_single(&net_msg, recipient);
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.basic.remove_member(member_id);
    }

    pub fn members(&self) -> &HashSet<NodeId> {
        &self.basic.members()
    }

    pub async fn deliver(&mut self) -> MemberStateMessage<M> where M: Serialize + fmt::Debug {
        let member_state = self.basic.deliver().await;

        match member_state.msg {
            MemberStateMessageType::Message(mut msg) => {
                if msg.sequence_num.is_none() {
                    trace!("network message from node {} ... one off message", member_state.member_id);
                    
                    return MemberStateMessage {
                        msg: MemberStateMessageType::Message(msg.msg),
                        member_id: member_state.member_id
                    };
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
                        return MemberStateMessage {
                            msg: MemberStateMessageType::DuplicateMessage,
                            member_id: original_sender
                        }
                    }
                }
    
                trace!("network message from node {} ... got {:?}", member_state.member_id, msg);
    
                self.prior_seq.insert(original_sender, msg_seq_num);
                msg.forwarded_for = Some(original_sender);
    
                self.basic.broadcast(&msg, Some(except));
                
                return MemberStateMessage {
                    msg: MemberStateMessageType::Message(msg.msg),
                    member_id: member_state.member_id
                };
            },
            MemberStateMessageType::DuplicateMessage => MemberStateMessage {
                msg: MemberStateMessageType::DuplicateMessage,
                member_id: member_state.member_id
            },
            MemberStateMessageType::NetworkError => MemberStateMessage {
                msg: MemberStateMessageType::NetworkError,
                member_id: member_state.member_id
            },
        }
    }
}