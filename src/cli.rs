use tokio::sync::mpsc::{UnboundedSender};
use crate::message::FromMulticast;
use crate::message::UserInput;
use std::io;

struct Cli {
    cli_send: UnboundedSender<UserInput>
    
}

impl Cli {
    pub fn new(cli_send: UnboundedSender<UserInput>) -> Self {
        Self {
            cli_send,
        }
    }

    pub fn taking_input(&mut self) {
        loop {
            let mut buffer = String::new();
            match io::stdin().read_line(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let delimited: Vec <_> = buffer.split_ascii_whitespace().collect();
                    match delimited[0] {
                        "DEPOSIT" => match delimited[2].parse() {
                            Ok(n) => self.cli_send.send(UserInput::Deposit(delimited[1].into(), n)).unwrap(),
                            Err(_) => continue
                        },
                        "TRANSFER" => match delimited[4].parse() {
                            Ok(n) => self.cli_send.send(UserInput::Transfer(delimited[1].into(), delimited[3].into(), n)).unwrap(),
                            Err(_) => continue
                        },
                        _ => ()
                    }
                }
            }
        }

    }
}

