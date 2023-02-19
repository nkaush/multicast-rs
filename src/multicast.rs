use crate::message::{PriorityMessageType, PriorityRequestType, UserInput, FromMulticast};
use tokio::{sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel}, select, net::TcpStream, io::AsyncWriteExt};
use tokio::io::{self, AsyncBufReadExt, BufStream};
use std::collections::BinaryHeap;
use std::time::Duration;

use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::Config;

pub struct Multicast {
    rcv: UnboundedReceiver<UserInput>,
    pq: BinaryHeap<PriorityMessageType>,
    buf: Vec<PriorityRequestType>,
    next_local_id: usize,
    next_priority_proposal: usize,
    bank_snd: UnboundedSender<FromMulticast>
}

struct MulticastMember {
    stream: BufStream<TcpStream>,
    member_id: String
}

#[derive(Default)]
struct MulticastGroup {
    members: Vec<MulticastMember>
}

impl MulticastGroup {
    fn admit_member(&mut self, stream: BufStream<TcpStream>, member_id: String) {
        eprintln!("{} joined the group!", member_id);
        self.members.push(MulticastMember { stream, member_id });
    }

    fn len(&self) -> usize {
        self.members.len()
    }
}

impl Multicast {
    pub fn new(bank_snd: UnboundedSender<FromMulticast>) -> (Self, UnboundedSender<UserInput>) {
        let (snd, rcv) = unbounded_channel();
        let this = Self {
            rcv,
            pq: BinaryHeap::new(),
            buf: Vec::new(),
            next_local_id: 0,
            next_priority_proposal: 0,
            bank_snd
        };

        (this, snd)
    }

    async fn retry_connect(ipport: &str) -> io::Result<TcpStream> {
        loop{
            match TcpStream::connect(ipport).await {
                Ok(stream) => return Ok(stream),
                Err(_) => continue,
            }
        }
    }

    async fn connect_to_node(this_node: String, node_id: String, host: String, port: u16, stream_snd: UnboundedSender<(BufStream<TcpStream>, String)>) {
        let server_addr = format!("{host}:{port}");
        let timeout = Duration::new(20, 0);
        eprintln!("Connecting to {} at {}...", node_id, server_addr);

        match tokio::time::timeout(timeout, Multicast::retry_connect(&server_addr)).await {
            Ok(s) => {
                let mut stream = BufStream::new(s.unwrap());
                eprintln!("Connected to {} at {}!", node_id, server_addr);

                stream.write_all(format!("{}\n", this_node).as_bytes()).await.unwrap();
                stream_snd.send((stream, node_id)).unwrap();
            },
            Err(e) => {
                eprintln!("Failed to connect to {}: {:?}", server_addr, e);
                std::process::exit(1);
            }
        }
    }

    pub async fn main_loop(&self, this_node: String, port: u16, config: &Config, nodes_to_connect_with: &Vec<String>) {
        let bind_addr: SocketAddr = ([0, 0, 0, 0], port).into();
        let tcp_listener = match TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind to {}: {:?}", bind_addr, e);
                std::process::exit(1);
            }
        };

        eprintln!("Listening on {:?}...", tcp_listener.local_addr().unwrap());
        eprintln!("Cancel this process with CRTL+C");

        let (stream_snd, mut stream_rcv) = unbounded_channel();
        let mut group = MulticastGroup::default();

        for node in nodes_to_connect_with.into_iter() {
            let (host, port, _) = config.get(node).cloned().unwrap();
            let snd_clone = stream_snd.clone();
            let tnc = this_node.clone();
            tokio::spawn(Multicast::connect_to_node(tnc, node.to_string(), host, port, snd_clone));
        }
        drop(stream_snd);
        
        loop {
            select! {
                client = tcp_listener.accept() => match client {
                    Ok((stream, _addr)) => {
                        // TODO maybe we need to do more here
                        let mut stream = BufStream::new(stream);
                        let mut member_id = String::new();
                        match stream.read_line(&mut member_id).await {
                            Ok(0) | Err(_) => continue,
                            Ok(_) => {
                                println!("server got {:?}", member_id);
                                group.admit_member(stream, member_id)
                            }
                        }

                        if group.len() == config.len() { break; }
                    },
                    Err(e) => {
                        println!("Could not accept client: {:?}", e);
                        continue
                    }
                },
                Some((stream, member_id)) = stream_rcv.recv() => {
                    // TODO maybe we need to do more here
                    group.admit_member(stream, member_id);

                    if group.len() == config.len() { break; }
                }
            }
        }

        println!("done connecting!")
    }
}