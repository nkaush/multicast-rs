use std::io::{BufRead, BufReader};
use std::fs::File;

pub type NodeId = usize;

#[derive(Clone)]
pub struct NodeConfiguration {
    pub node_id: NodeId,
    pub hostname: String,
    pub port: u16,
}

pub struct Config {
    configurations: Vec<NodeConfiguration>,
    next_id: usize
}

impl Config {
    fn new() -> Self {
        Self {
            configurations: Vec::new(),
            next_id: 0
        }
    }

    fn add_node(&mut self, hostname: String, port: u16) -> usize {
        let node_id = self.next_id;
        self.configurations.push(NodeConfiguration { node_id, hostname, port });
        self.next_id += 1;

        node_id
    }

    pub fn get(&self, node_id: usize) -> Option<&NodeConfiguration> {
        self.configurations.get(node_id)
    }

    pub fn get_connection_list(node_id: usize) -> impl Iterator<Item=usize> {
        0..node_id
    }

    pub fn len(&self) -> usize {
        self.configurations.len()
    }
}

pub fn parse_config(path: &str, given_node_name: &str) -> Result<(Config, NodeId), String> {
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
    let mut this_node_id: Option<NodeId> = None;
    while let Ok(n) = rdr.read_line(&mut buf) {
        if n == 0 { break }

        let delimited: Vec<_> = buf.split_ascii_whitespace().collect();
        match delimited[0..3] {
            [node_name, hostname, p] => match p.parse() {
                Ok(port) => {
                    let id = config.add_node(hostname.into(), port);

                    if node_name == given_node_name {
                        this_node_id = Some(id);
                    }
                },
                Err(_) => return Err(format!("Bad config: could not parse port for node with id: {}", n))
            },
            _ => return Err("Bad config: too little arguments per line".into())
        };

        buf.clear();
    }

    if this_node_id.is_none() {
        return Err(format!("Bad config: node identifier is not listed in config file"));
    } else if config.len() != node_count {
        return Err(format!("Bad config: expected node count ({}) does not match given count ({})", node_count, config.len()));
    }

    Ok((config, this_node_id.unwrap()))
}