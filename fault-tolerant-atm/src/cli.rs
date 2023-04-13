use crate::{Transaction, TransactionType};
use tokio_util::codec::{FramedRead, LinesCodec};
use async_recursion::async_recursion;
use futures::stream::StreamExt;
use multicast::NodeId;
use tokio::io::Stdin;

pub struct Cli {
    stdin: FramedRead<Stdin, LinesCodec>,
    next_local_tx_id: usize,
    node_id: NodeId
}

impl Cli {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            next_local_tx_id: 0,
            node_id,
            stdin: FramedRead::new(tokio::io::stdin(), LinesCodec::new()),
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

    #[async_recursion]
    pub async fn parse_input(&mut self) -> Option<Transaction> {
        let input = match self.stdin.next().await {
            Some(Ok(i)) => i,
            _ => return None
        };

        let delimited: Vec <_> = input
            .trim()
            .split_ascii_whitespace()
            .filter(|term| term.len() > 0)
            .collect();

        match delimited[..] {
            ["DEPOSIT", account, amt] => {
                if let Ok(amount) = amt.parse() {
                    Some(self.new_deposit(account, amount))
                } else {
                    self.parse_input().await
                }
            },
            ["TRANSFER", from, "->", to, amt] => {
                if let Ok(amount) = amt.parse() {
                    Some(self.new_transfer(from, to, amount))
                } else {
                    self.parse_input().await
                }
            },
            _ => self.parse_input().await
        }
    }
}

