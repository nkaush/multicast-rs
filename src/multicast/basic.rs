use super::{MemberStateMessage, MulticastGroup, IncomingChannel};
use crate::{NodeId, NetworkMessage};

pub(super) struct BasicMulticast {
    group: MulticastGroup,
    from_members: IncomingChannel
}

impl BasicMulticast {
    pub fn new(group: MulticastGroup, from_members: IncomingChannel) -> Self {
        Self { group, from_members }
    }

    pub fn size(&self) -> usize {
        self.group.len()
    }

    pub fn send_single(&self, msg: NetworkMessage, recipient: &NodeId) {
        if let Some(handle) = self.group.get(recipient) {
            handle.pass_message(msg).unwrap();
        }
    }

    pub fn broadcast(&self, msg: NetworkMessage, except: Option<String>) {
        match except {
            Some(except) => for handle in self.group.values() {
                if except != handle.member_id {
                    handle.pass_message(msg.clone()).unwrap();
                }
            },
            None => for handle in self.group.values() {
                handle.pass_message(msg.clone()).unwrap();
            }
        }
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.group.remove(member_id);
    }

    pub async fn deliver(&mut self) -> MemberStateMessage {
        self.from_members.recv().await.unwrap()
    }
}