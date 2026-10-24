import path from 'node:path';
import fs from 'node:fs/promises';
import { Graph } from '../graph.js';
import { readJSON, readText, exists, repoId } from '../util.js';

// Only "notable" deps become framework nodes - everything else is noise for a stack map.
const NOTABLE = {
  next: 'Next.js', react: 'React', vue: 'Vue', svelte: 'Svelte', express: 'Express', fastify: 'Fastify',
  hono: 'Hono', '@prisma/client': 'Prisma', drizzle_orm: 'Drizzle', 'drizzle-orm': 'Drizzle', tailwindcss: 'Tailwind',
  '@supabase/supabase-js': 'Supabase', stripe: 'Stripe', '@anthropic-ai/sdk': 'Anthropic SDK', openai: 'OpenAI SDK',
  ai: 'Vercel AI SDK', langchain: 'LangChain', '@modelcontextprotocol/sdk': 'MCP SDK', electron: 'Electron',
  vite: 'Vite', typescript: 'TypeScript', jest: 'Jest', vitest: 'Vitest', playwright: 'Playwright',
  fastapi: 'FastAPI', django: 'Django', flask: 'Flask', streamlit: 'Streamlit', pandas: 'pandas',
  numpy: 'NumPy', torch: 'PyTorch', anthropic: 'Anthropic SDK', langgraph: 'LangGraph', sqlalchemy: 'SQLAlchemy',
  tokio: 'Tokio', axum: 'Axum', actix_web: 'Actix', serde: 'Serde',
};

// Env var prefixes that imply an external service.
const SERVICE_HINTS = [
  ['SUPABASE', 'Supabase'], ['STRIPE', 'Stripe'], ['OPENAI', 'OpenAI'], ['ANTHROPIC', 'Anthropic'],
  ['VERCEL', 'Vercel'], ['AWS', 'AWS'], ['GOOGLE', 'Google Cloud'], ['FIREBASE', 'Firebase'], ['DATABASE_URL', 'Postgres'],
  ['REDIS', 'Redis'], ['SENDGRID', 'SendGrid'], ['RESEND', 'Resend'], ['TWILIO', 'Twilio'], ['CLERK', 'Clerk'],
  ['NEON', 'Neon'], ['UPSTASH', 'Upstash'], ['SENTRY', 'Sentry'], ['POSTHOG', 'PostHog'], ['GITHUB', 'GitHub'],
];

function link(g, rid, key, label, lang) {
  const id = `framework:${label}`;
  g.node(id, 'framework', label, { lang }, 'packages');
  g.edge(rid, id, 'depends-on', { via: key });
}

async function services(g, dir, rid) {
  const seen = new Set();
  for (const f of ['.env', '.env.example', '.env.local']) {
    const txt = await readText(path.join(dir, f));
    if (!txt) continue;
    for (const line of txt.split('\n')) {
      const key = line.split('=')[0].trim();
      for (const [prefix, label] of SERVICE_HINTS) {
        if (key.startsWith(prefix) && !seen.has(label)) {
          seen.add(label);
          g.node(`service:${label}`, 'service', label, {}, 'packages');
          g.edge(rid, `service:${label}`, 'calls', { via: `${f}:${key}` });
        }
      }
    }
  }
}

export async function collect({ graph }) {
  const g = new Graph();
  const repos = [...graph.nodes.values()].filter(n => n.type === 'repo');
  await Promise.all(repos.map(async r => {
    const dir = r.id.slice('repo:'.length);
    const rid = repoId(dir);
    const langs = new Set();

    const pkg = await readJSON(path.join(dir, 'package.json'));
    if (pkg) {
      langs.add('javascript');
      const deps = { ...pkg.dependencies, ...pkg.devDependencies };
      for (const k of Object.keys(deps)) if (NOTABLE[k]) link(g, rid, k, NOTABLE[k], 'js');
      g.edge(rid, 'runtime:node', 'runs-on');
    }
    for (const f of ['requirements.txt', 'pyproject.toml']) {
      const txt = await readText(path.join(dir, f));
      if (!txt) continue;
      langs.add('python');
      g.edge(rid, 'runtime:python3', 'runs-on');
      for (const [k, label] of Object.entries(NOTABLE)) {
        if (new RegExp(`(^|[\\s"'\\[])${k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}([\\s=<>~\\[;"']|$)`, 'mi').test(txt)) link(g, rid, k, label, 'py');
      }
    }
    const cargo = await readText(path.join(dir, 'Cargo.toml'));
    if (cargo) {
      langs.add('rust');
      g.edge(rid, 'runtime:cargo', 'runs-on');
      for (const [k, label] of Object.entries(NOTABLE)) if (new RegExp(`^${k}\\s*=`, 'm').test(cargo)) link(g, rid, k, label, 'rust');
    }
    if (await exists(path.join(dir, 'go.mod'))) { langs.add('go'); g.edge(rid, 'runtime:go', 'runs-on'); }
    if (await exists(path.join(dir, 'Dockerfile')) || await exists(path.join(dir, 'docker-compose.yml'))) {
      g.edge(rid, 'runtime:docker', 'runs-on');
    }
    if (await exists(path.join(dir, 'vercel.json')) || await exists(path.join(dir, '.vercel'))) {
      g.node('service:Vercel', 'service', 'Vercel', {}, 'packages');
      g.edge(rid, 'service:Vercel', 'deploys-to');
    }
    await services(g, dir, rid);
    g.node(rid, 'repo', r.label, { languages: [...langs] }, 'repos');
  }));
  return g;
}
