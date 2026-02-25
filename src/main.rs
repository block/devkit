mod backend;
mod git;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;

use backend::{Backend, all_backends};

#[derive(Parser)]
#[command(name = "kit", about = "Universal build tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,

    /// Base branch to diff against (default: main).
    #[arg(long, default_value = "main", global = true)]
    base: String,

    /// Repository root (auto-detected if not set).
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build changed targets (or specific directories).
    Build {
        /// Directories to build. If empty, builds targets affected by changes on the current branch.
        dirs: Vec<PathBuf>,
    },
    /// Test changed targets (or specific directories).
    Test {
        /// Directories to test. If empty, tests targets affected by changes on the current branch.
        dirs: Vec<PathBuf>,
    },
    /// Lint changed targets (or specific directories).
    Lint {
        /// Directories to lint. If empty, lints targets affected by changes on the current branch.
        dirs: Vec<PathBuf>,
    },
    /// Format changed files (or specific directories/files).
    Fmt {
        /// Files or directories to format. If empty, formats files changed on the current branch.
        dirs: Vec<PathBuf>,
    },
    /// Detect the build system(s) in the repository.
    Detect,
}

fn detect_backend<'a>(backends: &'a [Box<dyn Backend>], repo_root: &std::path::Path) -> Option<&'a dyn Backend> {
    backends
        .iter()
        .find_map(|b| if b.detect(repo_root) { Some(b.as_ref()) } else { None })
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo_root = match cli.repo {
        Some(p) => p
            .canonicalize()
            .with_context(|| format!("could not canonicalize repo root: {}", p.display()))?,
        None => {
            let root = git::repo_root().context("could not detect repo root")?;
            root.canonicalize()
                .with_context(|| format!("could not canonicalize repo root: {}", root.display()))?
        }
    };
    let backends = all_backends();

    let backend = match detect_backend(&backends, &repo_root) {
        Some(b) => b,
        None => {
            let supported: Vec<&str> = backends.iter().map(|b| b.name()).collect();
            anyhow::bail!(
                "kit does not support the build system in {}. \
                 kit cannot be used to build, test, lint, or format this project.\n\
                 Supported backends: {}",
                repo_root.display(),
                supported.join(", "),
            );
        }
    };

    eprintln!("kit: detected {} backend", backend.name());

    match cli.command {
        Cmd::Build { dirs } => {
            let targets = resolve_targets(backend, &repo_root, &cli.base, dirs)?;
            eprintln!("kit: building {} target(s)", targets.len());
            backend.build(&repo_root, &targets)
        }
        Cmd::Test { dirs } => {
            let targets = resolve_targets(backend, &repo_root, &cli.base, dirs)?;
            eprintln!("kit: testing {} target(s)", targets.len());
            backend.test(&repo_root, &targets)
        }
        Cmd::Lint { dirs } => {
            let targets = resolve_targets(backend, &repo_root, &cli.base, dirs)?;
            eprintln!("kit: linting {} target(s)", targets.len());
            backend.lint(&repo_root, &targets)
        }
        Cmd::Fmt { dirs } => {
            let files = if dirs.is_empty() {
                git::changed_files(&repo_root, &cli.base)?
            } else {
                resolve_file_args(&repo_root, dirs)?
            };
            eprintln!("kit: formatting {} file(s)", files.len());
            backend.fmt(&repo_root, &files)
        }
        Cmd::Detect => {
            println!("{}", backend.name());
            Ok(())
        }
    }
}

fn canonical_cwd() -> Result<PathBuf> {
    env::current_dir()
        .context("failed to get current directory")?
        .canonicalize()
        .context("failed to canonicalize current directory")
}

fn resolve_targets(
    backend: &dyn Backend,
    repo_root: &std::path::Path,
    base: &str,
    dirs: Vec<PathBuf>,
) -> Result<Vec<backend::Target>> {
    if dirs.is_empty() {
        let changed = git::changed_files(repo_root, base)?;
        eprintln!("kit: {} changed files on branch", changed.len());
        Ok(backend.affected_targets(repo_root, &changed))
    } else {
        let cwd = canonical_cwd()?;
        let mut targets = Vec::new();
        for d in dirs {
            let mut full = cwd.join(&d);
            if full.strip_prefix(repo_root).is_err() {
                anyhow::bail!("path {} is outside repository root", full.display());
            }
            if full.is_file() {
                full = full
                    .parent()
                    .with_context(|| format!("{} has no parent directory", d.display()))?
                    .to_path_buf();
            }
            targets.push(backend.resolve_target(repo_root, full));
        }
        Ok(targets)
    }
}

fn resolve_file_args(repo_root: &std::path::Path, dirs: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let cwd = canonical_cwd()?;
    let mut files = Vec::new();
    for d in dirs {
        let full = cwd.join(&d);
        if full.strip_prefix(repo_root).is_err() {
            anyhow::bail!("path {} is outside repository root", full.display());
        }
        let rel = full.strip_prefix(repo_root).unwrap().to_path_buf();
        files.push(rel);
    }
    Ok(files)
}
