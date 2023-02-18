use crate::message::{ToMulticast, PriorityMessageType, PriorityRequestType};
use tokio::{sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel}, select};
use std::collections::BinaryHeap;

use tokio::net::TcpListener;
use std::net::SocketAddr;

use crate::Config;

struct Multicast {
    rcv: UnboundedReceiver<ToMulticast>,
    pq: BinaryHeap<PriorityMessageType>,
    buf: Vec<PriorityRequestType>,
    next_local_id: usize,
    next_priority_proposal: usize
}

impl Multicast {
    pub async fn new(port: u16, config: Config, nodes_to_connect_with: Vec<String>) -> (Self, UnboundedSender<ToMulticast>) {
        let bind_addr: SocketAddr = ([0, 0, 0, 0], port).into();
        let tcp_listener = match TcpListener::bind(bind_addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind to {}: {:?}", bind_addr, e);
                std::process::exit(1);
            }
        };

        let (snd, rcv) = unbounded_channel();
        let this = Multicast {
            rcv,
            pq: BinaryHeap::new(),
            buf: Vec::new(),
            next_local_id: 0,
            next_priority_proposal: 0
        };

        (this, snd)
    }

    pub fn main_loop() {
        loop {
            // select! {
            //     _ = async move => ()
            // }
        }
    }
}