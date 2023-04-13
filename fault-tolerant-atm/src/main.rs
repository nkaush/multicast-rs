use fault_tolerant_atm::{Bank, Cli, TotalOrderedMulticast, parse_config};
use multicast::{Config, NodeId, Multicast};
use tokio::select;
use log::error;

#[tokio::main]
async fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <identifier> <configuration file>", args[0]);
        std::process::exit(1);
    }

    let (config, node_id): (Config, NodeId) = match parse_config(&args[2], &args[1]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let mut bank = Bank::new().await;
    let mut multicast = TotalOrderedMulticast::connect(node_id, config, 60).await;
    let mut cli = Cli::new(node_id);

    loop {
        select! {
            input = cli.parse_input() => match input {
                Some(transaction) => {
                    if let Err(e) = multicast.broadcast(transaction).await {
                        error!("broadcase error: {e:?}")
                    }
                },
                None => break
            },
            delivery = multicast.deliver() => match delivery {
                Ok(msg) => bank.process_transaction(msg).await,
                Err(e) => error!("Delivery failure: {e:?}")
            }
        }
    }
}