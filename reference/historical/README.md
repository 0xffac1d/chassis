# Historical claims

Destination for active-doc claims that have aged out — plans that were once recorded as forward intent but are no longer the path the project is taking. Files here are reference-only; nothing under this directory is enforced by `chassis validate`, `chassis trace`, `chassis-jsonrpc`, or CI.

Promotion direction is one-way: claims move *into* this directory when an active doc (README, CLAUDE.md, CONTRACT.yaml, `docs/WAVE-PLAN.md`, `docs/ASSURANCE-LADDER.md`, package READMEs) is rewritten to drop them. They do not get pulled back out — when a deprecated direction comes back, capture the new intent fresh in active docs.

Conventions:

- One file per superseded claim cluster. Date the filename.
- Lead with what was claimed, who claimed it (which active doc), when, and what replaced it. Link the replacing claim by current path.
- Do not edit existing files in this directory; supersede with a new file if the historical record itself becomes wrong.

See also `reference/docs-original/HISTORY.md` for the salvage narrative and `reference/adrs-original/` for predecessor ADRs.
