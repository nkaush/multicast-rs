use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use crate::message::FromMulticast;
use std::collections::BTreeMap;

struct Bank {
    rcv: UnboundedReceiver<FromMulticast>,
    accounts: BTreeMap<String, usize>
}

impl Bank {
    pub fn new() -> (Self, UnboundedSender<FromMulticast>) {
        let (snd, rcv) =  unbounded_channel();
        Self {
            rcv,
            accounts: BTreeMap::new()
        }
    }
}