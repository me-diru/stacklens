use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};
use crate::util::{read, read_json};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CC: &str = "agent:claude-code";

fn claude_home() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".claude")
}

fn frontmatter(txt: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(rest) = txt.strip_prefix("---\n") else {
        return out;
    };
    let Some(end) = rest.find("\n---") else {
        return out;
    };
    for line in rest[..end].lines() {
        if let Some((k, v)) = line.split_once(':') {
            out.insert(
                k.trim().into(),
                v.trim().trim_matches(|c| c == '"' || c == '\'').into(),
            );
        }
    }
    out
}

fn agent_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default()
}

fn add_mcp(g: &mut Graph, name: &str, cfg: &Value, owner: &str, scope: &str) {
    let id = format!("mcp:{name}");
    let n = g.node(&id, NodeType::Mcp, name, "agents");
    for k in ["command", "url", "type"] {
        if let Some(v) = cfg.get(k).and_then(|v| v.as_str()) {
            meta_set(n, k, v);
        }
    }
    meta_set(n, "scope", scope);
    g.edge(owner, &id, "uses-tools-from");
}

/// Claude Code encodes a project path by replacing every `/` and `.` with `-`.
fn encode(p: &str) -> String {
    p.replace(['/', '.'], "-")
}

fn sessions(dir: &str) -> (usize, Option<String>) {
    let sdir = claude_home().join("projects").join(encode(dir));
    let Ok(rd) = std::fs::read_dir(&sdir) else {
        return (0, None);
    };
    let mut count = 0;
    let mut latest: Option<std::time::SystemTime> = None;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let is_uuid = name.len() > 13
            && name.as_bytes()[8] == b'-'
            && name[..8].chars().all(|c| c.is_ascii_hexdigit());
        if !is_uuid {
            continue;
        }
        count += 1;
        if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
            latest = Some(latest.map_or(m, |l| l.max(m)));
        }
    }
    let latest = latest.map(|t| {
        let days = std::time::SystemTime::now()
            .duration_since(t)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            / 86400;
        if days == 0 {
            "today".to_string()
        } else {
            format!("{days} days ago")
        }
    });
    (count, latest)
}

pub fn collect(_c: &Config, graph: &Graph) -> Graph {
    let mut g = Graph::new();
    let home = claude_home();
    if !home.exists() {
        return g;
    }
    let cfg = read_json(&dirs::home_dir().unwrap_or_default().join(".claude.json"))
        .unwrap_or(Value::Null);

    let n = g.node(CC, NodeType::Agent, "Claude Code", "agents");
    meta_set(n, "kind", "coding-agent");
    meta_set(n, "vendor", "Anthropic");
    meta_set(n, "home", "~/.claude");
    g.edge(CC, "runtime:claude", "runs-on");

    for f in agent_files(&home.join("agents")) {
        let fm = frontmatter(&read(&f).unwrap_or_default());
        let name = fm
            .get("name")
            .cloned()
            .unwrap_or_else(|| f.file_stem().unwrap().to_string_lossy().into_owned());
        let id = format!("agent:{name}");
        let n = g.node(&id, NodeType::Agent, &name, "agents");
        meta_set(n, "kind", "subagent");
        meta_set(n, "scope", "user");
        for k in ["description", "model"] {
            if let Some(v) = fm.get(k) {
                meta_set(n, k, v);
            }
        }
        g.edge(&id, CC, "extends");
    }
    if let Some(m) = cfg.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, c) in m {
            add_mcp(&mut g, name, c, CC, "user");
        }
    }

    if let Some(plugins) = read_json(&home.join("plugins").join("installed_plugins.json"))
        .and_then(|v| v.get("plugins").cloned())
        .and_then(|v| v.as_object().cloned())
    {
        for (key, installs) in plugins {
            let name = key.split('@').next().unwrap_or(&key).to_string();
            let id = format!("agent:plugin:{name}");
            let arr = installs.as_array().cloned().unwrap_or_default();
            let n = g.node(&id, NodeType::Agent, &name, "agents");
            meta_set(n, "kind", "plugin");
            if let Some(v) = arr
                .first()
                .and_then(|i| i.get("version"))
                .and_then(|v| v.as_str())
            {
                meta_set(n, "version", v);
            }
            g.edge(&id, CC, "extends");
            for i in &arr {
                if let Some(p) = i.get("projectPath").and_then(|v| v.as_str()) {
                    g.edge(&id, &format!("repo:{p}"), "enabled-in");
                }
            }
        }
    }

    let marks = [
        (".cursor", "Cursor"),
        (".github/copilot-instructions.md", "GitHub Copilot"),
        ("AGENTS.md", "Codex / AGENTS.md"),
        (".windsurf", "Windsurf"),
        ("GEMINI.md", "Gemini CLI"),
    ];
    for r in graph.of_type(NodeType::Repo) {
        let dir_str = &r.id["repo:".len()..];
        let dir = Path::new(dir_str);
        let (count, latest) = sessions(dir_str);
        let proj = cfg
            .get("projects")
            .and_then(|p| p.get(dir_str))
            .cloned()
            .unwrap_or(Value::Null);
        {
            let n = g.node(&r.id, NodeType::Repo, &r.label, "repos");
            meta_set(n, "claudeSessions", count);
            if let Some(l) = latest {
                meta_set(n, "lastClaudeSession", l);
            }
            if let Some(c) = proj.get("lastCost").and_then(|v| v.as_f64()) {
                meta_set(n, "lastClaudeCostUSD", format!("{c:.2}"));
            }
            if let Some(m) = proj.get("lastModelUsage").and_then(|v| v.as_object()) {
                meta_set(
                    n,
                    "lastClaudeModels",
                    m.keys().cloned().collect::<Vec<_>>().join(", "),
                );
            }
        }
        let has_claude_md = r
            .meta
            .get("hasClaudeMd")
            .map(|v| v == "true")
            .unwrap_or(false);
        if count > 0 || has_claude_md {
            g.edge(CC, &r.id, "works-on")
                .meta
                .insert("sessions".into(), count.to_string());
        }

        for f in agent_files(&dir.join(".claude").join("agents")) {
            let fm = frontmatter(&read(&f).unwrap_or_default());
            let name = fm
                .get("name")
                .cloned()
                .unwrap_or_else(|| f.file_stem().unwrap().to_string_lossy().into_owned());
            let id = format!("agent:{}/{name}", r.label);
            let n = g.node(&id, NodeType::Agent, &name, "agents");
            meta_set(n, "kind", "subagent");
            meta_set(n, "scope", "project");
            for k in ["description", "model"] {
                if let Some(v) = fm.get(k) {
                    meta_set(n, k, v);
                }
            }
            g.edge(&id, CC, "extends");
            g.edge(&id, &r.id, "works-on");
        }
        if let Some(m) = read_json(&dir.join(".mcp.json"))
            .and_then(|v| v.get("mcpServers").cloned())
            .and_then(|v| v.as_object().cloned())
        {
            for (name, c) in &m {
                add_mcp(&mut g, name, c, &r.id, "project");
            }
        }
        if let Some(m) = proj.get("mcpServers").and_then(|v| v.as_object()) {
            for (name, c) in m {
                add_mcp(&mut g, name, c, &r.id, "local");
            }
        }
        for (p, label) in marks {
            if dir.join(p).exists() {
                let id = format!("agent:{label}");
                let n = g.node(&id, NodeType::Agent, label, "agents");
                meta_set(n, "kind", "coding-agent");
                g.edge(&id, &r.id, "works-on");
            }
        }
    }
    g
}
