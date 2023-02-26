use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use log::trace;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit(String, usize),
    Transfer(String, String, usize)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    tr: TransactionType,
    timestamp: f64
}

impl Transaction {
    pub fn new_deposit(account: &str, amount: usize) -> Self {
        Self {
            tr: TransactionType::Deposit(account.into(), amount),
            timestamp: crate::get_timestamp()
        }
    }

    pub fn new_transfer(from: &str, to: &str, amount: usize) -> Self {
        Self {
            tr: TransactionType::Transfer(from.into(), to.into(), amount),
            timestamp: crate::get_timestamp()
        }
    }
}

pub struct Bank {
    rcv: UnboundedReceiver<Transaction>,
    accounts: BTreeMap<String, usize>
}

impl Bank {
    pub fn new() -> (Self, UnboundedSender<Transaction>) {
        let (snd, rcv) = unbounded_channel();
        let this = Self {
            rcv,
            accounts: BTreeMap::new()
        };
        (this, snd)
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

