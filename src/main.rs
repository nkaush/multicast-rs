use tokio::sync::mpsc::unbounded_channel;
use std::io::{BufReader, BufRead};
use tokio::net::TcpListener;
use tokio::{select, signal};
use std::net::SocketAddr;
use std::fs::File;

use mp1_node::Config;

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

    let (port, to_connect_with) = match config.get(&args[1]) {
        Some((_, p, to_connect_with)) => (*p, to_connect_with),
        None => {
            eprintln!("Bad config: node identifier is not listed in config file");
            std::process::exit(1);
        }
    };

    let bind_addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let tcp_listener = match TcpListener::bind(bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {:?}", bind_addr, e);
            std::process::exit(1);
        }
    };

    eprintln!("Listening on {:?}...", tcp_listener.local_addr().unwrap());
    eprintln!("Cancel this process with CRTL+C");

    println!("{config:?}")

    // let (log_snd, log_rcv) = unbounded_channel();

    // let metric_parser_handle = tokio::spawn(parse_metrics(log_rcv));

    // loop {
    //     select! {
    //         client = tcp_listener.accept() => match client {
    //             Ok((socket, _addr)) => server.spawn_client(socket),
    //             Err(e) => {
    //                 eprintln!("Couldn't get client: {:?}", e);
    //                 continue;
    //             }
    //         },
    //         signal_result = signal::ctrl_c() => match signal_result {
    //             Ok(()) => break,
    //             Err(err) => {
    //                 eprintln!("Unable to listen for shutdown signal: {}", err);
    //                 break;
    //             }
    //         },
    //         Some(client_id) = server.receive_kill_sig() => server.remove_client(&client_id)
    //     }
    // }
}