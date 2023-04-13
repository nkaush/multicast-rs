use super::{
    member::MemberStateMessage, MulticastGroup, IncomingChannel, 
    config::{Config, NodeId}, Multicast, MulticastError
};
use std::collections::HashSet;
use async_trait::async_trait;
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

    pub(crate) fn broadcast_except(&mut self, msg: M, except: Vec<NodeId>) -> Result<M, MulticastError> where M: Serialize {
        let to_send = bincode::serialize(&msg).unwrap();
        let mut failures = Vec::new();

        for handle in self.group.values() {
            if !except.contains(&handle.member_id) {
                trace!("Sending message to {}: [...{} bytes...]", handle.member_id, to_send.len());
                if let Err(e) = handle.pass_message(to_send.clone()) {
                    error!("Failed to pass message to node {} handler: {:?}", handle.member_id, e);
                    failures.push(handle.member_id);
                }
            }
        }

        if failures.is_empty() {
            Ok(msg)
        } else {
            Err(MulticastError::BroadcastError(failures))
        }
    }

    pub fn remove_member(&mut self, member_id: &NodeId) {
        self.group.remove(member_id);
        self.active_members.remove(member_id);
    }

    pub fn members(&self) -> &HashSet<NodeId> {
        &self.active_members
    }

    pub(crate) async fn raw_deliver(&mut self) -> Option<MemberStateMessage<M>> {
        self.from_members.recv().await
    }
}

#[async_trait]
impl<M> Multicast<M> for BasicMulticast<M> where M: Serialize {
    fn connect(_: usize, _: Config) -> Self { todo!() }

    fn connect_timeout(_: usize, _: Config, _: u64) -> Self { todo!() }

    fn broadcast(&mut self, msg: M) -> Result<(), MulticastError> { 
        let to_send = bincode::serialize(&msg).unwrap();
        let mut failures = Vec::new();

        for handle in self.group.values() {
            trace!("Sending message to {}: [...{} bytes...]", handle.member_id, to_send.len());
            if let Err(e) = handle.pass_message(to_send.clone()) {
                error!("Failed to pass message to node {} handler: {:?}", handle.member_id, e);
                failures.push(handle.member_id);
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(MulticastError::BroadcastError(failures))
        }
    }

    fn send_to(&mut self, msg: M, recipient: NodeId) -> Result<(), MulticastError> { 
        if let Some(handle) = self.group.get(&recipient) {
            let to_send = bincode::serialize(&msg).unwrap();
            trace!("Sending message to {}: [...{} bytes...]", handle.member_id, to_send.len());

            if let Err(e) = handle.pass_message(to_send) {
                error!("Failed to pass message to node {} handler: {:?}", handle.member_id, e);
                return Err(MulticastError::ClientDisconnected(recipient));
            }

            Ok(())
        } else {
            Err(MulticastError::InvalidRecipient(recipient))
        }
    }

    async fn deliver(&mut self) -> Result<M, MulticastError> where M: Send { 
        use super::member::MemberStateMessageType::*;
        use MulticastError::*;

        match self.from_members.recv().await {
            Some(state) => match state.msg {
                Message(msg) => Ok(msg), 
                NetworkError => Err(ClientDisconnected(state.member_id))
            },
            None => Err(AllClientsDisconnected)
        }
    }
}