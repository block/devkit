use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

fn backend() -> BazelBackend {
    BazelBackend
}

#[test]
fn detect_workspace() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("WORKSPACE"), "").unwrap();
    assert!(backend().detect(tmp.path()));
}

#[test]
fn deduplicate_to_packages_collapses_targets() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let targets = vec![
        Target {
            label: "//pkg/foo:bar".to_string(),
            dir: root.join("pkg/foo"),
        },
        Target {
            label: "//pkg/foo:baz".to_string(),
            dir: root.join("pkg/foo"),
        },
    ];
    let deduped = BazelBackend::deduplicate_to_packages(root, &targets);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].label, "//pkg/foo:all");
    assert!(deduped[0].dir.is_absolute());
    assert_eq!(deduped[0].dir, root.join("pkg/foo"));
}

#[test]
fn deduplicate_to_packages_distinct_packages() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let targets = vec![
        Target {
            label: "//pkg/foo:bar".to_string(),
            dir: root.join("pkg/foo"),
        },
        Target {
            label: "//pkg/bar:baz".to_string(),
            dir: root.join("pkg/bar"),
        },
    ];
    let deduped = BazelBackend::deduplicate_to_packages(root, &targets);
    assert_eq!(deduped.len(), 2);
    let labels: Vec<&str> = deduped.iter().map(|t| t.label.as_str()).collect();
    assert!(labels.contains(&"//pkg/bar:all"));
    assert!(labels.contains(&"//pkg/foo:all"));
}

#[test]
fn resolve_target_subdir() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let dir = root.join("pkg");
    let target = backend().resolve_target(root, dir.clone());
    assert_eq!(target.label, "//pkg:all");
    assert_eq!(target.dir, dir);
}

#[test]
fn label_to_dir_strips_prefix() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    assert_eq!(label_to_dir(root, "//pkg/foo:bar"), root.join("pkg/foo"));
    assert_eq!(label_to_dir(root, "//:something"), root.join(""));
}

#[test]
fn fmt_filter_includes_build_files() {
    let included = [
        "BUILD",
        "BUILD.bazel",
        "WORKSPACE",
        "WORKSPACE.bazel",
        "MODULE.bazel",
        "defs.bzl",
    ];
    for name in &included {
        let f = PathBuf::from(name);
        let fname = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let matches = fname == "BUILD"
            || fname == "BUILD.bazel"
            || fname == "WORKSPACE"
            || fname == "WORKSPACE.bazel"
            || fname == "MODULE.bazel"
            || fname.ends_with(".bzl");
        assert!(matches, "{name} should be included");
    }
}

#[test]
fn fmt_filter_excludes_other_files() {
    let excluded = ["main.go", "lib.rs", "README.md", "Makefile"];
    for name in &excluded {
        let f = PathBuf::from(name);
        let fname = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let matches = fname == "BUILD"
            || fname == "BUILD.bazel"
            || fname == "WORKSPACE"
            || fname == "WORKSPACE.bazel"
            || fname == "MODULE.bazel"
            || fname.ends_with(".bzl");
        assert!(!matches, "{name} should be excluded");
    }
}
