//! Canonical schema fingerprint (ADR-0015 / ADR-0017) — byte-identical manifest
//! construction and hashing to `packages/chassis-types/scripts/fingerprint-schemas.mjs`.

use std::fmt;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

/// Keys retained from each `.schema.json` before hashing (`KEEP_KEYS` in Node).
pub const KEEP_KEYS: [&str; 22] = [
    "$id",
    "type",
    "required",
    "properties",
    "additionalProperties",
    "$defs",
    "definitions",
    "oneOf",
    "anyOf",
    "allOf",
    "enum",
    "items",
    "patternProperties",
    "propertyNames",
    "const",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "pattern",
    "format",
    "version",
];

/// Error type for fingerprint computation.
#[derive(Debug)]
pub enum FingerprintError {
    Io(io::Error),
    Json(String),
    NonFiniteNumber,
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FingerprintError::Io(e) => write!(f, "{e}"),
            FingerprintError::Json(msg) => write!(f, "{msg}"),
            FingerprintError::NonFiniteNumber => write!(f, "non-finite JSON number"),
        }
    }
}

impl std::error::Error for FingerprintError {}

impl From<io::Error> for FingerprintError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn canonical_subject(mut obj: Map<String, Value>) -> Result<Map<String, Value>, FingerprintError> {
    let mut keep = Map::new();
    for k in KEEP_KEYS {
        if let Some(v) = obj.remove(k) {
            keep.insert(k.to_string(), v);
        }
    }
    Ok(keep)
}

/// RFC 8785-shaped JSON serialization matching `canonicalize.mjs`:
/// sorted object keys recursively, compact separators, no ASCII escaping for Unicode.
pub fn canonicalize_json(value: &Value) -> Result<String, FingerprintError> {
    canonicalize_value(value)
}

fn canonicalize_value(v: &Value) -> Result<String, FingerprintError> {
    match v {
        Value::Null => Ok("null".to_string()),
        Value::Bool(true) => Ok("true".to_string()),
        Value::Bool(false) => Ok("false".to_string()),
        Value::Number(n) => Ok(canonical_number(n)?),
        Value::String(s) => {
            serde_json::to_string(s).map_err(|e| FingerprintError::Json(e.to_string()))
        }
        Value::Array(arr) => {
            let mut parts = Vec::with_capacity(arr.len());
            for item in arr {
                parts.push(canonicalize_value(item)?);
            }
            Ok(format!("[{}]", parts.join(",")))
        }
        Value::Object(map) => {
            let keys: Vec<_> = map.keys().map(String::as_str).collect::<Vec<_>>();
            let mut sorted: Vec<&str> = keys;
            sorted.sort_unstable();

            let mut parts = Vec::with_capacity(sorted.len());
            for k in sorted {
                let key_json =
                    serde_json::to_string(k).map_err(|e| FingerprintError::Json(e.to_string()))?;
                let val = map.get(k).unwrap();
                parts.push(format!("{}:{}", key_json, canonicalize_value(val)?));
            }
            Ok(format!("{{{}}}", parts.join(",")))
        }
    }
}

