//! Rendering an approved map snapshot into files.
use super::PartContent;
use super::store::structure::Snapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub path: String,
    pub bytes: Vec<u8>,
}
pub struct Planned {
    pub folders: Vec<String>,
    pub files: Vec<PlannedFile>,
}
pub struct RenderInput<'a> {
    pub project_name: &'a str,
    pub map_number: i64,
    pub parts: &'a [PartContent],
    pub standards_base: i64,
    pub standards_additions: Option<&'a str>,
}
impl Planned {
    /// The files `create_structure` writes; the stage read reports the same number.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}
impl Structure {
    /// The one plan for a snapshot, shared by the write and the stage's count.
    pub(crate) fn plan(snapshot: &Snapshot) -> Result<Planned, String> {
        Self::render(&RenderInput {
            project_name: &snapshot.name,
            map_number: snapshot.intent.map_number,
            parts: &snapshot.parts,
            standards_base: snapshot.intent.standards_version,
            standards_additions: snapshot.intent.standards_additions_version_id.as_deref(),
        })
    }
    pub fn render(input: &RenderInput<'_>) -> Result<Planned, String> {
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
    pub fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(bytes))
    }
}

pub struct Structure;
