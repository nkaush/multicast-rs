mod connection_pool;
mod client;

use crate::message::{NetworkMessage, PriorityMessageType, PriorityRequestType, UserInput};
use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel},
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
    task::JoinHandle, select
};
use std::collections::{HashMap, BinaryHeap};
use crate::Config;

use connection_pool::ConnectionPool;

/// Represents any message types a member handler thread could send the multicast engine
enum ClientStateMessageType {
    Message(NetworkMessage),
    NetworkError
}

/// Represents any messages a member handler thread could send the multicast engine
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
            to_bank: bank_snd,
            from_clients
        };

        (this, to_multicast)
    }

    pub async fn main_loop(&mut self) { 
        loop {
            select! {
                input = self.from_cli.recv() => match input {
                    Some(msg) => {
                        // for member in self.group.iter_mut() {
                        //     let serialized = bincode::serialize(&msg).unwrap();
                        //     let len = serialized.len() as u64;
                        //     member.stream.write_u64_le(len).await.unwrap();
                        //     // member.stream.
                        // }
                    },
                    None => break
                }
            }
        }
    }
}