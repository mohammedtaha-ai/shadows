
use super::*;

#[test]
fn lower_case_key_env_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let section = serde_json::from_value(serde_json::json!({
        "home_root": tmp.path(), "key_env": "bad_key", "base_url": "u", "model": "m",
    }))
    .unwrap();
    assert!(prepare(Some(section)).is_err());
}

#[test]
fn parse_dotenv_handles_quotes_export_comments_and_crlf() {
    let pairs = parse_dotenv("# c\r\n\r\nA=1\r\nexport B=\"two words\"\nC='3'\n");
    assert!(
        pairs
            == vec![
                ("A".into(), "1".into()),
                ("B".into(), "two words".into()),
                ("C".into(), "3".into())
            ],
        "parsed pairs"
    );
}

#[test]
fn parse_dotenv_preserves_equals_and_duplicates() {
    let pairs = parse_dotenv("A=x=y\ninvalid\n=value\nA=last\nEMPTY=\n");
    assert!(
        pairs
            == vec![
                ("A".into(), "x=y".into()),
                ("A".into(), "last".into()),
                ("EMPTY".into(), "".into())
            ],
        "parser retains order"
    );
}

#[tokio::test]
async fn import_reads_no_file_when_marker_exists() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    #[derive(Clone)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    let section = serde_json::from_value(serde_json::json!({
        "home_root": tmp.path(), "key_env": "API_KEY", "base_url": "u", "model": "m",
    }))
    .unwrap();
    let config = prepare(Some(section)).unwrap().unwrap().0;
    let core = AppCore::start(
        &shadows_core::StartConfig {
            db_path: tmp.path().join("test.db"),
            node_path: "absent".into(),
            adapter_path: "absent".into(),
            harness_path: "absent".into(),
            aylul: None,
            secret_key: Some(shadows_core::SecretKey::from_bytes([7; 32])),
        },
        "http://127.0.0.1:1/mcp".into(),
    )
    .await
    .unwrap();
    core.secrets()
        .import_env_once("aylul", vec![])
        .await
        .unwrap();
    let capture = Capture(Arc::new(Mutex::new(Vec::new())));
    let copy = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || copy.clone())
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    import_env(&core, &config, &tmp.path().join("absent-import.txt")).await;
    assert!(
        capture.0.lock().unwrap().is_empty(),
        "marker prevents missing-file error"
    );
    drop(guard);
    core.shut_down(std::time::Duration::from_secs(1), std::future::pending())
        .await
        .unwrap();
}

#[test]
fn seeding_creates_both_files_in_an_empty_home() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("h");
    seed_profile(&home).unwrap();
    assert!(
        std::fs::read_to_string(home.join("CLAUDE.md"))
            .unwrap()
            .contains("AYLUL")
    );
    assert!(
        std::fs::read_to_string(home.join("settings.json"))
            .unwrap()
            .contains("permissions")
    );
}

#[test]
fn seeding_keeps_existing_profile_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("CLAUDE.md"), "mine").unwrap();
    seed_profile(tmp.path()).unwrap();
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap(),
        "mine"
    );
    assert!(tmp.path().join("settings.json").exists());
}

#[test]
fn a_section_file_with_an_unknown_field_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("a.json");
    std::fs::write(
        &file,
        r#"{"home_root":"/x","key_env":"K","base_url":"u","model":"m","key":"s"}"#,
    )
    .unwrap();
    assert!(load(&file).is_err());
}

#[test]
fn a_section_with_a_relative_env_file_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let section: AylulSection = serde_json::from_value(serde_json::json!({
        "home_root": tmp.path(), "key_env": "K", "base_url": "u", "model": "m",
        "env_file": "relative/.env",
    }))
    .unwrap();
    assert!(prepare(Some(section)).is_err());
}

#[test]
fn no_aylul_config_means_no_aylul_project() {
    assert!(prepare(None).unwrap().is_none());
}

#[test]
fn a_section_builds_the_profile_with_context_window_and_no_secret() {
    let tmp = tempfile::tempdir().unwrap();
    let section: AylulSection = serde_json::from_value(serde_json::json!({
        "home_root": tmp.path(), "key_env": "MINIMAX_API_KEY",
        "base_url": "https://api.minimax.io/anthropic", "model": "MiniMax-M3",
    }))
    .unwrap();
    let config = prepare(Some(section)).unwrap().unwrap().0;
    let env = &config.profile.env;
    assert!(env.contains(&("ANTHROPIC_MODEL".into(), "MiniMax-M3".into())));
    assert!(env.contains(&("CLAUDE_CODE_MAX_CONTEXT_TOKENS".into(), "1000000".into())));
    assert_eq!(config.profile.key_env, "MINIMAX_API_KEY");
    assert_eq!(
        config.profile.home,
        tmp.path().join("profiles").join("aylul")
    );
}
