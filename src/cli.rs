use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::{UnboundedSender};
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
        let mut reader = BufReader::new(tokio::io::stdin());
        loop {
            let mut buffer = String::new();
            match reader.read_line(&mut buffer).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let delimited: Vec <_> = buffer
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

