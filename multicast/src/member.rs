use super::config::NodeId;
use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, error::SendError}, 
    task::JoinHandle, net::TcpStream, select
};
use serde::{de::DeserializeOwned, Serialize};
use tokio_util::codec::LengthDelimitedCodec;
use futures::{stream::StreamExt, SinkExt};
use log::{trace, error};

/// Represents any message types a member handler thread could send the multicast engine
#[derive(Debug)]
pub enum MemberStateMessageType<M> {
    Message(M),
    NetworkError
}

/// Represents any messages a member handler thread could send the multicast engine.
#[derive(Debug)]
pub struct MemberStateMessage<M> {
    pub msg: MemberStateMessageType<M>,
    pub member_id: NodeId
}

/// The handle that the multicast engine has for each member handler thread.
pub(super) struct MulticastMemberHandle {
    pub member_id: NodeId,
    pub to_client: UnboundedSender<Vec<u8>>,
    pub handle: JoinHandle<()>
}

impl MulticastMemberHandle {
    pub fn pass_message(&self, msg: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        self.to_client.send(msg)
    }
}

impl Drop for MulticastMemberHandle {
    fn drop(&mut self) {
        trace!("Aborting client thread for {}", self.member_id);
        self.handle.abort()
    }
}

pub(super) struct MulticastMemberData<M> {
    pub member_id: NodeId,
    pub to_engine: UnboundedSender<MemberStateMessage<M>>,
    pub from_engine: UnboundedReceiver<Vec<u8>>
}

impl<M> MulticastMemberData<M> {
    fn generate_state_msg(&self, msg: MemberStateMessageType<M>) -> MemberStateMessage<M> {
        MemberStateMessage {
            msg,
            member_id: self.member_id
        }
    }
    
    fn notify_client_message(&mut self, msg: MemberStateMessageType<M>) -> Result<(), SendError<MemberStateMessage<M>>> {
        self.to_engine.send(self.generate_state_msg(msg))
    }

    fn notify_network_error(&mut self) -> Result<(), SendError<MemberStateMessage<M>>> {
        self.to_engine.send(self.generate_state_msg(MemberStateMessageType::NetworkError))
    }
}

pub(super) async fn member_loop<M>(socket: TcpStream, mut member_data: MulticastMemberData<M>) where M: 'static + DeserializeOwned + Serialize {
    let mut stream = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_framed(socket);

    loop {
        select! {
            Some(to_send) = member_data.from_engine.recv() => {
                let bytes = bincode::serialize(&to_send).unwrap();
                if stream.send(bytes.into()).await.is_err() {
                    member_data.notify_network_error();
                    break;
                }
            },
            received = stream.next() => match received {
                Some(Ok(bytes)) => {
                    let msg = match bincode::deserialize(&bytes) {
                        Ok(m) => MemberStateMessageType::Message(m),
                        Err(e) => {
                            error!("deserialize error on client handler {}: {:?}", member_data.member_id, e);
                            continue
                        }
                    };
                    member_data.notify_client_message(msg);
                },
                _ => {
                    member_data.notify_network_error();
                    break;
                }
            }
        }
    }
}