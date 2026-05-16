//! Markdown bundle (`yaml-meta`) vs YAML export digest parity.
#![forbid(unsafe_code)]

use std::path::Path;

use chassis_core::spec_index::{
    digest_sha256_hex, export_from_source_yaml_path, CH_SPEC_MARKDOWN_EMPTY_FENCE,
    CH_SPEC_MARKDOWN_MULTIPLE_FENCES, CH_SPEC_MARKDOWN_NO_FENCE,
};
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

#[test]
fn markdown_no_fence_returns_stable_rule() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let md_path = root.join("fixtures/spec-kit/markdown-vs-yaml/no-fence.md");
    let err = export_from_spec_bundle_markdown_path(&md_path).expect_err("no fence");
    assert_eq!(err.rule_id, CH_SPEC_MARKDOWN_NO_FENCE);
}

#[test]
fn markdown_multiple_fences_returns_stable_rule() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let md_path = root.join("fixtures/spec-kit/markdown-vs-yaml/multiple-fences.md");
    let err = export_from_spec_bundle_markdown_path(&md_path).expect_err("multi fence");
    assert_eq!(err.rule_id, CH_SPEC_MARKDOWN_MULTIPLE_FENCES);
}

#[test]
fn markdown_empty_fence_returns_stable_rule() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let md_path = root.join("fixtures/spec-kit/markdown-vs-yaml/empty-fence.md");
    let err = export_from_spec_bundle_markdown_path(&md_path).expect_err("empty fence");
    assert_eq!(err.rule_id, CH_SPEC_MARKDOWN_EMPTY_FENCE);
}
