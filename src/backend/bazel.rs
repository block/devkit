use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::{Backend, Target};

pub struct BazelBackend;

impl BazelBackend {
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

    fn bazel_cmd() -> &'static str {
        if which_exists("bazelisk") { "bazelisk" } else { "bazel" }
    }

    /// Use `bazel query` with `rdeps` to find all targets affected by the changed files.
    fn query_rdeps(repo_root: &Path, changed_files: &[PathBuf]) -> Result<Vec<Target>> {
        if changed_files.is_empty() {
            return Ok(vec![]);
        }

        let file_labels: Vec<String> = changed_files
            .iter()
            .map(|f| f.to_string_lossy().replace('\\', "/"))
            .collect();

        let quoted: Vec<String> = file_labels.iter().map(|f| format!("\"{f}\"")).collect();
        let set_expr = quoted.join(" ");
        let query = format!("rdeps(//..., set({set_expr}))");

        let output = Command::new(Self::bazel_cmd())
            .args(["query", &query, "--keep_going", "--output=label"])
            .current_dir(repo_root)
            .output()
            .context("failed to run bazel query")?;

        let stdout = String::from_utf8(output.stdout).context("invalid utf-8 from bazel query")?;

        let targets: Vec<Target> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|label| {
                let dir = label_to_dir(repo_root, label);
                Target {
                    label: label.to_string(),
                    dir,
                }
            })
            .collect();

        Ok(targets)
    }

    /// Deduplicate targets to package-level wildcard patterns where possible.
    fn deduplicate_to_packages(repo_root: &Path, targets: &[Target]) -> Vec<Target> {
        let mut packages: BTreeSet<String> = BTreeSet::new();
        for t in targets {
            if let Some(pkg) = t.label.split(':').next() {
                packages.insert(format!("{pkg}:all"));
            }
        }
        packages
            .into_iter()
            .map(|label| {
                let rel = label.trim_start_matches("//").split(':').next().unwrap_or("");
                let dir = repo_root.join(rel);
                Target { label, dir }
            })
            .collect()
    }
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn label_to_dir(repo_root: &Path, label: &str) -> PathBuf {
    let pkg = label.trim_start_matches("//").split(':').next().unwrap_or("");
    repo_root.join(pkg)
}

impl Backend for BazelBackend {
    fn name(&self) -> &str {
        "bazel"
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("WORKSPACE").exists() || dir.join("WORKSPACE.bazel").exists() || dir.join("MODULE.bazel").exists()
    }

    fn affected_targets(&self, repo_root: &Path, changed_files: &[PathBuf]) -> Vec<Target> {
        match Self::query_rdeps(repo_root, changed_files) {
            Ok(targets) => Self::deduplicate_to_packages(repo_root, &targets),
            Err(e) => {
                eprintln!("kit: bazel query failed ({e:#}), falling back to package detection");
                let mut packages: BTreeSet<PathBuf> = BTreeSet::new();
                for file in changed_files {
                    let mut dir = file.parent().map(|p| repo_root.join(p));
                    while let Some(d) = dir {
                        if d.join("BUILD").exists() || d.join("BUILD.bazel").exists() {
                            packages.insert(d);
                            break;
                        }
                        if d == repo_root {
                            break;
                        }
                        dir = d.parent().map(|p| p.to_path_buf());
                    }
                }
                packages
                    .into_iter()
                    .map(|dir| self.resolve_target(repo_root, dir))
                    .collect()
            }
        }
    }

    fn resolve_target(&self, repo_root: &Path, dir: PathBuf) -> Target {
        let rel = dir
            .strip_prefix(repo_root)
            .unwrap_or(&dir)
            .to_string_lossy()
            .replace('\\', "/");
        let label = if rel.is_empty() {
            "//...:all".to_string()
        } else {
            format!("//{rel}:all")
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
        Self::run(Self::bazel_cmd(), &args, repo_root)
    }

    fn test(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let labels: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
        let mut args: Vec<&str> = vec!["test"];
        args.extend(&labels);
        Self::run(Self::bazel_cmd(), &args, repo_root)
    }

    fn lint(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        if which_exists("buildifier") {
            let labels: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
            let mut args = vec!["run", "//:buildifier", "--"];
            args.extend(&labels);
            Self::run(Self::bazel_cmd(), &args, repo_root).or_else(|_| {
                eprintln!("kit: //:buildifier target not found, running buildifier directly");
                let dirs: Vec<&str> = targets.iter().map(|t| t.dir.to_str().unwrap_or(".")).collect();
                let mut fallback_args = vec!["-lint=warn", "-r"];
                fallback_args.extend(&dirs);
                Self::run("buildifier", &fallback_args, repo_root)
            })
        } else {
            eprintln!("kit: buildifier not found, skipping lint");
            Ok(())
        }
    }

    fn fmt(&self, repo_root: &Path, changed_files: &[PathBuf]) -> Result<()> {
        let build_files: Vec<PathBuf> = changed_files
            .iter()
            .filter(|f| {
                let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name == "BUILD"
                    || name == "BUILD.bazel"
                    || name == "WORKSPACE"
                    || name == "WORKSPACE.bazel"
                    || name == "MODULE.bazel"
                    || name.ends_with(".bzl")
            })
            .map(|f| repo_root.join(f))
            .filter(|f| f.exists())
            .collect();

        if build_files.is_empty() {
            return Ok(());
        }

        if !which_exists("buildifier") {
            eprintln!("kit: buildifier not found, skipping format");
            return Ok(());
        }

        let mut args: Vec<&OsStr> = vec![OsStr::new("-mode=fix")];
        args.extend(build_files.iter().map(|f| f.as_os_str()));
        Self::run("buildifier", args, repo_root)
    }
}

#[cfg(test)]
#[path = "bazel_test.rs"]
mod tests;
