//! Offline validation of canonical shared OpenAPI content.
use super::{AgreementContent, AgreementIssue, AgreementRole, Design};
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::OnceLock;

fn schema() -> &'static jsonschema::Validator {
    static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let documents = [
            include_str!("openapi-3.1-schema.json"),
            include_str!("openapi-3.1-dialect.json"),
            include_str!("openapi-3.1-meta.json"),
        ];
        let mut registry = jsonschema::Registry::new();
        for document in documents {
            let value: Value =
                serde_json::from_str(document).expect("bundled OAS resource is JSON");
            let id = value["$id"]
                .as_str()
                .expect("OAS resource has an ID")
                .to_string();
            registry = registry
                .add(id, value)
                .expect("bundled OAS resource registers");
        }
        let registry = registry
            .prepare()
            .expect("bundled OAS resources resolve offline");
        let schema: Value = serde_json::from_str(include_str!("openapi-3.1-schema-base.json"))
            .expect("bundled OAS base schema is JSON");
        jsonschema::options()
            .offline()
            .with_registry(&registry)
            .build(&schema)
            .expect("bundled official OAS schema compiles offline")
    })
}

impl Design {
    pub fn validate_agreement(content: &AgreementContent) -> Vec<AgreementIssue> {
        let mut issues = Vec::new();
        for (path, text) in [
            ("/capability", &content.capability),
            ("/purpose", &content.purpose),
        ] {
            if text.trim().is_empty() {
                issue(&mut issues, path, "must not be blank");
            }
        }
        if content.acceptance.is_empty() || content.acceptance.iter().any(|s| s.trim().is_empty()) {
            issue(
                &mut issues,
                "/acceptance",
                "provide nonblank acceptance items",
            );
        }
        if !content
            .parties
            .iter()
            .any(|p| p.role == AgreementRole::Provides)
        {
            issue(
                &mut issues,
                "/parties",
                "declare a provider before agreement",
            );
        }
        let mut parties = BTreeSet::new();
        for party in &content.parties {
            if !parties.insert((party.part_id.to_string(), format!("{:?}", party.role))) {
                issue(&mut issues, "/parties", "duplicate participant declaration");
            }
        }
        for error in schema().iter_errors(&content.openapi) {
            issue(
                &mut issues,
                &format!("/openapi{}", error.instance_path()),
                &error.to_string(),
            );
        }
        check_refs(&content.openapi, &content.openapi, "/openapi", &mut issues);
        let mut ids = BTreeSet::new();
        if let Some(paths) = content.openapi.get("paths").and_then(Value::as_object) {
            for (path, item) in paths {
                for method in [
                    "get", "put", "post", "delete", "options", "head", "patch", "trace",
                ] {
                    let Some(op) = item.get(method) else { continue };
                    let location = format!(
                        "/openapi/paths/{}/{method}/x-shadows-operation-id",
                        escape(path)
                    );
                    match op.get("x-shadows-operation-id").and_then(Value::as_str) {
                        Some(id) if uuid::Uuid::parse_str(id).is_ok() => {
                            if !ids.insert(id) {
                                issue(&mut issues, &location, "duplicate operation identity");
                            }
                        }
                        _ => issue(
                            &mut issues,
                            &location,
                            "a stable UUID operation identity is required",
                        ),
                    }
                }
            }
        }
        if ids.is_empty() {
            issue(&mut issues, "/openapi/paths", "provide an HTTP operation");
        }
        issues
    }
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
pub(crate) fn operation_ids(content: &AgreementContent) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if let Some(paths) = content.openapi.get("paths").and_then(Value::as_object) {
        for item in paths.values() {
            for method in [
                "get", "put", "post", "delete", "options", "head", "patch", "trace",
            ] {
                if let Some(id) = item
                    .get(method)
                    .and_then(|op| op.get("x-shadows-operation-id"))
                    .and_then(Value::as_str)
                {
                    ids.insert(id.to_string());
                }
            }
        }
    }
    ids
}
fn issue(issues: &mut Vec<AgreementIssue>, path: &str, message: &str) {
    issues.push(AgreementIssue {
        path: path.into(),
        message: message.into(),
    });
}
fn check_refs(root: &Value, value: &Value, path: &str, issues: &mut Vec<AgreementIssue>) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref") {
                match reference.as_str() {
                    Some("#") => {}
                    Some(r) if r.starts_with("#/") && root.pointer(&r[1..]).is_some() => {}
                    _ => issue(
                        issues,
                        &format!("{path}/$ref"),
                        "unresolved reference; external retrieval is disabled",
                    ),
                }
            }
            for (key, child) in map {
                check_refs(root, child, &format!("{path}/{}", escape(key)), issues);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                check_refs(root, child, &format!("{path}/{index}"), issues);
            }
        }
        _ => {}
    }
}
