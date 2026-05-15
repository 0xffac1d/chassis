//! Read-only helpers over a local Git repository (`git2`).
//!
//! See ADR-0024 for drift scoring inputs.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use git2::{Repository, Sort};
use serde_yaml::{Sequence, Value as Yaml};

#[derive(Debug, Clone)]
pub struct GitError(pub String);

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GitError {}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError(e.message().to_string())
    }
}

fn ys(s: &str) -> Yaml {
    Yaml::String(s.to_string())
}

fn open(repo: &Path) -> Result<Repository, GitError> {
    Repository::open(repo).map_err(GitError::from)
}

fn rel(repo: &Repository, file: &Path) -> Result<PathBuf, GitError> {
    if let Some(workdir) = repo.workdir() {
        let base = workdir
            .canonicalize()
            .map_err(|e| GitError(e.to_string()))?;
        let candidate = if file.is_absolute() {
            file.to_path_buf()
        } else {
            workdir.join(file)
        };
        let abs = candidate
            .canonicalize()
            .map_err(|e| GitError(e.to_string()))?;
        return abs
            .strip_prefix(&base)
            .map(|p| p.to_path_buf())
            .map_err(|_| {
                GitError(format!(
                    "path {} not under repo {}",
                    file.display(),
                    base.display()
                ))
            });
    }

    if file.is_absolute() {
        return Err(GitError(format!(
            "bare repository path query must be relative (got {})",
            file.display()
        )));
    }
    Ok(file.to_path_buf())
}

fn commit_time(commit: &git2::Commit<'_>) -> Result<DateTime<Utc>, GitError> {
    let ts = commit.time().seconds();
    Utc.timestamp_opt(ts, 0)
        .single()
        .ok_or_else(|| GitError(format!("invalid commit timestamp for {}", ts)))
}

fn claim_signature_yaml(doc: &Yaml, claim_id: &str) -> Option<u64> {
    fn hash_claim(id: &str, text: &str) -> u64 {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        text.hash(&mut h);
        h.finish()
    }

    fn find_in_claim_list(seq: &Sequence, claim_id: &str) -> Option<String> {
        for item in seq {
            let map = item.as_mapping()?;
            let id = map.get(ys("id"))?.as_str()?;
            if id == claim_id {
                let text = map
                    .get(ys("text"))
                    .and_then(Yaml::as_str)
                    .unwrap_or("")
                    .to_string();
                return Some(text);
            }
        }
        None
    }

    let root = doc.as_mapping()?;
    let text = root
        .get(ys("invariants"))
        .and_then(|v| v.as_sequence())
        .and_then(|s| find_in_claim_list(s, claim_id))
        .or_else(|| {
            root.get(ys("edge_cases"))
                .and_then(|v| v.as_sequence())
                .and_then(|s| find_in_claim_list(s, claim_id))
        })?;

    Some(hash_claim(claim_id, &text))
}

fn claim_signature(blob: &[u8], claim_id: &str) -> Option<u64> {
    let doc: Yaml = serde_yaml::from_slice(blob).ok()?;
    claim_signature_yaml(&doc, claim_id)
}

