//! Rendering an approved map snapshot into files.
use super::PartContent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedFile {
    pub path: String,
    pub bytes: Vec<u8>,
}
pub(crate) struct Planned {
    pub folders: Vec<String>,
    pub files: Vec<PlannedFile>,
}
pub(crate) struct RenderInput<'a> {
    pub project_name: &'a str,
    pub map_number: i64,
    pub parts: &'a [PartContent],
    pub standards_base: i64,
    pub standards_additions: Option<&'a str>,
}
impl Structure {
    pub(crate) fn render(input: &RenderInput<'_>) -> Result<Planned, String> {
        use super::map_rules::MapRules;
        let mut folders = std::collections::BTreeSet::new();
        let mut contracts = Vec::new();
        let mut initial = Vec::new();
        for part in input.parts {
            let first = part
                .folders
                .first()
                .ok_or_else(|| format!("part has no folder: {}", part.title))?;
            for folder in &part.folders {
                let folder = MapRules::normalise_folder(folder)?;
                let mut parent = String::new();
                for segment in folder.split('/') {
                    if !parent.is_empty() {
                        parent.push('/');
                    }
                    parent.push_str(segment);
                    folders.insert(parent.clone());
                }
            }
            contracts.push(PlannedFile {
                path: format!("{}/contract.yaml", MapRules::normalise_folder(first)?),
                bytes: contract(part)?.into_bytes(),
            });
            for file in &part.files {
                let path = MapRules::normalise_folder(&file.path)?;
                MapRules::check_reserved(
                    &path,
                    input.parts.iter().filter_map(|p| p.folders.first()),
                )?;
                initial.push(PlannedFile {
                    path,
                    bytes: file.content.as_bytes().to_vec(),
                });
            }
        }
        let mut files = vec![PlannedFile {
            path: "SHADOWS.md".into(),
            bytes: format!(
                "# {}\n\nShadows wrote this structure from map version {}.\n\
         Standards: base v{}, additions {}.\n\
         Shadows is the source of truth for the vision, the map and each part's\n\
         contract; these files are their output.\n",
                input.project_name,
                input.map_number,
                input.standards_base,
                input.standards_additions.unwrap_or("none")
            )
            .into_bytes(),
        }];
        files.extend(contracts);
        files.extend(initial);
        let mut seen = std::collections::BTreeSet::new();
        for file in &files {
            let path = file.path.to_lowercase();
            if !seen.insert(path.clone()) {
                return Err(format!("duplicate file: {}", file.path));
            }
            if folders.iter().any(|f| {
                f.eq_ignore_ascii_case(&path) || f.to_lowercase().starts_with(&format!("{path}/"))
            }) || files
                .iter()
                .any(|f| f.path.to_lowercase().starts_with(&format!("{path}/")))
            {
                return Err(format!("file path is also a directory: {}", file.path));
            }
        }
        let mut folders: Vec<_> = folders.into_iter().collect();
        folders.sort_by_key(|f| (f.matches('/').count(), f.clone()));
        Ok(Planned { folders, files })
    }
}

fn contract(part: &PartContent) -> Result<String, String> {
    use yaml_rust2::{Yaml, YamlEmitter, yaml::Hash};
    let string = |s: &str| Yaml::String(s.into());
    let list = |values: &[String]| Yaml::Array(values.iter().map(|s| string(s)).collect());
    let mut fields = Hash::new();
    fields.insert(string("title"), string(&part.title));
    fields.insert(
        string("kind"),
        part.kind.as_deref().map(string).unwrap_or(Yaml::Null),
    );
    fields.insert(string("responsibility"), string(&part.responsibility));
    fields.insert(string("serves"), list(&part.serves));
    fields.insert(string("header"), string(&part.contract.header));
    fields.insert(string("rules"), list(&part.contract.rules));
    let mut out = String::new();
    YamlEmitter::new(&mut out)
        .dump(&Yaml::Hash(fields))
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "# Written by Shadows from the approved map. Shadows is the source.\n{}\n",
        out.strip_prefix("---\n").unwrap_or(&out)
    ))
}
impl Structure {
    pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(bytes))
    }
}

pub(in crate::design) struct Structure;

#[cfg(test)]
mod tests {
    use super::{RenderInput, Structure};
    use crate::design::{ContractHead, InitialFile, PartContent};

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
}
