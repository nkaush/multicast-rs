use tokio_util::codec::{FramedRead, LinesCodec};
use tokio::sync::mpsc::{UnboundedSender};
use futures::stream::StreamExt;
use crate::message::UserInput;

pub struct Cli {
    cli_send: UnboundedSender<UserInput>
}

impl Cli {
    pub fn new(cli_send: UnboundedSender<UserInput>) -> Self {
        Self {
            cli_send,
        }
    }

    pub async fn taking_input(&mut self) {
        let mut stdin = FramedRead::new(tokio::io::stdin(), LinesCodec::new());
        while let Some(buffer) = stdin.next().await {
            match buffer {
                Err(_) => break,
                Ok(line) => {
                    let delimited: Vec <_> = line
                        .trim()
                        .split_ascii_whitespace()
                        .collect();
                    println!("{delimited:?}");
                    if delimited.len() != 3 && delimited.len() != 5 {
                        continue;
                    }

                    match delimited[0] {
                        "DEPOSIT" => match delimited[2].parse() {
                            Ok(amount) => self.cli_send.send(
                                UserInput::Deposit(
                                    delimited[1].into(), 
                                    amount)
                                ).unwrap(),
                            Err(_) => continue
                        },
                        "TRANSFER" => match delimited[4].parse() {
                            Ok(amount) => self.cli_send.send(
                                UserInput::Transfer(
                                    delimited[1].into(), 
                                    delimited[3].into(), 
                                    amount)
                                ).unwrap(),
                            Err(_) => continue
                        },
                        _ => ()
                    }
                }
            }
        }
    }
}

