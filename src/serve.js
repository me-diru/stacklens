import http from 'node:http';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { scan } from './scan.js';

const UI = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'ui');
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml' };

export function serve(cwd, port = 4242, log = console.log) {
  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, 'http://x');
    try {
      if (url.pathname === '/api/graph') {
        const p = path.join(cwd, '.stacklens', 'graph.json');
        let body;
        try { body = await fs.readFile(p); } catch { body = JSON.stringify(await scan(cwd, log)); }
        res.writeHead(200, { 'content-type': 'application/json' }); return res.end(body);
      }
      if (url.pathname === '/api/rescan' && req.method === 'POST') {
        const g = await scan(cwd, log);
        res.writeHead(200, { 'content-type': 'application/json' }); return res.end(JSON.stringify(g));
      }
      let file = url.pathname === '/' ? '/index.html' : url.pathname;
      file = path.normalize(file).replace(/^(\.\.[\/\\])+/, '');
      const full = path.join(UI, file);
      if (!full.startsWith(UI)) throw Object.assign(new Error('forbidden'), { code: 403 });
      const data = await fs.readFile(full);
      res.writeHead(200, { 'content-type': MIME[path.extname(full)] || 'application/octet-stream' });
      res.end(data);
    } catch (err) {
      res.writeHead(err.code === 'ENOENT' ? 404 : err.code || 500); res.end(String(err.message));
    }
  });
  server.listen(port, '127.0.0.1', () => log(`stacklens ui  http://localhost:${port}`));
  return server;
}
