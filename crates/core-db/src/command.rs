//! Durable command identity and replay classification.

use sha2::{Digest, Sha256};

use super::{StorageError, Tx};

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub agent: Option<String>,
    pub principal_kind: String,
    pub principal_id: String,
    pub command_id: String,
    pub command_kind: String,
    pub command_schema_ver: i64,
    pub request_fingerprint: String,
}

impl CommandContext {
    pub fn user(command_id: String, kind: &str, params: serde_json::Value) -> Self {
        Self {
            agent: None,
            principal_kind: "User".into(),
            principal_id: "local".into(),
            command_id,
            command_kind: kind.into(),
            command_schema_ver: 1,
            request_fingerprint: Self::fingerprint(kind, &params),
        }
    }
}

impl CommandContext {
    pub fn fingerprint(command_kind: &str, params: &serde_json::Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(command_kind.as_bytes());
        hasher.update(b"\0");
        hasher.update(canonical(params).as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

fn canonical(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .iter()
                .map(|k| {
                    let key_json = serde_json::to_string(*k).expect("string keys always serialise");
                    format!("{}:{}", key_json, canonical(&map[*k]))
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}

impl CommandContext {
    pub async fn classify(
        conn: &mut Tx,
        ctx: &CommandContext,
        scope_kind: &str,
        scope_key: &str,
    ) -> Result<Option<String>, StorageError> {
        let existing: Option<(String, i64, String, Option<String>)> = sqlx::query_as(
            "SELECT command_kind, command_schema_ver, request_fingerprint, outcome_ref
           FROM command_record
          WHERE principal_kind = ? AND principal_id = ?
            AND command_scope_kind = ? AND command_scope_key = ?
            AND command_id = ?",
        )
        .bind(&ctx.principal_kind)
        .bind(&ctx.principal_id)
        .bind(scope_kind)
        .bind(scope_key)
        .bind(&ctx.command_id)
        .fetch_optional(&mut *conn)
        .await?;

        match existing {
            None => Ok(None),
            Some((kind, schema_ver, fp, outcome_ref)) => {
                if kind == ctx.command_kind
                    && schema_ver == ctx.command_schema_ver
                    && fp == ctx.request_fingerprint
                {
                    Ok(Some(
                        outcome_ref.ok_or(StorageError::NotFound("outcome_ref"))?,
                    ))
                } else {
                    Err(StorageError::CommandConflict)
                }
            }
        }
    }

    pub async fn record_command(
        conn: &mut Tx,
        ctx: &CommandContext,
        scope_kind: &str,
        scope_key: &str,
        entity_kind: &str,
        outcome_ref: &str,
        ts: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO command_record
           (principal_kind, principal_id, command_scope_kind, command_scope_key,
            command_id, command_kind, command_schema_ver, request_fingerprint,
            outcome_kind, entity_kind, outcome_ref, recorded_at, audit_principal, audit_agent)
         VALUES (?,?,?,?,?,?,?,?,'Entity',?,?,?,?,?)",
        )
        .bind(&ctx.principal_kind)
        .bind(&ctx.principal_id)
        .bind(scope_kind)
        .bind(scope_key)
        .bind(&ctx.command_id)
        .bind(&ctx.command_kind)
        .bind(ctx.command_schema_ver)
        .bind(&ctx.request_fingerprint)
        .bind(entity_kind)
        .bind(outcome_ref)
        .bind(ts)
        .bind(ctx.agent.as_ref().map(|_| "user:local"))
        .bind(&ctx.agent)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::CommandContext;
    use serde_json::json;

    #[test]
    fn fingerprints_match_persisted_values() {
        let cases = [
            (
                "project.create",
                json!({ "name": "Demo", "slug": "demo" }),
                "c0d9607592de82e95eb135537b3a6e3917fbbc9adea901a34e58917003996a71",
            ),
            (
                "PlanEdit",
                json!({ "a": 1 }),
                "c2daffdac71b1da272da47656f5a77a5fab0bc3b1890ac2d292b99e1cf4b7ad8",
            ),
            (
                "k",
                json!({
                    "z": [null, true, false, -7, 1.25, "é\n\""],
                    "a": { "b": 2, "a": 1 },
                    "a:1,bc": 2,
                }),
                "2f1656af179d76595cb6880593915bc40e35759d3c5bc166ed985f82c2447d00",
            ),
            (
                "",
                json!(null),
                "b018d21dce8b6a938aecc2bb5ffdef6c1cd0d821f0f282c0eb96a08a36ef7bfe",
            ),
        ];
        for (kind, params, expected) in cases {
            assert_eq!(CommandContext::fingerprint(kind, &params), expected);
        }
    }
}
