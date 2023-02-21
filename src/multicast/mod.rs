mod connection_pool;
mod client;
use crate::message::{NetworkMessage, PriorityMessageType, PriorityRequestType, UserInput};
use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel},
    task::JoinHandle, net::TcpStream, select
};
use std::collections::{HashMap, BinaryHeap};
use crate::Config;

use connection_pool::ConnectionPool;

/// Represents any message types a member handler thread could send the multicast engine
#[derive(Debug)]
enum ClientStateMessageType {
    Message(NetworkMessage),
    NetworkError
}

/// Represents any messages a member handler thread could send the multicast engine
#[derive(Debug)]
struct ClientStateMessage {
    msg: ClientStateMessageType,
    member_id: String
}

struct MulticastMemberHandle {
    member_id: String,
    to_client: UnboundedSender<NetworkMessage>,
    handle: JoinHandle<()>
}

struct MulticastMemberData {
    member_id: String,
    socket: TcpStream, 
    to_engine: UnboundedSender<ClientStateMessage>,
    from_engine: UnboundedReceiver<NetworkMessage>
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
    from_clients: UnboundedReceiver<ClientStateMessage>,
}

impl Multicast {
    pub async fn new(node_id: String, config: &Config, bank_snd: UnboundedSender<UserInput>) -> (Self, UnboundedSender<UserInput>) {
        let (to_multicast, from_cli) = unbounded_channel();

        let mut pool = ConnectionPool::new(node_id.clone());
        pool.connect(config).await;
        eprintln!("done connecting!");
        let (group, from_clients) = pool.take_resources();

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

    /// We got some input from the CLI, now we want to request a priority for it
    fn request_priority(&mut self, msg: UserInput) {
        let id = self.get_local_id();
        self.local_messages.insert(id, msg);
        
        let request = NetworkMessage::PriorityRequest(PriorityRequestType {
            sender: self.node_id.clone(),
            local_id: id
        });

        for client_handle in self.group.values() {
            /// @todo maybe we need to handle errors here
            client_handle.to_client.send(request.clone());
        }
    }

    pub async fn main_loop(&mut self) { 
        loop {
            select! {
                input = self.from_cli.recv() => match input {
                    Some(msg) => self.request_priority(msg),
                    None => break
                },
                Some(msg) = self.from_clients.recv() => match msg.msg {
                    ClientStateMessageType::Message(m) => {
                        eprintln!("Got network message from {}: {:?}", msg.member_id, m)
                    },
                    ClientStateMessageType::NetworkError => {
                        eprintln!("Got network error for {}", msg.member_id)
                    }
                }
            }
        }
    }
}