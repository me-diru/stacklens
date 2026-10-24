import path from 'node:path';
import fs from 'node:fs/promises';
import { Graph } from './graph.js';
import { readJSON } from './util.js';

const ORDER = ['runtimes', 'repos', 'packages', 'docker', 'ports', 'agents'];

export async function loadConfig(cwd) {
  const cfg = await readJSON(path.join(cwd, 'stacklens.config.json'));
  return { roots: ['~'], maxDepth: 2, collectors: ORDER, ...cfg };
}

export async function scan(cwd, log = () => {}) {
  const config = await loadConfig(cwd);
  const graph = new Graph();
  // Sequential on purpose: later collectors enrich nodes created by earlier ones.
  for (const name of ORDER) {
    if (!config.collectors.includes(name)) continue;
    const t = Date.now();
    const mod = await import(`./collectors/${name}.js`);
    try {
      const part = await mod.collect({ config, graph });
      graph.merge(part);
      log(`${name.padEnd(10)} ${String(part.nodes.size).padStart(4)} nodes ${String(part.edges.size).padStart(4)} edges  ${Date.now() - t}ms`);
    } catch (err) {
      log(`${name.padEnd(10)} FAILED: ${err.message}`);
    }
  }
  const out = graph.toJSON();
  const outDir = path.join(cwd, '.stacklens');
  await fs.mkdir(outDir, { recursive: true });
  await fs.writeFile(path.join(outDir, 'graph.json'), JSON.stringify(out, null, 2));
  return out;
}
