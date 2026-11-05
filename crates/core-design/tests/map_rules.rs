use core_design::{PartContent, TestMapPart as MapPart, TestMapRules as MapRules, VisionContent};
use core_standards::EffectiveStandards;

fn refused(contents: &[PartContent]) -> core_design::MapRefusal {
    let keys: Vec<_> = (0..contents.len()).map(|n| format!("part-{n}")).collect();
    let parts: Vec<_> = contents
        .iter()
        .zip(&keys)
        .map(|(content, key)| MapPart {
            key,
            parent: None,
            content,
        })
        .collect();
    let written: Vec<_> = keys.iter().map(String::as_str).collect();
    MapRules::check_shape(
        &parts,
        &written,
        &VisionContent::default(),
        &EffectiveStandards {
            base: core_standards::Standards::base().clone(),
            additions: None,
        },
    )
    .unwrap_err()
}

#[test]
fn a_file_outside_its_part_names_the_part_and_its_folders() {
    let error = refused(&[PartContent {
        title: "Backend".into(),
        folders: vec!["app".into()],
        files: vec![core_design::InitialFile {
            path: "composer.json".into(),
            content: "".into(),
        }],
        ..Default::default()
    }]);
    assert_eq!(error.rule, "file_outside_part");
    assert!(error.message.contains("Backend"));
    assert!(error.message.contains("composer.json"));
    assert!(error.message.contains("app"));
    assert!(error.message.contains("Move"));
}

#[test]
fn the_same_folder_twice_names_both_parts() {
    let error = refused(&[
        PartContent {
            title: "Backend".into(),
            folders: vec!["src".into()],
            ..Default::default()
        },
        PartContent {
            title: "Frontend".into(),
            folders: vec!["src".into()],
            ..Default::default()
        },
    ]);
    assert_eq!(error.rule, "folder_claimed_twice");
    assert!(error.message.contains("Backend") && error.message.contains("Frontend"));
    assert!(error.message.contains("src") && error.message.contains("one owner"));
}

#[test]
fn a_nested_folder_names_both_parts() {
    let error = refused(&[
        PartContent {
            title: "Backend".into(),
            folders: vec!["app".into()],
            ..Default::default()
        },
        PartContent {
            title: "Services".into(),
            folders: vec!["app/services".into()],
            ..Default::default()
        },
    ]);
    assert_eq!(error.rule, "folder_claimed_twice");
    assert!(error.message.contains("Backend") && error.message.contains("Services"));
    assert!(error.message.contains("app/services"));
}

#[test]
fn two_rules_never_share_a_message() {
    let part = PartContent {
        title: "Backend".into(),
        ..Default::default()
    };
    let file = |path: &str| core_design::InitialFile {
        path: path.into(),
        content: "".into(),
    };
    let cases = [
        (vec![part.clone(), part.clone()], "duplicate_sibling"),
        (
            vec![PartContent {
                serves: vec!["G99".into()],
                ..part.clone()
            }],
            "unknown_vision_item",
        ),
        (
            vec![PartContent {
                contract: core_design::ContractHead {
                    rules: vec!["unknown".into()],
                    ..Default::default()
                },
                ..part.clone()
            }],
            "unknown_rule",
        ),
        (
            vec![PartContent {
                files: vec![file("outside.json")],
                ..part.clone()
            }],
            "file_outside_part",
        ),
        (
            vec![PartContent {
                folders: vec!["src".into()],
                files: vec![file("src/a.rs"), file("src/a.rs")],
                ..part.clone()
            }],
            "duplicate_file",
        ),
        (
            vec![PartContent {
                folders: vec!["src".into()],
                files: vec![file("src/api"), file("src/api/main.rs")],
                ..part.clone()
            }],
            "file_is_folder",
        ),
        (
            vec![
                PartContent {
                    folders: vec!["src".into()],
                    ..part.clone()
                },
                PartContent {
                    title: "Frontend".into(),
                    folders: vec!["src".into()],
                    ..Default::default()
                },
            ],
            "folder_claimed_twice",
        ),
        (
            vec![PartContent {
                folders: vec!["src".into()],
                files: vec![file("SHADOWS.md")],
                ..part.clone()
            }],
            "reserved_file",
        ),
        (
            vec![PartContent {
                folders: vec!["../src".into()],
                ..part.clone()
            }],
            "invalid_path",
        ),
    ];
    let mut rules = std::collections::BTreeSet::new();
    let mut messages = std::collections::BTreeSet::new();
    for (contents, expected) in cases {
        let error = refused(&contents);
        assert_eq!(error.rule, expected);
        assert!(rules.insert(error.rule), "duplicate rule {expected}");
        assert!(
            messages.insert(error.message),
            "duplicate message for {expected}"
        );
    }
    let standards = EffectiveStandards {
        base: core_standards::Standards::base().clone(),
        additions: None,
    };
    for (parts, expected) in [
        (
            vec![
                MapPart {
                    key: "same",
                    parent: None,
                    content: &part,
                },
                MapPart {
                    key: "same",
                    parent: None,
                    content: &part,
                },
            ],
            "duplicate_part",
        ),
        (
            vec![MapPart {
                key: "child",
                parent: Some("missing"),
                content: &part,
            }],
            "missing_parent",
        ),
    ] {
        let error =
            MapRules::check_shape(&parts, &[], &VisionContent::default(), &standards).unwrap_err();
        assert_eq!(error.rule, expected);
        assert!(rules.insert(error.rule));
        assert!(messages.insert(error.message));
    }
}

