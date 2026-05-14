---
id: ADR-0021
title: Capability marker proc-macro for stable Rust surface fingerprinting
status: accepted
date: "2026-04-26"
enforces:
  - rule: CAPABILITY-MARKER-CANONICAL
    description: "Stable Rust capability surface items are marked with #[chassis::capability], not doc-comment substrings."
  - rule: CAPABILITY-MARKER-ATTRIBUTE-MODE
    description: "Capability fingerprint attribute mode inspects rustdoc JSON for the canonical marker (attrs when present, else docs substring from the proc-macro doc line)."
applies_to:
  - "crates/chassis-capability-derive/**"
  - "crates/chassis-runtime-api/**"
  - "scripts/chassis/scripts/emit_capability_fingerprint.py"
supersedes: []
tags:
  - chassis
  - capability
  - fingerprint
  - rust
  - proc-macro
---

# ADR-0021: Capability marker proc-macro

## Context

The capability fingerprint is the release-side identity for a crate's
stable public surface. It feeds drift detection, app-composition
`consumes:` / `provides:` checks, and schema breaking-change review, so
the input must change if and only if a stable surface item is added,
removed, or changed.

The initial emitter supported a `docline` heuristic: any rustdoc item
whose docs contained the substring `chassis:capability` was included.
That is useful for bootstrapping but too weak as the canonical signal.
Doc comments are prose, easy to copy accidentally, easy to spoof, and
not tied to the Rust item grammar. A marker that drives compatibility
decisions needs to be intentional syntax on the item being fingerprinted.

ADR-0016 records the lifecycle for deferred module extractions; it does
not define a capability marker. ADR-0015 establishes the parallel schema
fingerprint pipeline. This ADR supplies the Rust capability-surface marker
that complements that schema identity work.

## Decision

1. **`#[chassis::capability]` is the canonical marker.** Public Rust
   items that form a crate's stable capability surface are annotated with
   this attribute.
2. **The consumer-facing path is re-exported by `chassis-runtime-api`.**
   Consumers depend on one stable API crate and may alias it as
   `chassis`, preserving the intended `#[chassis::capability]` path.
3. **`chassis-capability-derive` owns the proc-macro implementation.**
   The proc-macro is published as a separate Rust crate because
   proc-macros must live in a crate with `proc-macro = true`.
4. **v0.1 leaves a rustdoc-visible marker without changing semantics.** The
   macro prepends `#[doc = "<!-- chassis:capability -->"]` so the docs
   field contains the canonical substring, because current rustdoc JSON often
   omits proc-macro attributes from `attrs`. The emitter’s `attribute` mode
   matches `attrs` when present and otherwise the same substring in `docs`.
5. **Arguments are reserved.** v0.1 rejects all arguments so future forms
   such as `#[chassis::capability(stable_since = "1.2.0")]` can be added
   deliberately instead of being silently ignored today.

## Consequences

- Consumers add `#[chassis::capability]` to public items they want
  included in capability fingerprints.
- The emitter's `--marker attribute` mode prefers rustdoc JSON `attrs` and
  falls back to the `docs` substring from the proc-macro.
- `docline` mode remains supported as a fallback for one minor release
  after the attribute path is available, then is removed.
- The marker crate remains small and operationally separate from runtime
  behavior; generated code or richer marker metadata is deferred to a
  future ADR or revision of this one.

## Alternatives considered

- **Keep docline as canonical.** Rejected because comments are not a
  stable machine contract and can include accidental or spoofed marker
  substrings.
- **Define a plain inert Rust attribute without a proc-macro.** Rejected
  because stable Rust requires attributes in this position to resolve.
  Consumers need a real attribute they can compile today.
- **Expose the proc-macro crate directly to consumers.** Rejected because
  it makes every consumer depend on two crates for one API surface and
  obscures the intended `chassis::capability` path.

## References

- ADR-0016 (deferred extractions): lifecycle context for module
  extraction and stable capability review.
- ADR-0015 (schema fingerprint identity): the schema fingerprint pipeline
  this capability fingerprint complements.
- ADR-0008 (schema versioning): compatibility discipline for schema
  changes that capability fingerprints help enforce at composition time.
- `scripts/chassis/scripts/emit_capability_fingerprint.py`: fingerprint
  emitter with `docline` and `attribute` marker modes.

## Status

Accepted. The proc-macro and runtime API re-export ship in Chassis 0.1.

### Status update — 2026-04-26

`emit_capability_fingerprint.py` defaults to `--marker attribute` (was
`docline`). `chassis-runtime-api` and `chassis-runtime` public surface items
carry `#[chassis::capability]`; baseline `capability.fingerprint.json` sidecars
are committed. `docline` remains available via `--marker docline` for one
minor; removal is tracked with the ADR “Consequences” follow-up.
