//! Supply-chain rule IDs enforced by ADR-0025 (`docs/adr/ADR-0025-supply-chain-policy.md`).
//!
//! These rules are gated by external tooling (cargo-deny, archive scripts),
//! not by chassis-core itself. The constants here are the canonical wire
//! identifiers so ADR-0025 `enforces[]` rows bind to a kernel-side surface,
//! matching the convention used by every other CH-* rule family.

pub mod rule_id {
    /// Workspace dependency licenses must match the SPDX allowlist in `deny.toml`.
    pub const LICENSE_ALLOW: &str = "CH-SUPPLY-LICENSE-ALLOW";
    /// RustSec advisories block CI unless individually justified in `deny.toml`.
    pub const ADVISORY_CLEAN: &str = "CH-SUPPLY-ADVISORY-CLEAN";
    /// openssl / native-tls / reqwest / hyper / tokio are banned in `deny.toml`.
    pub const NO_NETWORK_CRATES: &str = "CH-SUPPLY-NO-NETWORK-CRATES";
    /// Source archives must be produced from `git archive` (no build/cache artifacts).
    pub const ARCHIVE_HYGIENE: &str = "CH-SUPPLY-ARCHIVE-HYGIENE";
}
