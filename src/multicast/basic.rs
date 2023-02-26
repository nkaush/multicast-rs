use super::{MulticastGroup, IncomingChannel, NetworkMessage, MemberStateMessage, NodeId};
use log::trace;
use std::fmt;

pub(super) struct BasicMulticast<M> {
    group: MulticastGroup<M>,
    from_members: IncomingChannel<M>
}

impl<M> BasicMulticast<M> {
    pub fn new(group: MulticastGroup<M>, from_members: IncomingChannel<M>) -> Self {
        Self { group, from_members }
    }

    pub fn size(&self) -> usize {
        self.group.len()
    }

    pub fn send_single(&self, msg: NetworkMessage<M>, recipient: &NodeId) where M: fmt::Debug {
        if let Some(handle) = self.group.get(recipient) {
            trace!("sending message to {}: {:?}\n", handle.member_id, msg);
            handle.pass_message(msg).unwrap();
        }
    }

    pub fn broadcast(&self, msg: NetworkMessage<M>, except: Option<Vec<NodeId>>) where M: fmt::Debug + Clone {
        match except {
            Some(except) => for handle in self.group.values() {
                if !except.contains(&handle.member_id) {
                    trace!("sending message to {}: {:?}\n", handle.member_id, msg);
                    handle.pass_message(msg.clone()).unwrap();
                }
            },
            None => for handle in self.group.values() {
                trace!("sending message to {}: {:?}\n", handle.member_id, msg);
                handle.pass_message(msg.clone()).unwrap();
            }
        }
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.group.remove(member_id);
    }

    pub async fn deliver(&mut self) -> MemberStateMessage<M> {
        self.from_members.recv().await.unwrap()
    }
}