use tokio::{io::AsyncWriteExt, fs::File};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use multicast::NodeId;
use std::io::Cursor;
use log::trace;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit(String, usize),
    Transfer(String, String, usize)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub node_id: NodeId,
    pub id: usize,
    pub timestamp: f64,
    pub tr: TransactionType
}

pub struct Bank {
    accounts: BTreeMap<String, usize>,
    latency_log: File
}

impl Bank {
    pub async fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            latency_log: File::create("latencies.log").await.unwrap()
        }
    }

    async fn log_latency(&mut self, tx: &Transaction) {
        let latency = crate::get_timestamp() - tx.timestamp;
        let log_line = format!("n{}-t{},{}\n", tx.node_id, tx.id, latency);

        let mut cursor = Cursor::new(log_line);
        self.latency_log.write_all_buf(&mut cursor).await.unwrap();
    }

    fn print_balances(&self) {
        print!("BALANCES ");
        for (account, balance) in &self.accounts {
            print!("{account}:{balance} ");
        }
        println!("");
    }

    pub async fn process_transaction(&mut self, tx: Transaction) {
        self.log_latency(&tx).await;
        match tx.tr {
            TransactionType::Deposit(person, amt) => {
                trace!("DEPOSIT {} {}", person, amt);
                self.accounts.entry(person).and_modify(|curr| *curr += amt).or_insert(amt); 
            },
            TransactionType::Transfer(person1, person2, amt) => {
                match self.accounts.get(&person1) {
                    Some(balance1) => if balance1 >= &amt { 
                        trace!("TRANSFER {} -> {} {}", person1, person2, amt);
                        self.accounts.entry(person1).and_modify(|curr| *curr -= amt);
                        self.accounts.entry(person2).and_modify(|curr| *curr += amt).or_insert(amt);
                    },
                    None => ()
                }
            }
        }

        self.print_balances();
    }
}

