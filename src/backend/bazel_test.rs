use super::*;
use tempfile::TempDir;

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
fn label_to_dir_strips_prefix() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    assert_eq!(label_to_dir(root, "//pkg/foo:bar"), root.join("pkg/foo"));
    assert_eq!(label_to_dir(root, "//:something"), root.join(""));
}
