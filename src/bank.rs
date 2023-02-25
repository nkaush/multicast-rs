use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use log::trace;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserInput {
    Deposit(String, usize),
    Transfer(String, String, usize)
}

pub struct Bank {
    rcv: UnboundedReceiver<UserInput>,
    accounts: BTreeMap<String, usize>
}

impl Bank {
    pub fn new() -> (Self, UnboundedSender<UserInput>) {
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
            match sample {
                UserInput::Deposit(person, amt) => {
                    trace!("[BANK] DEPOSIT {} {}", person, amt);
                    self.accounts.entry(person).and_modify(|curr| *curr += amt).or_insert(amt); 
                    self.print_balances();
                }
    
                UserInput::Transfer(person1, person2, amt) => {
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

