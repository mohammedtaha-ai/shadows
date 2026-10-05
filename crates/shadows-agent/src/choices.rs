//! One job: reading the harness's offered choices (spec §12.4).
//!
//! The model, effort and mode lists are the harness's own, read from the ACP
//! session's `configOptions` by category, in the harness's order. Shadows
//! keeps no list of its own; `policy.rs` only filters the modes.

use serde_json::Value;

use super::{TurnSettings, policy};

/// One value the session offers for a setting (spec §12.4). `enabled: false`
/// carries the `reason` it cannot be chosen.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct Choice {
    pub id: String,
    pub label: String,
    #[schema(required)]
    pub description: Option<String>,
    pub enabled: bool,
    #[schema(required)]
    pub reason: Option<String>,
}

/// What the thread's session offers now (spec §12.4). `efforts` are the
/// current model's. `modes` are after Shadows' policy; a mode the project does
/// not allow is present with `enabled: false` and `reason: "Not allowed in
/// this project"`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
pub struct SessionChoices {
    pub models: Vec<Choice>,
    pub efforts: Vec<Choice>,
    pub modes: Vec<Choice>,
    pub current: TurnSettings,
}

/// The ACP `configId` of each setting, found by its category: the harness
/// names them as it likes (Claude's effort option is `effort`).
#[derive(Debug, Clone, PartialEq)]
pub struct OptionIds {
    pub model: String,
    pub effort: Option<String>,
    pub mode: String,
}

/// Everything the session offers, before Shadows' policy.
#[derive(Debug, Clone, PartialEq)]
pub struct Offered {
    pub models: Vec<Choice>,
    pub efforts: Vec<Choice>,
    pub modes: Vec<Choice>,
    pub current: TurnSettings,
    pub ids: OptionIds,
}

impl Offered {
    fn offers(list: &[Choice], id: &str) -> bool {
        list.iter().any(|c| c.id == id)
    }
    pub fn offers_model(&self, id: &str) -> bool {
        Self::offers(&self.models, id)
    }
    pub fn offers_effort(&self, id: &str) -> bool {
        Self::offers(&self.efforts, id)
    }
    pub fn offers_mode(&self, id: &str) -> bool {
        Self::offers(&self.modes, id)
    }
}

pub const NOT_ALLOWED: &str = "Not allowed in this project";

/// Reads `configOptions` (the serialized ACP `SessionConfigOption` list):
/// `category` `model`, `thought_level` and `mode`; options flat or grouped.
/// Anything else is ignored. A model with no `thought_level` option offers no
/// effort. `Err` when the model or mode option is missing.
pub fn parse(options: &Value) -> Result<Offered, String> {
    let list = options
        .as_array()
        .ok_or("the harness's config options are not a list")?;
    let mut model = None;
    let mut effort = None;
    let mut mode = None;
    for option in list {
        let slot = match option.get("category").and_then(Value::as_str) {
            Some("model") => &mut model,
            Some("thought_level") => &mut effort,
            Some("mode") => &mut mode,
            _ => continue,
        };
        *slot = Some(select(option)?);
    }
    let (model_id, models, current_model) = model.ok_or("the harness offers no model option")?;
    let (mode_id, modes, current_mode) = mode.ok_or("the harness offers no mode option")?;
    let (effort_id, efforts, current_effort) = match effort {
        Some((id, list, current)) => (Some(id), list, Some(current)),
        None => (None, Vec::new(), None),
    };
    Ok(Offered {
        models,
        efforts,
        modes,
        current: TurnSettings {
            model: current_model,
            mode: current_mode,
            effort: current_effort,
        },
        ids: OptionIds {
            model: model_id,
            effort: effort_id,
            mode: mode_id,
        },
    })
}

