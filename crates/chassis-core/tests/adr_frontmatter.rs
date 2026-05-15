//! @claim chassis.adr-frontmatter-valid
use std::fs;
use std::path::Path;

// @claim chassis.adr-frontmatter-valid
#[test]
fn every_adr_frontmatter_validates_against_schema() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo.join("schemas/adr.schema.json")).expect("read adr schema"),
    )
    .expect("adr schema parses");
    let validator = jsonschema::validator_for(&schema).expect("compile adr schema");

    let adr_dir = repo.join("docs/adr");
    let mut checked = 0usize;
    for ent in fs::read_dir(&adr_dir).expect("read adr dir").flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let raw = fs::read_to_string(&p).expect("read adr");
        let body = raw
            .strip_prefix("---\n")
            .unwrap_or_else(|| panic!("no leading frontmatter in {}", p.display()));
        let end = body
            .find("\n---\n")
            .or_else(|| body.find("\n---"))
            .unwrap_or_else(|| panic!("no closing frontmatter delimiter in {}", p.display()));
        let fm = &body[..end];
        let v: serde_json::Value =
            serde_yaml::from_str(fm).unwrap_or_else(|e| panic!("yaml in {}: {e}", p.display()));
        let errs: Vec<String> = validator.iter_errors(&v).map(|e| e.to_string()).collect();
        assert!(
            errs.is_empty(),
            "{} fails adr.schema.json: {:?}",
            p.display(),
            errs
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "expected at least one ADR under {}",
        adr_dir.display()
    );
}
