use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, error::SendError}, 
    task::JoinHandle, net::TcpStream, select
};
use tokio_util::codec::LengthDelimitedCodec;
use futures::{stream::StreamExt, SinkExt};
use super::{NetworkMessage, NodeId};
use log::{trace, error};

/// Represents any message types a member handler thread could send the multicast engine
#[derive(Debug)]
pub(super) enum MemberStateMessageType {
    Message(NetworkMessage),
    DuplicateMessage,
    NetworkError
}

/// Represents any messages a member handler thread could send the multicast engine.
#[derive(Debug)]
pub(super) struct MemberStateMessage {
    pub msg: MemberStateMessageType,
    pub member_id: NodeId
}

/// The handle that the multicast engine has for each member handler thread.
pub(super) struct MulticastMemberHandle {
    pub member_id: NodeId,
    pub to_client: UnboundedSender<NetworkMessage>,
    pub handle: JoinHandle<()>
}

impl MulticastMemberHandle {
    pub fn pass_message(&self, msg: NetworkMessage) -> Result<(), SendError<NetworkMessage>> {
        self.to_client.send(msg)
    }
}

impl Drop for MulticastMemberHandle {
    fn drop(&mut self) {
        trace!("Aborting client thread for {}", self.member_id);
        self.handle.abort()
    }
}

pub(super) struct MulticastMemberData {
    pub member_id: NodeId,
    pub to_engine: UnboundedSender<MemberStateMessage>,
    pub from_engine: UnboundedReceiver<NetworkMessage>
}

impl MulticastMemberData {
    fn generate_state_msg(&self, msg: MemberStateMessageType) -> MemberStateMessage {
        MemberStateMessage {
            msg,
            member_id: self.member_id
        }
    }
    
    fn notify_client_message(&mut self, msg: MemberStateMessageType) -> Result<(), SendError<MemberStateMessage>> {
        self.to_engine.send(self.generate_state_msg(msg))
    }

    fn notify_network_error(&mut self) -> Result<(), SendError<MemberStateMessage>> {
        self.to_engine.send(self.generate_state_msg(MemberStateMessageType::NetworkError))
    }
}

pub(super) async fn member_loop(socket: TcpStream, mut member_data: MulticastMemberData) {
    let mut stream = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_framed(socket);

    loop {
        select! {
            Some(to_send) = member_data.from_engine.recv() => {
                let bytes = bincode::serialize(&to_send).unwrap();
                if stream.send(bytes.into()).await.is_err() {
                    member_data.notify_network_error().unwrap();
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
                    member_data.notify_client_message(msg).unwrap();
                },
                _ => member_data.notify_network_error().unwrap()
            }
        }
    }
}