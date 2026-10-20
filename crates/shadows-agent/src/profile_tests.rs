use super::*;
fn profile() -> ClaudeProfile {
    ClaudeProfile {
        name: "work".into(),
        home: "C:/isolated/work".into(),
        env: vec![(
            "ANTHROPIC_BASE_URL".into(),
            "https://provider.example".into(),
        )],
    }
}
fn last(env: &[(String, String)], key: &str) -> Option<String> {
    env.iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
}
#[test]
fn profile_requires_an_explicit_token() {
    assert!(
        profile()
            .spawn_env_with(&[("PATH".into(), "synthetic".into())], None)
            .is_err()
    );
    assert!(profile().spawn_env_with(&[], Some("")).is_err());
}
#[test]
fn explicit_token_and_home_override_stored_pairs() {
    let stored = vec![
        ("ANTHROPIC_AUTH_TOKEN".into(), "wrong".into()),
        ("CLAUDE_CONFIG_DIR".into(), "wrong".into()),
        ("ANTHROPIC_BASE_URL".into(), "https://wrong.example".into()),
    ];
    let p = profile();
    let env = p
        .spawn_env_with(&stored, Some("synthetic-explicit"))
        .unwrap();
    assert!(last(&env, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("synthetic-explicit"));
    assert_eq!(
        last(&env, "CLAUDE_CONFIG_DIR"),
        Some(p.home.to_string_lossy().into())
    );
    assert_eq!(
        last(&env, "ANTHROPIC_BASE_URL").as_deref(),
        Some("https://provider.example")
    );
    assert_eq!(last(&env, "ANTHROPIC_API_KEY").as_deref(), Some(""));
}
#[test]
fn an_edit_applies_on_the_next_spawn() {
    let p = profile();
    let one = p.spawn_env_with(&[], Some("synthetic-one")).unwrap();
    let two = p.spawn_env_with(&[], Some("synthetic-two")).unwrap();
    assert!(last(&one, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("synthetic-one"));
    assert!(last(&two, "ANTHROPIC_AUTH_TOKEN").as_deref() == Some("synthetic-two"));
}
#[test]
fn profile_error_never_prints_a_value() {
    let error = profile()
        .spawn_env_with(&[("API_KEY".into(), "synthetic-private".into())], None)
        .unwrap_err();
    assert!(!format!("{error:?} {error}").contains("synthetic-private"));
}
