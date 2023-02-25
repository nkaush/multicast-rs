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
        while let Some(Ok(line)) = stdin.next().await {
            let delimited: Vec <_> = line
                .trim()
                .split_ascii_whitespace()
                .filter(|term| term.len() > 0)
                .collect();

            match delimited[..] {
                ["DEPOSIT", from, amt] => {
                    if let Ok(amt) = amt.parse() {
                        self.cli_send.send(UserInput::Deposit(from.to_string(), amt)).unwrap()
                    }
                },
                ["TRANSFER", from, "->", to, amt] => {
                    if let Ok(amt) = amt.parse() {
                        self.cli_send.send(UserInput::Transfer(from.to_string(), to.to_string(), amt)).unwrap();
                    }
                },
                _ => ()
            }
        }
    }
}

