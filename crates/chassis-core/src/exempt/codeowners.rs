//! CODEOWNERS parser and matcher.
//!
//! Implements last-match-wins semantics as documented by GitHub. Patterns use
//! gitignore-style globs (handled here with `globset`). Empty CODEOWNERS = no
//! required signoffs for any path.

use globset::{Glob, GlobMatcher};
use std::fmt;

#[derive(Debug, Clone)]
pub struct CodeownersRule {
    /// Original glob pattern, kept for diagnostics.
    pub pattern: String,
    /// Owners (emails or @-handles).
    pub owners: Vec<String>,
    /// 1-based line number in the source file, for error reporting.
    pub line: usize,
    matcher: GlobMatcher,
}

#[derive(Debug, Clone, Default)]
pub struct Codeowners {
    pub rules: Vec<CodeownersRule>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

impl Codeowners {
    /// Empty CODEOWNERS = no required signoffs for any path.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Parse from CODEOWNERS file content. Empty lines and `#` comments ignored.
    ///
    /// Tokenisation matches GitHub's CODEOWNERS format closely enough for our
    /// purposes: a pattern followed by one or more whitespace-separated owners.
    /// Inline `#` comments are stripped. Patterns ending in `/` match everything
    /// beneath that directory; bare patterns may match any depth (handled via
    /// `**` expansion below).
    pub fn parse(content: &str) -> Result<Self, ParseError> {
        let mut rules = Vec::new();
        for (idx, raw) in content.lines().enumerate() {
            let lineno = idx + 1;
            let line = match raw.find('#') {
                Some(pos) => &raw[..pos],
                None => raw,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let pattern = parts.next().ok_or_else(|| ParseError {
                line: lineno,
                message: "missing pattern".to_string(),
            })?;
            let owners: Vec<String> = parts.map(|s| s.to_string()).collect();
            if owners.is_empty() {
                return Err(ParseError {
                    line: lineno,
                    message: format!("pattern `{}` has no owners", pattern),
                });
            }
            let glob = expand_codeowners_pattern(pattern);
            let matcher = Glob::new(&glob)
                .map_err(|e| ParseError {
                    line: lineno,
                    message: format!("invalid glob `{}`: {}", pattern, e),
                })?
                .compile_matcher();
            rules.push(CodeownersRule {
                pattern: pattern.to_string(),
                owners,
                line: lineno,
                matcher,
            });
        }
        Ok(Self { rules })
    }

    /// Owners required for a single path. Last-match-wins per GitHub CODEOWNERS
    /// semantics; an unmatched path returns the empty vec.
    pub fn owners_for(&self, path: &str) -> Vec<String> {
        let norm = path.trim_start_matches('/');
        let mut hit: Option<&CodeownersRule> = None;
        for rule in &self.rules {
            if rule.matcher.is_match(norm) {
                hit = Some(rule);
            }
        }
        hit.map(|r| r.owners.clone()).unwrap_or_default()
    }

    /// Union of owners required across a set of paths. Order is insertion order
    /// (first occurrence wins) so callers get a deterministic ordering.
    pub fn required_owners(&self, paths: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for p in paths {
            for owner in self.owners_for(p) {
                if !out.contains(&owner) {
                    out.push(owner);
                }
            }
        }
        out
    }
}

/// Translate a CODEOWNERS pattern into a globset glob.
///
/// The CODEOWNERS format itself is gitignore-like; the main quirks are:
///   * A pattern starting with `/` is anchored to repo root.
///   * A pattern ending in `/` matches the directory contents (`**`).
///   * A bare path component pattern (no slashes) matches at any depth.
fn expand_codeowners_pattern(pat: &str) -> String {
    let trimmed = pat.trim_start_matches('/');
    let mut out = String::new();
    if !pat.starts_with('/') && !trimmed.contains('/') && !trimmed.contains("**") {
        out.push_str("**/");
    }
    out.push_str(trimmed);
    if pat.ends_with('/') {
        out.push_str("**");
    }
    out
}
