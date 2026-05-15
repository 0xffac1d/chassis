//! Print the schemas manifest fingerprint hex (ADR-0015 / ADR-0017).
//!
//! Usage: `CHASSIS_REPO_ROOT=/path/to/repo cargo run -p chassis-core --example fingerprint`

use std::path::PathBuf;

use chassis_core::fingerprint::compute;

fn main() {
    let root = match std::env::var_os("CHASSIS_REPO_ROOT") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..") // crates/chassis-core -> repo root
            .canonicalize()
            .expect("repo root"),
    };
    let digest =
        compute(&root).unwrap_or_else(|e| panic!("fingerprint.compute({}): {e}", root.display()));
    println!("{digest}");
}
