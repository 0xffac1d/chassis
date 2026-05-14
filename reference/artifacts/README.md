# Attestation artifact shape

`release-gate.example.json` is the shape the rewrite's attestation artifact should produce on every release. Preserve in the redesign: top-level `verdict`, `commit`, `git_commit_full`, `working_tree_dirty`, `sha256` (computed over the artifact body, written last), `timestamp`, `started_at`, `elapsed_ms`. Per-group: `id`, `status`, `summary`, `findings` (array of structured diagnostics), `elapsed_ms`.

The original implementation did **not** sign this artifact. The rewrite should sign with cosign or in-toto.
