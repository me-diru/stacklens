mod collectors;
mod config;
mod graph;
mod scan;
mod ui;
mod util;

use clap::{Parser, Subcommand};

/// stacklens: see your whole tech stack, and the AI agents working on it, as one graph.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Folder(s) to search for repos; overrides stacklens.config.json
    #[arg(long, global = true)]
    root: Vec<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scan the machine and open the graph UI (default)
    Ui,
    /// Scan the machine and write .stacklens/graph.json
    Scan {
        /// Print the graph JSON to stdout instead of writing the file
        #[arg(long)]
        json: bool,
    },
    /// Open the UI from the last scan without rescanning
    Open,
}

fn main() {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().expect("cwd");
    let mut config = config::Config::load(&cwd);
    if !cli.root.is_empty() {
        config.roots = cli.root.clone();
    }

    match cli.cmd.unwrap_or(Cmd::Ui) {
        Cmd::Scan { json } => {
            let g = scan::scan(&config, |l| eprintln!("{l}"));
            if json {
                println!("{}", serde_json::to_string_pretty(&g).unwrap());
            } else {
                let p = scan::save(&g, &cwd).expect("write graph");
                eprintln!(
                    "total      {} nodes {} edges -> {}",
                    g.nodes.len(),
                    g.edges.len(),
                    p.display()
                );
            }
        }
        Cmd::Ui => {
            let g = scan::scan(&config, |l| eprintln!("{l}"));
            let _ = scan::save(&g, &cwd);
            eprintln!("total      {} nodes {} edges", g.nodes.len(), g.edges.len());
            ui::run(g, config);
        }
        Cmd::Open => {
            let g = scan::load(&cwd).unwrap_or_else(|| scan::scan(&config, |l| eprintln!("{l}")));
            ui::run(g, config);
        }
    }
}
