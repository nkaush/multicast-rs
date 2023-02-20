pub mod multicast;
pub mod message;
pub mod bank;
pub mod cli;

pub use multicast::*;
pub use message::*;
pub use bank::*;
pub use cli::*;

pub use std::collections::HashMap;

pub type ConnectionList = Vec<String>;
pub type HostName = String;
pub type Port = u16;

pub type Config = HashMap<String, (HostName, Port, ConnectionList)>;

use tokio::{net::TcpStream, io::{AsyncReadExt, AsyncWriteExt}};
use serde::{Deserialize, Serialize};
use bytes::BytesMut;

pub async fn read_from_socket<'a, T: ?Sized>(socket: &mut TcpStream) -> tokio::io::Result<T> where for <'de> T: Deserialize<'de> + 'a {
    match socket.read_u64_le().await {
        Ok(len) => {
            let mut buf = BytesMut::with_capacity(len as usize);
            match socket.read_buf(&mut buf).await {
                Ok(_) => Ok(bincode::deserialize::<T>(&buf).unwrap()),
                Err(e) => Err(e)
            }
        },
        Err(e) => Err(e)
    }
}

pub async fn write_to_socket<T: Serialize>(socket: &mut TcpStream, data: T) -> tokio::io::Result<()> {
    let bytes = bincode::serialize(&data).unwrap();
    socket.write_u64_le(bytes.len() as u64).await?;
    let mut slice = bytes.as_slice();
    socket.write_all_buf(&mut slice).await
}