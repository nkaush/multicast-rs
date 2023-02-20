use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use crate::message::UserInput;
use std::collections::BTreeMap;

pub struct Bank {
    rcv: UnboundedReceiver<UserInput>,
    accounts: BTreeMap<String, usize>
}

impl Bank {
    pub fn new() -> (Self, UnboundedSender<UserInput>) {
        let (snd, rcv) = unbounded_channel();
        let this = Self {
            rcv,
            accounts: BTreeMap::new()
        };

        (this, snd)
    }
}