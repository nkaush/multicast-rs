use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use crate::message::ToMulticast;

struct Multicast {
    rcv: UnboundedReceiver<ToMulticast>
}

impl Multicast {
    pub fn new() -> (Self, UnboundedSender<ToMulticast>) {
        todo!()
    }
}