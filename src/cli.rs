use crate::{Transaction, TransactionType, multicast::NodeId};
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio::sync::mpsc::{UnboundedSender};
use futures::stream::StreamExt;

pub struct Cli {
    cli_send: UnboundedSender<Transaction>,
    next_local_tx_id: usize,
    node_id: NodeId
}

impl Cli {
    pub fn new(node_id: NodeId, cli_send: UnboundedSender<Transaction>) -> Self {
        Self {
            next_local_tx_id: 0,
            node_id,
            cli_send,
        }
    }

    pub fn new_deposit(&mut self, account: &str, amount: usize) -> Transaction {
        self.next_local_tx_id += 1;
        Transaction {
            node_id: self.node_id,
            id: self.next_local_tx_id,
            tr: TransactionType::Deposit(account.into(), amount),
            timestamp: crate::get_timestamp()
        }
    }

    pub fn new_transfer(&mut self, from: &str, to: &str, amount: usize) -> Transaction {
        self.next_local_tx_id += 1;
        Transaction {
            node_id: self.node_id,
            id: self.next_local_tx_id,
            tr: TransactionType::Transfer(from.into(), to.into(), amount),
            timestamp: crate::get_timestamp()
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
                        let tx = self.new_deposit(account, amount);
                        self.cli_send.send(tx).unwrap()
                    }
                },
                ["TRANSFER", from, "->", to, amt] => {
                    if let Ok(amount) = amt.parse() {
                        let tx = self.new_transfer(from, to, amount);
                        self.cli_send.send(tx).unwrap();
                    }
                },
                _ => ()
            }
        }
    }
}

