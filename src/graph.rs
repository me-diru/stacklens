//! In-memory graph shared by all collectors. Node types are a fixed vocabulary so the UI can
//! assign colours in a stable order.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Repo,
    Agent,
    Service,
    Framework,
    Runtime,
    Container,
    Port,
    Mcp,
}

impl NodeType {
    pub const ALL: [NodeType; 8] = [
        NodeType::Repo,
        NodeType::Agent,
        NodeType::Service,
        NodeType::Framework,
        NodeType::Runtime,
        NodeType::Container,
        NodeType::Port,
        NodeType::Mcp,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NodeType::Repo => "Repos",
            NodeType::Agent => "AI agents",
            NodeType::Service => "Services",
            NodeType::Framework => "Frameworks",
            NodeType::Runtime => "Runtimes",
            NodeType::Container => "Containers",
            NodeType::Port => "Ports",
            NodeType::Mcp => "MCP servers",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: NodeType,
    pub label: String,
    /// Free-form metadata; values are strings for simplicity.
    pub meta: BTreeMap<String, String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    #[serde(default)]
    pub meta: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Graph {
    #[serde(default)]
    pub generated_at: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Idempotent: a second call with the same id merges `meta` into the existing node.
    pub fn node(&mut self, id: &str, kind: NodeType, label: &str, source: &str) -> &mut Node {
        if let Some(i) = self.nodes.iter().position(|n| n.id == id) {
            return &mut self.nodes[i];
        }
        self.nodes.push(Node {
            id: id.into(),
            kind,
            label: label.into(),
            meta: BTreeMap::new(),
            source: source.into(),
        });
        self.nodes.last_mut().unwrap()
    }

    pub fn edge(&mut self, from: &str, to: &str, kind: &str) -> &mut Edge {
        if let Some(i) = self
            .edges
            .iter()
            .position(|e| e.from == from && e.to == to && e.kind == kind)
        {
            return &mut self.edges[i];
        }
        self.edges.push(Edge {
            from: from.into(),
            to: to.into(),
            kind: kind.into(),
            meta: BTreeMap::new(),
        });
        self.edges.last_mut().unwrap()
    }

    pub fn get(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn of_type(&self, kind: NodeType) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(move |n| n.kind == kind)
    }

    pub fn merge(&mut self, other: Graph) {
        for n in other.nodes {
            let target = self.node(&n.id, n.kind, &n.label, &n.source);
            target.meta.extend(n.meta);
        }
        for e in other.edges {
            let target = self.edge(&e.from, &e.to, &e.kind);
            target.meta.extend(e.meta);
        }
    }

    /// Drop edges whose endpoints never materialised.
    pub fn prune(&mut self) {
        let ids: std::collections::HashSet<&str> =
            self.nodes.iter().map(|n| n.id.as_str()).collect();
        let keep: Vec<bool> = self
            .edges
            .iter()
            .map(|e| ids.contains(e.from.as_str()) && ids.contains(e.to.as_str()))
            .collect();
        let mut i = 0;
        self.edges.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });
    }

    pub fn degree(&self, id: &str) -> usize {
        self.edges
            .iter()
            .filter(|e| e.from == id || e.to == id)
            .count()
    }
}

pub fn meta_set(n: &mut Node, key: &str, val: impl ToString) {
    n.meta.insert(key.into(), val.to_string());
}
