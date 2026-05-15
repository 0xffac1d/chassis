//! @claim chassis.schemas-self-contained
use std::fs;
use std::path::Path;

// @claim chassis.schemas-self-contained
#[test]
fn no_schema_has_external_refs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let mut external = Vec::new();
    walk(&root, &mut |path, contents| {
        let v: serde_json::Value = serde_json::from_str(contents).expect("schema parses");
        find_refs(&v, |r| {
            if r.starts_with("http://") || r.starts_with("https://") || r.starts_with("//") {
                external.push(format!("{}: {}", path.display(), r));
            }
        });
    });
    assert!(
        external.is_empty(),
        "external $refs present:\n  {}",
        external.join("\n  ")
    );
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path, &str)) {
    for ent in fs::read_dir(dir).expect("read schemas").flatten() {
        let p = ent.path();
        if p.is_dir() {
            walk(&p, f);
            continue;
        }
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let s = fs::read_to_string(&p).expect("read schema");
        f(&p, &s);
    }
}

// @claim chassis.schemas-self-contained
fn find_refs(v: &serde_json::Value, mut cb: impl FnMut(&str)) {
    fn rec(v: &serde_json::Value, cb: &mut dyn FnMut(&str)) {
        match v {
            serde_json::Value::Object(m) => {
                if let Some(serde_json::Value::String(s)) = m.get("$ref") {
                    cb(s);
                }
                for (_, x) in m {
                    rec(x, cb);
                }
            }
            serde_json::Value::Array(a) => {
                for x in a {
                    rec(x, cb);
                }
            }
            _ => {}
        }
    }
    rec(v, &mut cb);
}
