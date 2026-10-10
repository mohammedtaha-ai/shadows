//! Compiled-in standards and project additions (§23.2).

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use utoipa::ToSchema;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MandatoryPart {
    pub name: String,
    pub owns: String,
    pub waivable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StandardRule {
    pub id: String,
    pub text: String,
    pub parts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ContractTemplate {
    pub rules: Vec<String>,
    pub shape: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BaseStandards {
    pub version: i64,
    pub parts: Vec<MandatoryPart>,
    pub rules: Vec<StandardRule>,
    pub contract_template: ContractTemplate,
}

pub fn base() -> &'static BaseStandards {
    static BASE: OnceLock<BaseStandards> = OnceLock::new();
    BASE.get_or_init(|| parse(include_str!("standards.yaml")).expect("valid standards.yaml"))
}

fn parse(source: &str) -> Result<BaseStandards, String> {
    let docs = YamlLoader::load_from_str(source).map_err(|e| e.to_string())?;
    let doc = docs.first().ok_or("empty standards.yaml")?;
    Ok(BaseStandards {
        version: doc["version"].as_i64().ok_or("version")?,
        parts: list(&doc["parts"])?
            .iter()
            .map(|p| {
                Ok(MandatoryPart {
                    name: text(&p["name"])?,
                    owns: text(&p["owns"])?,
                    waivable: p["waivable"].as_bool().ok_or("waivable")?,
                })
            })
            .collect::<Result<_, String>>()?,
        rules: list(&doc["rules"])?
            .iter()
            .map(|r| {
                Ok(StandardRule {
                    id: text(&r["id"])?,
                    text: text(&r["text"])?,
                    parts: strings(&r["parts"])?,
                })
            })
            .collect::<Result<_, String>>()?,
        contract_template: ContractTemplate {
            rules: strings(&doc["contract_template"]["rules"])?,
            shape: strings(&doc["contract_template"]["shape"])?,
        },
    })
}

fn list(y: &Yaml) -> Result<&Vec<Yaml>, String> {
    y.as_vec()
        .ok_or_else(|| format!("expected a list, found {y:?}"))
}

fn text(y: &Yaml) -> Result<String, String> {
    y.as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("expected text, found {y:?}"))
}

fn strings(y: &Yaml) -> Result<Vec<String>, String> {
    list(y)?.iter().map(text).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct AdditionalPart {
    pub name: String,
    pub owns: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
pub struct StandardsAdditions {
    pub rules: Vec<StandardRule>,
    pub parts: Vec<AdditionalPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct StandardsAdditionsVersion {
    #[serde(skip)]
    pub id: String,
    pub number: i64,
    pub content: StandardsAdditions,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EffectiveStandards {
    pub base: BaseStandards,
    pub additions: Option<StandardsAdditionsVersion>,
}

impl EffectiveStandards {
    /// Base parts followed by added parts, in declared order.
    pub fn mandatory_parts(&self) -> Vec<&str> {
        self.base
            .parts
            .iter()
            .map(|p| p.name.as_str())
            .chain(
                self.additions
                    .iter()
                    .flat_map(|a| &a.content.parts)
                    .map(|p| p.name.as_str()),
            )
            .collect()
    }
}

pub(crate) fn validate(additions: &StandardsAdditions) -> Result<(), String> {
    let mut names: Vec<_> = base().parts.iter().map(|p| p.name.as_str()).collect();
    for part in &additions.parts {
        let name = part.name.as_str();
        let shaped = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !shaped {
            return Err(format!(
                "part \"{name}\": use lowercase letters, digits and -"
            ));
        }
        if names.contains(&name) {
            return Err(format!("part \"{name}\" is already a mandatory part"));
        }
        if part.owns.trim().is_empty() {
            return Err(format!("part \"{name}\" needs what it owns"));
        }
        names.push(name);
    }
    let mut ids = Vec::new();
    for rule in &additions.rules {
        let id = rule.id.as_str();
        let numbered = id.strip_prefix('P').is_some_and(|n| {
            n.starts_with(|c: char| c.is_ascii_digit() && c != '0')
                && n.bytes().all(|c| c.is_ascii_digit())
        });
        if !numbered {
            return Err(format!("rule \"{id}\": a project rule's id is P1, P2, ..."));
        }
        if ids.contains(&id) {
            return Err(format!("rule {id} appears twice"));
        }
        if rule.text.trim().is_empty() {
            return Err(format!("rule {id} has no text"));
        }
        if let Some(unknown) = rule.parts.iter().find(|p| !names.contains(&p.as_str())) {
            return Err(format!(
                "rule {id} names \"{unknown}\", which is not a part"
            ));
        }
        ids.push(id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_standards_parse_with_the_four_mandatory_parts() {
        let b = base();
        assert!(b.version >= 1);
        let parts: Vec<_> = b
            .parts
            .iter()
            .map(|p| (p.name.as_str(), p.waivable))
            .collect();
        assert_eq!(
            parts,
            [
                ("backend", false),
                ("database", false),
                ("api", true),
                ("frontend", true)
            ]
        );
        assert!(b.rules.len() >= 5);
        for rule in &b.rules {
            assert!(rule.id.starts_with('S'));
            for part in &rule.parts {
                assert!(
                    b.parts.iter().any(|p| &p.name == part),
                    "{} names {part}",
                    rule.id
                );
            }
        }
        assert!(
            b.contract_template
                .shape
                .contains(&"obligations".to_owned())
        );
        assert!(!b.contract_template.rules.is_empty());
    }
}
