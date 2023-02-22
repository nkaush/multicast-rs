mod connection_pool;
mod member;

use crate::message::{NetworkMessage, PriorityMessageType, PriorityRequestType, UserInput};
use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel},
    sync::mpsc::error::SendError, task::JoinHandle, select
};
use std::collections::{HashMap, BinaryHeap};
use crate::Config;

use connection_pool::ConnectionPool;

/// Represents any message types a member handler thread could send the multicast engine
#[derive(Debug)]
enum MemberStateMessageType {
    Message(NetworkMessage),
    NetworkError
}

/// Represents any messages a member handler thread could send the multicast engine.
#[derive(Debug)]
struct MemberStateMessage {
    msg: MemberStateMessageType,
    member_id: String
}

/// The handle that the multicast engine has for each member handler thread.
struct MulticastMemberHandle {
    member_id: String,
    to_client: UnboundedSender<NetworkMessage>,
    handle: JoinHandle<()>
}

impl MulticastMemberHandle {
    fn pass_message(&self, msg: NetworkMessage) -> Result<(), SendError<NetworkMessage>> {
        self.to_client.send(msg)
    }

    fn abort(&self) {
        self.handle.abort()
    }
}

impl Drop for MulticastMemberHandle {
    fn drop(&mut self) {
        eprintln!("Aborting client thread for {}", self.member_id);
        self.abort()
    }
}

struct MulticastMemberData {
    member_id: String,
    to_engine: UnboundedSender<MemberStateMessage>,
    from_engine: UnboundedReceiver<NetworkMessage>
}

impl MulticastMemberData {
    pub fn generate_state_msg(&self, msg: MemberStateMessageType) -> MemberStateMessage {
        MemberStateMessage {
            msg,
            member_id: self.member_id.clone()
        }
    }
    
    fn notify_client_message(&mut self, msg: MemberStateMessageType) -> Result<(), SendError<MemberStateMessage>> {
        self.to_engine.send(self.generate_state_msg(msg))
    }

    fn notify_network_error(&mut self) -> Result<(), SendError<MemberStateMessage>> {
        self.to_engine.send(self.generate_state_msg(MemberStateMessageType::NetworkError))
    }
}

pub struct Multicast {
    node_id: String,
    
    pq: BinaryHeap<PriorityMessageType>,
    buf: Vec<PriorityRequestType>,
    next_local_id: usize,
    next_priority_proposal: usize,

    local_messages: HashMap<usize, UserInput>,

    /// Stores all of the multicast group member thread handles
    group: HashMap<String, MulticastMemberHandle>,

    /// Receive handle to take input from the CLI loop
    from_cli: UnboundedReceiver<UserInput>,

    /// Send handle to pass messages to the bank logic thread
    to_bank: UnboundedSender<UserInput>,

    /// Receive handle for client handling threads to send multicast engine any messages
    from_clients: UnboundedReceiver<MemberStateMessage>,
}

impl Multicast {
    pub async fn new(node_id: String, config: &Config, bank_snd: UnboundedSender<UserInput>) -> (Self, UnboundedSender<UserInput>) {
        let (to_multicast, from_cli) = unbounded_channel();

        let (group, from_clients) = ConnectionPool::new(node_id.clone())
            .connect(config)
            .await
            .consume();
        eprintln!("done connecting!");

        let this = Self {
            node_id,
            group,
            from_cli,
            pq: BinaryHeap::new(),
            buf: Vec::new(),
            next_local_id: 0,
            next_priority_proposal: 0,
            local_messages: HashMap::new(),
            to_bank: bank_snd,
            from_clients
        };

        (this, to_multicast)
    }

    fn get_local_id(&mut self) -> usize {
        let id = self.next_local_id;
        self.next_local_id += 1;
        id
    }

    /// We got some input from the CLI, now we want to request a priority for it.
    fn request_priority(&mut self, msg: UserInput) {
        todo!();
    }

    /// We got a request from another process for priority, so propose a priority.
    fn propose_priority(&mut self) {
        todo!();
    }

    /// We got all the priorities for our message back, so send out the 
    /// confirmed priority along with the message.
    fn confirm_message_priority(&mut self) {
        todo!();
    }

    pub async fn main_loop(&mut self) { 
        loop {
            select! {
                input = self.from_cli.recv() => match input {
                    Some(msg) => {
                        self.request_priority(msg)
                    },
                    None => {
                        println!("from_cli channel closed");
                        break
                    }
                },
                Some(msg) = self.from_clients.recv() => match msg.msg {
                    MemberStateMessageType::Message(net_msg) => {
                        eprintln!("Got network message from {}: {:?}", msg.member_id, net_msg);
                        match net_msg {
                            NetworkMessage::PriorityRequest(_) => todo!(),
                            NetworkMessage::PriorityProposal(_) => todo!(),
                            NetworkMessage::PriorityMessage(_) => todo!()
                        }
                    },
                    MemberStateMessageType::NetworkError => {
                        // remove the client from the group if it encounters a network error
                        // TODO: do we need to notify other clients that this client has died?
                        self.group.remove(&msg.member_id);
                    }
                }
            }
        }
    }
}