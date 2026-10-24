# stacklens

**See your whole tech stack, and the AI agents working on it, as one graph.**

stacklens scans your machine and draws a map of everything you actually run: your repos, the frameworks
and external services they depend on, the runtimes you have installed, running containers and ports, and
the AI coding agents (Claude Code, subagents, plugins, MCP servers, Cursor, Codex…) wired into each project.

Written in Rust. The UI is a native window built with [egui](https://github.com/emilk/egui), so there is no
browser, no JavaScript and no server. Local-first: nothing leaves your machine.

> Status: early (v0.1). It works and the collector model is designed to be extended. Expect rough edges.

## Why

Developer portals like Backstage map a *team's* services and need a catalog someone maintains.
Code visualizers map *one* codebase at a time. Agent dashboards show *one* agent's config.
Nothing shows a single developer the whole picture: what is on this machine, how it connects, and
which agents touch which project. stacklens is that picture, generated from what is already on disk.

## Install

```bash
cargo install stacklens
```

Or from source:

```bash
git clone https://github.com/me-diru/stacklens
cd stacklens
cargo run --release -- --root ~/projects
```

## Use

```bash
stacklens --root ~/projects          # scan, then open the graph window
stacklens --root ~/projects scan     # scan only, write .stacklens/graph.json
stacklens scan --json > stack.json   # print the graph to stdout
stacklens open                       # reopen the last scan without rescanning
```

Instead of `--root`, drop a `stacklens.config.json` in the folder you run from
(or in `~/.config/stacklens/config.json`):

```json
{
  "roots": ["~/Desktop/projects", "~/work"],
  "max_depth": 2,
  "collectors": ["runtimes", "repos", "packages", "docker", "ports", "agents"]
}
```

## What you get

- **Graph view**: force-directed graph, coloured by node type, with a legend that doubles as a type filter.
  Click a node to highlight its neighbourhood and open a detail panel with everything collected about it.
  Drag empty space to pan, scroll to zoom, drag a node to pin it.
- **Table view**: the same nodes as a sortable list.
- **Search**: matches names and metadata (try a framework or service name).
- **Stat tiles**: repos, agents, services, frameworks, runtimes, connections, Claude sessions.
- **Rescan** button: re-runs all collectors in the background.
- **`.stacklens/graph.json`**: the raw graph, so you can feed it to anything else.

## Collectors

Each collector reads one kind of source and returns a partial graph. They run in order so later ones can
enrich earlier nodes.

| Collector | Reads | Produces |
|---|---|---|
| `runtimes` | `which` + `--version` for node, npm, pnpm, bun, python, uv, go, cargo, rustc, docker, git, gh, vercel, claude, psql, redis | runtime nodes |
| `repos` | `.git` directories under `roots` | repo nodes with remote, branch, last commit, commit count, CLAUDE.md flag |
| `packages` | package.json, requirements.txt, pyproject.toml, Cargo.toml, go.mod, Dockerfile, vercel.json, `.env*` keys | framework nodes, service nodes, `depends-on` / `calls` / `deploys-to` / `runs-on` edges |
| `docker` | `docker ps -a` | container and port nodes |
| `ports` | `lsof` listening TCP ports, linked to a repo via the process cwd | port nodes |
| `agents` | `~/.claude` (agents, plugins, sessions, MCP), `~/.claude.json`, `.mcp.json`, `.claude/agents`, AGENTS.md, `.cursor`, `.windsurf`, copilot-instructions | agent and mcp nodes, `works-on` / `extends` / `uses-tools-from` edges |

See [docs/collectors.md](docs/collectors.md) for the node and edge schema and how to write your own.

## Privacy

Everything runs locally. Nothing is sent anywhere. The `.env` parser reads only variable *names* (to infer
which services a repo talks to), never values. `.stacklens/` is gitignored.

## Platform notes

macOS and Linux. The `ports` collector uses `lsof`; `docker` needs a running daemon and fails soft without one.
Windows should build but the collectors have not been tested there.

## Roadmap

- [ ] Cloud account connectors via existing CLIs (Vercel, Supabase, AWS, GitHub)
- [ ] Live agent activity: tail Claude Code session logs and show which agent is touching which repo right now
- [ ] Token and cost per repo from agent sessions
- [ ] Export to Mermaid / Graphviz for docs
- [ ] More agents: Codex, Gemini CLI, Aider, OpenHands config
- [ ] Windows support for `ports`

## Contributing

Add a collector, add a detector for a framework or service, or improve the UI. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT
