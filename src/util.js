import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

const execFileP = promisify(execFile);

export function expandHome(p) {
  return p.startsWith('~') ? path.join(os.homedir(), p.slice(1)) : p;
}

export async function run(cmd, args = [], opts = {}) {
  try {
    const { stdout } = await execFileP(cmd, args, { timeout: 8000, maxBuffer: 8 * 1024 * 1024, ...opts });
    return stdout.trim();
  } catch {
    return null;
  }
}

export async function exists(p) {
  try { await fs.access(p); return true; } catch { return false; }
}

export async function readJSON(p) {
  try { return JSON.parse(await fs.readFile(p, 'utf8')); } catch { return null; }
}

export async function readText(p) {
  try { return await fs.readFile(p, 'utf8'); } catch { return null; }
}

export function repoId(dir) {
  return `repo:${dir}`;
}

export function shortPath(p) {
  const home = os.homedir();
  return p.startsWith(home) ? '~' + p.slice(home.length) : p;
}
