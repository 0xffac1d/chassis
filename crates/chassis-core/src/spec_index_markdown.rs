//! Spec Kit markdown bridge: embedded canonical YAML in a fenced block (`yaml-meta`).
//!
//! Narrative prose may surround the block; the exporter extracts only the fence
//! (ADR-0029).

use std::fs;
use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::spec_index::{
    export_from_source_yaml_bytes, ExportError, SpecIndex, CH_SPEC_MARKDOWN_EMPTY_FENCE,
    CH_SPEC_MARKDOWN_MULTIPLE_FENCES, CH_SPEC_MARKDOWN_NO_FENCE,
};

/// Extract **`yaml-meta` fences** (ADR-0029) and parse as SpecIndex YAML.
pub fn export_from_spec_bundle_markdown_path(path: &Path) -> Result<SpecIndex, ExportError> {
    let raw = fs::read_to_string(path).map_err(|e| ExportError {
        rule_id: crate::spec_index::CH_SPEC_SOURCE_PARSE,
        message: format!("read {}: {e}", path.display()),
    })?;
    export_from_spec_bundle_markdown_bytes(path, &raw)
}

pub fn export_from_spec_bundle_markdown_bytes(
    subject: &Path,
    raw: &str,
) -> Result<SpecIndex, ExportError> {
    let yaml = extract_yaml_meta_fence(raw).map_err(|e| ExportError {
        rule_id: e.rule_id,
        message: format!("{}: {}", subject.display(), e.message),
    })?;
    export_from_source_yaml_bytes(yaml.as_bytes())
}

#[derive(Debug)]
struct YamlMetaExtractError {
    rule_id: &'static str,
    message: String,
}

fn extract_yaml_meta_fence(raw: &str) -> Result<String, YamlMetaExtractError> {
    let bodies = collect_yaml_meta_fence_bodies(raw);
    match bodies.as_slice() {
        [] => Err(YamlMetaExtractError {
            rule_id: CH_SPEC_MARKDOWN_NO_FENCE,
            message: "expected exactly one ```yaml-meta fenced block".to_string(),
        }),
        [_, _, ..] => Err(YamlMetaExtractError {
            rule_id: CH_SPEC_MARKDOWN_MULTIPLE_FENCES,
            message: format!(
                "expected exactly one ```yaml-meta fenced block, found {}",
                bodies.len()
            ),
        }),
        [body] => {
            let t = body.trim();
            if t.is_empty() {
                Err(YamlMetaExtractError {
                    rule_id: CH_SPEC_MARKDOWN_EMPTY_FENCE,
                    message: "```yaml-meta` fence body is empty".to_string(),
                })
            } else {
                Ok(t.to_string())
            }
        }
    }
}

fn collect_yaml_meta_fence_bodies(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_yaml_meta = false;
    for event in Parser::new_ext(doc, Options::empty()) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                in_yaml_meta = info.as_ref() == "yaml-meta";
                if in_yaml_meta {
                    buf.clear();
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_yaml_meta {
                    out.push(buf.clone());
                    in_yaml_meta = false;
                }
            }
            Event::Text(t) if in_yaml_meta => buf.push_str(&t),
            Event::Code(t) if in_yaml_meta => buf.push_str(&t),
            Event::SoftBreak | Event::HardBreak if in_yaml_meta => buf.push('\n'),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_yaml_meta_fence() {
        let md = r#"# Title

Intro paragraph.

```yaml-meta
version: 1
chassis_preset_version: 1
feature_id: demo
title: t
summary: s
constitution_principles:
  - id: P1
    text: hello
non_goals: []
requirements:
  - id: REQ-1
    title: r
    description: d
    acceptance_criteria:
      - a
    claim_ids:
      - chassis.foo
    related_task_ids:
      - T1
    touched_paths:
      - src/lib.rs
tasks:
  - id: T1
    title: tt
    depends_on: []
    touched_paths:
      - src/lib.rs
implementation_constraints: []
```

tail
"#;
        let y = extract_yaml_meta_fence(md).expect("fence");
        assert!(y.contains("feature_id: demo"));
    }

    #[test]
    fn rejects_zero_yaml_meta_fences() {
        let md = "# Title\n\nNo fence here.\n";
        let e = extract_yaml_meta_fence(md).expect_err("must reject");
        assert_eq!(e.rule_id, CH_SPEC_MARKDOWN_NO_FENCE);
    }

    #[test]
    fn rejects_multiple_yaml_meta_fences() {
        let md = r"```yaml-meta
a: 1
```
```yaml-meta
b: 2
```";
        let e = extract_yaml_meta_fence(md).expect_err("must reject");
        assert_eq!(e.rule_id, CH_SPEC_MARKDOWN_MULTIPLE_FENCES);
    }

    #[test]
    fn rejects_empty_yaml_meta_fence() {
        let md = "```yaml-meta\n   \n```\n";
        let e = extract_yaml_meta_fence(md).expect_err("must reject");
        assert_eq!(e.rule_id, CH_SPEC_MARKDOWN_EMPTY_FENCE);
    }
}
