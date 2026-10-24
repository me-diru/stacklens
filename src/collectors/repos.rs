use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};
use crate::util::{expand_home, repo_id, run_in, short_path};
use std::path::{Path, PathBuf};

fn find_repos(root: &Path, max_depth: usize, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let entries: Vec<_> = entries.flatten().collect();
    if entries.iter().any(|e| e.file_name() == ".git") {
        out.push(root.to_path_buf());
        return;
    }
    for e in entries {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            find_repos(&e.path(), max_depth, depth + 1, out);
        }
    }
}

pub fn collect(config: &Config, _g: &Graph) -> Graph {
    let mut g = Graph::new();
    let mut repos = Vec::new();
    for r in &config.roots {
        find_repos(&expand_home(r), config.max_depth, 0, &mut repos);
    }

    let infos: Vec<_> = std::thread::scope(|s| {
        let hs: Vec<_> = repos
            .iter()
            .map(|dir| {
                s.spawn(move || {
                    let git = |args: &[&str]| run_in("git", args, Some(dir));
                    (
                        dir.clone(),
                        git(&["remote", "get-url", "origin"]),
                        git(&["rev-parse", "--abbrev-ref", "HEAD"]),
                        git(&["log", "-1", "--format=%cI %s"]),
                        git(&["rev-list", "--count", "HEAD"]),
                    )
                })
            })
            .collect();
        hs.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    for (dir, remote, branch, last, count) in infos {
        let id = repo_id(&dir);
        let label = dir
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let n = g.node(&id, NodeType::Repo, &label, "repos");
        meta_set(n, "path", short_path(&dir));
        if let Some(v) = remote {
            meta_set(n, "remote", v);
        }
        if let Some(v) = branch {
            meta_set(n, "branch", v);
        }
        if let Some(v) = last {
            meta_set(n, "lastCommit", v);
        }
        if let Some(v) = count {
            meta_set(n, "commits", v);
        }
        meta_set(n, "hasClaudeMd", dir.join("CLAUDE.md").exists());
        g.edge(&id, "runtime:git", "managed-by");
    }
    g
}
