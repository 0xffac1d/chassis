#!/usr/bin/env node
// ADR-0008 gate: every canonical schemas/**/*.schema.json must declare
// `$schema`, `$id`, `version` (semver MAJOR.MINOR.PATCH), and `title`.
//
// Exit codes:
//   0  all canonical schemas conform
//   1  one or more schemas violate ADR-0008 (details printed to stderr)
//   2  internal error (cannot read schemas dir, malformed JSON, ...)

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const REQUIRED_KEYS = ['$schema', '$id', 'version', 'title'];
const SEMVER_RE = /^\d+\.\d+\.\d+$/;

function repoRoot() {
  const env = process.env.CHASSIS_REPO_ROOT;
  if (env && env.length > 0) return env;
  const here = fileURLToPath(new URL('.', import.meta.url));
  return join(here, '..', '..', '..');
}

function listSchemas(schemasDir) {
  const out = [];
  const walk = (dir) => {
    let entries;
    try {
      entries = readdirSync(dir);
    } catch (e) {
      console.error(`verify-schema-metadata: cannot read ${dir}: ${e.message}`);
      process.exit(2);
    }
    for (const name of entries) {
      const full = join(dir, name);
      const st = statSync(full);
      if (st.isDirectory()) walk(full);
      else if (st.isFile() && name.endsWith('.schema.json')) out.push(full);
    }
  };
  walk(schemasDir);
  out.sort();
  return out;
}

function checkSchema(absPath, root) {
  const rel = relative(root, absPath).split(sep).join('/');
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(absPath, 'utf8'));
  } catch (e) {
    return [{ path: rel, problem: `malformed JSON: ${e.message}` }];
  }
  const findings = [];
  for (const key of REQUIRED_KEYS) {
    if (!Object.prototype.hasOwnProperty.call(parsed, key)) {
      findings.push({ path: rel, problem: `missing top-level "${key}"` });
      continue;
    }
    const value = parsed[key];
    if (typeof value !== 'string' || value.length === 0) {
      findings.push({ path: rel, problem: `"${key}" must be a non-empty string` });
    }
  }
  if (typeof parsed.version === 'string' && !SEMVER_RE.test(parsed.version)) {
    findings.push({
      path: rel,
      problem: `"version" "${parsed.version}" is not MAJOR.MINOR.PATCH semver`,
    });
  }
  return findings;
}

function main() {
  const root = repoRoot();
  const schemasDir = join(root, 'schemas');
  const files = listSchemas(schemasDir);
  if (files.length === 0) {
    console.error(`verify-schema-metadata: no *.schema.json under ${schemasDir}`);
    process.exit(2);
  }
  const violations = [];
  for (const f of files) violations.push(...checkSchema(f, root));
  if (violations.length === 0) {
    console.log(`verify-schema-metadata: OK (${files.length} canonical schemas)`);
    return;
  }
  console.error(`verify-schema-metadata: ${violations.length} ADR-0008 violation(s):`);
  for (const v of violations) console.error(`  ${v.path}: ${v.problem}`);
  process.exit(1);
}

main();
