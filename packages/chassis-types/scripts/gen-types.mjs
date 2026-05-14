#!/usr/bin/env node
// Generate TypeScript types from chassis JSON schemas.
//
// - Walks <repo-root>/schemas/**/*.schema.json in sorted (deterministic) order.
// - Runs json-schema-to-typescript's compile() on each.
// - Writes per-schema .d.ts into dist/<domain>/<name>.d.ts (mirrors schema layout).
// - Emits a barrel dist/index.d.ts that re-exports every generated type.
// - Emits a tiny dist/index.js so "main": "dist/index.js" is resolvable at
//   runtime. The package is primarily a types-only package; the runtime
//   surface is an empty object.

import { readFileSync, writeFileSync, mkdirSync, readdirSync, statSync, rmSync, existsSync } from 'node:fs';
import { dirname, join, relative, sep, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

import { compile } from 'json-schema-to-typescript';

function repoRoot() {
  const env = process.env.CHASSIS_REPO_ROOT;
  if (env && env.length > 0) return env;
  const here = fileURLToPath(new URL('.', import.meta.url));
  return join(here, '..', '..', '..');
}

function pkgRoot() {
  const here = fileURLToPath(new URL('.', import.meta.url));
  return join(here, '..');
}

function walkSchemas(dir) {
  const out = [];
  const recurse = (d) => {
    for (const name of readdirSync(d)) {
      const full = join(d, name);
      const st = statSync(full);
      if (st.isDirectory()) recurse(full);
      else if (st.isFile() && name.endsWith('.schema.json')) out.push(full);
    }
  };
  recurse(dir);
  out.sort();
  return out;
}

// Convert "exemption-registry" -> "ExemptionRegistry"
function toPascalCase(name) {
  return name
    .split(/[-_]/)
    .filter(Boolean)
    .map((p) => p[0].toUpperCase() + p.slice(1))
    .join('');
}

// Derive the top-level TypeScript identifier for a schema file.
// exemption-registry.schema.json -> ExemptionRegistry
function topLevelName(schemaPath) {
  const base = basename(schemaPath).replace(/\.schema\.json$/, '');
  return toPascalCase(base);
}

async function main() {
  const root = repoRoot();
  const pkg = pkgRoot();
  const schemasDir = join(root, 'schemas');
  const distDir = join(pkg, 'dist');

  // Clean dist for deterministic output.
  if (existsSync(distDir)) rmSync(distDir, { recursive: true, force: true });
  mkdirSync(distDir, { recursive: true });

  const files = walkSchemas(schemasDir);
  const barrelLines = [];
  const runtimeLines = [];
  let ok = 0;
  let skipped = 0;

  for (const file of files) {
    const rel = relative(schemasDir, file).split(sep).join('/');
    const relNoExt = rel.replace(/\.schema\.json$/, '');
    const outRel = `${relNoExt}.d.ts`;
    const outPath = join(distDir, outRel);
    const raw = JSON.parse(readFileSync(file, 'utf8'));

    // json-schema-to-typescript picks the top-level name from $id first,
    // then title, then the name we pass. We want file-name-derived names
    // (ExemptionRegistry) so the exported API is predictable from the schema
    // path — strip both $id and title from the local copy before compile.
    // The on-disk schema is unchanged; the fingerprint script re-reads
    // from disk so this mutation does not affect the fingerprint.
    if ('$id' in raw) delete raw.$id;
    if ('title' in raw) delete raw.title;
    const typeName = topLevelName(file);

    let ts;
    try {
      ts = await compile(raw, typeName, {
        bannerComment:
          '/**\n * AUTO-GENERATED — do not edit.\n * Source: schemas/' + rel + '\n */',
        additionalProperties: false,
        style: { singleQuote: true },
        declareExternallyReferenced: true,
        // Set cwd so cross-file $refs (e.g. "capability.schema.json#/$defs/Capability")
        // resolve relative to the schema being compiled, not process.cwd().
        cwd: dirname(file),
        $refOptions: {},
      });
    } catch (err) {
      console.warn(`gen-types: skipping ${rel}: ${err.message}`);
      skipped += 1;
      continue;
    }

    mkdirSync(dirname(outPath), { recursive: true });
    writeFileSync(outPath, ts, 'utf8');

    // Barrel: re-export the top-level type under its bare name when the
    // name is unique across the schema set, and always re-export the
    // full module under a domain-qualified namespace for collision-free
    // access. Name collisions (e.g. multiple "Policy" schemas) are
    // resolved by the namespace form.
    //
    // Namespace identifier: "<domain>_<name>" (both PascalCase), e.g.
    // contract.schema.json becomes `Contract_Contract`.
    const importPath = './' + relNoExt;
    const domain = relNoExt.split('/')[0];
    const namespaceName = toPascalCase(domain) + '_' + typeName;
    barrelLines.push(
      `export * as ${namespaceName} from '${importPath}';`,
    );
    // Track the top-level type name for a post-pass that re-exports
    // only unique names at the bare top level.
    barrelLines.push(`// __TOPLEVEL__ ${typeName} ${importPath}`);
    ok += 1;
  }

  // Partition barrelLines: "export * as <ns> from <path>" stays, and
  // "// __TOPLEVEL__ <name> <path>" markers feed the bare-name pass.
  const nsExports = [];
  const topLevel = new Map(); // name -> [importPath]
  for (const line of barrelLines) {
    if (line.startsWith('// __TOPLEVEL__ ')) {
      const [, , name, path] = line.split(' ');
      if (!topLevel.has(name)) topLevel.set(name, []);
      topLevel.get(name).push(path);
    } else {
      nsExports.push(line);
    }
  }
  const bareExports = [];
  for (const [name, paths] of [...topLevel.entries()].sort()) {
    if (paths.length === 1) {
      bareExports.push(`export type { ${name} } from '${paths[0]}';`);
    }
    // else: collision — only accessible via the domain-qualified namespace.
  }

  // index.d.ts barrel.
  const header = [
    '/**',
    ' * AUTO-GENERATED — do not edit.',
    ' *',
    ' * Barrel re-exporting every generated schema type.',
    ' * - Bare names: top-level schema types with a globally unique name',
    ' *   (e.g. Contract, Diagnostic). Use `import { Contract }`.',
    ' * - Namespaced: every schema module is always available under a',
    ' *   domain-qualified namespace (e.g. Contract_Contract) so',
    ' *   collisions (multiple "Policy" schemas) remain reachable.',
    ' * - Generated from schemas at build time; re-run `npm run build`',
    ' *   after schema changes, and verify the fingerprint matches',
    ' *   `node scripts/fingerprint-schemas.mjs`.',
    ' */',
    '',
  ].join('\n');
  const body = [
    '// --- Bare top-level re-exports (collision-free names only) ---',
    ...bareExports,
    '',
    '// --- Namespaced re-exports (every schema, always reachable) ---',
    ...nsExports,
  ].join('\n');
  writeFileSync(join(distDir, 'index.d.ts'), header + body + '\n', 'utf8');

  // Runtime index.js — types-only package, nothing to export.
  const jsBody = [
    '// AUTO-GENERATED — types-only package; no runtime surface.',
    "'use strict';",
    'module.exports = {};',
    '',
  ].join('\n');
  writeFileSync(join(distDir, 'index.js'), jsBody, 'utf8');

  console.log(`gen-types: generated ${ok} schema modules (${skipped} skipped) in ${relative(pkg, distDir)}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
