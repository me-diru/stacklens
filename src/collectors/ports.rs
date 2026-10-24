use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};
use crate::util::run;
use std::collections::BTreeMap;

struct Proc {
    pid: String,
    cmd: String,
    ports: Vec<String>,
}

/// Local listening TCP ports, linked to a repo when the owning process cwd is inside one.
pub fn collect(_c: &Config, graph: &Graph) -> Graph {
    let mut g = Graph::new();
    let Some(out) = run("lsof", &["-iTCP", "-sTCP:LISTEN", "-P", "-n", "-F", "pcn"]) else {
        return g;
    };
    let mut procs: BTreeMap<String, Proc> = BTreeMap::new();
    let mut cur: Option<String> = None;
    for line in out.lines() {
        let (tag, val) = (
            line.chars().next().unwrap_or(' '),
            &line[line.len().min(1)..],
        );
        match tag {
            'p' => {
                procs.entry(val.into()).or_insert(Proc {
                    pid: val.into(),
                    cmd: String::new(),
                    ports: vec![],
                });
                cur = Some(val.into());
            }
            'c' => {
                if let Some(p) = cur.as_ref().and_then(|c| procs.get_mut(c)) {
                    p.cmd = val.into();
                }
            }
            'n' => {
                if let Some(p) = cur.as_ref().and_then(|c| procs.get_mut(c)) {
                    if let Some(port) = val.rsplit(':').next() {
                        if port.chars().all(|c| c.is_ascii_digit())
                            && !p.ports.contains(&port.to_string())
                        {
                            p.ports.push(port.into());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let repos: Vec<&str> = graph
        .of_type(NodeType::Repo)
        .map(|n| &n.id["repo:".len()..])
        .collect();
    for p in procs.values() {
        let cwd = run("lsof", &["-a", "-p", &p.pid, "-d", "cwd", "-Fn"]).and_then(|o| {
            o.lines()
                .find(|l| l.starts_with('n'))
                .map(|l| l[1..].to_string())
        });
        let repo = cwd.as_deref().and_then(|c| {
            repos
                .iter()
                .find(|r| c == **r || c.starts_with(&format!("{r}/")))
                .copied()
        });
        for port in &p.ports {
            let id = format!("port:{port}");
            let n = g.node(&id, NodeType::Port, &format!(":{port}"), "ports");
            meta_set(n, "pid", &p.pid);
            meta_set(n, "command", &p.cmd);
            if let Some(c) = &cwd {
                meta_set(n, "cwd", c);
            }
            if let Some(r) = repo {
                g.edge(&format!("repo:{r}"), &id, "listens-on");
            }
        }
    }
    g
}
