use core_design::{ContractHead, InitialFile, PartContent};
use core_design::{TestRenderInput as RenderInput, TestStructure as Structure};

fn part() -> PartContent {
    PartContent {
        title: "Backend".into(),
        kind: Some("backend".into()),
        responsibility: "Own logic".into(),
        folders: vec!["backend/src".into()],
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
fn renders_folders_contracts_files_and_shadows_md() {
    let mut second = part();
    second.title = "Database".into();
    second.folders = vec!["db".into()];
    second.files.clear();
    let result = Structure::render(&input(&[part(), second])).unwrap();
    assert_eq!(result.folders, ["backend", "db", "backend/src"]);
    assert_eq!(
        result
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        [
            "SHADOWS.md",
            "backend/src/contract.yaml",
            "db/contract.yaml",
            "backend/src/main.rs"
        ]
    );
    assert_eq!(
        String::from_utf8(result.files[1].bytes.clone()).unwrap(),
        "# Written by Shadows from the approved map. Shadows is the source.\n\
         title: Backend\nkind: backend\nresponsibility: Own logic\nserves:\n  - G1\n\
         header: \"API: safe\"\nrules:\n  - S1\n"
    );
}

#[test]
fn a_part_without_a_folder_refuses() {
    let mut p = part();
    p.folders.clear();
    assert_eq!(
        Structure::render(&input(&[p])).err().unwrap(),
        "part has no folder: Backend"
    );
}

#[test]
fn a_reserved_path_refuses() {
    let mut p = part();
    p.files[0].path = "backend/src/CONTRACT.yaml".into();
    assert_eq!(
        Structure::render(&input(&[p])).err().unwrap(),
        "reserved path: backend/src/CONTRACT.yaml"
    );
}

#[test]
fn shadows_md_names_map_and_standards() {
    let result = Structure::render(&input(&[part()])).unwrap();
    assert_eq!(
        String::from_utf8(result.files[0].bytes.clone()).unwrap(),
        "# Demo\n\nShadows wrote this structure from map version 3.\n\
         Standards: base v1, additions add-2.\n\
         Shadows is the source of truth for the vision, the map and each part's\n\
         contract; these files are their output.\n"
    );
    assert_eq!(
        Structure::sha256_hex(b"hello"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}
