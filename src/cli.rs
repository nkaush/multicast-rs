use tokio_util::codec::{FramedRead, LinesCodec};
use tokio::sync::mpsc::{UnboundedSender};
use futures::stream::StreamExt;
use crate::Transaction;

pub struct Cli {
    cli_send: UnboundedSender<Transaction>
}

impl Cli {
    pub fn new(cli_send: UnboundedSender<Transaction>) -> Self {
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
                ["DEPOSIT", account, amt] => {
                    if let Ok(amount) = amt.parse() {
                        self.cli_send.send(Transaction::new_deposit(account, amount)).unwrap()
                    }
                },
                ["TRANSFER", from, "->", to, amt] => {
                    if let Ok(amount) = amt.parse() {
                        self.cli_send.send(Transaction::new_transfer(from, to, amount)).unwrap();
                    }
                },
                _ => ()
            }
        }
    }
}

