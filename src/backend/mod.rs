mod bazel;
mod go;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub use bazel::BazelBackend;
pub use go::GoBackend;

/// A build target identified by a backend.
#[derive(Debug, Clone)]
pub struct Target {
    /// Human-readable label (e.g. "./internal/db/...")
    pub label: String,
    /// The directory this target lives in.
    pub dir: PathBuf,
}

/// Trait implemented by each build system backend.
pub trait Backend {
    fn name(&self) -> &str;

    /// Returns true if this backend owns the given directory.
    fn detect(&self, dir: &Path) -> bool;

    /// Given a set of changed files, return the targets that need to be operated on.
    fn affected_targets(&self, repo_root: &Path, changed_files: &[PathBuf]) -> Vec<Target>;

    /// Format a directory path as a backend-specific target label.
    fn resolve_target(&self, repo_root: &Path, dir: PathBuf) -> Target;

    fn build(&self, repo_root: &Path, targets: &[Target]) -> Result<()>;
    fn test(&self, repo_root: &Path, targets: &[Target]) -> Result<()>;
    fn lint(&self, repo_root: &Path, targets: &[Target]) -> Result<()>;
    fn fmt(&self, repo_root: &Path, changed_files: &[PathBuf]) -> Result<()>;
}

/// Returns all registered backends.
pub fn all_backends() -> Vec<Box<dyn Backend>> {
    vec![Box::new(BazelBackend), Box::new(GoBackend)]
}
