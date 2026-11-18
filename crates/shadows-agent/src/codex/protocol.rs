//! What Shadows uses of the Codex app-server protocol, and comparing a CLI's against it.
//!
//! The roots are the methods Shadows calls, the server requests it answers, the notifications it
//! reads, and the answers it parses. In the 0.169.0 bundle the answers are
//! `InitializeResponse`, `CommandExecutionRequestApprovalResponse`,
//! `FileChangeRequestApprovalResponse` and `PermissionsRequestApprovalResponse` under
//! `definitions`, and `ThreadStartResponse`, `ThreadResumeResponse`, `ThreadForkResponse`,
//! `TurnStartResponse`, `ModelListResponse`, `GetAccountResponse`,
//! `GetAccountRateLimitsResponse`, `ConfigReadResponse` and `ListMcpServerStatusResponse` under
//! `definitions.v2`. A snapshot key is the definition's path below `definitions`, such as
//! `v2/ThreadStartResponse`; a change names it without the `v2/`.
use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

pub(super) const TESTED: &str = include_str!("tested_protocol.json");

struct Used {
    client: &'static [&'static str],
    server: &'static [&'static str],
    notifications: &'static [&'static str],
    answers: &'static [&'static str],
}

const USED: Used = Used {
    client: &[
        "initialize",
        "thread/start",
        "thread/resume",
        "thread/fork",
        "turn/start",
        "turn/steer",
        "turn/interrupt",
        "model/list",
        "account/read",
        "account/rateLimits/read",
        "config/read",
        "mcpServerStatus/list",
    ],
    server: &[
        "item/commandExecution/requestApproval",
        "item/fileChange/requestApproval",
        "item/permissions/requestApproval",
    ],
    notifications: &[
        "turn/completed",
        "item/started",
        "item/completed",
        "item/agentMessage/delta",
        "thread/tokenUsage/updated",
        "account/rateLimits/updated",
    ],
    answers: &[
        "InitializeResponse",
        "ThreadStartResponse",
        "ThreadResumeResponse",
        "ThreadForkResponse",
        "TurnStartResponse",
        "ModelListResponse",
        "GetAccountResponse",
        "GetAccountRateLimitsResponse",
        "ConfigReadResponse",
        "ListMcpServerStatusResponse",
        "CommandExecutionRequestApprovalResponse",
        "FileChangeRequestApprovalResponse",
        "PermissionsRequestApprovalResponse",
    ],
};

const UNIONS: [&str; 3] = ["ClientRequest", "ServerRequest", "ServerNotification"];

fn used_methods(union: &str) -> &'static [&'static str] {
    match union {
        "ClientRequest" => USED.client,
        "ServerRequest" => USED.server,
        _ => USED.notifications,
    }
}

/// The tested snapshot's version: the floor a CLI must reach.
pub(super) fn tested_version() -> String {
    serde_json::from_str::<Value>(TESTED)
        .ok()
        .and_then(|tested| tested["version"].as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn lookup<'a>(definitions: &'a Value, key: &str) -> Option<&'a Value> {
    key.split('/')
        .try_fold(definitions, |value, part| value.get(part))
}

fn refs(value: &Value, into: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                match (key.as_str(), inner) {
                    ("$ref", Value::String(target)) => {
                        if let Some(key) = target.strip_prefix("#/definitions/") {
                            into.push(key.to_owned());
                        }
                    }
                    _ => refs(inner, into),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| refs(item, into)),
        _ => {}
    }
}

fn first_enum<'a>(variant: &'a Value, field: &str) -> Option<&'a str> {
    variant["properties"][field]["enum"][0].as_str()
}

/// The used part of a schema bundle, in the snapshot's form. A used root the bundle lacks is left
/// out, so `compare` names it.
pub(super) fn extract(bundle: &Value, version: &str) -> Result<Value, String> {
    let definitions = bundle
        .get("definitions")
        .filter(|value| value.is_object())
        .ok_or("the schema bundle has no definitions")?;
    let mut out = Map::new();
    let mut queue = Vec::new();
    for union in UNIONS {
        let Some(Value::Object(definition)) = definitions.get(union) else {
            continue;
        };
        let methods = used_methods(union);
        let mut definition = definition.clone();
        let variants: Vec<Value> = definition
            .get("oneOf")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|variant| first_enum(variant, "method").is_some_and(|m| methods.contains(&m)))
            .cloned()
            .collect();
        definition.insert("oneOf".into(), Value::Array(variants));
        let definition = Value::Object(definition);
        refs(&definition, &mut queue);
        out.insert(union.into(), definition);
    }
    for answer in USED.answers {
        for key in [answer.to_string(), format!("v2/{answer}")] {
            if lookup(definitions, &key).is_some() {
                queue.push(key);
            }
        }
    }
    while let Some(key) = queue.pop() {
        if out.contains_key(&key) {
            continue;
        }
        if let Some(definition) = lookup(definitions, &key) {
            refs(definition, &mut queue);
            out.insert(key, definition.clone());
        }
    }
    Ok(json!({ "version": version, "definitions": out }))
}

