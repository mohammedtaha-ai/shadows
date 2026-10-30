use super::{AylulSection, prepare};
#[test]
fn old_section_keys_are_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    for name in ["key_env", "base_url", "model", "env", "env_file"] {
        let mut value = serde_json::json!({"home_root":tmp.path()});
        value[name] = serde_json::json!("synthetic");
        let error = serde_json::from_value::<AylulSection>(value)
            .err()
            .expect("old field refused");
        assert!(
            error
                .to_string()
                .contains(&format!("unknown field `{name}`"))
        );
    }
}
#[test]
fn section_needs_only_home_root() {
    let tmp = tempfile::tempdir().unwrap();
    let section = serde_json::from_value(serde_json::json!({"home_root":tmp.path()})).unwrap();
    let config = prepare(Some(section)).unwrap().unwrap();
    assert_eq!(config.home_root, tmp.path());
    assert_eq!(config.project_slug, "aylul");
}
#[test]
fn relative_home_root_is_refused() {
    let section = serde_json::from_value(serde_json::json!({"home_root":"relative"})).unwrap();
    assert!(prepare(Some(section)).is_err());
    assert!(prepare(None).unwrap().is_none());
}
