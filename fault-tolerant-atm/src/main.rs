use fault_tolerant_atm::{Bank, Cli, TotalOrderedMulticast, parse_config};
use multicast::{Config, NodeId};
use tokio::select;

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

    let (mut bank, bank_snd) = Bank::new().await;
    let (mut multicast, multicast_snd) = TotalOrderedMulticast::new(node_id, &config, bank_snd).await;
    let mut cli = Cli::new(node_id, multicast_snd);

    select! {
        _ = cli.taking_input() => (),
        _ = multicast.main_loop() => (),
        _ = bank.main_loop() => ()
    };
}