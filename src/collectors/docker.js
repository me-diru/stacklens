import { Graph } from '../graph.js';
import { run } from '../util.js';

export async function collect() {
  const g = new Graph();
  const out = await run('docker', ['ps', '-a', '--format', '{{json .}}'], { timeout: 5000 });
  if (!out) return g;
  for (const line of out.split('\n').filter(Boolean)) {
    let c; try { c = JSON.parse(line); } catch { continue; }
    const id = `container:${c.Names}`;
    g.node(id, 'container', c.Names, { image: c.Image, status: c.Status, state: c.State, ports: c.Ports }, 'docker');
    g.edge(id, 'runtime:docker', 'runs-on');
    const project = c.Labels?.match(/com\.docker\.compose\.project=([^,]+)/)?.[1];
    if (project) g.node(id, 'container', c.Names, { composeProject: project }, 'docker');
    for (const m of (c.Ports || '').matchAll(/:(\d+)->/g)) {
      g.node(`port:${m[1]}`, 'port', `:${m[1]}`, {}, 'docker');
      g.edge(id, `port:${m[1]}`, 'listens-on');
    }
  }
  return g;
}