fn label(key: &str) -> &str {
    key.strip_prefix("v2/").unwrap_or(key)
}

/// Every change from `tested` to `found` that removes or changes what Shadows uses.
pub(super) fn compare(tested: &Value, found: &Value) -> Vec<String> {
    let mut changes = Vec::new();
    let empty = Map::new();
    let tested = tested["definitions"].as_object().unwrap_or(&empty);
    let found = &found["definitions"];
    for (key, definition) in tested {
        let Some(other) = found.get(key) else {
            if UNIONS.contains(&key.as_str()) {
                for variant in variants(definition, "oneOf") {
                    changes.push(format!("{} removed", variant.0));
                }
            } else {
                changes.push(format!("{} removed", label(key)));
            }
            continue;
        };
        if UNIONS.contains(&key.as_str()) {
            let found_variants = variants(other, "oneOf");
            for (method, variant) in variants(definition, "oneOf") {
                match found_variants.iter().find(|(other, _)| *other == method) {
                    Some((_, other)) => node(variant, other, &method, &mut changes),
                    None => changes.push(format!("{method} removed")),
                }
            }
        } else {
            node(definition, other, label(key), &mut changes);
        }
    }
    changes
}

/// A union's variants, each keyed by its `type` or `method` value, otherwise by its position.
fn variants<'a>(definition: &'a Value, field: &str) -> Vec<(String, &'a Value)> {
    definition[field]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, variant)| {
            let key = first_enum(variant, "type")
                .or_else(|| first_enum(variant, "method"))
                .map_or_else(|| format!("#{index}"), str::to_owned);
            (key, variant)
        })
        .collect()
}

