import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { Graph } from '../graph.js';
import { readJSON, readText, exists, repoId } from '../util.js';

const CLAUDE_HOME = path.join(os.homedir(), '.claude');
const CC = 'agent:claude-code';

function frontmatter(txt) {
  const m = txt.match(/^---\n([\s\S]*?)\n---/);
  const out = {};
  if (!m) return out;
  for (const line of m[1].split('\n')) {
    const i = line.indexOf(':');
    if (i > 0) out[line.slice(0, i).trim()] = line.slice(i + 1).trim().replace(/^["']|["']$/g, '');
  }
  return out;
}

async function agentFiles(dir) {
  try { return (await fs.readdir(dir)).filter(f => f.endsWith('.md')).map(f => path.join(dir, f)); } catch { return []; }
}

function addMcp(g, name, cfg, owner, scope) {
  const id = `mcp:${name}`;
  g.node(id, 'mcp', name, { command: cfg.command, url: cfg.url, type: cfg.type, scope }, 'agents');
  g.edge(owner, id, 'uses-tools-from');
}

// Claude Code encodes a project path by replacing every / and . with -
const encode = p => p.replace(/[\/.]/g, '-');

async function sessions(dir) {
  const sdir = path.join(CLAUDE_HOME, 'projects', encode(dir));
  try {
    const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-/i;
    const files = (await fs.readdir(sdir)).filter(f => UUID.test(f));
    let latest = 0;
    for (const f of files) { const st = await fs.stat(path.join(sdir, f)); latest = Math.max(latest, st.mtimeMs); }
    return { count: files.length, latest: latest ? new Date(latest).toISOString() : null };
  } catch { return { count: 0, latest: null }; }
}

export async function collect({ graph }) {
  const g = new Graph();
  const cfg = await readJSON(path.join(os.homedir(), '.claude.json')) || {};
  if (!(await exists(CLAUDE_HOME))) return g;

  g.node(CC, 'agent', 'Claude Code', { kind: 'coding-agent', vendor: 'Anthropic', home: '~/.claude' }, 'agents');
  g.edge(CC, 'runtime:claude', 'runs-on');

  // Global custom subagents
  for (const f of await agentFiles(path.join(CLAUDE_HOME, 'agents'))) {
    const fm = frontmatter((await readText(f)) || '');
    const name = fm.name || path.basename(f, '.md');
    g.node(`agent:${name}`, 'agent', name, { kind: 'subagent', scope: 'user', description: fm.description, model: fm.model }, 'agents');
    g.edge(`agent:${name}`, CC, 'extends');
  }
  // Global MCP servers
  for (const [name, c] of Object.entries(cfg.mcpServers || {})) addMcp(g, name, c, CC, 'user');

  // Plugins
  const plugins = await readJSON(path.join(CLAUDE_HOME, 'plugins', 'installed_plugins.json'));
  for (const [key, installs] of Object.entries(plugins?.plugins || {})) {
    const name = key.split('@')[0];
    const id = `agent:plugin:${name}`;
    g.node(id, 'agent', name, { kind: 'plugin', version: installs[0]?.version, scopes: installs.map(i => i.scope) }, 'agents');
    g.edge(id, CC, 'extends');
    for (const i of installs) if (i.projectPath) g.edge(id, repoId(i.projectPath), 'enabled-in');
  }

  // Per-repo wiring
  const repos = [...graph.nodes.values()].filter(n => n.type === 'repo');
  await Promise.all(repos.map(async r => {
    const dir = r.id.slice(5);
    const s = await sessions(dir);
    const proj = cfg.projects?.[dir];
    const meta = { claudeSessions: s.count, lastClaudeSession: s.latest };
    if (proj?.lastCost != null) meta.lastClaudeCostUSD = Number(proj.lastCost.toFixed(2));
    if (proj?.lastModelUsage) meta.lastClaudeModels = Object.keys(proj.lastModelUsage);
    g.node(r.id, 'repo', r.label, meta, 'repos');
    if (s.count > 0 || r.meta.hasClaudeMd) g.edge(CC, r.id, 'works-on', { sessions: s.count });

    for (const f of await agentFiles(path.join(dir, '.claude', 'agents'))) {
      const fm = frontmatter((await readText(f)) || '');
      const name = fm.name || path.basename(f, '.md');
      const id = `agent:${r.label}/${name}`;
      g.node(id, 'agent', name, { kind: 'subagent', scope: 'project', description: fm.description, model: fm.model }, 'agents');
      g.edge(id, CC, 'extends');
      g.edge(id, r.id, 'works-on');
    }
    const mcp = await readJSON(path.join(dir, '.mcp.json'));
    for (const [name, c] of Object.entries(mcp?.mcpServers || {})) addMcp(g, name, c, r.id, 'project');
    for (const [name, c] of Object.entries(proj?.mcpServers || {})) addMcp(g, name, c, r.id, 'local');

    // Other agent tooling footprints
    const marks = [['.cursor', 'Cursor'], ['.github/copilot-instructions.md', 'GitHub Copilot'], ['AGENTS.md', 'Codex / AGENTS.md'], ['.windsurf', 'Windsurf'], ['GEMINI.md', 'Gemini CLI']];
    for (const [p, label] of marks) {
      if (await exists(path.join(dir, p))) {
        const id = `agent:${label}`;
        g.node(id, 'agent', label, { kind: 'coding-agent' }, 'agents');
        g.edge(id, r.id, 'works-on');
      }
    }
  }));
  return g;
}