/// One select option: its id, its values in order, its current value.
fn select(option: &Value) -> Result<(String, Vec<Choice>, String), String> {
    let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_owned);
    let id = text(option, "id").ok_or("a config option has no id")?;
    let current =
        text(option, "currentValue").ok_or(format!("option {id} has no current value"))?;
    let mut choices = Vec::new();
    for item in option
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        // A group holds its own list; a flat option is itself a value.
        let values = match item.get("options").and_then(Value::as_array) {
            Some(group) => group.iter().collect::<Vec<_>>(),
            None => vec![item],
        };
        for v in values {
            let Some(value) = text(v, "value") else {
                continue;
            };
            choices.push(Choice {
                label: text(v, "name").unwrap_or_else(|| value.clone()),
                id: value,
                description: text(v, "description"),
                enabled: true,
                reason: None,
            });
        }
    }
    Ok((id, choices, current))
}

/// What a client is offered: modes outside Shadows' policy are dropped; the
/// rest are disabled, with the reason, when the project does not allow them.
pub fn for_client(offered: &Offered, harness: &str, allowed: &[String]) -> SessionChoices {
    let policy = policy::allowed_modes(harness);
    let modes = offered
        .modes
        .iter()
        .filter(|m| policy.contains(&m.id.as_str()))
        .map(|m| {
            let permitted = allowed.contains(&m.id);
            Choice {
                enabled: permitted,
                reason: (!permitted).then(|| NOT_ALLOWED.to_string()),
                ..m.clone()
            }
        })
        .collect();
    SessionChoices {
        models: offered.models.clone(),
        efforts: offered.efforts.clone(),
        modes,
        current: offered.current.clone(),
    }
}

