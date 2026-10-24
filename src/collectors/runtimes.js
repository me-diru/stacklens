import { Graph } from '../graph.js';
import { run } from '../util.js';

const TOOLS = [
  ['node', ['--version']], ['npm', ['--version']], ['pnpm', ['--version']], ['bun', ['--version']],
  ['python3', ['--version']], ['uv', ['--version']], ['go', ['version']], ['cargo', ['--version']],
  ['docker', ['--version']], ['git', ['--version']], ['gh', ['--version']], ['vercel', ['--version']],
  ['claude', ['--version']], ['psql', ['--version']], ['redis-server', ['--version']],
];

export async function collect() {
  const g = new Graph();
  await Promise.all(TOOLS.map(async ([name, args]) => {
    const where = await run('which', [name]);
    if (!where) return;
    const v = (await run(name, args)) || '';
    g.node(`runtime:${name}`, 'runtime', name, { version: v.split('\n')[0], path: where }, 'runtimes');
  }));
  return g;
}
