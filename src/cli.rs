use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use crate::message::FromMulticast;
use std::collections::BTreeMap;

struct Cli {
    
    
}

impl Cli {
    pub fn new() -> (Self, UnboundedSender<FromMulticast>) {
        let (snd, rcv) =  unbounded_channel();
        Self {
            rcv,
        }
    }
}