pub mod agents;
pub mod docker;
pub mod packages;
pub mod ports;
pub mod repos;
pub mod runtimes;

use crate::config::Config;
use crate::graph::Graph;

/// Collectors run in this order; later ones may read nodes produced by earlier ones.
pub const ORDER: [&str; 6] = ["runtimes", "repos", "packages", "docker", "ports", "agents"];

pub fn run_one(name: &str, config: &Config, graph: &Graph) -> Option<Graph> {
    Some(match name {
        "runtimes" => runtimes::collect(config, graph),
        "repos" => repos::collect(config, graph),
        "packages" => packages::collect(config, graph),
        "docker" => docker::collect(config, graph),
        "ports" => ports::collect(config, graph),
        "agents" => agents::collect(config, graph),
        _ => return None,
    })
}
