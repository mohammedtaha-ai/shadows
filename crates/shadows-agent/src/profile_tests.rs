use super::*;

fn profile(key: &str) -> ClaudeProfile {
    ClaudeProfile {
        name: "a".into(),
        home: "C:/a/home".into(),
        key_env: key.into(),
        env: vec![("ANTHROPIC_MODEL".into(), "from-config".into())],
    }
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

fn last(env: &[(String, String)], name: &str) -> Option<String> {
    env.iter()
        .rev()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

#[test]
fn spawn_env_sets_home_key_and_blank_api_key() {
    let key = std::env::var("PATH").unwrap();
    let env = profile("PATH").spawn_env_with(&[]).unwrap();
    assert!(
        last(&env, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some(key.as_str()),
        "daemon fallback reaches token"
    );
    assert!(
        last(&env, "CLAUDE_CONFIG_DIR").as_deref() == Some("C:/a/home"),
        "profile home"
    );
    assert!(
        last(&env, "ANTHROPIC_API_KEY").as_deref() == Some(""),
        "API key stays blank"
    );
}

#[test]
fn stored_pairs_reach_the_session() {
    let stored = pairs(&[("A", "1"), ("B", "two words"), ("MK", "synthetic")]);
    let env = profile("MK").spawn_env_with(&stored).unwrap();
    assert!(last(&env, "A").as_deref() == Some("1"), "first pair");
    assert!(
        last(&env, "B").as_deref() == Some("two words"),
        "second pair"
    );
}

#[test]
fn stored_key_becomes_the_token() {
    let env = profile("PATH")
        .spawn_env_with(&pairs(&[("PATH", "synthetic")]))
        .unwrap();
    assert!(
        last(&env, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("synthetic"),
        "stored key wins over daemon"
    );
}

#[test]
fn stored_pairs_cannot_override_home_or_token() {
    let stored = pairs(&[
        ("CLAUDE_CONFIG_DIR", "elsewhere"),
        ("ANTHROPIC_AUTH_TOKEN", "wrong"),
        ("ANTHROPIC_API_KEY", "wrong"),
        ("ANTHROPIC_MODEL", "stored-model"),
        ("MK", "synthetic"),
    ]);
    let p = profile("MK");
    let env = p.spawn_env_with(&stored).unwrap();
    assert!(
        last(&env, "CLAUDE_CONFIG_DIR").as_deref() == Some("C:/a/home"),
        "home wins"
    );
    assert!(
        last(&env, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("synthetic"),
        "token wins"
    );
    assert!(
        last(&env, "ANTHROPIC_API_KEY").as_deref() == Some(""),
        "API key blank"
    );
    assert!(
        last(&env, "ANTHROPIC_MODEL").as_deref() == Some("stored-model"),
        "stored overrides"
    );
    assert!(
        env[..p.env.len()] == p.env,
        "profile values precede stored pairs"
    );
}

#[test]
fn an_edit_applies_on_the_next_spawn() {
    let p = profile("MK");
    let first = p.spawn_env_with(&pairs(&[("MK", "one")])).unwrap();
    let second = p.spawn_env_with(&pairs(&[("MK", "two")])).unwrap();
    assert!(
        last(&first, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("one"),
        "first spawn"
    );
    assert!(
        last(&second, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("two"),
        "next spawn"
    );
}

#[test]
fn profile_error_never_prints_a_value() {
    let result = profile("T_NEVER_SET_KEY_10").spawn_env_with(&pairs(&[("OTHER", "synthetic")]));
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("missing key must be refused"),
    };
    for shown in [error.to_string(), format!("{error:?}")] {
        assert!(!shown.contains("synthetic"), "error carries no value");
    }
    assert!(matches!(error, ProfileError::MissingKey(_)));
}

#[test]
fn a_profile_spawn_spec_never_prints_its_key() {
    let adapter = crate::claude::ClaudeAdapter {
        node: "node".into(),
        adapter: "a.js".into(),
        agent: "claude".into(),
        adapter_version: String::new(),
        agent_version: String::new(),
    };
    let p = profile("MK");
    let spec = adapter
        .process_spec_with(
            "h".as_ref(),
            Some(&p),
            &pairs(&[("MK", "synthetic-stored")]),
        )
        .unwrap();
    assert!(
        !format!("{spec:?}").contains("synthetic-stored"),
        "spawn debug redacts values"
    );
}
