

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use super::{ClientStateMessage, MulticastMemberData, MulticastMemberHandle, client::client_loop};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufStream};
use tokio_retry::{Retry, strategy::FixedInterval};
use tokio::net::{TcpStream, TcpListener};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::select;
use crate::Config;

pub struct ConnectionPool {
    group: HashMap<String, MulticastMemberHandle>,
    node_id: String,
    from_clients: UnboundedReceiver<ClientStateMessage>,
    client_snd_handle: UnboundedSender<ClientStateMessage>
}

static CONNECTION_RETRY_ATTEMPS: usize = 200;
static CONNECTION_RETRY_DELAY_MS: u64 = 100;

impl ConnectionPool {
    pub(in crate::multicast) fn new(node_id: String) -> Self {
        let (client_snd_handle, from_clients) = unbounded_channel();

        Self {
            group: Default::default(),
            node_id,
            from_clients,
            client_snd_handle
        }
    }

    pub(in crate::multicast) fn take_resources(self) -> (HashMap<String, MulticastMemberHandle>, UnboundedReceiver<ClientStateMessage>) {
        (self.group, self.from_clients)
    }

    async fn connect_to_node(this_node: String, node_id: String, host: String, port: u16, stream_snd: UnboundedSender<(TcpStream, String)>) {
        let server_addr = format!("{host}:{port}");
        eprintln!("Connecting to {} at {}...", node_id, server_addr);

        let retry_strategy = FixedInterval::from_millis(CONNECTION_RETRY_DELAY_MS)
            .take(CONNECTION_RETRY_ATTEMPS); // limit to 100 retries

        match Retry::spawn(retry_strategy, || TcpStream::connect(&server_addr)).await {
            Ok(mut stream) => {
                eprintln!("Connected to {} at {}", node_id, server_addr);

                stream.write_all(format!("{}\n", this_node).as_bytes()).await.unwrap();
                stream.flush().await.unwrap();

                stream_snd.send((stream, node_id)).unwrap();
            },
            Err(e) => {
                eprintln!("Failed to connect to {}: {:?}", server_addr, e);
                std::process::exit(1);
            }
        }
    }

    fn admit_member(&mut self, socket: TcpStream, member_id: String) {
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

    pub(in crate::multicast) async fn connect(&mut self, config: &Config) {
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
            tokio::spawn(Self::connect_to_node(tnc, node.to_string(), host, port, snd_clone));
        }
        drop(stream_snd);
        
        loop {
            select! {
                client = tcp_listener.accept() => match client {
                    Ok((stream, _addr)) => { // TODO maybe we need to do more here
                        let mut stream = BufStream::new(stream);
                        let mut member_id = String::new();
                        match stream.read_line(&mut member_id).await {
                            Ok(0) | Err(_) => continue,
                            Ok(_) => self.admit_member(stream.into_inner(), member_id.trim().into())
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
    }
}