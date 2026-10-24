// Tiny in-memory graph builder shared by all collectors.
// Node types are a fixed vocabulary so the UI can assign colors in a stable order.
export const NODE_TYPES = ['repo', 'agent', 'service', 'runtime', 'container', 'framework', 'port', 'mcp'];

export class Graph {
  constructor() {
    this.nodes = new Map();
    this.edges = new Map();
  }
  node(id, type, label, meta = {}, source = 'unknown') {
    if (!NODE_TYPES.includes(type)) throw new Error(`unknown node type ${type}`);
    const existing = this.nodes.get(id);
    if (existing) {
      Object.assign(existing.meta, meta);
      return existing;
    }
    const n = { id, type, label, meta, source };
    this.nodes.set(id, n);
    return n;
  }
  edge(from, to, kind = 'uses', meta = {}) {
    const key = `${from}|${kind}|${to}`;
    if (!this.edges.has(key)) this.edges.set(key, { from, to, kind, meta });
    return this.edges.get(key);
  }
  merge(other) {
    for (const n of other.nodes.values()) this.node(n.id, n.type, n.label, n.meta, n.source);
    for (const e of other.edges.values()) this.edge(e.from, e.to, e.kind, e.meta);
  }
  toJSON() {
    // Drop edges whose endpoints never materialised.
    const edges = [...this.edges.values()].filter(e => this.nodes.has(e.from) && this.nodes.has(e.to));
    return { generatedAt: new Date().toISOString(), nodes: [...this.nodes.values()], edges };
  }
}
