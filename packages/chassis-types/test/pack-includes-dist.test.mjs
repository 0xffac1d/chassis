#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const testDir = dirname(fileURLToPath(import.meta.url));
const pkgRoot = join(testDir, '..');

const raw = execFileSync('npm', ['pack', '--dry-run', '--json'], {
  cwd: pkgRoot,
  encoding: 'utf8',
});
const payload = JSON.parse(raw.trim());
const pkg = Array.isArray(payload) ? payload[0] : payload;
const files = new Set(
  (pkg.files || [])
    .filter((e) => e && typeof e.path === 'string')
    .map((e) => e.path.replace(/\\/g, '/')),
);

const required = ['dist/index.js', 'dist/index.d.ts', 'fingerprint.sha256'];
const missing = required.filter((p) => !files.has(p));
if (missing.length) {
  console.error('pack-includes-dist: npm pack would omit:', missing);
  console.error('files seen:', [...files].filter((p) => p.startsWith('dist/') || p.includes('fingerprint')));
  process.exit(1);
}

console.log('pack-includes-dist: OK (dist + fingerprint in pack manifest)');
