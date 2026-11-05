use core_design::{ContractHead, InitialFile, PartContent};
use core_design::{TestRenderInput as RenderInput, TestStructure as Structure};

fn part() -> PartContent {
    PartContent {
        title: "Backend".into(),
        kind: Some("backend".into()),
        responsibility: "Own logic".into(),
        folder: "backend/src".into(),
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

fn input(parts: &[PartContent]) -> RenderInput<'_> {
    RenderInput {
        project_name: "Demo",
        map_number: 3,
        parts,
        standards_base: 1,
        standards_additions: Some("add-2"),
    }
}

#[test]
fn structure_writes_no_contract_files() {
    let mut second = part();
    second.title = "Database".into();
    second.folder = "db".into();
    second.files.clear();
    let result = Structure::render(&input(&[part(), second])).unwrap();
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
        Structure::render(&input(&[p])).err().unwrap(),
        "part has no folder: Backend"
    );
}

#[test]
fn a_reserved_path_refuses() {
    let mut p = part();
    p.files[0].path = "shadows.MD".into();
    assert_eq!(
        Structure::render(&input(&[p])).err().unwrap(),
        "reserved path: shadows.MD"
    );
}

#[test]
fn a_starter_file_may_now_be_named_contract_yaml() {
    let mut p = part();
    p.files[0].path = "backend/src/contract.yaml".into();
    let result = Structure::render(&input(&[p])).unwrap();
    assert!(
        result
            .files
            .iter()
            .any(|f| f.path == "backend/src/contract.yaml")
    );
}

#[test]
fn shadows_md_names_map_and_standards() {
    let result = Structure::render(&input(&[part()])).unwrap();
    assert_eq!(
        String::from_utf8(result.files[0].bytes.clone()).unwrap(),
        "# Demo\n\nShadows wrote this structure from map version 3.\n\
         Standards: base v1, additions add-2.\n\
         Shadows is the source of truth for the vision, the map and the plans.\n\
         Ask Shadows over MCP where a part's code lives.\n"
    );
    assert_eq!(
        Structure::sha256_hex(b"hello"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}
