//! Markdown bundle (`yaml-meta`) vs YAML export digest parity.
#![forbid(unsafe_code)]

use std::path::Path;

use chassis_core::spec_index::{digest_sha256_hex, export_from_source_yaml_path};
use chassis_core::spec_index_markdown::export_from_spec_bundle_markdown_path;

#[test]
fn markdown_bundle_matches_yaml_source_digest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let yaml_path = root.join(".chassis/spec-index-source.yaml");
    let md_path = root.join("fixtures/spec-kit/markdown-vs-yaml/bundle.md");
    let from_yaml = export_from_source_yaml_path(&yaml_path).expect("yaml export");
    let from_md = export_from_spec_bundle_markdown_path(&md_path).expect("markdown export");
    assert_eq!(
        digest_sha256_hex(&from_yaml).expect("digest yaml"),
        digest_sha256_hex(&from_md).expect("digest md"),
    );
}
