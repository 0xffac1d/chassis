#!/usr/bin/env node
// Compute a SHA-256 fingerprint over the canonical chassis schemas.
//
//   - Walks <repo-root>/schemas/**/*.schema.json in sorted order.
//   - Canonicalizes each (RFC 8785 JCS-shaped), SHA-256 each.
//   - Builds {version:1, kind:"chassis-schemas-manifest", count, entries}.
//   - Canonicalizes the manifest, SHA-256 it — that's the fingerprint.
//   - Writes fingerprint.sha256 next to package.json.

import { readFileSync, writeFileSync, readdirSync, statSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalize } from './canonicalize.mjs';

// _canonical_subject keep-list, preserved in the same order as the Python
// source (order is immaterial because we sort keys on output, but we keep
// it aligned for audit).
const KEEP_KEYS = [
  '$id',
  'type',
  'required',
  'properties',
  'additionalProperties',
  '$defs',
  'definitions',
  'oneOf',
  'anyOf',
  'allOf',
  'enum',
  'items',
  'patternProperties',
  'propertyNames',
  'const',
  'minimum',
  'maximum',
  'minLength',
  'maxLength',
  'pattern',
  'format',
  'version',
];

function canonicalSubject(content) {
  const keep = {};
  for (const k of KEEP_KEYS) {
    if (Object.prototype.hasOwnProperty.call(content, k)) {
      keep[k] = content[k];
    }
  }
  return keep;
}

function repoRoot() {
  const env = process.env.CHASSIS_REPO_ROOT;
  if (env && env.length > 0) return env;
  // package lives at <repo>/packages/chassis-types/
  const here = fileURLToPath(new URL('.', import.meta.url));
  return join(here, '..', '..', '..');
}

function iterSchemaFiles(schemasDir) {
  // Match Python's Path.rglob('*.schema.json') in sorted order.
  // Python's rglob yields sorted descendants when we sort() the result.
  const out = [];
  const walk = (dir) => {
    let entries;
    try {
      entries = readdirSync(dir);
    } catch {
      return;
    }
    for (const name of entries) {
      const full = join(dir, name);
      let st;
      try {
        st = statSync(full);
      } catch {
        continue;
      }
      if (st.isDirectory()) walk(full);
      else if (st.isFile() && name.endsWith('.schema.json')) out.push(full);
    }
  };
  walk(schemasDir);
  // Python sorts with PosixPath ordering — lexicographic on the full path.
  // relative() gives us the same sort key if we normalize separators.
  out.sort();
  return out;
}

function sha256Hex(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

export function buildManifest(root) {
  const schemasDir = join(root, 'schemas');
  const files = iterSchemaFiles(schemasDir);
  const entries = [];
  for (const path of files) {
    const rel = relative(root, path).split(sep).join('/');
    const raw = JSON.parse(readFileSync(path, 'utf8'));
    const subject = canonicalSubject(raw);
    const payload = canonicalize(subject);
    const entryHash = sha256Hex(Buffer.from(payload, 'utf8'));
    entries.push({ path: rel, sha256: entryHash });
  }
  return {
    version: 1,
    kind: 'chassis-schemas-manifest',
    count: entries.length,
    entries,
  };
}

export function manifestHash(manifest) {
  return sha256Hex(Buffer.from(canonicalize(manifest), 'utf8'));
}

function main() {
  const root = repoRoot();
  const manifest = buildManifest(root);
  const digest = manifestHash(manifest);

  const here = fileURLToPath(new URL('.', import.meta.url));
  const pkgRoot = join(here, '..');
  const outPath = join(pkgRoot, 'fingerprint.sha256');
  writeFileSync(outPath, `${digest}  chassis-schemas-manifest\n`, 'utf8');
  console.log(`fingerprint-schemas: wrote ${relative(pkgRoot, outPath)} (${digest})`);
  console.log(`fingerprint-schemas: ${manifest.count} schemas under ${relative(pkgRoot, join(root, 'schemas'))}`);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
