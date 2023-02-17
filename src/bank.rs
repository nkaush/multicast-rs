use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use crate::message::FromMulticast;

struct Bank {
    rcv: UnboundedReceiver<FromMulticast>
}

impl Bank {
    pub fn new() -> (Self, UnboundedSender<FromMulticast>) {
        todo!()
    }
}