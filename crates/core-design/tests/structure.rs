use core_design::{ContractHead, InitialFile, PartContent};
use core_design::{TestRenderInput as RenderInput, TestStructure as Structure};

fn part() -> PartContent {
    PartContent {
        title: "Backend".into(),
        kind: Some("backend".into()),
        responsibility: "Own logic".into(),
        folder: "backend/src".into(),
        layers: [
            "domain",
            "application",
            "interface",
            "infrastructure",
            "composition",
        ]
        .map(|id| core_design::PartLayer {
            layer: id.into(),
            place: String::new(),
            not_used: Some("Not needed by this fixture".into()),
        })
        .into(),
        serves: vec!["G1".into()],
        contract: ContractHead {
            header: "API: safe".into(),
            rules: vec!["S1".into()],
        },
        files: vec![InitialFile {
            path: "backend/src/main.rs".into(),
            content: "hello".into(),
        }],
        ..Default::default()
    }
}

fn render(parts: &[PartContent]) -> Result<core_design::TestPlanned, String> {
    let parents: Vec<_> = (0..parts.len()).map(|n| (n.to_string(), None)).collect();
    render_tree(parts, &parents)
}

fn render_tree(
    parts: &[PartContent],
    parents: &[(String, Option<String>)],
) -> Result<core_design::TestPlanned, String> {
    let standards = core_standards::EffectiveStandards {
        base: core_standards::Standards::base().clone(),
        additions: None,
    };
    Structure::render(&RenderInput {
        project_name: "Demo",
        map_number: 3,
        parts,
        parents,
        standards: &standards,
        standards_base: 2,
        standards_additions: Some("add-2"),
    })
}

#[test]
fn structure_writes_no_contract_files() {
    let mut second = part();
    second.title = "Database".into();
    second.folder = "db".into();
    second.files.clear();
    let result = render(&[part(), second]).unwrap();
    assert_eq!(result.folders, ["backend", "db", "backend/src"]);
    let paths: Vec<_> = result.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["SHADOWS.md", "backend/src/main.rs"]);
    assert!(!paths.iter().any(|p| p.ends_with("contract.yaml")));
    let text = String::from_utf8(result.files[0].bytes.clone()).unwrap();
    assert!(text.contains("Shadows is the source of truth"));
    assert!(!text.contains("contract"));
}

#[test]
fn a_part_without_a_folder_refuses() {
    let mut p = part();
    p.folder.clear();
    assert_eq!(
        render(&[p]).err().unwrap(),
        "part \"Backend\" has no folder. Give the part one folder."
    );
}

#[test]
fn a_reserved_path_refuses() {
    let mut p = part();
    p.files[0].path = "shadows.MD".into();
    assert_eq!(render(&[p]).err().unwrap(), "reserved path: shadows.MD");
}

#[test]
fn a_starter_file_may_now_be_named_contract_yaml() {
    let mut p = part();
    p.files[0].path = "backend/src/contract.yaml".into();
    let result = render(&[p]).unwrap();
    assert!(
        result
            .files
            .iter()
            .any(|f| f.path == "backend/src/contract.yaml")
    );
}

#[test]
fn shadows_md_names_map_and_standards() {
    let result = render(&[part()]).unwrap();
    assert_eq!(
        String::from_utf8(result.files[0].bytes.clone()).unwrap(),
        "# Demo\n\nShadows wrote this structure from map version 3.\n\
         Standards: base v2, additions add-2.\n\
         Shadows is the source of truth for the vision, the map and the plans.\n\
         Ask Shadows over MCP where a part's code lives.\n"
    );
    assert_eq!(
        Structure::sha256_hex(b"hello"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn structure_makes_each_layer_folder() {
    let mut p = part();
    p.layers[0].place = "backend/src/app/Domain".into();
    p.layers[0].not_used = None;
    let plan = render(&[p]).unwrap();
    assert!(plan.folders.iter().any(|p| p == "backend/src/app/Domain"));
    assert!(plan.folders.iter().any(|p| p == "backend/src/app"));
}

#[test]
fn a_module_pattern_place_makes_no_folder() {
    let mut p = part();
    p.layers[0].place = "backend/src/core-{module}/model.rs".into();
    p.layers[0].not_used = None;
    let plan = render(&[p]).unwrap();
    assert!(!plan.folders.iter().any(|p| p.contains("{module}")));
}

#[test]
fn a_layer_folder_never_holds_a_generated_contract() {
    let mut p = part();
    p.layers[0].place = "backend/src/domain".into();
    p.layers[0].not_used = None;
    let plan = render(&[p]).unwrap();
    assert!(!plan.files.iter().any(|p| p.path.ends_with("contract.yaml")));
}

#[test]
fn render_refuses_a_snapshot_that_breaks_the_map_rules() {
    let mut p = part();
    p.folder.clear();
    assert!(
        render(&[p])
            .err()
            .unwrap()
            .contains("Give the part one folder")
    );
}

#[test]
fn render_checks_child_folders_against_their_parents() {
    let parent = part();
    let mut child = part();
    child.title = "Child".into();
    child.folder = "elsewhere".into();
    child.files.clear();
    let parents = [
        ("parent".into(), None),
        ("child".into(), Some("parent".into())),
    ];
    let error = render_tree(&[parent, child], &parents).err().unwrap();
    assert!(error.contains("outside its parent"), "{error}");
}
