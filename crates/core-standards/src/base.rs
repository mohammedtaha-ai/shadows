//! Parsing the bundled base standards.

use super::{BaseStandards, LayerRef, LayerSpec, PartKind, Presence, StandardRule, Standards};
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
                Ok(PartKind {
                    name: text(&p["name"])?,
                    owns: text(&p["owns"])?,
                    presence: presence(&p["presence"])?,
                    layers: layers(&p["layers"])?,
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
    })
}

fn presence(y: &Yaml) -> Result<Presence, String> {
    match text(y)?.as_str() {
        "required" => Ok(Presence::Required),
        "waivable" => Ok(Presence::Waivable),
        "optional" => Ok(Presence::Optional),
        other => Err(format!(
            "presence: expected required, waivable or optional, found {other}"
        )),
    }
}

fn layers(y: &Yaml) -> Result<Vec<LayerSpec>, String> {
    list(y)?
        .iter()
        .map(|l| {
            Ok(LayerSpec {
                id: text(&l["id"])?,
                role: text(&l["role"])?,
                required: l["required"].as_bool().ok_or("required")?,
                elements: strings(&l["elements"])?,
                may_use: list(&l["may_use"])?
                    .iter()
                    .map(|m| layer_ref(&text(m)?))
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}

/// A name without a dot is a layer of the same part; `kind.layer` names
/// another kind.
fn layer_ref(source: &str) -> Result<LayerRef, String> {
    match source.matches('.').count() {
        0 => Ok(LayerRef {
            part_kind: None,
            layer: source.to_owned(),
        }),
        1 => {
            let (kind, layer) = source.split_once('.').expect("one dot");
            Ok(LayerRef {
                part_kind: Some(kind.to_owned()),
                layer: layer.to_owned(),
            })
        }
        _ => Err(format!(
            "may_use \"{source}\": use a layer of this part or \"kind.layer\""
        )),
    }
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
