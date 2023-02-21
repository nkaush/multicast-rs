use tokio_util::codec::LengthDelimitedCodec;
use futures::{stream::StreamExt, SinkExt};
use super::MulticastMemberData;
use tokio::select;
use bytes::Bytes;

pub(in crate::multicast) async fn client_loop(mut member_data: MulticastMemberData) {
    let (read, write) = member_data.socket.into_split();

    let mut write = LengthDelimitedCodec::builder()
        .length_field_type::<u16>()
        .new_write(write);

    loop {
        select! {
            Some(to_send) = member_data.from_engine.recv() => {
                let bytes = Bytes::from(bincode::serialize(&to_send).unwrap());
                write.send(bytes).await;
            }
        }
    }
}