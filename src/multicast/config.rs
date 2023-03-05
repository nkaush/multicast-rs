use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use std::fs::File;

pub type NodeId = usize;

#[derive(Clone)]
pub struct NodeConfiguration {
    pub node_id: NodeId,
    pub hostname: String,
    pub port: u16,
    pub connection_list: Vec<NodeId>
}

impl NodeConfiguration {
    fn new(node_id: NodeId, hostname: String, port: u16, connection_list: Vec<NodeId>) -> Self {
        Self {
            node_id,
            hostname,
            port,
            connection_list
        }
    }
}

pub type Config = HashMap<NodeId, NodeConfiguration>;

pub fn parse_config(path: &str, given_node_name: &str) -> Result<(Config, NodeId), String> {
    let mut config: HashMap<NodeId, NodeConfiguration> = Config::new();
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
    let mut next_node_id: NodeId = 0;
    let mut this_node_id: Option<NodeId> = None;
    while let Ok(n) = rdr.read_line(&mut buf) {
        if n == 0 { break }

        let delimited: Vec<_> = buf.split_ascii_whitespace().collect();
        match delimited[0..3] {
            [node_name, hostname, p] => match p.parse() {
                Ok(port) => {
                    config.insert(next_node_id, NodeConfiguration::new(next_node_id, hostname.into(), port, nodes.clone()));
                    nodes.push(next_node_id);

                    if node_name == given_node_name {
                        this_node_id = Some(next_node_id);
                    }
                },
                Err(_) => return Err(format!("Bad config: could not parse port for node with id: {}", n))
            },
            _ => return Err("Bad config: too little arguments per line".into())
        };

        buf.clear();
        next_node_id += 1;
    }

    if this_node_id.is_none() {
        return Err(format!("Bad config: node identifier is not listed in config file"));
    } else if config.len() != node_count {
        return Err(format!("Bad config: expected node count ({}) does not match given count ({})", node_count, config.len()));
    }

    Ok((config, this_node_id.unwrap()))
}