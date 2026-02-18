use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

/// Find the root of the current git repository.
pub fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("not in a git repository: {}", stderr.trim());
    }
    let path = String::from_utf8(output.stdout)
        .context("invalid utf-8 from git")?
        .trim()
        .to_string();
    Ok(PathBuf::from(path))
}

/// Find the merge base between HEAD and the given base branch.
fn merge_base(repo_root: &Path, base: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["merge-base", base, "HEAD"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git merge-base")?;
    if !output.status.success() {
        anyhow::bail!("git merge-base failed — is '{base}' a valid ref?");
    }
    Ok(String::from_utf8(output.stdout)
        .context("invalid utf-8")?
        .trim()
        .to_string())
}

/// Return files changed in the current branch relative to a base branch.
/// Paths are relative to the repo root.
pub fn changed_files(repo_root: &Path, base: &str) -> Result<Vec<PathBuf>> {
    let base_commit = merge_base(repo_root, base)?;

    let branch_diff = Command::new("git")
        .args(["diff", "--name-only", "-z", "--diff-filter=ACMRD", &base_commit, "HEAD"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git diff")?;

    let unstaged = Command::new("git")
        .args(["diff", "--name-only", "-z", "--diff-filter=ACMRD"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git diff (unstaged)")?;

    let staged = Command::new("git")
        .args(["diff", "--name-only", "-z", "--diff-filter=ACMRD", "--cached"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git diff (staged)")?;

    let untracked = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git ls-files")?;

    let mut all = std::collections::BTreeSet::new();
    for output in [branch_diff, unstaged, staged, untracked] {
        let text = String::from_utf8(output.stdout).context("invalid utf-8")?;
        for entry in text.split('\0').filter(|s| !s.is_empty()) {
            all.insert(PathBuf::from(entry));
        }
    }

    Ok(all.into_iter().collect())
}
