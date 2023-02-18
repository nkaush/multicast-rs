use crate::message::{PriorityMessageType, PriorityRequestType, UserInput, FromMulticast};
use tokio::{sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel}, select};
use std::collections::BinaryHeap;

use tokio::net::TcpListener;
use std::net::SocketAddr;

use crate::Config;

pub struct Multicast {
    rcv: UnboundedReceiver<UserInput>,
    pq: BinaryHeap<PriorityMessageType>,
    buf: Vec<PriorityRequestType>,
    next_local_id: usize,
    next_priority_proposal: usize,
    bank_snd: UnboundedSender<FromMulticast>
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

    pub async fn main_loop(port: u16, config: Config, nodes_to_connect_with: Vec<String>) {
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

        loop {
            // select! {
            //     _ = async move => ()
            // }
        }
    }
}