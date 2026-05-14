# Decisions: Chassis distribution

The repo-root `CONTRACT.yaml` records *what* invariants the distribution
holds. This file records *why* those invariants are shaped the way they
are, plus distribution-wide design choices that span more than one
subsystem.

## Why this repository exists

Chassis is the canonical home for the metadata-governance substrate:
JSON Schemas, structure rules, blueprints, templates, generated
multi-language bindings (`crates/chassis-schemas/` Rust + committed
`@chassis/types`), and the `scripts/chassis/chassis` CLI plus expansion
gate suite. Application code, runtime services, and product schemas
live in downstream consumer repositories — see `REPO_BOUNDARY.md`.

## Why standalone vs full-product scope is split everywhere

Every governance artifact has two scopes:

* **Standalone-distribution scope** (this repo) — `config/chassis/*.toml`,
  `config/chassis/baselines/*.json`. Targets only what this repo owns.
* **Full-product scope** (consumer repos) — shipped under
  `templates/ci/gate-configs/*.consumer.*`. Consumers copy and adapt.

Mixing the two leaks product-runtime references into the distribution
(e.g. `crates/agent/`, `docs/security/`, route-alignment scripts) and
makes the standalone gate non-portable. The split is enforced by
`validate-distribution-layout`, the `release-standalone-gate`
root-separation check, and AGENTS.md "Boundaries — what to NOT do".

## Why `chassis-schemas` is the only canonical generated tree

Every emitter (`chassis codegen --lang <tgt>`) writes deterministic
output. Only the Rust crate `crates/chassis-schemas/` and the
committed `@chassis/types` tree under `codegen/ts-types/dist/` are
class-1 (committed, fingerprinted). All other languages
(`generated/{python,csharp,go,typespec,typescript}/`) are class-2:
on-demand only, gitignored. This keeps Rust workspace builds
hermetic without dragging the codegen dependency graph into every
language toolchain, and lets language consumers regenerate bindings
locally without invading the repo.

## Why baselines are `--update-baseline`-driven, not policy-driven

The `decomposition` and `panic-budget` gates compare current
counts against `config/chassis/baselines/*.json`. Both gates ship
`--update-baseline` and `--ratchet` flags: bumping a baseline is
explicit and reviewable, never automatic. Baselines for
`aux-chassis-schemas` (164 panic sites; 27 crate-root pub mods) are
dominated by codegen output — each generated validator wrapper
contains two `LazyLock` `.expect(` calls that fail-fast on bundled
schema corruption. Re-ratchet whenever the schema set changes.

## Why every gate emits the diagnostic schema

`schemas/diagnostics/diagnostic.schema.json` is the machine contract
for agent-routable output. Every gate's `--json` mode emits findings
that conform: `ruleId`, `severity`, `message`, optional
`violated.convention` (ADR id), `docs`, `fix.applicability`,
`location.range`. The `binding-link` gate enforces that every
emitted `ruleId` resolves to an ADR's `enforces[].rule`. Two
consequences:

* Gates that share the diagnostic shape can be consumed by any
  agent without bespoke parsing.
* Adding a new diagnostic requires either an existing ADR
  with a matching `enforces[]` entry or a new ADR shipped in the
  same change set — drift is impossible.

## Why `release-gate` and `release-standalone-gate` are separate

`release-gate` runs the expansion-pack suite configured in
`scripts/chassis/gates/release_gate.py` (**19** subprocess gates today, including claim-drift …
quarter-review; see [`docs/chassis/guides/expansion-pack.md`](docs/chassis/guides/expansion-pack.md)).
It surfaces governance regressions during normal CI.

`release-standalone-gate` is the certification path that proves the
chassis distribution is internally consistent without any sibling
checkout: it runs python-import-safety, validate-termination,
python-wheel-smoke, fixture-matrix, strict-enforcement,
generated-artifacts, runtime-package-posture, docs-consistency,
release-public-surface-audit, permissions, and root-separation.
`SPLIT_READINESS.md` / `STANDALONE_READINESS_REPORT.{md,json}` are regenerated
pointers—the signed JSON at `ARTIFACTS/chassis/standalone-release-gate.json`
is the authority for the verdict and commit pin.

## Why posture/surface gates emit info (not warn) when scope is standalone

`config/chassis/posture.toml` and `config/chassis/surface.toml` carry
a `[meta] scope = "standalone-distribution"` marker. When the gate
sees that marker and the matching data is empty, it emits an
`info`-level diagnostic instead of a warn. Consumer-scope copies (in
`templates/ci/gate-configs/`) omit the scope marker, so an empty
field-set there still surfaces as a configuration warning.

## Why scaffold output contains TODO placeholders

`chassis scaffold` emits CONTRACT.yaml / DECISIONS.md / ADR / projection
templates that intentionally include `TODO` strings, sentinel
placeholders, and `Replace this …` markers. Bootstrap *standard mode*
accepts those placeholders so the gate suite passes on a fresh
scaffold; *strict mode* refuses to certify them. The
`agent-tell` gate distinguishes `must` placeholders (block) from
`should` placeholders (warn) and reads `crates/<crate>/.chassis/placeholders.json`
to reconcile resolved entries. See `skills/chassis-metadata/SKILL.md`.

## Why `chassis-runtime` ships as a placeholder shell

The `crates/chassis-runtime/` API surface (introspect, exemptions,
diagnostics, assurance) is shipped as a thin shell with
documented stub semantics: `coherence_report()` returns a fixed
placeholder, `check_exemption()` returns `Exemption::Denied`, and
each emits a one-shot warning diagnostic
(`RUNTIME-INTROSPECT-SCAFFOLD`, `RUNTIME-EXEMPTIONS-SCAFFOLD`).
Consumers can wire a real backend by implementing the
`chassis-runtime-api` traits without rebuilding the shell. The
crate's README explicitly says "most functions are placeholder
shells" so callers fail closed by default rather than mistaking
the shell for production policy enforcement. Real backends (TOML
exemption registry, per-component attestation rollup, runtime
coherence discovery) ship as follow-up engineering work items—use
[`RELEASE-CHECKLIST.md`](RELEASE-CHECKLIST.md) and downstream trackers for concrete next steps.

## Why memory-discipline is a conditional block in agent-rule templates

Agent-rule templates (`templates/structure-agent-rules/`) ship a structure-discipline
block that teaches agents how to use `chassis scaffold` and `chassis audit`. A second
optional block — memory-discipline — teaches agents how to use a persistent-memory
provider (MCP server, vault, etc.) responsibly: search before answering, store
sparingly, two-tier retrieval, no secrets in memory.

The block is source-agnostic: it names capability-style operations
(`search_memory`, `load_task_context`, `store_memory`) rather than vendor-specific
tool identifiers. Consumers map these to their concrete tool names.

Emission is controlled by a `memory_augmented: bool` parameter (default `false`) on
`emit_agent_rules()`. When false, a post-processing pass strips the
`CHASSIS-MEMORY-DISCIPLINE` marker block from the rendered template before writing.
This avoids burdening non-memory-augmented consumers with irrelevant rules while
keeping the templates maintainable in a single source of truth.

## References

* `REPO_BOUNDARY.md` — what stays in chassis vs downstream consumer repos
* `AGENTS.md` — canonical agent instructions (CLAUDE.md, .cursor, .windsurfrules etc are projections)
* `docs/adr/` — authoritative rule-id → ADR linkage
* `RELEASE-CHECKLIST.md` — tag-time procedure
* `STANDALONE_READINESS_REPORT.{md,json}` — regenerated mirror of standalone release evidence at repo root