#[test]
fn a_duplicate_file_names_both_owners() {
    let contents: Vec<_> = ["Backend", "Frontend"]
        .into_iter()
        .map(|title| PartContent {
            title: title.into(),
            folders: vec!["src".into()],
            files: vec![core_design::InitialFile {
                path: "src/main.rs".into(),
                content: "".into(),
            }],
            ..Default::default()
        })
        .collect();
    let error = refused(&contents);
    assert_eq!(error.rule, "duplicate_file");
    assert!(error.message.contains("Backend") && error.message.contains("Frontend"));
}

#[test]
fn windows_device_names_are_refused() {
    for path in ["nul", "backend/con.txt", "Com1", "LPT9.rs"] {
        assert!(MapRules::normalise_folder(path).is_err(), "{path}");
    }
}

#[test]
fn reserved_initial_files_are_refused() {
    for path in ["SHADOWS.md", "shadows.MD"] {
        let content = PartContent {
            title: "Backend".into(),
            folders: vec!["backend".into()],
            files: vec![core_design::InitialFile {
                path: path.into(),
                content: "".into(),
            }],
            ..Default::default()
        };
        let parts = [MapPart {
            key: "p1",
            parent: None,
            content: &content,
        }];
        let standards = EffectiveStandards {
            base: core_standards::Standards::base().clone(),
            additions: None,
        };
        let error = MapRules::check_shape(&parts, &["p1"], &VisionContent::default(), &standards)
            .unwrap_err();
        assert_eq!(error.rule, "reserved_file");
        assert!(error.message.contains(path));
    }
}

#[test]
fn equal_folder_claims_require_distinct_ownership_even_between_parent_and_child() {
    let parent = PartContent {
        title: "Backend".into(),
        folders: vec!["src".into()],
        ..Default::default()
    };
    let child = PartContent {
        title: "Services".into(),
        ..parent.clone()
    };
    let parts = [
        MapPart {
            key: "parent",
            parent: None,
            content: &parent,
        },
        MapPart {
            key: "child",
            parent: Some("parent"),
            content: &child,
        },
    ];
    let error = MapRules::check_shape(
        &parts,
        &["parent", "child"],
        &VisionContent::default(),
        &EffectiveStandards {
            base: core_standards::Standards::base().clone(),
            additions: None,
        },
    )
    .unwrap_err();
    assert_eq!(error.rule, "folder_claimed_twice");
    assert!(error.message.contains("Backend") && error.message.contains("Services"));
    assert!(!error.message.contains("make one part the other's child"));
    assert!(error.message.contains("one owner"));
}

fn tree(parent: &str, child: &str) -> Result<(), core_design::MapRefusal> {
    let content = |title: &str, folder: &str| PartContent {
        title: title.into(),
        folders: if folder.is_empty() {
            vec![]
        } else {
            vec![folder.into()]
        },
        ..Default::default()
    };
    let (a, b) = (content("Parent", parent), content("Child", child));
    let parts = [
        MapPart {
            key: "p",
            parent: None,
            content: &a,
        },
        MapPart {
            key: "c",
            parent: Some("p"),
            content: &b,
        },
    ];
    MapRules::check_shape(
        &parts,
        &["p", "c"],
        &VisionContent::default(),
        &EffectiveStandards {
            base: core_standards::Standards::base().clone(),
            additions: None,
        },
    )
}

#[test]
fn a_part_needs_exactly_one_folder() {
    for folders in [vec![], vec!["app".into(), "config".into()]] {
        let error = refused(&[PartContent {
            title: "Backend".into(),
            folders,
            ..Default::default()
        }]);
        assert_eq!(error.rule, "one_folder");
        assert!(error.message.contains("Backend"));
        assert!(error.message.contains("Give the part exactly one folder"));
    }
}

#[test]
fn a_child_folder_sits_inside_its_parents() {
    let error = tree("frontend", "dashboard").unwrap_err();
    assert_eq!(error.rule, "folder_outside_parent");
    assert!(error.message.contains("dashboard") && error.message.contains("frontend"));
    assert!(tree("frontend", "frontend/dashboard").is_ok());
}

#[test]
fn moving_a_parent_checks_descendant_folders() {
    assert_eq!(
        tree("b", "a/child").unwrap_err().rule,
        "folder_outside_parent"
    );
}

#[test]
fn renaming_a_parent_folder_checks_unchanged_children() {
    let error = tree("web", "frontend/dashboard").unwrap_err();
    assert_eq!(error.rule, "folder_outside_parent");
    assert!(error.message.contains("Child"));
}

#[test]
fn windows_case_and_segment_boundaries_use_the_same_policy() {
    assert!(tree("Frontend", "frontend/dashboard").is_ok());
    assert_eq!(
        tree("front", "frontend/x").unwrap_err().rule,
        "folder_outside_parent"
    );
}
