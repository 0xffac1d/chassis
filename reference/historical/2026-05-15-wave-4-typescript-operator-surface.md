# Wave 4 plan: TypeScript CLI + TypeScript MCP server (superseded 2026-05-15)

## Where it lived

`docs/WAVE-PLAN.md` (Wave 4 section), referenced by `CLAUDE.md` "Immediate next work" and `README.md` (the "no CLI binary yet; no MCP server yet" line).

## What was claimed

> ## Wave 4 — Operator interfaces
>
> - Consolidated TypeScript CLI under `packages/chassis-cli/` wrapping `chassis-core` (NAPI or WASM per Wave 4 prep ADR). Subcommands: `validate`, `diff`, `exempt`, `trace`, `drift`, `attest`, `doctor`.
> - MCP server (TypeScript) in `packages/chassis-mcp/` exposing `what_governs`, `what_breaks_if_i_change`, `is_exempt`, `validate_contract`. Reference semantics in `reference/python-cli/mcp_server.py`.

And, in `README.md`:

> No CLI binary yet; no MCP server yet; not yet published to crates.io or npm.

## What replaced it

The operator and machine surfaces shipped in Rust during Wave 3, not as Wave 4 TypeScript packages:

- Operator surface: `crates/chassis-cli/` (binary `chassis`) with `validate`, `diff`, `exempt verify`, `trace`, `drift`, `attest sign`, `attest verify`. No `doctor` yet (tracked as Wave 4 polish).
- Machine surface: `crates/chassis-jsonrpc/` (binary `chassis-jsonrpc`) — newline-delimited JSON-RPC 2.0 over stdio with six methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`). Explicitly **not** the Model Context Protocol; whether to add an MCP shim on top is the Wave 4 decision recorded in the current `docs/WAVE-PLAN.md`.

A TypeScript CLI is no longer planned by default — there is no consumer requiring it. If one surfaces, a Wave 4 prep ADR will pick NAPI vs WASM at that time.

## Why preserved

Captures the original intent (TS-first operator + MCP-first agent surface) so that future readers comparing the early ADRs and `reference/python-cli/mcp_server.py` to today's tree understand the direction change.