fn blob_at_path(
    repo: &Repository,
    tree: &git2::Tree<'_>,
    path: &Path,
) -> Result<Option<Vec<u8>>, GitError> {
    let entry = match tree.get_path(path) {
        Ok(e) => e,
        Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match entry.kind() {
        Some(git2::ObjectType::Blob) => {
            let blob = repo.find_blob(entry.id())?;
            Ok(Some(blob.content().to_vec()))
        }
        _ => Ok(None),
    }
}

fn claim_sig_at(
    repo: &Repository,
    tree: &git2::Tree<'_>,
    path: &Path,
    claim_id: &str,
) -> Result<Option<u64>, GitError> {
    let Some(blob) = blob_at_path(repo, tree, path)? else {
        return Ok(None);
    };
    Ok(claim_signature(&blob, claim_id))
}

fn parent_tree_blob(
    repo: &Repository,
    commit: &git2::Commit<'_>,
    relpath: &Path,
) -> Result<Option<Vec<u8>>, GitError> {
    let Some(p) = commit.parent_ids().next() else {
        return Ok(None);
    };
    let pc = repo.find_commit(p)?;
    let pt = pc.tree()?;
    blob_at_path(repo, &pt, relpath)
}

/// Latest UTC time when the claim snapshot `(id,text)` on `contract_path`
/// differs from its parent revision (introduction or edit).
pub fn last_claim_edit(
    repo_path: &Path,
    contract_path: &Path,
    claim_id: &str,
) -> Result<Option<DateTime<Utc>>, GitError> {
    let repo = open(repo_path)?;
    let relpath = rel(&repo, contract_path)?;

    let mut rw = repo.revwalk()?;
    rw.push_head()?;
    rw.set_sorting(Sort::TIME)?;

    for oid in rw {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let curr = claim_sig_at(&repo, &tree, &relpath, claim_id)?;

        let parent_sig = {
            let parents: Vec<_> = commit.parent_ids().collect();
            if let Some(p) = parents.first() {
                let pc = repo.find_commit(*p)?;
                let pt = pc.tree()?;
                claim_sig_at(&repo, &pt, &relpath, claim_id)?
            } else {
                None
            }
        };

        match (curr.as_ref(), parent_sig.as_ref()) {
            (Some(_c), _) if curr == parent_sig => continue,
            (None, _) => continue,
            _ => {}
        }

        return Ok(Some(commit_time(&commit)?));
    }

    Ok(None)
}

/// Most recent UTC time `file` changed in reachable Git history (`None` if never tracked).
pub fn last_file_edit(repo_path: &Path, file: &Path) -> Result<Option<DateTime<Utc>>, GitError> {
    let repo = open(repo_path)?;
    let relpath = rel(&repo, file)?;

    let mut rw = repo.revwalk()?;
    rw.push_head()?;
    rw.set_sorting(Sort::TIME)?;

    for oid in rw {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let this = blob_at_path(&repo, &tree, &relpath)?;

        let parent_blob = parent_tree_blob(&repo, &commit, &relpath)?;

        let Some(blob) = &this else { continue };
        if Some(blob) == parent_blob.as_ref() {
            continue;
        }

        return Ok(Some(commit_time(&commit)?));
    }

    Ok(None)
}

/// Count commits touching `file` whose committer time is **strictly after** `since`.
pub fn commits_touching_file_since(
    repo_path: &Path,
    file: &Path,
    since: DateTime<Utc>,
) -> Result<usize, GitError> {
    let repo = open(repo_path)?;
    let relpath = rel(&repo, file)?;
    let since_ts = since.timestamp();

    let mut rw = repo.revwalk()?;
    rw.push_head()?;
    rw.set_sorting(Sort::TIME)?;

    let mut count = 0usize;
    for oid in rw {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        if commit.time().seconds() <= since_ts {
            continue;
        }

        let tree = commit.tree()?;
        let Some(blob) = blob_at_path(&repo, &tree, &relpath)? else {
            continue;
        };

        let parent_blob = parent_tree_blob(&repo, &commit, &relpath)?;

        let touched = Some(&blob) != parent_blob.as_ref();

        if touched {
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    fn fixture_bare_repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/drift-repo/drift_fixture.git")
            .canonicalize()
            .expect("fixture drift-repo/drift_fixture.git exists (run scripts/build-drift-fixture-repo.sh)")
    }

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(y, mo, d, h, mi, s)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn last_claim_edit_fixture() {
        let root = fixture_bare_repo();
        let got = last_claim_edit(&root, Path::new("CONTRACT.yaml"), "drift.fixture.alpha")
            .unwrap()
            .expect("present");
        assert_eq!(got, dt(2024, 6, 10, 12, 0, 5));
    }

    #[test]
    fn last_file_edit_fixture() {
        let root = fixture_bare_repo();
        let got = last_file_edit(&root, Path::new("src_impl.rs"))
            .unwrap()
            .expect("present");
        assert_eq!(got, dt(2024, 6, 12, 15, 0, 9));
    }

    #[test]
    fn churn_since_fixture_counts_impl_commits_strictly_after() {
        let root = fixture_bare_repo();
        let since = dt(2024, 6, 2, 0, 0, 0);
        let n = commits_touching_file_since(&root, Path::new("src_impl.rs"), since).unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn last_file_edit_missing_is_none() {
        let root = fixture_bare_repo();
        assert!(
            last_file_edit(&root, Path::new("never_existed_anywhere.rs"))
                .unwrap()
                .is_none()
        );
    }
}
