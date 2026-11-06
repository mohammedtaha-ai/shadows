use core_design::{PartContent, TestMapPart as MapPart, TestMapRules as MapRules, VisionContent};
use core_standards::EffectiveStandards;

fn standards() -> EffectiveStandards {
    EffectiveStandards {
        base: core_standards::Standards::base().clone(),
        additions: None,
    }
}

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
    MapRules::check_shape(&parts, &written, &VisionContent::default(), &standards()).unwrap_err()
}

/// The parts of a candidate map, with their parents.
fn checked(parts: &[(&str, Option<&str>, PartContent)]) -> Result<(), core_design::MapRefusal> {
    let views: Vec<MapPart<'_>> = parts
        .iter()
        .map(|(key, parent, content)| MapPart {
            key,
            parent: *parent,
            content,
        })
        .collect();
    let written: Vec<&str> = parts.iter().map(|(key, _, _)| *key).collect();
    MapRules::check_shape(&views, &written, &VisionContent::default(), &standards())
}

#[test]
fn the_same_folder_twice_names_both_parts() {
    let error = refused(&[
        PartContent {
            title: "Backend".into(),
            folder: "src".into(),
            ..Default::default()
        },
        PartContent {
            title: "Frontend".into(),
            folder: "src".into(),
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
            folder: "app".into(),
            ..Default::default()
        },
        PartContent {
            title: "Services".into(),
            folder: "app/services".into(),
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
            vec![
                PartContent {
                    folder: "src".into(),
                    ..part.clone()
                },
                PartContent {
                    title: "Frontend".into(),
                    folder: "src".into(),
                    ..Default::default()
                },
            ],
            "folder_claimed_twice",
        ),
        (
            vec![PartContent {
                folder: "../src".into(),
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
fn windows_device_names_are_refused() {
    for path in ["nul", "backend/con.txt", "Com1", "LPT9.rs"] {
        assert!(MapRules::normalise_folder(path).is_err(), "{path}");
    }
}

#[test]
fn equal_folder_claims_require_distinct_ownership_even_between_parent_and_child() {
    let parent = PartContent {
        title: "Backend".into(),
        folder: "src".into(),
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
        folder: folder.into(),
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
    let error = refused(&[PartContent {
        title: "Backend".into(),
        ..Default::default()
    }]);
    assert_eq!(error.rule, "one_folder");
    assert!(error.message.contains("Backend"));
    assert!(error.message.contains("Give the part one folder"));
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

fn layer(id: &str, place: &str) -> core_design::PartLayer {
    core_design::PartLayer {
        layer: id.into(),
        place: place.into(),
        not_used: None,
    }
}

fn not_used(id: &str, reason: &str) -> core_design::PartLayer {
    core_design::PartLayer {
        layer: id.into(),
        place: String::new(),
        not_used: Some(reason.into()),
    }
}

/// A part of a catalog kind.
fn part_of(
    kind: &str,
    title: &str,
    folder: &str,
    layers: Vec<core_design::PartLayer>,
) -> PartContent {
    PartContent {
        title: title.into(),
        kind: Some(kind.into()),
        folder: folder.into(),
        layers,
        ..Default::default()
    }
}

/// The backend's five layers, each with a place inside the folder.
fn backend_layers(base: &str) -> Vec<core_design::PartLayer> {
    [
        "domain",
        "application",
        "interface",
        "infrastructure",
        "composition",
    ]
    .iter()
    .map(|id| layer(id, &format!("{base}/app/{id}")))
    .collect()
}

/// The frontend's four layers, each with a place inside the folder.
fn frontend_layers(base: &str) -> Vec<core_design::PartLayer> {
    ["screens", "features", "data_access", "design_system"]
        .iter()
        .map(|id| layer(id, &format!("{base}/{id}")))
        .collect()
}

/// A backend whose listed layers are used; the rest of its catalog is not.
fn backend_with(folder: &str, used: Vec<core_design::PartLayer>) -> PartContent {
    let mut layers = used;
    for id in [
        "domain",
        "application",
        "interface",
        "infrastructure",
        "composition",
    ] {
        if !layers.iter().any(|l| l.layer == id) {
            layers.push(not_used(id, "not used in this project"));
        }
    }
    part_of("backend", "Backend", folder, layers)
}

#[test]
fn a_missing_required_layer_is_refused() {
    let mut content = part_of("backend", "Backend", "backend", backend_layers("backend"));
    content.layers.retain(|l| l.layer != "composition");
    let error = refused(&[content]);
    assert_eq!(error.rule, "missing_layer");
    assert!(error.message.contains("Backend"));
    assert!(error.message.contains("composition"));
}

#[test]
fn an_unknown_layer_is_refused() {
    let mut unknown = part_of("backend", "Backend", "backend", backend_layers("backend"));
    unknown.layers.push(layer("ports", "backend/app/Ports"));
    let error = refused(&[unknown]);
    assert_eq!(error.rule, "unknown_layer");
    assert!(error.message.contains("ports"));

    let mut twice = part_of("backend", "Backend", "backend", backend_layers("backend"));
    twice.layers.push(layer("domain", "backend/app/Domain2"));
    let error = refused(&[twice]);
    assert_eq!(error.rule, "unknown_layer");
    assert!(error.message.contains("twice"));
}

#[test]
fn two_layers_sharing_a_place_are_refused() {
    let mut content = part_of("backend", "Backend", "backend", backend_layers("backend"));
    content.layers[1].place = content.layers[0].place.clone();
    let error = refused(&[content]);
    assert_eq!(error.rule, "layer_place_shared");
    assert!(error.message.contains("domain"));
    assert!(error.message.contains("application"));
}

#[test]
fn a_not_used_layer_needs_a_reason_and_no_place() {
    let mut blank = part_of("backend", "Backend", "backend", backend_layers("backend"));
    blank.layers[0] = core_design::PartLayer {
        layer: "domain".into(),
        place: String::new(),
        not_used: Some("  ".into()),
    };
    let error = refused(&[blank]);
    assert_eq!(error.rule, "layer_not_used_without_reason");
    assert!(error.message.contains("domain"));

    let mut both = part_of("backend", "Backend", "backend", backend_layers("backend"));
    both.layers[0].not_used = Some("no clients yet".into());
    let error = refused(&[both]);
    assert_eq!(error.rule, "layer_not_used_without_reason");
    assert!(error.message.contains("place"));
}

#[test]
fn a_layer_above_a_child_folder_does_not_claim_the_child() {
    let parent = backend_with("backend", vec![layer("domain", "backend/modules")]);
    let child = part_of("custom", "Billing", "backend/modules/billing", vec![]);
    checked(&[("parent", None, parent), ("child", Some("parent"), child)]).unwrap();
}

#[test]
fn a_grouping_part_needs_no_layers() {
    let parent = part_of("frontend", "Frontend", "frontend", vec![]);
    let web = part_of(
        "frontend",
        "Web",
        "frontend/web",
        frontend_layers("frontend/web"),
    );
    let mobile = part_of(
        "frontend",
        "Mobile",
        "frontend/mobile",
        frontend_layers("frontend/mobile"),
    );
    assert!(
        checked(&[
            ("parent", None, parent),
            ("web", Some("parent"), web),
            ("mobile", Some("parent"), mobile),
        ])
        .is_ok()
    );
}

#[test]
fn module_patterns_are_one_place_per_module() {
    let content = part_of(
        "backend",
        "Backend",
        "crates",
        vec![
            layer("domain", "crates/core-{module}/src/model.rs"),
            layer("application", "crates/core-{module}/src/lib.rs"),
            layer("interface", "crates/core-{module}/src/http.rs"),
            layer("infrastructure", "crates/core-{module}/src/store"),
            layer("composition", "crates/core-{module}/src/main.rs"),
        ],
    );
    assert!(checked(&[("backend", None, content)]).is_ok());
}

#[test]
fn a_part_folder_cannot_hold_a_module_marker() {
    let content = backend_with("crates/core-{module}", vec![]);
    let error = refused(&[content]);
    assert_eq!(error.rule, "invalid_path");
    assert!(error.message.contains("Backend"));
    assert!(error.message.contains("layer place"));
}

#[test]
fn layer_places_use_windows_case_rules() {
    let content = backend_with(
        "backend",
        vec![
            layer("domain", "Backend/App/Domain"),
            layer("application", "backend/app/domain"),
        ],
    );
    let error = refused(&[content]);
    assert_eq!(error.rule, "layer_place_shared");
}

#[test]
fn an_included_optional_layer_needs_a_place() {
    let mut p = part_of("frontend", "Web", "web", frontend_layers("web"));
    p.layers.retain(|layer| layer.layer != "design_system");
    p.layers.push(layer("design_system", ""));
    assert_eq!(refused(&[p]).rule, "missing_layer");
}

#[test]
fn a_laravel_map_with_a_nested_database_part_is_approved() {
    let backend = part_of("backend", "Backend", "backend", backend_layers("backend"));
    let database = part_of(
        "database",
        "Database",
        "backend/database",
        vec![
            layer("schema", "backend/database/schema"),
            layer("migrations", "backend/database/migrations"),
            layer("connection", "backend/config/database.php"),
        ],
    );
    checked(&[
        ("backend", None, backend),
        ("database", Some("backend"), database),
    ])
    .unwrap();
}

#[test]
fn a_place_outside_its_part_is_accepted() {
    let content = backend_with("backend", vec![layer("domain", "shared/Domain")]);
    checked(&[("backend", None, content)]).unwrap();
}

#[test]
fn a_marker_may_appear_more_than_once() {
    let mut content = backend_with(
        "backend",
        vec![layer("domain", "backend/{module}/src/{module}")],
    );
    MapRules::normalise(&mut content).unwrap();
    checked(&[("backend", None, content)]).unwrap();
}

#[test]
fn a_used_layer_place_must_be_a_valid_relative_path() {
    for path in [
        "/outside",
        "C:/outside",
        "backend/../outside",
        "backend//domain",
    ] {
        let content = backend_with("backend", vec![layer("domain", path)]);
        assert_eq!(refused(&[content]).rule, "invalid_path", "{path}");
    }
}

#[test]
fn a_nested_part_satisfies_its_required_kind() {
    let backend = backend_with("backend", vec![]);
    let database = part_of("database", "Database", "backend/database", vec![]);
    let parts = [
        MapPart {
            key: "b",
            parent: None,
            content: &backend,
        },
        MapPart {
            key: "d",
            parent: Some("b"),
            content: &database,
        },
    ];
    assert_eq!(
        MapRules::missing_kinds(&parts, &["api".into(), "frontend".into()], &standards()),
        Vec::<String>::new(),
    );
}
