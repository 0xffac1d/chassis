# cli-minimal

Minimal happy-path fixture for `kind: cli` contracts.

Exercises the kind-specific required fields:

- `entrypoint` — binary name.
- `argsSummary` — human-readable synopsis.
- `subcommands` — optional structured array.

No source tree is shipped; the fixture is purely about validating the contract shape.
