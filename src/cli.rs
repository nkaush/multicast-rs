use tokio::sync::mpsc::{UnboundedSender};
use crate::message::UserInput;
use std::io;

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
        loop {
            let mut buffer = String::new();
            match io::stdin().read_line(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let delimited: Vec <_> = buffer.split_ascii_whitespace().collect();
                    if delimited.len() != 3 && delimited.len() != 5 {
                        continue;
                    }

                    match delimited[0] {
                        "DEPOSIT" => match delimited[2].parse() {
                            Ok(amount) => self.cli_send.send(
                                UserInput::Deposit(
                                    delimited[1].into(), amount)
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

