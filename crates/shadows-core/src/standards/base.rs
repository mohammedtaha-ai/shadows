//! Parsing the bundled base standards.

use super::{BaseStandards, ContractTemplate, MandatoryPart, StandardRule, Standards};
use std::sync::OnceLock;
use yaml_rust2::{Yaml, YamlLoader};

impl Standards {
    /// The immutable standards bundled with this binary.
    pub fn base() -> &'static BaseStandards {
        static BASE: OnceLock<BaseStandards> = OnceLock::new();
        BASE.get_or_init(|| parse(include_str!("standards.yaml")).expect("valid standards.yaml"))
    }
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

#[cfg(test)]
mod tests {
    use super::Standards;

    #[test]
    fn the_base_standards_parse_with_the_four_mandatory_parts() {
        let b = Standards::base();
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