/// The first of model, effort and mode that the session does not offer, or
/// (for the mode) that Shadows' policy refuses, as `(setting, value)`.
/// `offered` must be the options for `s.model`: a caller changing the model
/// sets it first and passes the answer. A model with efforts needs one of
/// them; a model with none needs none.
pub fn refusal(
    offered: &Offered,
    harness: &str,
    s: &TurnSettings,
) -> Option<(&'static str, String)> {
    if !offered.offers_model(&s.model) {
        return Some(("model", s.model.clone()));
    }
    match (&s.effort, offered.efforts.is_empty()) {
        (Some(e), false) if offered.offers_effort(e) => {}
        (None, true) => {}
        (effort, _) => return Some(("effort", effort.clone().unwrap_or_else(|| "none".into()))),
    }
    let in_policy = policy::allowed_modes(harness).contains(&s.mode.as_str());
    if !in_policy || !offered.offers_mode(&s.mode) {
        return Some(("mode", s.mode.clone()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `fake_acp`'s options for a session on `model`, as it answers them.
    fn fake_options(model: &str) -> Value {
        let efforts: &[&str] = match model {
            "fake-large" => &["low", "high", "max"],
            "fake-small" => &["low", "high"],
            _ => &[],
        };
        let mut v = vec![
            json!({
                "id": "mode",
                "name": "Mode",
                "category": "mode",
                "type": "select",
                "currentValue": "auto",
                "options": (
                    ["default", "acceptEdits", "plan", "auto", "bypassPermissions"]
                        .iter()
                        .map(|v| json!({"value": v, "name": v}))
                        .collect::<Vec<_>>()
                ),
            }),
            json!({
                "id": "model",
                "name": "Model",
                "category": "model",
                "type": "select",
                "currentValue": model,
                "options": [
                    {"value":"fake-large","name":"Fake Large","description":"The biggest fake"},
                    {"value":"fake-small","name":"Fake Small"},
                    {"value":"fake-tiny","name":"Fake Tiny"},
                    {"value":"fake-locked","name":"Fake Locked"},
                ],
            }),
        ];
        if !efforts.is_empty() {
            v.push(
                json!({"id":"effort","name":"Effort","category":"thought_level","type":"select",
                "currentValue":"high",
                "options":efforts.iter().map(|v| json!({"value":v,"name":v})).collect::<Vec<_>>()}),
            );
        }
        v.push(json!({
            "id": "fast",
            "name": "Fast",
            "category": "model_config",
            "type": "select",
            "currentValue": "off",
            "options": [
                {"value":"on","name":"On"},
                {"value":"off","name":"Off"},
            ],
        }));
        Value::Array(v)
    }

    fn ids(list: &[Choice]) -> Vec<&str> {
        list.iter().map(|c| c.id.as_str()).collect()
    }

    #[test]
    fn parse_reads_models_efforts_and_modes_by_category_in_the_harness_order() {
        let o = parse(&fake_options("fake-large")).unwrap();
        assert_eq!(
            ids(&o.models),
            ["fake-large", "fake-small", "fake-tiny", "fake-locked"]
        );
        assert_eq!(ids(&o.efforts), ["low", "high", "max"]);
        assert_eq!(
            ids(&o.modes),
            [
                "default",
                "acceptEdits",
                "plan",
                "auto",
                "bypassPermissions"
            ]
        );
        assert_eq!(o.current.model, "fake-large");
        assert_eq!(
            o.ids.effort.as_deref(),
            Some("effort"),
            "found by category, not by id"
        );
        assert_eq!(o.models[0].label, "Fake Large");
        assert_eq!(o.models[0].description.as_deref(), Some("The biggest fake"));
    }

    #[test]
    fn grouped_options_are_read_flat_in_order() {
        let v = json!([
            {"id":"m","name":"Model","category":"model","type":"select","currentValue":"b",
             "options":[{"group":"g1","name":"G1","options":[{"value":"a","name":"A"}]},
                        {"group":"g2","name":"G2","options":[{"value":"b","name":"B"}]}]},
            {"id":"x","name":"Mode","category":"mode","type":"select","currentValue":"auto",
             "options":[{"value":"auto","name":"Auto"}]}
        ]);
        let o = parse(&v).unwrap();
        assert_eq!(ids(&o.models), ["a", "b"]);
        assert_eq!((o.ids.model.as_str(), o.ids.mode.as_str()), ("m", "x"));
    }

    #[test]
    fn a_model_without_effort_parses_to_no_efforts_and_no_effort_id() {
        let o = parse(&fake_options("fake-tiny")).unwrap();
        assert!(o.efforts.is_empty());
        assert_eq!(
            (o.ids.effort.as_deref(), o.current.effort.as_deref()),
            (None, None)
        );
    }

    #[test]
    fn only_policy_modes_reach_the_client_and_disallowed_ones_say_why() {
        let o = parse(&fake_options("fake-large")).unwrap();
        let c = for_client(&o, "claude-code", &["acceptEdits".to_string()]);
        assert_eq!(ids(&c.modes), ["acceptEdits", "auto"]);
        let auto = c.modes.iter().find(|m| m.id == "auto").unwrap();
        assert_eq!(
            (auto.enabled, auto.reason.as_deref()),
            (false, Some(NOT_ALLOWED))
        );
        let edits = c.modes.iter().find(|m| m.id == "acceptEdits").unwrap();
        assert_eq!((edits.enabled, edits.reason.as_deref()), (true, None));
    }

    #[test]
    fn refusal_names_what_is_not_offered() {
        let o = parse(&fake_options("fake-small")).unwrap();
        let ok = TurnSettings {
            model: "fake-small".into(),
            mode: "acceptEdits".into(),
            effort: Some("high".into()),
        };
        assert_eq!(refusal(&o, "claude-code", &ok), None);
        let max = TurnSettings {
            effort: Some("max".into()),
            ..ok.clone()
        };
        assert_eq!(refusal(&o, "claude-code", &max).unwrap().0, "effort");
        let none = TurnSettings {
            effort: None,
            ..ok.clone()
        };
        assert_eq!(
            refusal(&o, "claude-code", &none).unwrap().0,
            "effort",
            "an offered effort must be chosen"
        );
        let tiny = parse(&fake_options("fake-tiny")).unwrap();
        let none = TurnSettings {
            model: "fake-tiny".into(),
            effort: None,
            ..ok.clone()
        };
        assert_eq!(refusal(&tiny, "claude-code", &none), None);
        let high = TurnSettings {
            effort: Some("high".into()),
            ..none
        };
        assert_eq!(refusal(&tiny, "claude-code", &high).unwrap().0, "effort");
        let plan = TurnSettings {
            mode: "plan".into(),
            ..ok.clone()
        };
        assert_eq!(refusal(&o, "claude-code", &plan).unwrap().0, "mode");
        let gpt = TurnSettings {
            model: "gpt".into(),
            ..ok
        };
        assert_eq!(refusal(&o, "claude-code", &gpt).unwrap().0, "model");
    }
}
