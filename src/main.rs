use std::io::{BufReader, BufRead};
use tokio::{select, signal};
use std::fs::File;

use mp1_node::{Bank, Cli, Config, TotalOrderedMulticast};

fn parse_config(path: &str) -> Result<Config, String> {
    let mut config = Config::new();
    let mut rdr = match File::open(path) {
        Ok(f) => BufReader::new(f),
        Err(e) => return Err(e.to_string())
    };
    
    let mut buf = String::new();
    let node_count: usize = match rdr.read_line(&mut buf) {
        Ok(_) => match buf.trim().parse() {
            Ok(count) => count,
            Err(_) => return Err("Bad config: could not parse node count".into())
        },
        Err(_) => return Err("Bad config: missing node count".into())
    };

    buf.clear();
    let mut nodes = Vec::new();
    while let Ok(n) = rdr.read_line(&mut buf) {
        if n == 0 { break }

        let delimited: Vec<_> = buf.split_ascii_whitespace().collect();
        match delimited[0..3] {
            [node_id, hostname, p] => match p.parse() {
                Ok(port) => {
                    config.insert(node_id.into(), (hostname.into(), port, nodes.clone()));
                    nodes.push(node_id.into());
                },
                Err(_) => return Err(format!("Bad config: could not parse port for node with id: {}", n))
            },
            _ => return Err("Bad config: too little arguments per line".into())
        };

        buf.clear();
    }

    if config.len() != node_count {
        return Err(format!("Bad config: expected node count ({}) does not match given count ({})", node_count, config.len()));
    }

    Ok(config)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <identifier> <configuration file>", args[0]);
        std::process::exit(1);
    }

    let config = match parse_config(&args[2]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if !config.contains_key(&args[1]) {
        eprintln!("Bad config: node identifier is not listed in config file");
        std::process::exit(1);
    }

    let node_id = args[1].clone();

    let (mut bank, bank_snd) = Bank::new();
    let (mut multicast, multicast_snd) = TotalOrderedMulticast::new(node_id, &config, bank_snd).await;
    let mut cli = Cli::new(multicast_snd);

    select! {
        _ = cli.taking_input() => (),
        _ = multicast.main_loop() => (),
        _ = bank.main_loop() => ()
    };
}