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

    pub async fn multicast_connection(&mut self) {

        let mut transaction_type = 0;
        let mut account1:String = String::new();
        let mut account2:String = String::new();
        let mut amount:usize = 0;
        
        while let Some(sample) = self.rcv.recv().await {
            match sample {
                UserInput::Deposit(person, amt) => {
                    transaction_type = 0;
                    account1 = person;
                    amount = amt;
                }
    
                UserInput::Transfer(person1, person2, amt) => {
                    transaction_type = 1;
                    account1 = person1;
                    account2 = person2;
                    amount = amt;
                }
                
            }
        }
    
        // it's a deposit
        if (transaction_type == 0) {
            if self.accounts.get(&account1) != None {
                match self.accounts.get(&account1) {
                    Some(amount) => {
                        
                    }
                    None => {

                    }
                }
            }
        }
    
    
    
    }
}

