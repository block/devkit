use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

fn backend() -> GoBackend {
    GoBackend
}

#[test]
fn detect_go_mod() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("go.mod"), "module test").unwrap();
    assert!(backend().detect(tmp.path()));
}

#[test]
fn affected_targets_go_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("pkg/foo")).unwrap();
    std::fs::create_dir_all(root.join("pkg/bar")).unwrap();

    let changed = vec![
        PathBuf::from("pkg/foo/main.go"),
        PathBuf::from("pkg/bar/util.go"),
    ];
    let targets = backend().affected_targets(root, &changed);
    let labels: Vec<&str> = targets.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels, vec!["./pkg/bar/...", "./pkg/foo/..."]);
}

#[test]
fn affected_targets_ignores_non_go_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("pkg")).unwrap();

    let changed = vec![
        PathBuf::from("pkg/readme.md"),
        PathBuf::from("pkg/data.json"),
    ];
    let targets = backend().affected_targets(root, &changed);
    assert!(targets.is_empty());
}

#[test]
fn affected_targets_go_mod_and_go_sum() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let changed = vec![PathBuf::from("go.mod"), PathBuf::from("go.sum")];
    let targets = backend().affected_targets(root, &changed);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].label, "./...");
}

#[test]
fn affected_targets_nested_go_mod() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("sub")).unwrap();

    let changed = vec![PathBuf::from("sub/go.mod")];
    let targets = backend().affected_targets(root, &changed);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].label, "./sub/...");
}

#[test]
fn resolve_target_subdir() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let dir = root.join("pkg/foo");
    let target = backend().resolve_target(root, dir.clone());
    assert_eq!(target.label, "./pkg/foo/...");
    assert_eq!(target.dir, dir);
}
