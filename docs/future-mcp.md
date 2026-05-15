# Future work: real Model Context Protocol (MCP) surface

Status: **future**. Not shipped today.

The Rust crate `crates/chassis-jsonrpc/` is a **custom** newline-delimited JSON-RPC 2.0 sidecar over stdio. It exposes six chassis methods (`validate_contract`, `diff_contracts`, `trace_claim`, `drift_report`, `release_gate`, `list_exemptions`) using bespoke shapes and is **not** an MCP server. Earlier copy that called it "the canonical MCP surface" was wrong and has been corrected; see `CONTRACT.yaml` and the Wave 5 entry of `docs/WAVE-PLAN.md`.

A real MCP surface — likely landing as the TypeScript `packages/chassis-mcp/` planned in Wave 4 — must satisfy the items below before any user-facing copy may call it MCP. References are to the Model Context Protocol specification at <https://modelcontextprotocol.io/>.

## Requirements for a real MCP server

1. **`initialize` lifecycle.** Implement the protocol handshake: client sends `initialize` with its `protocolVersion` and `capabilities`; server replies with its own `protocolVersion`, `serverInfo` (name, version), and `capabilities`. Server then waits for the `initialized` notification before accepting any other requests. Reject pre-initialize traffic with the spec-defined error.

2. **Capability negotiation.** Advertise only what the server actually supports — at minimum a `tools` capability for the chassis verbs, with explicit flags such as `listChanged` if dynamic. Do not advertise `resources`, `prompts`, `logging`, `sampling`, etc., unless they are implemented end-to-end. Fail closed if the client requests unsupported features.

3. **`tools/list`.** Return one MCP `Tool` entry per chassis verb, each with: a stable `name`, a human-readable `description`, an `inputSchema` (JSON Schema, draft 2020-12, matching the spec's tool schema constraints), and — when output structure is part of the contract — an `outputSchema`. The tool list must be deterministic and round-trippable.

4. **`tools/call`.** Dispatch by tool `name`, validate `arguments` against the published `inputSchema`, and return an MCP `CallToolResult`: a `content` array of typed parts (`text`, `resource`, etc.) plus `isError` for tool-level failures. Transport-level failures still go through JSON-RPC error objects with the spec's error codes; never conflate the two.

5. **Schema-valid tool outputs.** Every successful `tools/call` response — and, if `outputSchema` is declared, the `structuredContent` field — must validate against the declared schema. Add a self-check: every tool published in `tools/list` is exercised against its schemas in CI before release.

## Out of scope until the items above ship

- Marketing or doc copy that uses the bare word "MCP" for the current sidecar.
- Claiming MCP compliance in `CONTRACT.yaml` claims, ADRs, or README.
- Listing the sidecar in any MCP client registry or extension catalog.

When a future PR satisfies all five requirements, update this file (or replace it with the real surface's design doc), flip the relevant `CONTRACT.yaml` claim, and add a Wave 4/5 ADR documenting the chosen MCP SDK and transport.
