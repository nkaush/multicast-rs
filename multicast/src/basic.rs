use super::{member::MemberStateMessage, MulticastGroup, IncomingChannel, config::NodeId};
use std::{fmt, collections::HashSet};
use log::{error, trace};
use serde::Serialize;

pub struct BasicMulticast<M> {
    group: MulticastGroup,
    from_members: IncomingChannel<M>,
    active_members: HashSet<NodeId>
}

impl<M> BasicMulticast<M> {
    pub(crate) fn new(group: MulticastGroup, from_members: IncomingChannel<M>) -> Self {
        let active_members = group.keys().cloned().collect();
        Self { group, from_members, active_members }
    }

    pub fn send_single(&self, msg: &M, recipient: &NodeId) where M: fmt::Debug + Serialize {
        let to_send = bincode::serialize(msg).unwrap();

        if let Some(handle) = self.group.get(recipient) {
            trace!("sending message to {}: {:?}", handle.member_id, msg);
            if let Err(e) = handle.pass_message(to_send) {
                error!("Failed to pass message to node {} handler: {:?}", handle.member_id, e);
            }
        }
    }

    pub fn broadcast(&self, msg: &M, except: Option<Vec<NodeId>>) where M: fmt::Debug + Serialize {
        let to_send = bincode::serialize(msg).unwrap();

        match except {
            Some(except) => for handle in self.group.values() {
                if !except.contains(&handle.member_id) {
                    trace!("sending message to {}: {:?}", handle.member_id, msg);
                    if let Err(e) = handle.pass_message(to_send.clone()) {
                        error!("Failed to pass message to node {} handler: {:?}", handle.member_id, e);
                    }
                }
            },
            None => for handle in self.group.values() {
                trace!("sending message to {}: {:?}", handle.member_id, msg);
                if let Err(e) = handle.pass_message(to_send.clone()) {
                    error!("Failed to pass message to node {} handler: {:?}", handle.member_id, e);
                }
            }
        }
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.group.remove(member_id);
        self.active_members.remove(member_id);
    }

    pub fn members(&self) -> &HashSet<NodeId> {
        &self.active_members
    }

    pub async fn deliver(&mut self) -> MemberStateMessage<M> {
        self.from_members.recv().await.unwrap()
    }
}