use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use crate::message::UserInput;
use std::collections::BTreeMap;

pub struct Bank {
    // add variables
    rcv: UnboundedReceiver<UserInput>,
    accounts: BTreeMap<String, usize>
}

impl Bank {
    // add variables
    pub fn new() -> (Self, UnboundedSender<UserInput>) {
        let (snd, rcv) = unbounded_channel();
        let this = Self {
            rcv,
            accounts: BTreeMap::new()
        };
        (this, snd)
    }

    pub async fn main_loop(&mut self) {
        
        while let Some(sample) = self.rcv.recv().await {
            match sample {
                UserInput::Deposit(person, amt) => {
                    self.accounts.entry(person).and_modify(|curr| *curr += amt).or_insert(amt); 
                    print!("BALANCES: ");
                    for (account, balance) in &self.accounts {
                        print!("{account}: {balance}\t");
                    }
                    println!("\n");
                }
    
                UserInput::Transfer(person1, person2, amt) => {
                    match self.accounts.get(&person1) {
                        Some(balance1) => if balance1 >= &amt { 
                            self.accounts.entry(person1).and_modify(|curr| *curr -= amt);
                            self.accounts.entry(person2).and_modify(|curr| *curr += amt).or_insert(amt);

                            print!("BALANCES: ");
                            for (account, balance) in &self.accounts {
                                print!("{account}: {balance}\t");
                            }
                            println!("\n");
                        },
                        None => ()
                    }
                }
                
            }
        }
    
    
    
    }
}

