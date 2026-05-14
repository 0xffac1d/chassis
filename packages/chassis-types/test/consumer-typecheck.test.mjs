#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const testDir = dirname(fileURLToPath(import.meta.url));
const pkgRoot = join(testDir, '..');
const fixture = join(pkgRoot, 'test', 'fixtures', 'consumer');

const run = (cmd, args, cwd) => {
  execFileSync(cmd, args, { cwd, stdio: 'inherit' });
};

run('npm', ['install', '--no-audit', '--no-fund'], fixture);
run('npm', ['run', 'typecheck'], fixture);

console.log('consumer-typecheck: OK (file: dependency + tsc --noEmit)');
