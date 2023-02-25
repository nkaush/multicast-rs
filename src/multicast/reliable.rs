use super::{member::{MemberStateMessage, MemberStateMessageType}, NetworkMessageType};
use crate::multicast::{IncomingChannel, MulticastGroup, basic::BasicMulticast};
use crate::{NetworkMessage, NodeId};
use std::collections::HashMap;
use log::trace;

pub(super) struct ReliableMulticast {
    basic: BasicMulticast,
    prior_seq: HashMap<NodeId, usize>,
    next_seq_num: usize
}

impl ReliableMulticast {
    pub fn new(group: MulticastGroup, from_members: IncomingChannel) -> Self {
        Self { 
            basic: BasicMulticast::new(group, from_members),
            prior_seq: HashMap::new(),
            next_seq_num: 0
        }
    }

    pub fn size(&self) -> usize {
        self.basic.size()
    }

    fn get_next_seq_num(&mut self) -> Option<usize> {
        let id = self.next_seq_num;
        self.next_seq_num += 1;
        Some(id)
    }

    pub fn broadcast(&mut self, msg_type: NetworkMessageType) {
        let net_msg = NetworkMessage {
            msg_type,
            forwarded_for: None,
            sequence_num: self.get_next_seq_num()
        };
        self.basic.broadcast(net_msg, None);
    }

    pub fn send_single(&mut self, msg_type: NetworkMessageType, recipient: &NodeId) {
        let net_msg = NetworkMessage {
            msg_type,
            forwarded_for: None,
            sequence_num: None
        };
        self.basic.send_single(net_msg, recipient);
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.basic.remove_member(member_id);
    }

    pub async fn deliver(&mut self) -> MemberStateMessage {
        // TODO maybe exclude BOTH forwarded_for AND member_id of sender
        let member_state = self.basic.deliver().await;
        if let MemberStateMessageType::Message(msg) = &member_state.msg {
            if msg.sequence_num.is_none() {
                trace!("\treliable got network message from {} ... one off message", member_state.member_id);
                return member_state;
            }

            let msg_seq_num = msg.sequence_num.clone().unwrap();

            let mut except = vec![member_state.member_id.clone()];
            let original_sender = match &msg.forwarded_for {
                Some(original_sender) => {
                    except.push(original_sender.clone());
                    original_sender
                },
                None => &member_state.member_id
            };

            if let Some(last_seq) = self.prior_seq.get(original_sender) {
                if last_seq >= &msg_seq_num {
                    trace!("\treliable got network message from {} ... skipping... last_seq={} and msg.sequence_num={}", member_state.member_id, last_seq, msg_seq_num);
                    return MemberStateMessage {
                        msg: MemberStateMessageType::DuplicateMessage,
                        member_id: original_sender.to_string()
                    }
                }
            }

            trace!("\treliable got network message from {} ... got {:?}\n", member_state.member_id, msg);

            self.prior_seq.insert(original_sender.into(), msg_seq_num);
            let msg = NetworkMessage {
                forwarded_for: Some(original_sender.into()),
                sequence_num: msg.sequence_num,
                msg_type: msg.msg_type.clone()
            };
            self.basic.broadcast(msg, Some(except));
        } 

        member_state
    }
}