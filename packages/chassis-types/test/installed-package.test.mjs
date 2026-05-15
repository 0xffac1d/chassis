#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const testDir = dirname(fileURLToPath(import.meta.url));
const pkgRoot = join(testDir, '..');

const packOut = execFileSync('npm', ['pack', '--silent'], {
  cwd: pkgRoot,
  encoding: 'utf8',
});

const tarballName = packOut
  .trim()
  .split(/\r?\n/)
  .filter(Boolean)
  .pop();

if (!tarballName?.endsWith('.tgz')) {
  console.error(`installed-package: unexpected npm pack stdout: ${JSON.stringify(packOut)}`);
  process.exit(1);
}

const tarball = join(pkgRoot, tarballName);
const stageDir = mkdtempSync(join(tmpdir(), 'ch-core-types-installed-'));
try {
  execFileSync('npm', ['init', '-y'], { cwd: stageDir, stdio: 'inherit' });
  execFileSync('npm', ['install', tarball, '--no-audit', '--no-fund'], {
    cwd: stageDir,
    stdio: 'inherit',
  });
  const verifyScript = join(
    stageDir,
    'node_modules',
    '@chassis',
    'core-types',
    'scripts',
    'verify-fingerprint.mjs',
  );
  execFileSync(process.execPath, [verifyScript], { cwd: stageDir, stdio: 'inherit' });
  console.log('installed-package: OK');
} finally {
  rmSync(stageDir, { recursive: true, force: true });
  rmSync(tarball, { force: true });
}
