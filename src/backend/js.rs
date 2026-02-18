use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::{Backend, Target};

enum Orchestrator {
    Nx,
    Turbo,
    Plain,
}

impl Orchestrator {
    fn detect(repo_root: &Path) -> Self {
        if repo_root.join("nx.json").exists() {
            Self::Nx
        } else if repo_root.join("turbo.json").exists() {
            Self::Turbo
        } else {
            Self::Plain
        }
    }

    fn name(&self) -> Option<&'static str> {
        match self {
            Self::Nx => Some("nx"),
            Self::Turbo => Some("turbo"),
            Self::Plain => None,
        }
    }
}

fn run<I, S>(cmd: &str, args: I, dir: &Path) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = std::process::Command::new(cmd)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| format!("failed to run {cmd}"))?;
    if !status.success() {
        anyhow::bail!("{cmd} exited with {status}");
    }
    Ok(())
}

pub struct JsBackend {
    /// Backend name (e.g. "pnpm", "yarn").
    name: &'static str,
    /// Lock files that identify this backend.
    lock_files: &'static [&'static str],
    /// Command used to install and run scripts.
    cmd: &'static str,
}

pub const PNPM: JsBackend = JsBackend {
    name: "pnpm",
    lock_files: &["pnpm-workspace.yaml", "pnpm-lock.yaml"],
    cmd: "pnpm",
};

pub const YARN: JsBackend = JsBackend {
    name: "yarn",
    lock_files: &["yarn.lock"],
    cmd: "yarn",
};

impl JsBackend {
    fn run_script(&self, orch: &Orchestrator, repo_root: &Path, target: &str) -> Result<()> {
        match orch {
            Orchestrator::Nx => run("nx", ["affected", &format!("--target={target}")], repo_root),
            Orchestrator::Turbo => run("turbo", ["run", target, "--filter=...[origin/main]"], repo_root),
            Orchestrator::Plain => run(self.cmd, [target], repo_root),
        }
    }

    fn orch(&self, repo_root: &Path) -> Orchestrator {
        let orch = Orchestrator::detect(repo_root);
        eprintln!("kit: using {} orchestrator", orch.name().unwrap_or(self.name));
        orch
    }
}

impl Backend for JsBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn detect(&self, dir: &Path) -> bool {
        self.lock_files.iter().any(|f| dir.join(f).exists())
    }

    fn affected_targets(&self, repo_root: &Path, _changed_files: &[PathBuf]) -> Vec<Target> {
        vec![Target {
            label: ".".to_string(),
            dir: repo_root.to_path_buf(),
        }]
    }

    fn resolve_target(&self, repo_root: &Path, dir: PathBuf) -> Target {
        let rel = dir.strip_prefix(repo_root).unwrap_or(&dir).to_string_lossy();
        let rel = rel.replace('\\', "/");
        let label = if rel.is_empty() {
            ".".to_string()
        } else {
            format!("./{rel}")
        };
        Target { label, dir }
    }

    fn build(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let orch = self.orch(repo_root);
        run(self.cmd, ["install"], repo_root)?;
        self.run_script(&orch, repo_root, "build")
    }

    fn test(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let orch = self.orch(repo_root);
        self.run_script(&orch, repo_root, "test")
    }

    fn lint(&self, repo_root: &Path, targets: &[Target]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let orch = self.orch(repo_root);
        self.run_script(&orch, repo_root, "lint")
    }

    fn fmt(&self, repo_root: &Path, _changed_files: &[PathBuf]) -> Result<()> {
        let orch = self.orch(repo_root);
        match orch {
            Orchestrator::Nx => run("nx", ["format:write"], repo_root),
            _ => self.run_script(&orch, repo_root, "format"),
        }
    }
}
