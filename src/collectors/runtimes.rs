use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};
use crate::util::{run, which};

const TOOLS: &[(&str, &[&str])] = &[
    ("node", &["--version"]),
    ("npm", &["--version"]),
    ("pnpm", &["--version"]),
    ("bun", &["--version"]),
    ("python3", &["--version"]),
    ("uv", &["--version"]),
    ("go", &["version"]),
    ("cargo", &["--version"]),
    ("rustc", &["--version"]),
    ("docker", &["--version"]),
    ("git", &["--version"]),
    ("gh", &["--version"]),
    ("vercel", &["--version"]),
    ("claude", &["--version"]),
    ("psql", &["--version"]),
    ("redis-server", &["--version"]),
];

pub fn collect(_c: &Config, _g: &Graph) -> Graph {
    let mut g = Graph::new();
    let found: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = TOOLS
            .iter()
            .map(|(name, args)| {
                s.spawn(move || {
                    let path = which(name)?;
                    let v = run(name, args).unwrap_or_default();
                    Some((*name, path, v.lines().next().unwrap_or("").to_string()))
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect()
    });
    for (name, path, version) in found {
        let n = g.node(
            &format!("runtime:{name}"),
            NodeType::Runtime,
            name,
            "runtimes",
        );
        meta_set(n, "version", version);
        meta_set(n, "path", path);
    }
    g
}
