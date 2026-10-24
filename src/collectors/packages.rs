use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};
use crate::util::{read, read_json};
use regex::Regex;
use std::path::Path;

/// Only "notable" deps become framework nodes; everything else is noise on a stack map.
const NOTABLE: &[(&str, &str)] = &[
    ("next", "Next.js"),
    ("react", "React"),
    ("vue", "Vue"),
    ("svelte", "Svelte"),
    ("express", "Express"),
    ("fastify", "Fastify"),
    ("hono", "Hono"),
    ("@prisma/client", "Prisma"),
    ("drizzle-orm", "Drizzle"),
    ("tailwindcss", "Tailwind"),
    ("@supabase/supabase-js", "Supabase"),
    ("stripe", "Stripe"),
    ("@anthropic-ai/sdk", "Anthropic SDK"),
    ("openai", "OpenAI SDK"),
    ("ai", "Vercel AI SDK"),
    ("langchain", "LangChain"),
    ("@modelcontextprotocol/sdk", "MCP SDK"),
    ("electron", "Electron"),
    ("vite", "Vite"),
    ("typescript", "TypeScript"),
    ("jest", "Jest"),
    ("vitest", "Vitest"),
    ("playwright", "Playwright"),
    ("fastapi", "FastAPI"),
    ("django", "Django"),
    ("flask", "Flask"),
    ("streamlit", "Streamlit"),
    ("pandas", "pandas"),
    ("numpy", "NumPy"),
    ("torch", "PyTorch"),
    ("anthropic", "Anthropic SDK"),
    ("langgraph", "LangGraph"),
    ("sqlalchemy", "SQLAlchemy"),
    ("tokio", "Tokio"),
    ("axum", "Axum"),
    ("actix-web", "Actix"),
    ("serde", "Serde"),
    ("eframe", "egui"),
    ("bevy", "Bevy"),
    ("clap", "clap"),
];

/// Env var prefixes that imply an external service. Only variable NAMES are read, never values.
const SERVICE_HINTS: &[(&str, &str)] = &[
    ("SUPABASE", "Supabase"),
    ("STRIPE", "Stripe"),
    ("OPENAI", "OpenAI"),
    ("ANTHROPIC", "Anthropic"),
    ("VERCEL", "Vercel"),
    ("AWS", "AWS"),
    ("GOOGLE", "Google Cloud"),
    ("FIREBASE", "Firebase"),
    ("DATABASE_URL", "Postgres"),
    ("REDIS", "Redis"),
    ("SENDGRID", "SendGrid"),
    ("RESEND", "Resend"),
    ("TWILIO", "Twilio"),
    ("CLERK", "Clerk"),
    ("NEON", "Neon"),
    ("UPSTASH", "Upstash"),
    ("SENTRY", "Sentry"),
    ("POSTHOG", "PostHog"),
    ("GITHUB", "GitHub"),
];

fn link(g: &mut Graph, rid: &str, key: &str, label: &str, lang: &str) {
    let id = format!("framework:{label}");
    let n = g.node(&id, NodeType::Framework, label, "packages");
    meta_set(n, "lang", lang);
    let e = g.edge(rid, &id, "depends-on");
    e.meta.insert("via".into(), key.into());
}

fn services(g: &mut Graph, dir: &Path, rid: &str) {
    let mut seen = std::collections::HashSet::new();
    for f in [".env", ".env.example", ".env.local"] {
        let Some(txt) = read(&dir.join(f)) else {
            continue;
        };
        for line in txt.lines() {
            let key = line.split('=').next().unwrap_or("").trim();
            for (prefix, label) in SERVICE_HINTS {
                if key.starts_with(prefix) && seen.insert(*label) {
                    let sid = format!("service:{label}");
                    g.node(&sid, NodeType::Service, label, "packages");
                    g.edge(rid, &sid, "calls")
                        .meta
                        .insert("via".into(), format!("{f}:{key}"));
                }
            }
        }
    }
}

pub fn collect(_c: &Config, graph: &Graph) -> Graph {
    let mut g = Graph::new();
    for r in graph.of_type(NodeType::Repo) {
        let dir = Path::new(&r.id["repo:".len()..]);
        let rid = r.id.clone();
        let mut langs: Vec<&str> = Vec::new();

        if let Some(pkg) = read_json(&dir.join("package.json")) {
            langs.push("javascript");
            for section in ["dependencies", "devDependencies"] {
                if let Some(deps) = pkg.get(section).and_then(|d| d.as_object()) {
                    for k in deps.keys() {
                        if let Some((_, label)) = NOTABLE.iter().find(|(n, _)| n == k) {
                            link(&mut g, &rid, k, label, "js");
                        }
                    }
                }
            }
            g.edge(&rid, "runtime:node", "runs-on");
        }
        for f in ["requirements.txt", "pyproject.toml"] {
            let Some(txt) = read(&dir.join(f)) else {
                continue;
            };
            langs.push("python");
            g.edge(&rid, "runtime:python3", "runs-on");
            for (k, label) in NOTABLE {
                if k.contains('@') || k.contains('/') {
                    continue;
                }
                let re = Regex::new(&format!(
                    r#"(?mi)(^|[\s"'\[]){}([\s=<>~\[;"']|$)"#,
                    regex::escape(k)
                ))
                .unwrap();
                if re.is_match(&txt) {
                    link(&mut g, &rid, k, label, "py");
                }
            }
        }
        if let Some(cargo) = read(&dir.join("Cargo.toml")) {
            langs.push("rust");
            g.edge(&rid, "runtime:cargo", "runs-on");
            for (k, label) in NOTABLE {
                let re = Regex::new(&format!(r"(?m)^{}\s*=", regex::escape(k))).unwrap();
                if re.is_match(&cargo) {
                    link(&mut g, &rid, k, label, "rust");
                }
            }
        }
        if dir.join("go.mod").exists() {
            langs.push("go");
            g.edge(&rid, "runtime:go", "runs-on");
        }
        if dir.join("Dockerfile").exists() || dir.join("docker-compose.yml").exists() {
            g.edge(&rid, "runtime:docker", "runs-on");
        }
        if dir.join("vercel.json").exists() || dir.join(".vercel").exists() {
            g.node("service:Vercel", NodeType::Service, "Vercel", "packages");
            g.edge(&rid, "service:Vercel", "deploys-to");
        }
        services(&mut g, dir, &rid);
        langs.dedup();
        if !langs.is_empty() {
            let n = g.node(&rid, NodeType::Repo, &r.label, "repos");
            meta_set(n, "languages", langs.join(", "));
        }
    }
    g
}
