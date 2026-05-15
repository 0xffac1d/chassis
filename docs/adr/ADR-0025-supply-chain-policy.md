---
id: ADR-0025
title: "Supply-chain policy: license allowlist, advisory denial, serde_yaml retention"
status: accepted
date: "2026-05-15"
supersedes: []
enforces:
  - rule: CH-SUPPLY-LICENSE-ALLOW
    description: "Workspace dependency licenses must match the SPDX allowlist in deny.toml."
  - rule: CH-SUPPLY-ADVISORY-CLEAN
    description: "RustSec advisories block CI unless individually justified in deny.toml."
  - rule: CH-SUPPLY-NO-NETWORK-CRATES
    description: "openssl/native-tls/reqwest/hyper/tokio are explicitly banned in deny.toml."
  - rule: CH-SUPPLY-ARCHIVE-HYGIENE
    description: "Source archives are produced from `git archive` and rejected on build/cache artifacts."
---

# ADR-0025 — Supply-chain policy

## Context

The workspace ships three Rust crates and one TypeScript package and intends
to publish to crates.io and npm. Before that, we need:

- a written license policy that fails CI if a transitive dependency switches
  to a license we cannot ship under (`MIT OR Apache-2.0`);
- a vulnerability gate (RustSec advisories) that blocks publication;
- an explicit set of crates we refuse to depend on (network/TLS/async
  runtimes) because the kernel is sync- and offline-by-design;
- archive hygiene so accidental tarballs cannot leak `.git/`, `target/`,
  `node_modules/`, or local absolute paths into a release.

The workspace also depends on `serde_yaml 0.9.34+deprecated`, which carries
the RustSec informational advisory **RUSTSEC-2024-0320 ("unmaintained")**.
This ADR records the decision to keep it for the current wave and the
trigger that retires the exemption.

## Decision

### License policy

`deny.toml` allows only permissive SPDX expressions:

```
MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause,
BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, CC0-1.0,
MPL-2.0, 0BSD, MIT-0
```

Anything else fails the build. Per-crate clarifications live in
`[[licenses.clarify]]` blocks with a justifying comment. Copyleft licenses
(GPL, LGPL, AGPL) are rejected by omission.

### Advisory policy

`cargo audit` and `cargo deny check advisories` run on every CI build.
Advisory ignores require an `id` plus a `reason` and a link to a follow-up
(an ADR or an issue).

### Banned crates

Hard-banned in `deny.toml`:
`openssl`, `openssl-sys`, `native-tls`, `reqwest`, `hyper`, `tokio`,
`async-std`. The kernel performs only local file IO and uses
`vendored-libgit2` for repository operations; if any of the above appear
transitively, that's a dependency choice that needs a follow-up ADR, not
a silent inclusion.

### serde_yaml retention

We keep `serde_yaml 0.9.34+deprecated` for now and ignore RUSTSEC-2024-0320
in `deny.toml`. The reasoning:

1. **No drop-in replacement preserves byte-for-byte parse behavior across
   our fixtures.** The validator, diff, exempt, drift, and trace test
   suites all parse YAML into `serde_json::Value` via `serde_yaml`. The
   community forks (`serde-yml`, `serde_yaml_ng`, `serde_norway`) each
   diverge in tag handling, anchor expansion, or numeric coercion. Swapping
   would require re-baselining ~30 fixtures and is out of scope for the
   current wave.
2. **The advisory is informational, not a vulnerability.** Upstream marked
   the crate "no further development planned"; the implementation is
   `unsafe-libyaml`-backed and battle-tested. There is no known
   memory-safety or correctness defect at the pinned version.
3. **The blast radius is bounded.** YAML enters the system only via
   developer-authored contracts and exemption files in trusted git
   working trees, never from untrusted network input. The threat model
   for a YAML-parser CVE is therefore "developer DoSes their own CI."
4. **The cost of a swap is non-trivial.** Tests (`exempt::tests`,
   `diff::tests`, `drift::git`, `trace::graph`) compare deserialized YAML
   structures and would need a parity sweep against the new parser.

#### Exit criteria (when to revisit)

We revisit `serde_yaml` when **any** of the following becomes true:

- A non-informational RustSec advisory lands against `serde_yaml` or
  `unsafe-libyaml`.
- The kernel grows a YAML-parsing path that consumes untrusted input
  (e.g. an MCP tool that accepts user-supplied YAML over the wire).
- A YAML parser fork stabilizes a feature flag for serde_yaml-compatible
  output, removing the fixture-divergence cost.
- We start work on the planned WASM bindings (`packages/chassis-core-wasm`),
  where pulling in `unsafe-libyaml` C code adds binary-size pressure.

When the trigger fires, the migration plan is:

1. Pin both old and new parsers behind a `cfg` flag.
2. Run the workspace test suite under both for one wave.
3. Promote the replacement to default; remove `serde_yaml`.
4. Drop the `RUSTSEC-2024-0320` ignore from `deny.toml`.

### Archive hygiene

Source archives for release are produced via `scripts/build-source-archive.sh`,
which (a) uses `git archive` so untracked files are excluded by construction,
and (b) runs `scripts/check-archive-hygiene.sh` against the produced
tarball. The hygiene script fails on:

- root `.git/`
- `target/` at any depth
- `node_modules/` at any depth
- `__pycache__/` at any depth
- `*.pyc` files
- absolute developer-machine paths under the prior project's mount
  (matched by the regex baked into the hygiene script) anywhere in the
  tree **except** under `reference/`, where they are intentionally
  preserved as a snapshot of the prior project's filesystem layout.
  `reference/historical/` is the canonical home for aged-out claims;
  `reference/artifacts/` and `reference/docs-original/` may also retain
  the prior layout's literal paths because the entire `reference/` tree
  is study-only and never imported by active code.

Both checks run in CI (`supply-chain` job).

## Consequences

- License churn in the dependency graph fails CI immediately rather than
  surfacing during a release attempt.
- Adding a network-touching dependency requires either an explicit
  `deny.toml` removal (with this ADR's review) or a new ADR explaining
  why the kernel needs network IO.
- The `serde_yaml` exemption is auditable: `deny.toml` cites this ADR by
  filename, and the exit criteria are written down.
- Source archives shipped from CI cannot accidentally include build
  output, vendored caches, Python bytecode, or developer-machine paths.
- **Trace id:** `chassis.archive-self-verifying` ties self-verifying `git archive`
  extracts (including `CLAUDE.md`, `.gitignore`, and `.github/workflows/`) to this policy.

## Alternatives considered

- **Allow the `Unlicense` SPDX**: rejected. It is contentious and not
  needed by the current dep graph; revisit only if a required transitive
  dep adopts it.
- **Migrate to `serde-yml` immediately**: rejected for this wave. See
  point 1 above; the fixture parity work is the bottleneck, not the
  swap itself.
- **Skip `cargo deny` and rely only on `cargo audit`**: rejected. Only
  cargo-deny enforces the license allowlist and the banned-crates list,
  which we want now and not after the first surprise PR.
