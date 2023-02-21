use crate::NetworkMessage;

use super::{ClientStateMessage, ClientStateMessageType};
use tokio_util::codec::LengthDelimitedCodec;
use futures::{stream::StreamExt, SinkExt};

use super::MulticastMemberData;
use tokio::select;
use bytes::Bytes;

pub(in crate::multicast) async fn client_loop(mut member_data: MulticastMemberData) {
    let (read, write) = member_data.socket.into_split();

    let mut write = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_write(write);

    let mut read = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_read(read);

    loop {
        select! {
            Some(to_send) = member_data.from_engine.recv() => {
                let bytes = Bytes::from(bincode::serialize(&to_send).unwrap());
                
                if write.send(bytes).await.is_err() {
                    member_data.to_engine.send(ClientStateMessage {
                        msg: ClientStateMessageType::NetworkError,
                        member_id: member_data.member_id.clone()
                    }).unwrap();
                }
            },
            Some(received) = read.next() => {
                match received {
                    Ok(bytes) => {
                        let msg = match bincode::deserialize(&bytes) {
                            Ok(m) => ClientStateMessageType::Message(m),
                            Err(e) => {
                                eprintln!("deserialize error on client handler {}: {:?}", member_data.member_id, e);
                                continue
                            }
                        };
                        member_data.to_engine.send(ClientStateMessage {
                            msg,
                            member_id: member_data.member_id.clone()
                        }).unwrap();
                    },
                    Err(_) => {
                        member_data.to_engine.send(ClientStateMessage {
                            msg: ClientStateMessageType::NetworkError,
                            member_id: member_data.member_id.clone()
                        }).unwrap();
                    }
                }
            }
        }
    }
}