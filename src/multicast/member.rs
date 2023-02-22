use super::{MemberStateMessageType, MulticastMemberData};
use tokio_util::codec::LengthDelimitedCodec;
use futures::{stream::StreamExt, SinkExt};
use tokio::{net::TcpStream, select};

pub(super) async fn member_loop(socket: TcpStream, mut member_data: MulticastMemberData) {
    let (read, write) = socket.into_split();
    let mut write = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_write(write);

    let mut read = LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .new_read(read);

    loop {
        select! {
            Some(to_send) = member_data.from_engine.recv() => {
                let bytes = bincode::serialize(&to_send).unwrap();
                if write.send(bytes.into()).await.is_err() {
                    member_data.notify_network_error().unwrap();
                }
            },
            Some(received) = read.next() => {
                match received {
                    Ok(bytes) => {
                        let msg = match bincode::deserialize(&bytes) {
                            Ok(m) => MemberStateMessageType::Message(m),
                            Err(e) => {
                                eprintln!("deserialize error on client handler {}: {:?}", member_data.member_id, e);
                                continue
                            }
                        };
                        member_data.notify_client_message(msg).unwrap();
                    },
                    Err(_) => {
                        member_data.notify_network_error().unwrap();
                    }
                }
            }
        }
    }
}