fn required(schema: &Value) -> BTreeSet<&str> {
    schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn node(tested: &Value, found: &Value, path: &str, changes: &mut Vec<String>) {
    for field in ["$ref", "type", "const"] {
        if tested.get(field) != found.get(field) {
            changes.push(format!("{path} {field} changed"));
        }
    }
    if let Some(values) = tested["enum"].as_array() {
        let others = found["enum"].as_array();
        for value in values {
            if !others.is_some_and(|others| others.contains(value)) {
                changes.push(format!("{path} value {value} removed"));
            }
        }
    }
    let empty = Map::new();
    let properties = tested["properties"].as_object().unwrap_or(&empty);
    let found_properties = found["properties"].as_object().unwrap_or(&empty);
    let (was, is) = (required(tested), required(found));
    for (name, property) in properties {
        let at = format!("{path}.{name}");
        let Some(other) = found_properties.get(name) else {
            changes.push(format!("{at} removed"));
            continue;
        };
        match (was.contains(name.as_str()), is.contains(name.as_str())) {
            (false, true) => changes.push(format!("{at} became required")),
            (true, false) => changes.push(format!("{at} stopped being required")),
            _ => {}
        }
        node(property, other, &at, changes);
    }
    for name in found_properties.keys() {
        if !properties.contains_key(name) && is.contains(name.as_str()) {
            changes.push(format!("{path}.{name} is new and required"));
        }
    }
    for field in ["items", "additionalProperties"] {
        if let Some(inner) = tested.get(field).filter(|inner| inner.is_object()) {
            node(inner, &found[field], &format!("{path}.{field}"), changes);
        }
    }
    for field in ["oneOf", "anyOf", "allOf"] {
        let found_variants = variants(found, field);
        for (key, variant) in variants(tested, field) {
            match found_variants.iter().find(|(other, _)| *other == key) {
                Some((_, other)) => node(variant, other, &format!("{path}[{key}]"), changes),
                None => changes.push(format!("{path} variant {key} removed")),
            }
        }
    }
}

/// Whether `found` is at or above `floor`. A pre-release suffix sorts below its bare triple.
pub(super) fn version_at_least(found: &str, floor: &str) -> bool {
    fn parse(version: &str) -> Option<([u64; 3], bool)> {
        let (numbers, suffix) = match version.split_once('-') {
            Some((numbers, _)) => (numbers, false),
            None => (version, true),
        };
        let mut parts = numbers.split('.').map(|part| part.parse::<u64>().ok());
        let triple = [parts.next()??, parts.next()??, parts.next()??];
        parts.next().is_none().then_some((triple, suffix))
    }
    match (parse(found), parse(floor)) {
        (Some(found), Some(floor)) => found >= floor,
        _ => false,
    }
}

/// The refusal for a CLI whose protocol changed: five changes, then a count of the rest.
pub(super) fn refusal(version: &str, changes: &[String]) -> String {
    let mut message = format!(
        "Codex CLI {version} changed what Shadows uses: {}",
        changes
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ")
    );
    if changes.len() > 5 {
        message.push_str(&format!(" and {} more", changes.len() - 5));
    }
    message
}

/// A value with every object's keys sorted, so the committed snapshot diffs readably.
pub(super) fn sorted(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(entries.into_iter().map(|(k, v)| (k, sorted(v))).collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tested() -> Value {
        serde_json::from_str(TESTED).unwrap()
    }

    fn definition<'a>(snapshot: &'a mut Value, key: &str) -> &'a mut Value {
        snapshot["definitions"].get_mut(key).unwrap()
    }

    fn one_change_containing(changes: &[String], parts: &[&str]) {
        assert_eq!(changes.len(), 1, "{changes:?}");
        for part in parts {
            assert!(changes[0].contains(part), "{changes:?}");
        }
    }

    #[test]
    fn the_tested_protocol_matches_itself() {
        assert!(compare(&tested(), &tested()).is_empty());
    }

    #[test]
    fn an_added_optional_property_passes() {
        let mut found = tested();
        definition(&mut found, "v2/ThreadStartResponse")["properties"]["extra"] =
            json!({"type": "string"});
        assert!(compare(&tested(), &found).is_empty());
    }

    #[test]
    fn an_added_method_passes() {
        let mut found = tested();
        definition(&mut found, "ClientRequest")["oneOf"]
            .as_array_mut()
            .unwrap()
            .push(json!({"properties": {"method": {"enum": ["modelProvider/list"]}}}));
        assert!(compare(&tested(), &found).is_empty());
    }

    #[test]
    fn an_added_enum_value_passes() {
        let mut found = tested();
        definition(&mut found, "v2/TurnStatus")["enum"]
            .as_array_mut()
            .unwrap()
            .push(json!("paused"));
        assert!(compare(&tested(), &found).is_empty());
    }

    #[test]
    fn a_removed_method_refuses_and_names_it() {
        let mut found = tested();
        definition(&mut found, "ClientRequest")["oneOf"]
            .as_array_mut()
            .unwrap()
            .retain(|variant| first_enum(variant, "method") != Some("turn/steer"));
        assert_eq!(compare(&tested(), &found), ["turn/steer removed"]);
    }

    #[test]
    fn a_removed_property_refuses_and_names_it() {
        let mut found = tested();
        let response = definition(&mut found, "v2/ThreadStartResponse");
        response["properties"]
            .as_object_mut()
            .unwrap()
            .remove("approvalsReviewer");
        response["required"]
            .as_array_mut()
            .unwrap()
            .retain(|name| name != "approvalsReviewer");
        one_change_containing(
            &compare(&tested(), &found),
            &["ThreadStartResponse.approvalsReviewer"],
        );
    }

    #[test]
    fn a_changed_type_refuses() {
        let mut found = tested();
        definition(&mut found, "v2/ModelListResponse")["properties"]["data"] =
            json!({"type": "object"});
        let changes = compare(&tested(), &found);
        assert!(!changes.is_empty());
        assert!(
            changes.iter().all(|c| c.contains("ModelListResponse.data")),
            "{changes:?}"
        );
    }

    #[test]
    fn a_newly_required_parameter_refuses() {
        let mut found = tested();
        let params = definition(&mut found, "v2/TurnSteerParams");
        params["properties"]["mode"] = json!({"type": "string"});
        params["required"]
            .as_array_mut()
            .unwrap()
            .push(json!("mode"));
        one_change_containing(&compare(&tested(), &found), &["TurnSteerParams.mode"]);
    }

    #[test]
    fn a_removed_union_variant_refuses() {
        let mut found = tested();
        definition(&mut found, "v2/ThreadItem")["oneOf"]
            .as_array_mut()
            .unwrap()
            .retain(|variant| first_enum(variant, "type") != Some("agentMessage"));
        one_change_containing(&compare(&tested(), &found), &["ThreadItem", "agentMessage"]);
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(version_at_least("0.170.0", "0.169.0"));
        assert!(version_at_least("0.169.0", "0.169.0"));
        assert!(!version_at_least("0.168.9", "0.169.0"));
        assert!(!version_at_least("0.169.0-alpha.1", "0.169.0"));
    }

    #[test]
    fn the_snapshot_holds_every_used_method() {
        let tested = tested();
        for union in UNIONS {
            let methods: Vec<_> = variants(&tested["definitions"][union], "oneOf")
                .into_iter()
                .map(|(method, _)| method)
                .collect();
            for method in used_methods(union) {
                assert!(methods.iter().any(|m| m == method), "{method}");
            }
        }
        for answer in USED.answers {
            let definitions = &tested["definitions"];
            assert!(
                definitions.get(answer).is_some()
                    || definitions.get(format!("v2/{answer}")).is_some(),
                "{answer}"
            );
        }
    }

    #[test]
    fn the_refusal_names_five_changes_then_counts() {
        let changes: Vec<_> = (1..=7).map(|n| format!("change {n}")).collect();
        let message = refusal("0.171.0", &changes);
        assert!(message.contains("change 5"));
        assert!(!message.contains("change 6"));
        assert!(message.ends_with("and 2 more"), "{message}");
    }
}
