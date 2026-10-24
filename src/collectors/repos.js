import fs from 'node:fs/promises';
import path from 'node:path';
import { Graph } from '../graph.js';
import { run, exists, expandHome, repoId, shortPath } from '../util.js';

async function findRepos(root, maxDepth, depth = 0, out = []) {
  if (depth > maxDepth) return out;
  let entries;
  try { entries = await fs.readdir(root, { withFileTypes: true }); } catch { return out; }
  if (entries.some(e => e.name === '.git')) { out.push(root); return out; }
  for (const e of entries) {
    if (!e.isDirectory() || e.name.startsWith('.') || e.name === 'node_modules') continue;
    await findRepos(path.join(root, e.name), maxDepth, depth + 1, out);
  }
  return out;
}

export async function collect({ config }) {
  const g = new Graph();
  const repos = [];
  for (const r of config.roots) repos.push(...await findRepos(expandHome(r), config.maxDepth ?? 2));
  await Promise.all(repos.map(async dir => {
    const [remote, branch, last, count] = await Promise.all([
      run('git', ['-C', dir, 'remote', 'get-url', 'origin']),
      run('git', ['-C', dir, 'rev-parse', '--abbrev-ref', 'HEAD']),
      run('git', ['-C', dir, 'log', '-1', '--format=%cI %s']),
      run('git', ['-C', dir, 'rev-list', '--count', 'HEAD']),
    ]);
    const n = g.node(repoId(dir), 'repo', path.basename(dir), {
      path: shortPath(dir), remote, branch, lastCommit: last, commits: Number(count) || 0,
      hasClaudeMd: await exists(path.join(dir, 'CLAUDE.md')),
    }, 'repos');
    g.edge(n.id, 'runtime:git', 'managed-by');
  }));
  return g;
}
