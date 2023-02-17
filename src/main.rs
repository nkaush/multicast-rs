use tokio::sync::mpsc::unbounded_channel;
use tokio::net::TcpListener;
use tokio::{select, signal};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <port>", args[0]);
        std::process::exit(1);
    }

    let port: u16 = match args[1].parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Invalid port: {}", args[1]);
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

    let (log_snd, log_rcv) = unbounded_channel();

    let metric_parser_handle = tokio::spawn(parse_metrics(log_rcv));

    loop {
        select! {
            client = tcp_listener.accept() => match client {
                Ok((socket, _addr)) => server.spawn_client(socket),
                Err(e) => {
                    eprintln!("Couldn't get client: {:?}", e);
                    continue;
                }
            },
            signal_result = signal::ctrl_c() => match signal_result {
                Ok(()) => break,
                Err(err) => {
                    eprintln!("Unable to listen for shutdown signal: {}", err);
                    break;
                }
            },
            Some(client_id) = server.receive_kill_sig() => server.remove_client(&client_id)
        }
    }
}