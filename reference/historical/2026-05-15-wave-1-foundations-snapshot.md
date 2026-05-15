# Foundations notes — Wave 1 ADR snapshot (relocated 2026-05-15)

Originally lived at `docs/FOUNDATIONS-NOTES.md`. Frozen on 2026-05-14 with the Wave 1 set; Waves 2–6 added ADR-0018 through ADR-0025, which this snapshot does not cover. Current ADR coverage lives in `CLAUDE.md` (the "What's in the tree → docs/adr/" row) and the per-wave entries of `docs/WAVE-PLAN.md`; the canonical rule-prefix → ADR map is `docs/STABLE-IDS.md`.

Snapshot of decisions recorded as ADRs on 2026-05-14. Each row answers "what," "why," and "how it differs from `reference/adrs-original/`."

| ADR | Decision | Why | Predecessor delta |
|-----|----------|-----|-------------------|
| ADR-0002 | Evidence-backed ladder with explicit verifier artifacts; sequential promotion; logged demotion; unblock order coherent→verified→enforced→observed | Trace + attestation need stable meanings independent of Python gates | Dropped `assurance_promotion.py` coupling; clarified artifacts per rung |
| ADR-0003 | Formal claim grammar; invariant vs edge-case semantics; claim vs rule vs ADR separation; structured-only rows | Spec-first repos cannot rely on prose-only claims | Removed string-form invariant compatibility from canonical schema |
| ADR-0004 | 90-day + 25-cap enforced at write + CI; calendar “active”; CODEOWNERS union across scopes; expired rows fail CI | Prevents suppression debt without governance | Deferred per-file cap until exemption CLI returns |
| ADR-0005 | _Superseded by ADR-0023 (2026-05-15)._ Original decision admitted Rust `// @claim` plus TS `/** @claim */`. The TS divergence caused JSDoc claims to silently disappear from the trace graph. | Historical — see ADR-0023 for the active grammar | New ADR (no direct predecessor) |
| ADR-0023 | Single canonical `// @claim <id>` line comment for both Rust and TypeScript; immediate placement; one claim per line; rejected JSDoc form surfaces `CH-TRACE-MALFORMED-CLAIM` so it cannot fail silently; tests use the same markers as production | Eliminates scanner/grammar mismatch; preserves static, line-oriented extraction | Supersedes ADR-0005 |
| ADR-0008 | Semver on every schema file; CI bump obligation; parallel majors via sibling filenames after bootstrap | Makes breakage visible to Rust + TS consumers | Removed unrelated completeness/process gates bundled historically |
| ADR-0011 | Lock grammar + immutability + supersession etiquette | Diagnostics route purely on stable IDs | Deemphasized transitional `*-GENERIC` migration story |
| ADR-0015 | Node reference fingerprint (`fingerprint-schemas.mjs`) + CI parity | Matches `@chassis/core-types` tarball (`fingerprint.sha256` + `manifest.json`) | Legacy Python filenames dropped; parity with Rust digest in chassis-core (`ADR-0017`) |
| ADR-0016 | Deferred work stays under `reference/` with README + promotion criteria | Salvage boundary stays crisp | Replaced monolith extraction tables with repo-local deferrals |
