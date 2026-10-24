use crate::collectors;
use crate::config::Config;
use crate::graph::Graph;
use std::path::Path;
use std::time::Instant;

pub fn scan(config: &Config, mut log: impl FnMut(String)) -> Graph {
    let mut graph = Graph::new();
    for name in collectors::ORDER {
        if !config.collectors.iter().any(|c| c == name) {
            continue;
        }
        let t = Instant::now();
        match collectors::run_one(name, config, &graph) {
            Some(part) => {
                log(format!(
                    "{name:<10} {:>4} nodes {:>4} edges  {}ms",
                    part.nodes.len(),
                    part.edges.len(),
                    t.elapsed().as_millis()
                ));
                graph.merge(part);
            }
            None => log(format!("{name:<10} unknown collector")),
        }
    }
    graph.prune();
    graph.generated_at = crate::util::now_iso();
    graph
}

pub fn output_path(cwd: &Path) -> std::path::PathBuf {
    cwd.join(".stacklens").join("graph.json")
}

pub fn save(graph: &Graph, cwd: &Path) -> std::io::Result<std::path::PathBuf> {
    let p = output_path(cwd);
    std::fs::create_dir_all(p.parent().unwrap())?;
    std::fs::write(&p, serde_json::to_string_pretty(graph).unwrap())?;
    Ok(p)
}

pub fn load(cwd: &Path) -> Option<Graph> {
    serde_json::from_str(&std::fs::read_to_string(output_path(cwd)).ok()?).ok()
}
