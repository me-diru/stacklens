use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};
use crate::util::run;
use regex::Regex;

pub fn collect(_c: &Config, _g: &Graph) -> Graph {
    let mut g = Graph::new();
    let Some(out) = run("docker", &["ps", "-a", "--format", "{{json .}}"]) else {
        return g;
    };
    let port_re = Regex::new(r":(\d+)->").unwrap();
    for line in out.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(c) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let s = |k: &str| c.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let name = s("Names");
        let id = format!("container:{name}");
        let n = g.node(&id, NodeType::Container, &name, "docker");
        for k in ["Image", "Status", "State", "Ports"] {
            meta_set(n, &k.to_lowercase(), s(k));
        }
        if let Some(p) = s("Labels")
            .split(',')
            .find_map(|l| l.strip_prefix("com.docker.compose.project="))
        {
            meta_set(n, "composeProject", p);
        }
        g.edge(&id, "runtime:docker", "runs-on");
        for cap in port_re.captures_iter(&s("Ports")) {
            let pid = format!("port:{}", &cap[1]);
            g.node(&pid, NodeType::Port, &format!(":{}", &cap[1]), "docker");
            g.edge(&id, &pid, "listens-on");
        }
    }
    g
}
