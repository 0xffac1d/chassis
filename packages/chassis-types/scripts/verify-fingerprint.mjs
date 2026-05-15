#!/usr/bin/env node
// Compare the build-time fingerprint of the schemas tree against the
// committed fingerprint.sha256. Exit 1 on mismatch.
//
// Runs automatically during `prepublishOnly`. Consumers can run after install:
//   node node_modules/@chassis/core-types/scripts/verify-fingerprint.mjs
// When shipped without a repo `schemas/` tree, verifies against bundled `manifest.json`.

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

const repo = repoRoot();
const schemasRoot = join(repo, 'schemas');
let current;
if (existsSync(schemasRoot)) {
  current = manifestHash(buildManifest(repo));
} else {
  const manifestPath = join(pkgRoot(), 'manifest.json');
  if (!existsSync(manifestPath)) {
    console.error(
      'verify-fingerprint: no schemas/ beside repo root and bundled manifest.json missing; cannot verify.',
    );
    process.exit(2);
  }
  const bundled = JSON.parse(readFileSync(manifestPath, 'utf8'));
  current = manifestHash(bundled);
}

if (committed !== current) {
  console.error('verify-fingerprint: DRIFT — schemas diverge from committed fingerprint.');
  console.error(`  committed: ${committed}`);
  console.error(`  current:   ${current}`);
  console.error('Refresh with `npm run build` and re-commit fingerprint.sha256 (+ manifest.json).');
  process.exit(1);
}

console.log(`verify-fingerprint: OK (${current})`);
