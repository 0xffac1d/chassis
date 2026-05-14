# typescript-vite

A minimal Vite-shaped TypeScript project (`package.json` + `tsconfig.json` + `src/`) with a hand-authored `CONTRACT.yaml` that validates against `schemas/contract.schema.json`.

Pair with `rust-minimal` as the two happy-path references — one per supported language (Rust + TypeScript, per ADR-0001). Future tooling (the planned TypeScript CLI under `packages/chassis-cli/`) is expected to validate and bootstrap cleanly against trees of this shape.
