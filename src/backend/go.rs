use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::{Backend, Target};

pub struct GoBackend;

impl GoBackend {
    fn run<I, S>(cmd: &str, args: I, dir: &Path) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let status = Command::new(cmd)
            .args(args)
            .current_dir(dir)
            .status()
            .with_context(|| format!("failed to run {cmd}"))?;
        if !status.success() {
            anyhow::bail!("{cmd} exited with {status}");
        }
        Ok(())
    }
}

impl Backend for GoBackend {
    fn name(&self) -> &str {
        "go"
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("go.mod").exists() || dir.join("go.work").exists()
    }

    fn affected_targets(&self, repo_root: &Path, changed_files: &[PathBuf]) -> Vec<Target> {
        let mut packages: BTreeSet<PathBuf> = BTreeSet::new();

        for file in changed_files {
            let is_dep_file = file
                .file_name()
                .is_some_and(|name| name == "go.mod" || name == "go.sum" || name == "go.work" || name == "go.work.sum");

            if is_dep_file {
                let dir = file
                    .parent()
                    .map(|p| repo_root.join(p))
                    .unwrap_or_else(|| repo_root.to_path_buf());
                packages.insert(dir);
            } else if file.extension().is_some_and(|ext| ext == "go") {
                #[allow(clippy::collapsible_if)]
                if let Some(parent) = file.parent() {
                    let dir = repo_root.join(parent);
                    if dir.exists() {
                        packages.insert(dir);
                    }
                }
            }
        }

        packages
            .into_iter()
            .map(|dir| self.resolve_target(repo_root, dir))
            .collect()
    }

    fn resolve_target(&self, repo_root: &Path, dir: PathBuf) -> Target {
        let rel = dir.strip_prefix(repo_root).unwrap_or(&dir).to_string_lossy();
        let rel = rel.replace('\\', "/");
        let label = if rel.is_empty() {
            "./...".to_string()
        } else {
            format!("./{rel}/...")
        };
        Target { label, dir }
    }

    fn build(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let labels: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
        let mut args = vec!["build"];
        args.extend(&labels);
        Self::run("go", &args, repo_root)
    }

    fn test(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let labels: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
        let mut args = vec!["test"];
        args.extend(&labels);
        Self::run("go", &args, repo_root)
    }

    fn lint(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let dirs: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
        let mut args = vec!["run"];
        args.extend(&dirs);
        Self::run("golangci-lint", &args, repo_root).context("failed to run golangci-lint — is it installed?")
    }

    fn fmt(&self, repo_root: &Path, changed_files: &[PathBuf]) -> Result<()> {
        let go_files: Vec<PathBuf> = changed_files
            .iter()
            .filter(|f| f.extension().is_some_and(|ext| ext == "go"))
            .map(|f| repo_root.join(f))
            .filter(|f| f.exists())
            .collect();

        if go_files.is_empty() {
            return Ok(());
        }

        let mut args: Vec<&OsStr> = vec![OsStr::new("-w")];
        args.extend(go_files.iter().map(|f| f.as_os_str()));
        Self::run("gofmt", args, repo_root)
    }
}

#[cfg(test)]
#[path = "go_test.rs"]
mod tests;