fn canonical_number(n: &Number) -> Result<String, FingerprintError> {
    let Some(f) = n.as_f64() else {
        return Err(FingerprintError::NonFiniteNumber);
    };
    if !f.is_finite() {
        return Err(FingerprintError::NonFiniteNumber);
    }
    serde_json::to_string(&Value::Number(n.clone()))
        .map_err(|e| FingerprintError::Json(e.to_string()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn iter_schema_paths(schemas_dir: &Path) -> io::Result<Vec<PathBuf>> {
    fn walk(dir: &Path, acc: &mut Vec<PathBuf>) -> io::Result<()> {
        let rd = match fs::read_dir(dir) {
            Ok(r) => r,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        for entry in rd {
            let entry = entry?;
            let path = entry.path();
            let meta = fs::metadata(&path)?;
            if meta.is_dir() {
                walk(&path, acc)?;
            } else if meta.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".schema.json") {
                    acc.push(path);
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(schemas_dir, &mut out)?;
    out.sort_unstable();
    Ok(out)
}

/// Build the manifest `{ version: 1, kind, count, entries: [{ path, sha256 }] }` from `schemas/`.
pub fn build_manifest(repo_root: &Path) -> Result<Value, FingerprintError> {
    let schemas_dir = repo_root.join("schemas");
    let files = iter_schema_paths(&schemas_dir)?;
    let mut entries = Vec::new();
    for path in files {
        let rel = normalize_rel(repo_root, &path)?;
        let raw: Value = serde_json::from_reader(fs::File::open(&path)?)
            .map_err(|e| FingerprintError::Json(format!("{}: {e}", path.display())))?;

        let obj = raw
            .as_object()
            .ok_or_else(|| {
                FingerprintError::Json(format!("{} is not a JSON object", path.display()))
            })?
            .clone();

        let subject = canonical_subject(obj)?;
        let canonical = canonicalize_json(&Value::Object(subject))?;
        let digest = sha256_hex(canonical.as_bytes());
        entries.push(json_entry(rel, digest));
    }
    Ok(manifest(entries))
}

fn json_entry(path: String, sha256: String) -> Value {
    serde_json::json!({"path": path, "sha256": sha256})
}

fn manifest(entries: Vec<Value>) -> Value {
    let count = entries.len();
    serde_json::json!({
        "version": 1_i64,
        "kind": "chassis-schemas-manifest",
        "count": count,
        "entries": Value::Array(entries),
    })
}

fn normalize_rel(repo_root: &Path, path: &Path) -> Result<String, FingerprintError> {
    let rel = path.strip_prefix(repo_root).map_err(|_| {
        FingerprintError::Json(format!(
            "path {} outside repo {}",
            path.display(),
            repo_root.display()
        ))
    })?;
    let s = rel
        .to_str()
        .ok_or_else(|| FingerprintError::Json(format!("non-UTF8 path {}", rel.display())))?
        .replace('\\', "/");
    Ok(s)
}

/// Digest of canonical JSON for `manifest` (same as Node `manifestHash`).
pub fn manifest_hash(manifest: &Value) -> Result<String, FingerprintError> {
    let canon = canonicalize_json(manifest)?;
    Ok(sha256_hex(canon.as_bytes()))
}

/// Full fingerprint for the schemas tree under `repo_root` (= same as committed `fingerprint.sha256`).
pub fn compute(repo_root: &Path) -> Result<String, FingerprintError> {
    let m = build_manifest(repo_root)?;
    manifest_hash(&m)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;

    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    #[test]
    fn committed_fingerprint_matches_rust_digest() {
        let repo = repo_root();
        let pkg_fp = repo.join("packages/chassis-types/fingerprint.sha256");
        let text = fs::read_to_string(&pkg_fp).unwrap();
        let committed = text.split_whitespace().next().expect("committed digest");
        let rust = compute(&repo).unwrap();
        assert_eq!(
            committed,
            rust.as_str(),
            "run `npm run build` in chassis-types after schema changes"
        );
    }

    #[test]
    fn parity_with_node_when_available() {
        if Command::new("node").arg("--version").output().is_err() {
            return;
        }
        let repo = repo_root();
        let script = repo.join("packages/chassis-types/scripts/fingerprint-schemas.mjs");
        assert!(script.exists(), "node script {}", script.display());
        let out = Command::new("node")
            .current_dir(&repo)
            .env("CHASSIS_REPO_ROOT", &repo)
            .args(["--input-type=module", "-e"])
            .arg(
                "import { buildManifest, manifestHash } from './packages/chassis-types/scripts/fingerprint-schemas.mjs'; \
                 const root = process.env.CHASSIS_REPO_ROOT; \
                 console.log(manifestHash(buildManifest(root)));",
            )
            .output()
            .expect("spawn node");

        assert!(
            out.status.success(),
            "node stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let node_digest = String::from_utf8(out.stdout)
            .expect("utf8")
            .trim()
            .to_string();
        let rust_digest = compute(&repo).unwrap();
        assert_eq!(node_digest, rust_digest);
    }

    #[test]
    fn canonicalize_matches_fixture_number_and_string() {
        let v = serde_json::json!({"z": null, "a": [1_i64, "\"x"], "b": "hello"});
        let s = canonicalize_json(&v).unwrap();
        assert_eq!(s, "{\"a\":[1,\"\\\"x\"],\"b\":\"hello\",\"z\":null}");
    }
}
