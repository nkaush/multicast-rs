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

use tokio::{io, net::TcpStream, io::AsyncReadExt};
use bytes::BytesMut;

pub async fn read_socket(mut stream: TcpStream) -> io::Result<BytesMut> {
    match stream.read_u64_le().await {
        Ok(n) => {
            let mut buf = BytesMut::with_capacity(n as usize);
            stream.read_exact(&mut buf);

            Ok(buf)
        },
        Err(e) => Err(e)
    }
}