use sha2::{Digest, Sha256};

/// Spec section 5.2, external write origin: everything needed to decide whether
/// a submission is new, a replay, or a conflict.
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub principal_kind: String,
    pub principal_id: String,
    pub command_id: String,
    pub command_kind: String,
    pub command_schema_ver: i64,
    pub request_fingerprint: String,
}

/// Canonicalise before hashing, so that key order — which carries no meaning —
/// cannot turn a replay into a conflict. The command kind is mixed in so the
/// same params under a different command are not interchangeable.
pub fn fingerprint(command_kind: &str, params: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(command_kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical(params).as_bytes());
    format!("{:x}", hasher.finalize())
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
