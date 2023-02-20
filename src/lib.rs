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
            println!("expecting {len} bytes...");
            let mut buf = BytesMut::with_capacity(len as usize);
            match socket.read_exact(&mut buf).await {
                Ok(_) => {
                    println!("got bytes: {:?}", &buf[..]);
                    Ok(bincode::deserialize::<T>(&buf[..]).unwrap())
                },
                Err(e) => Err(e)
            }
        },
        Err(e) => Err(e)
    }
}

pub async fn write_to_socket<T: Serialize>(socket: &mut TcpStream, to_write: T) -> tokio::io::Result<()> {
    let mut data = Vec::new();
    let bytes = bincode::serialize(&to_write).unwrap();
    data.write_u64_le(bytes.len() as u64).await?;
    data.write(bytes.as_slice()).await?;
    
    println!("{:?} => {:?}", bytes.len(), data);
    socket.write_all_buf(&mut data.as_slice()).await
}