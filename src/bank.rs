use tokio::{
    sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel},
    io::{BufWriter, AsyncWriteExt}, fs::File
};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use crate::multicast::NodeId;
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
    rcv: UnboundedReceiver<Transaction>,
    accounts: BTreeMap<String, usize>,
    latency_log: BufWriter<File>
}

impl Bank {
    pub async fn new() -> (Self, UnboundedSender<Transaction>) {
        let (snd, rcv) = unbounded_channel();
        let this = Self {
            rcv,
            accounts: BTreeMap::new(),
            latency_log: BufWriter::new(File::create("latencies.log").await.unwrap())
        };
        (this, snd)
    }

    async fn log_latency(&mut self, tx: &Transaction) {
        let latency = crate::get_timestamp() - tx.timestamp;
        let log_line = format!("n{}-t{},{}", tx.node_id, tx.id, latency);

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

    pub async fn main_loop(&mut self) {
        while let Some(sample) = self.rcv.recv().await {
            self.log_latency(&sample).await;
            match sample.tr {
                TransactionType::Deposit(person, amt) => {
                    trace!("[BANK] DEPOSIT {} {}", person, amt);
                    self.accounts.entry(person).and_modify(|curr| *curr += amt).or_insert(amt); 
                    self.print_balances();
                }
    
                TransactionType::Transfer(person1, person2, amt) => {
                    match self.accounts.get(&person1) {
                        Some(balance1) => if balance1 >= &amt { 
                            eprintln!("[BANK] TRANSFER {} -> {} {}", person1, person2, amt);
                            self.accounts.entry(person1).and_modify(|curr| *curr -= amt);
                            self.accounts.entry(person2).and_modify(|curr| *curr += amt).or_insert(amt);
                            
                            self.print_balances();
                        },
                        None => ()
                    }
                }
            }
        }
    }
}

