# Python reference implementation

Original Python implementations of: JSON Schema validation (`jsonschema_support.py`, `metadata_semantics.py`), exemption registry CLI (`exempt.py`, `exemptions*.py`), ADR validation (`adr.py`), claims validation (`claims_validate.py`), coherence report (`coherence_report.py`), doctor diagnostics (`doctor.py`), MCP server (`mcp_server.py`), and contract-diff / breaking-change detection (`contract-diff/`).

**Reference material only.** The production implementation will be rewritten in TypeScript (CLI) and Rust (core library). Treat this code as semantic specification, not a runtime dependency. Do not invest in fixing import errors here.

`mcp_server.py` is the highest-priority study target — the rewrite's primary integration path is an MCP server, and this file is the closest existing reference for the tool surface (`what_governs`, `is_exempt`, etc.).
