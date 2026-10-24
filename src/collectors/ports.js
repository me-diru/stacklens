import { Graph } from '../graph.js';
import { run } from '../util.js';

// Local listening TCP ports -> who owns them. Links to a repo when the process cwd is inside one.
export async function collect({ graph }) {
  const g = new Graph();
  const out = await run('lsof', ['-iTCP', '-sTCP:LISTEN', '-P', '-n', '-F', 'pcn']);
  if (!out) return g;
  const procs = new Map();
  let cur = null;
  for (const line of out.split('\n')) {
    const tag = line[0], val = line.slice(1);
    if (tag === 'p') { cur = procs.get(val) || { pid: val, ports: new Set() }; procs.set(val, cur); }
    else if (tag === 'c' && cur) cur.cmd = val;
    else if (tag === 'n' && cur) { const port = val.match(/:(\d+)$/)?.[1]; if (port) cur.ports.add(port); }
  }
  const repos = [...graph.nodes.values()].filter(n => n.type === 'repo').map(n => n.id.slice(5));
  await Promise.all([...procs.values()].map(async p => {
    const cwdOut = await run('lsof', ['-a', '-p', p.pid, '-d', 'cwd', '-Fn']);
    const cwd = cwdOut?.split('\n').find(l => l.startsWith('n'))?.slice(1);
    const repo = cwd && repos.find(r => cwd === r || cwd.startsWith(r + '/'));
    for (const port of p.ports) {
      const id = `port:${port}`;
      g.node(id, 'port', `:${port}`, { pid: p.pid, command: p.cmd, cwd }, 'ports');
      if (repo) g.edge(`repo:${repo}`, id, 'listens-on');
    }
  }));
  return g;
}
