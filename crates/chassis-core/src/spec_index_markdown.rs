//! Spec Kit markdown bridge: embedded canonical YAML in a fenced block (`yaml-meta`).
//!
//! Narrative prose may surround the block; the exporter extracts only the fence
//! (ADR-0029).

use std::fs;
use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::spec_index::{export_from_source_yaml_bytes, ExportError, SpecIndex};

/// Extract the first ` ```yaml-meta ` fenced code block and parse it as SpecIndex YAML.
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
    let yaml = extract_yaml_meta_fence(raw).ok_or_else(|| ExportError {
        rule_id: crate::spec_index::CH_SPEC_SOURCE_PARSE,
        message: format!(
            "no ```yaml-meta fenced block found in {}",
            subject.display()
        ),
    })?;
    export_from_source_yaml_bytes(yaml.as_bytes())
}

fn extract_yaml_meta_fence(doc: &str) -> Option<String> {
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
                    let t = buf.trim();
                    if !t.is_empty() {
                        return Some(t.to_string());
                    }
                    in_yaml_meta = false;
                }
            }
            Event::Text(t) if in_yaml_meta => buf.push_str(&t),
            Event::Code(t) if in_yaml_meta => buf.push_str(&t),
            Event::SoftBreak | Event::HardBreak if in_yaml_meta => buf.push('\n'),
            _ => {}
        }
    }
    None
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
}
