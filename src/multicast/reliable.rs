use super::{MemberStateMessage, MemberStateMessageType, NetworkMessageType};
use crate::multicast::basic::BasicMulticast;
use crate::{NetworkMessage, NodeId};
use std::collections::HashMap;

pub(super) struct ReliableMulticast {
    basic: BasicMulticast,
    prior_seq: HashMap<NodeId, usize>,
    next_seq_num: usize
}

impl ReliableMulticast {
    pub fn new(basic: BasicMulticast) -> Self {
        Self { 
            basic,
            prior_seq: HashMap::new(),
            next_seq_num: 0
        }
    }

    pub fn size(&self) -> usize {
        self.basic.size()
    }

    fn get_next_seq_num(&mut self) -> usize {
        let id = self.next_seq_num;
        self.next_seq_num += 1;
        id
    }

    pub fn broadcast(&mut self, msg_type: NetworkMessageType, except: Option<String>) {
        let net_msg = NetworkMessage {
            msg_type,
            forwarded_for: None,
            sequence_num: self.get_next_seq_num()
        };
        self.basic.broadcast(net_msg, except);
    }

    pub fn send_single(&mut self, msg_type: NetworkMessageType, recipient: &NodeId) {
        let net_msg = NetworkMessage {
            msg_type,
            forwarded_for: None,
            sequence_num: self.get_next_seq_num()
        };
        self.basic.send_single(net_msg, recipient);
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.basic.remove_member(member_id);
    }

    pub async fn deliver(&mut self) -> Option<MemberStateMessage> {
        // TODO maybe exclude BOTH forwarded_for AND member_id of sender
        let member_state = self.basic.deliver().await;
        if let MemberStateMessageType::Message(msg) = &member_state.msg {
            let original_sender = match &msg.forwarded_for {
                Some(original_sender) => original_sender,
                None => &member_state.member_id
            };

            if let Some(last_seq) = self.prior_seq.get(original_sender) {
                if last_seq >= &msg.sequence_num {
                    return None;
                }
            }

            self.prior_seq.insert(original_sender.into(), msg.sequence_num);
            let msg = NetworkMessage {
                forwarded_for: Some(original_sender.into()),
                sequence_num: msg.sequence_num,
                msg_type: msg.msg_type.clone()
            };
            self.basic.broadcast(msg, Some(original_sender.into()));
        } 

        Some(member_state)
    }
}