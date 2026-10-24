#!/usr/bin/env node
import { scan } from '../src/scan.js';
import { serve } from '../src/serve.js';

const [cmd = 'dev', ...rest] = process.argv.slice(2);
const cwd = process.cwd();
const port = Number(rest.find(a => a.startsWith('--port='))?.split('=')[1] || process.env.PORT || 4242);

const help = `stacklens - see your tech stack and AI agents as one graph

  stacklens scan     scan the machine, write .stacklens/graph.json
  stacklens serve    serve the UI from the last scan (scans if none)
  stacklens dev      scan then serve (default)
  --port=4242`;

if (cmd === 'scan' || cmd === 'dev') {
  const g = await scan(cwd, console.log);
  console.log(`total      ${g.nodes.length} nodes ${g.edges.length} edges -> .stacklens/graph.json`);
}
if (cmd === 'serve' || cmd === 'dev') serve(cwd, port);
if (!['scan', 'serve', 'dev'].includes(cmd)) console.log(help);
