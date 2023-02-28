use super::{
    member::{member_loop, MulticastMemberData}, Config, NodeId,
    MulticastMemberHandle, MemberStateMessage, MulticastGroup
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    io::{AsyncWriteExt, AsyncBufReadExt, BufStream},
    net::{TcpStream, TcpListener}, select
};
use tokio_retry::{Retry, strategy::FixedInterval};
use serde::{Serialize, de::DeserializeOwned};
use std::{net::SocketAddr, fmt};
use log::{trace, error};

pub(super) struct ConnectionPool<M> {
    pub group: MulticastGroup<M>,
    pub node_id: NodeId,
    pub from_members: UnboundedReceiver<MemberStateMessage<M>>,
    pub client_snd_handle: UnboundedSender<MemberStateMessage<M>>
}

static CONNECTION_RETRY_ATTEMPS: usize = 600;
static CONNECTION_RETRY_DELAY_MS: u64 = 100;

impl<M> ConnectionPool<M> {
    pub(super) fn new(node_id: NodeId) -> Self {
        let (client_snd_handle, from_clients) = unbounded_channel();

        Self {
            group: Default::default(),
            node_id,
            from_members: from_clients,
            client_snd_handle
        }
    }

    async fn connect_to_node(this_node: NodeId, node_id: NodeId, host: String, port: u16, stream_snd: UnboundedSender<(TcpStream, NodeId)>) {
        let server_addr = format!("{host}:{port}");
        trace!("Connecting to {} at {}...", node_id, server_addr);

        let retry_strategy = FixedInterval::from_millis(CONNECTION_RETRY_DELAY_MS)
            .take(CONNECTION_RETRY_ATTEMPS);

        match Retry::spawn(retry_strategy, || TcpStream::connect(&server_addr)).await {
            Ok(mut stream) => {
                trace!("Connected to {} at {}", node_id, server_addr);

                stream.write_all(format!("{}\n", this_node).as_bytes()).await.unwrap();
                stream.flush().await.unwrap();

                stream_snd.send((stream, node_id)).unwrap();
            },
            Err(e) => {
                error!("Failed to connect to {}: {:?}", server_addr, e);
                std::process::exit(1);
            }
        }
    }

    fn admit_member(&mut self, socket: TcpStream, member_id: NodeId) where M: 'static + Send + Serialize + DeserializeOwned + fmt::Debug {
        let (to_client, from_engine) = unbounded_channel();
        let member_data = MulticastMemberData {
            member_id: member_id,
            to_engine: self.client_snd_handle.clone(),
            from_engine: from_engine
        };

        let handle = tokio::spawn(member_loop(socket, member_data));
        self.group.insert(member_id, MulticastMemberHandle { 
            member_id,
            to_client,
            handle
        });
    }

    pub(super) async fn connect(mut self, config: &Config) -> Self where M: 'static + Send + Serialize + DeserializeOwned + fmt::Debug {
        let node_config = config.get(&self.node_id).unwrap();

        let bind_addr: SocketAddr = ([0, 0, 0, 0], node_config.port).into();
        let tcp_listener = match TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to bind to {}: {:?}", bind_addr, e);
                std::process::exit(1);
            }
        };

        let (stream_snd, mut stream_rcv) = unbounded_channel();

        for node in node_config.connection_list.iter() {
            let connect_config = config.get(&node).unwrap();
            let snd_clone = stream_snd.clone();
            tokio::spawn(Self::connect_to_node(
                self.node_id, 
                *node, 
                connect_config.hostname.clone(), 
                connect_config.port, 
                snd_clone
            ));
        }
        drop(stream_snd);
        
        loop {
            select! {
                client = tcp_listener.accept() => match client {
                    Ok((stream, _addr)) => {
                        let mut stream = BufStream::new(stream);
                        let mut member_id = String::new();
                        match stream.read_line(&mut member_id).await {
                            Ok(0) | Err(_) => continue,
                            Ok(_) => {
                                let member_id: NodeId = member_id
                                    .trim()
                                    .parse()
                                    .unwrap();
                                self.admit_member(stream.into_inner(), member_id)
                            }
                        }

                        if self.group.len() == config.len() - 1 { break self; }
                    },
                    Err(e) => {
                        error!("Could not accept client: {:?}", e);
                        continue
                    }
                },
                Some((stream, member_id)) = stream_rcv.recv() => {
                    self.admit_member(stream, member_id);
                    if self.group.len() == config.len() - 1 { break self; }
                }
            }
        } 
    }
}