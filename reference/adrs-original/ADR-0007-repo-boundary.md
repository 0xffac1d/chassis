---
id: ADR-0007
title: Repository boundary — standalone distribution scope vs consumer scope
status: accepted
date: "2026-04-19"
enforces:
  - rule: BOUNDARY-CONSUMER-REF-IN-STANDALONE
    description: "A standalone-scope config file references consumer-only paths (e.g. crates/agent/, scripts/ci/check-route-alignment.sh) that don't exist in this distribution."
  - rule: BOUNDARY-LAYOUT-VIOLATION
    description: "A directory or file appears at a path forbidden by the standalone layout (e.g. docs/guides/ at repo root instead of docs/chassis/guides/)."
  - rule: BOUNDARY-TEMPLATE-NAMING
    description: "A consumer-scope template lacks the .consumer.* suffix that distinguishes it from standalone-scope configs."
  - rule: DOC-BINDING-PATH-MISSING
    description: "A document references a path (in `path:` or `applies_to:`) that does not exist."
  - rule: DOC-BINDING-PATH-INVALID
    description: "A document references a path with a syntactically invalid form (e.g. absolute path where a glob is expected)."
  - rule: DOC-BINDING-RATIO-EXCESSIVE
    description: "A module's docs/code ratio exceeds the baseline; the doc surface has grown faster than the code surface."
  - rule: SURFACE-WIRING-ROUTE-UNCLASSIFIED
    description: "A route, endpoint, or surface entry was added to a manifest without classification (internal/external/admin/...)."
  - rule: SURFACE-WIRING-ROUTE-DEAD
    description: "A route is listed in a manifest but has no source-side definition (handler) or vice versa."
  - rule: SURFACE-WIRING-ROUTE-UNTESTED
    description: "A route is classified as external but has no e2e or integration test referencing it."
  - rule: SURFACE-WIRING-SURFACE-COUNT-MISMATCH
    description: "The number of declared routes/endpoints in a manifest doesn't match the count derived from source."
applies_to:
  - "REPO_BOUNDARY.md"
  - "config/chassis/**"
  - "templates/ci/**"
  - "scripts/chassis/validate_distribution_layout.py"
tags:
  - chassis
  - governance
  - boundary
---

# ADR-0007: Repository boundary

## Context

This repository is the **standalone Chassis distribution**: schemas,
templates, CLI, gates, and the Rust binding crate. It does NOT ship the
downstream runtime, dashboard, deployment manifests, or product-specific
wiring. The boundary was documented in `REPO_BOUNDARY.md` but never
recorded as a formal decision; this ADR pins it and registers the
binding-link-relevant rule IDs.

This ADR also registers rule IDs for `doc_binding` and `surface_wiring`
gates because both speak to the boundary between "documented surface"
and "actual surface" — a topic adjacent to repository-scope discipline.

## Decision

1. **What this repo owns** is enumerated in REPO_BOUNDARY.md.
2. **What this repo does NOT own** is also enumerated. Configs and
   docs that reference consumer-only paths must use the `*.consumer.*`
   naming suffix and live under `templates/ci/gate-configs/` (not in
   the standalone-scope `config/chassis/`).
3. **Standalone-scope configs are minimal.** They target only what
   this repo's own gates need to validate. Consumer-scope expansions
   are shipped as templates that downstream repos adapt.
4. **`validate-distribution-layout` enforces the layout.** New
   directories at the repo root or under `docs/`, `config/`, `schemas/`,
   etc. are checked against the canonical map.

## Consequences

- The standalone repo is small and offline-installable; no consumer-
  specific dependencies leak in.
- Consumer-scope templates (`*.consumer.*`) document the expansion
  surface without forcing it on every adopter.
- The `doc_binding` and `surface_wiring` gates now have specific rule
  IDs and resolve to this ADR — every "documented vs actual" mismatch
  is bounded to a concrete rule.

## Status

Accepted. Layout enforcement shipped in Phase 0; this ADR formalizes
the policy.
