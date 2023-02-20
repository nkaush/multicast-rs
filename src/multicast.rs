use crate::{message::{NetworkMessage, PriorityMessageType, PriorityRequestType, UserInput}, write_to_socket, read_from_socket};
use bytes::Bytes;
use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel},
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream, tcp::{ReadHalf, WriteHalf}},
    task::JoinHandle, select
};
use tokio_util::codec::{FramedRead, Framed, LengthDelimitedCodec, FramedWrite};
use tokio_retry::{Retry, strategy::FixedInterval};
use tokio::io::{AsyncRead, AsyncWrite};
use std::collections::{HashMap, BinaryHeap};
use std::net::SocketAddr;
use crate::Config;
use tokio_util::codec::Decoder;
use futures::{sink::SinkExt, StreamExt};

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
    
    /// Send handle to clone and give to client handling threads to send messages to multicast engine
    client_snd_handle: UnboundedSender<ClientStateMessage>
}

async fn client_loop(member_data: MulticastMemberData) {
    loop {

    }
}

impl Multicast {
    pub fn new(node_id: String, bank_snd: UnboundedSender<UserInput>) -> (Self, UnboundedSender<UserInput>) {
        let (to_multicast, from_cli) = unbounded_channel();
        let (client_snd_handle, from_clients) = unbounded_channel();

        let this = Self {
            node_id,
            group: HashMap::new(),
            from_cli,
            pq: BinaryHeap::new(),
            buf: Vec::new(),
            next_local_id: 0,
            next_priority_proposal: 0,
            to_bank: bank_snd,
            from_clients,
            client_snd_handle
        };

        (this, to_multicast)
    }

    pub fn admit_member(&mut self, socket: TcpStream, member_id: String) {
        eprintln!("{} joined the group!", member_id);

        let (to_client, from_engine) = unbounded_channel();
        let member_data = MulticastMemberData {
            member_id: member_id.clone(),
            socket,
            to_engine: self.client_snd_handle.clone(),
            from_engine: from_engine
        };

        let handle = tokio::spawn(client_loop(member_data));
        self.group.insert(member_id.clone(), MulticastMemberHandle { 
            member_id,
            to_client,
            handle
        });
    }

    async fn connect_to_node(this_node: String, node_id: String, host: String, port: u16, stream_snd: UnboundedSender<(TcpStream, String)>) {
        let server_addr = format!("{host}:{port}");
        eprintln!("Connecting to {} at {}...", node_id, server_addr);

        let retry_strategy = FixedInterval::from_millis(100).take(100); // limit to 100 retries

        match Retry::spawn(retry_strategy, || TcpStream::connect(&server_addr)).await {
            Ok(mut socket) => {
                eprintln!("Connected to {} at {}", node_id, server_addr);
                // let mut stream = BufStream::new(stream);
        
                let (_, write_half) = socket.split();
                let mut framed_write = LengthDelimitedCodec::builder()
                    .length_field_type::<u32>()
                    .new_write(write_half);

                let name_msg = NetworkMessage::NameMessage(this_node);
                let to_send = Bytes::from(bincode::serialize(&name_msg).unwrap());
                framed_write.send(to_send).await.unwrap();

                stream_snd.send((socket, node_id)).unwrap();
            },
            Err(e) => {
                eprintln!("Failed to connect to {}: {:?}", server_addr, e);
                std::process::exit(1);
            }
        }
    }

    pub async fn initiate_connection_pool(&mut self, config: &Config) {
        let (_, port, to_connect_with) = config.get(&self.node_id).unwrap();

        let bind_addr: SocketAddr = ([0, 0, 0, 0], *port).into();
        let tcp_listener = match TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind to {}: {:?}", bind_addr, e);
                std::process::exit(1);
            }
        };

        let (stream_snd, mut stream_rcv) = unbounded_channel();

        for node in to_connect_with.into_iter() {
            let (host, port, _) = config.get(node).cloned().unwrap();
            let snd_clone = stream_snd.clone();
            let tnc = self.node_id.clone();
            tokio::spawn(Multicast::connect_to_node(tnc, node.to_string(), host, port, snd_clone));
        }
        drop(stream_snd);
        
        loop {
            select! {
                client = tcp_listener.accept() => match client {
                    Ok((mut socket, _addr)) => { // TODO maybe we need to do more here
                        let (read_half, _) = socket.split();
                        let mut framed_read = LengthDelimitedCodec::builder()
                            .length_field_type::<u32>()
                            .new_read(read_half);
                        eprintln!("got a client");
                        if let Some(msg) = framed_read.next().await {
                            println!("got frame");
                            match bincode::deserialize::<NetworkMessage>(&msg.unwrap()).unwrap() {
                                NetworkMessage::NameMessage(member_id) => self.admit_member(socket, member_id),
                                _ => ()
                            }
                        }

                        if self.group.len() == config.len() - 1 { break; }
                    },
                    Err(e) => {
                        eprintln!("Could not accept client: {:?}", e);
                        continue
                    }
                },
                Some((stream, member_id)) = stream_rcv.recv() => { // TODO maybe we need to do more here
                    self.admit_member(stream, member_id);
                    if self.group.len() == config.len() - 1 { break; }
                }
            }
        }

        eprintln!("done connecting!")
    }

    pub async fn main_loop(&mut self, config: &Config) { 
        self.initiate_connection_pool(config).await;

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