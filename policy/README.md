# OPA release policy

Rego policies for **external** release decisions over Chassis evidence. They do not ship inside `chassis-core`; CI and operators load them with the OPA CLI.

| File | Purpose |
|------|---------|
| `chassis_release.rego` | `package chassis.release` — `allow`, `deny_reasons`, and `result` over `{ "input": … }` from `chassis export --format opa`. |
| `chassis_release_test.rego` | `opa test` cases for pass/deny paths. |

Run locally:

```bash
opa test policy/
./scripts/policy-gate.sh .
```

Input shape matches `schemas/policy-input.schema.json` nested under the OPA `input` key (see `schemas/opa-input.schema.json`).
