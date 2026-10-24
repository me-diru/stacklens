# Graph schema and collectors

## Schema

`.stacklens/graph.json`:

```json
{
  "generatedAt": "2026-10-24T21:41:08.589Z",
  "nodes": [{ "id": "repo:/Users/me/work/app", "type": "repo", "label": "app", "meta": { "...": "..." }, "source": "repos" }],
  "edges": [{ "from": "repo:/Users/me/work/app", "to": "framework:Next.js", "kind": "depends-on", "meta": { "via": "next" } }]
}
```

### Node types

| type | id convention | meaning |
|---|---|---|
| `repo` | `repo:<absolute path>` | a git repository |
| `agent` | `agent:<name>` or `agent:plugin:<name>` | a coding agent, subagent or plugin |
| `service` | `service:<Name>` | an external SaaS or infra a repo talks to |
| `framework` | `framework:<Name>` | a notable library or framework |
| `runtime` | `runtime:<binary>` | an installed tool on the machine |
| `container` | `container:<name>` | a docker container |
| `port` | `port:<number>` | a listening TCP port |
| `mcp` | `mcp:<name>` | an MCP server configured for an agent |

### Edge kinds

`depends-on`, `calls`, `deploys-to`, `runs-on`, `managed-by`, `listens-on`, `works-on`, `extends`,
`uses-tools-from`, `enabled-in`. Direction is "from uses to".

## Writing a collector

```rust
use crate::config::Config;
use crate::graph::{meta_set, Graph, NodeType};

pub fn collect(_config: &Config, graph: &Graph) -> Graph {
    let mut g = Graph::new();
    let n = g.node("service:Example", NodeType::Service, "Example", "example");
    meta_set(n, "region", "us-east-1");
    for repo in graph.of_type(NodeType::Repo) {
        g.edge(&repo.id, "service:Example", "calls");
    }
    g
}
```

`Graph::node` is idempotent: calling it again with the same id returns the existing node so you can add
metadata. Edges whose endpoints never materialise are dropped by `Graph::prune` after all collectors run,
so it is safe to reference nodes another collector may or may not produce (for example `runtime:docker`).

Collectors run in the order listed in `src/collectors/mod.rs`: `runtimes`, `repos`, `packages`, `docker`,
`ports`, `agents`. Put yours after anything it needs to read from `graph`.
