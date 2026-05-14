#!/usr/bin/env node
// Compare the build-time fingerprint of the schemas tree against the
// committed fingerprint.sha256. Exit 1 on mismatch.
//
// Runs automatically during `prepublishOnly`. Also useful as a
// consumer-side gate: after `pnpm add @chassis/types`, a consumer CI
// job can run `node node_modules/@chassis/types/scripts/verify-fingerprint.mjs`
// to confirm the types they installed were generated from the same
// schemas they pinned.

import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { buildManifest, manifestHash } from './fingerprint-schemas.mjs';

function pkgRoot() {
  const here = fileURLToPath(new URL('.', import.meta.url));
  return join(here, '..');
}

function repoRoot() {
  const env = process.env.CHASSIS_REPO_ROOT;
  if (env && env.length > 0) return env;
  return join(pkgRoot(), '..', '..');
}

const fpPath = join(pkgRoot(), 'fingerprint.sha256');
if (!existsSync(fpPath)) {
  console.error('verify-fingerprint: fingerprint.sha256 not found; run `npm run build` first.');
  process.exit(2);
}

const committed = readFileSync(fpPath, 'utf8').split(/\s+/)[0];
const current = manifestHash(buildManifest(repoRoot()));

if (committed !== current) {
  console.error('verify-fingerprint: DRIFT — schemas diverge from committed fingerprint.');
  console.error(`  committed: ${committed}`);
  console.error(`  current:   ${current}`);
  console.error('Refresh with `npm run build` and re-commit fingerprint.sha256.');
  process.exit(1);
}

console.log(`verify-fingerprint: OK (${current})`);
