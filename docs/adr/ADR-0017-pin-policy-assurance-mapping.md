---
id: ADR-0017
title: Pin-policy mapping — assurance ladder determines permitted pin kinds
status: accepted
date: "2026-04-20"
enforces:
  - rule: PIN-POLICY-ASSURANCE-UNKNOWN
    description: "app.yaml declares an assurance_level outside {bronze, silver, gold}; the mapping is typed."
  - rule: PIN-POLICY-BRONZE-PATH-UNMARKED
    description: "Bronze app pins a module from a path source without metadata.lifecycle=experimental; advisory warning on unreviewed path deps."
  - rule: PIN-POLICY-SILVER-RANGE-PIN
    description: "Silver app pins any module with pin=range; silver requires exact or sha."
  - rule: PIN-POLICY-SILVER-FINGERPRINT-MISSING
    description: "Silver app pins a module whose capability fingerprint sidecar is absent; warn-only."
  - rule: PIN-POLICY-GOLD-NON-SHA-PIN
    description: "Gold app pins any module with pin != sha; gold requires sha-pinning uniformly."
  - rule: PIN-POLICY-GOLD-FINGERPRINT-MISSING
    description: "Gold app pins a module whose capability fingerprint sidecar is absent; gold is hard-fail."
  - rule: PIN-POLICY-GOLD-TIER1-ATTESTATION-MISSING
    description: "Gold app composes from a chassis release without a verifiable tier-1 in-toto/cosign attestation; the supply-chain root is not anchored."
applies_to:
  - "schemas/app/app.schema.json"
  - "scripts/chassis/generate_app_lock.py"
  - "docs/chassis/guides/pin-policy-enforcement.md"
tags:
  - chassis
  - governance
  - assurance
  - pin-policy
  - app-composition
  - supply-chain
---

# ADR-0017: Pin-policy assurance mapping

## Context

ADR-0002 defined the assurance ladder (bronze / silver / gold) as a
posture concept. ADR-0012 introduced `app.yaml` + `app.lock` and the
`modules[].pin` field. ADR-0014 set governance tiers keyed on
capability. The blueprint repeatedly referenced "gold ⇒ sha-pinned,
bronze ⇒ range allowed" as the enforcement rule — but never landed
the machine-readable mapping. Without one, the app-level gate
introduced in ADR-0012 has nothing concrete to enforce, and
`generate_app_lock.py` has to guess what "assurance level" means for
its input validation step.

The cost of leaving this implicit is asymmetric: silver and gold
consumers who *think* they are enforcing a pin floor will discover
they aren't, at the moment a range-pinned module floats under them.
The cost of pinning it down is a table and an enforcement pass.

## Decision

1. **Three assurance levels, closed set.** `bronze`, `silver`,
   `gold`. Any other value in `app.yaml`'s `assurance_level` field
   fails the app-composition gate with
   `PIN-POLICY-ASSURANCE-UNKNOWN`.
2. **Canonical mapping table.** Each row pins permitted `pin`
   values, required source kinds, and the governance tier the app
   is composed against:

   | Assurance | Permitted `pin` values | Required source kinds | Registry tier |
   |---|---|---|---|
   | `bronze` | `range`, `exact`, `sha` | git, registry, path (experimental only) | any |
   | `silver` | `exact`, `sha` | git, registry | tier-3 minimum |
   | `gold` | `sha` | git (with rev pinning), registry (exact version + resolved checksum) | tier-1 required |

3. **Bronze enforcement.** Zero mandatory pinning. The only
   enforcement is an **advisory warning** when a module is sourced
   from a `path` dependency without
   `metadata.lifecycle=experimental`; this catches accidental path
   deps that leak into non-experimental apps. Rule:
   `PIN-POLICY-BRONZE-PATH-UNMARKED` (warn, not fail).
4. **Silver enforcement.** The gate rejects `app.yaml` if any
   `modules[].pin == "range"` (`PIN-POLICY-SILVER-RANGE-PIN`,
   hard-fail). It additionally warns (`warn-only`) if a pinned
   module has no capability fingerprint sidecar
   (`PIN-POLICY-SILVER-FINGERPRINT-MISSING`). Silver is composed
   against chassis releases that meet tier-3 at minimum — the
   schema fingerprint identity from ADR-0015 must be verifiable.
