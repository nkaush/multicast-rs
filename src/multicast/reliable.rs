use super::{
    IncomingChannel, MulticastGroup, NetworkMessageType, NetworkMessage,
    MemberStateMessage, MemberStateMessageType, basic::BasicMulticast, NodeId
};
use std::{collections::HashMap, fmt};
use log::trace;

/// A reliable multicast implementation that guarantees delivery to all 
/// members of the group if a message is delivered to at least one member.
pub(super) struct ReliableMulticast<M> {
    /// The underlying basic multicast protocol
    basic: BasicMulticast<M>,
    prior_seq: HashMap<NodeId, usize>,
    next_seq_num: usize
}

impl<M> ReliableMulticast<M> {
    pub fn new(group: MulticastGroup<M>, from_members: IncomingChannel<M>) -> Self {
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

    pub fn broadcast(&mut self, msg_type: NetworkMessageType<M>) where M: fmt::Debug + Clone {
        let net_msg = NetworkMessage {
            msg_type,
            forwarded_for: None,
            sequence_num: self.get_next_seq_num()
        };
        self.basic.broadcast(net_msg, None);
    }

    pub fn send_single(&mut self, msg_type: NetworkMessageType<M>, recipient: &NodeId) where M: fmt::Debug + Clone {
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

    pub async fn deliver(&mut self) -> MemberStateMessage<M> where M: Clone + fmt::Debug {
        let member_state = self.basic.deliver().await;
        if let MemberStateMessageType::Message(msg) = &member_state.msg {
            if msg.sequence_num.is_none() {
                trace!("\treliable got network message from {} ... one off message", member_state.member_id);
                return member_state;
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
                    trace!("\treliable got network message from {} ... skipping ... last_seq={} and msg.sequence_num={}", member_state.member_id, last_seq, msg_seq_num);
                    return MemberStateMessage {
                        msg: MemberStateMessageType::DuplicateMessage,
                        member_id: original_sender
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