5. **Gold enforcement.** The gate rejects `app.yaml` unless **all**
   three conditions hold:
   - every module is sha-pinned
     (`PIN-POLICY-GOLD-NON-SHA-PIN`, hard-fail);
   - every module has a capability fingerprint sidecar
     (`PIN-POLICY-GOLD-FINGERPRINT-MISSING`, hard-fail);
   - the chassis release the app composes from has a verifiable
     tier-1 in-toto/cosign attestation at app.lock generation time
     (`PIN-POLICY-GOLD-TIER1-ATTESTATION-MISSING`, hard-fail).
6. **Enforcement surface is the generator, not the schema.**
   `schemas/app/app.schema.json` remains the *structural* contract:
   what fields exist, what types they have. The assurance →
   pin-kind mapping is *enforcement*: semantic validation done by
   `generate_app_lock.py` before writing `app.lock`. Keeping the
   schema shape-only avoids encoding three parallel copies of the
   schema for one axis.
7. **Exit contract of the generator.** `generate_app_lock.py` exits
   `0` on success, `2` on any hard-fail rule (the caller
   distinguishes pin-policy failures from IO failures), and
   `1` is reserved for tool-side errors. Warnings print to stderr
   with the rule ID and never affect the exit code.
8. **No silent downgrade.** The generator never downgrades
   assurance to make validation pass. If `app.yaml` declares `gold`
   and any hard-fail fires, the generator exits `2`; it never
   rewrites the level or emits a partial lock.

### Reference fixture surface

The app.lock generator's test surface is expected to include
`tests/fixtures/pin_policy/` with one fixture per row of the
table (bronze range-ok, silver range-fails, gold sha-ok, gold
range-fails, gold missing-fingerprint-fails, gold
missing-attestation-fails, bronze path-without-experimental-warns,
silver missing-fingerprint-warns). This ADR does **not** author
those fixtures — the app.lock agent owns them; the surface is
referenced here only to make the enforcement requirement concrete.

## Consequences

- `generate_app_lock.py` gains a semantic validation pass it
  previously had to improvise. The rule IDs above are the stable
  surface the gate emits.
- `app.yaml` authors get one clear table to consult; no more
  guessing what "assurance: gold" means for pin values.
- The composition gate in ADR-0012 now has concrete rules to
  enforce. Previously it was a shape-only gate; with ADR-0017 it
  is a shape-plus-semantics gate.
- Silver's warn-only fingerprint rule is intentional: the middle
  tier is the migration bridge, not the terminal state. Authors
  who want hard-fail on missing fingerprints opt up to gold.
- Gold's tier-1 attestation requirement tightens the supply-chain
  root: a gold app composed against a tier-3 chassis release will
  fail the gate, preventing "gold in name only" configurations.
- `schemas/app/app.schema.json` does **not** change. The mapping
  is not a shape, so it does not belong in the schema.

## Alternatives considered

- **Single-tier enforcement (one policy for all apps).** Defeats
  the three-tier assurance model that ADR-0002 and ADR-0014 both
  depend on. Enterprise consumers need gold's hard-fail; hobbyist
  consumers would fork under it.
- **Allow-all-with-warnings at every tier.** Reduces the rules to
  advisory and erases the reason for having tiers. Silver and gold
  exist precisely to *fail* compositions that would slip past
  bronze.
- **Mirror Cargo's SemVer ranges without assurance context.** The
  point of the assurance ladder is that pin policy is not a
  uniform global choice. Cargo's default (`^`) is correct at
  bronze and wrong at gold.
- **Encode the mapping in `app.schema.json`.** JSON Schema can
  express "if assurance=gold, pin=sha" via conditional subschemas,
  but doing so spreads the policy across the schema, the generator,
  and the gate. Enforcement-only keeps the single source of truth
  at the generator.
- **Per-module assurance instead of per-app.** Breaks the invariant
  that an app has a single supply-chain posture. Mixing gold
  modules with bronze modules in one app yields the weakest
  posture of the set — which is bronze, which is already expressible.

## References

- ADR-0002 (assurance ladder): the bronze/silver/gold model this
  mapping consumes.
- ADR-0012 (app composition artifact): `app.yaml`, `app.lock`,
  and the composition gate this ADR feeds.
- ADR-0014 (governance tiers): the registry tier rows in the
  mapping table consume ADR-0014's ungoverned / tier-3 / tier-1
  ladder.
- ADR-0015 (schema fingerprint identity): the primitive the
  fingerprint sidecar requirement relies on.
- `docs/chassis/guides/pin-policy-enforcement.md`: operator
  walkthrough of this mapping.
- `scripts/chassis/generate_app_lock.py`: where enforcement lives.

## Status

Accepted. Mapping lands alongside this ADR; generator enforcement
wiring is owned by the app.lock agent as follow-up.
