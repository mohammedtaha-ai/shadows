//! Explicit harness configuration and the pinned adapter's version: the
//! configuration a harness is started from, not anything it answers.

use shadows::config::{ConfigError, codex_executable, codex_home, harness_path};
use shadows_agent::codex::CodexProgram;
use shadows_agent::connection::{McpServerSpec, SessionSetup};
use shadows_core::testing::adapter_version;

/// Codex is launched from an explicit absolute executable and its own
/// signed-in home, never the person's regular profile.
#[test]
fn codex_paths_are_absolute_and_exist() {
    let dir = tempfile::tempdir().unwrap();
    assert!(codex_executable(std::path::Path::new("codex.exe")).is_err());
    assert!(codex_home(std::path::Path::new("codex-home")).is_err());
    assert!(codex_home(&dir.path().join("missing")).is_err());
    assert_eq!(codex_home(dir.path()).unwrap(), dir.path());
    let executable = dir.path().join("codex.exe");
    std::fs::write(&executable, "").unwrap();
    assert_eq!(codex_executable(&executable).unwrap(), executable);
}

/// The bearer is a secret: it must never appear in the child's arguments,
/// which other local processes can read, only in its environment.
#[test]
fn the_codex_grant_reaches_only_the_child_environment() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let program = CodexProgram::new(dir.path().join("codex.exe"), home.clone());
    let grant = "grant-value-for-the-test";
    let setup = SessionSetup {
        mcp: vec![McpServerSpec::Http {
            name: "shadows".into(),
            url: "http://127.0.0.1:9/mcp".into(),
            headers: vec![("Authorization".into(), format!("Bearer {grant}"))],
        }],
        ..SessionSetup::default()
    };
    let spec = program.process_spec(dir.path(), &setup).unwrap();
    assert!(spec.args.iter().all(|arg| !arg.contains(grant)));
    let get = |key: &str| {
        spec.env
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("SHADOWS_CODEX_MCP_TOKEN"), Some(grant));
    assert_eq!(get("CODEX_HOME"), home.to_str());
    for setting in [
        "features.apps=false",
        "features.plugins=false",
        "features.hooks=false",
        "features.multi_agent=false",
        "web_search=\"disabled\"",
        "shell_environment_policy.exclude=[\"SHADOWS_CODEX_MCP_TOKEN\"]",
    ] {
        assert!(spec.args.iter().any(|arg| arg == setting), "{setting}");
    }
    assert!(
        spec.args
            .iter()
            .any(|arg| arg.contains("trust_level=\"untrusted\""))
    );
    assert!(
        program
            .process_spec(dir.path(), &SessionSetup::default())
            .is_err(),
        "no Codex child starts without a Shadows grant"
    );
}

/// The harness is resolved from explicit configuration, never from
/// `PATH`. `claude` is what a PATH lookup looks like when it is spelled as a
/// path, and it was the CLI's default until this test existed.
///
/// What it costs if this regresses: an Operation records the harness path and
/// version it ran. If the path means "whatever PATH resolved at spawn time",
/// that record says nothing, and an auto-update can change the stream contract
/// underneath a running install with no signal.
#[test]
fn a_bare_program_name_is_refused_as_a_harness_path() {
    for bare in ["claude", "claude.exe", "./claude", "bin/claude"] {
        let err = harness_path(std::path::Path::new(bare))
            .expect_err("a non-absolute harness path must be refused");
        assert_eq!(err, ConfigError::HarnessNotAbsolute(bare.to_string()));
    }

    let dir = tempfile::tempdir().unwrap();
    let absolute = dir.path().join("claude");
    std::fs::write(&absolute, "").unwrap();
    assert_eq!(harness_path(&absolute).unwrap(), absolute);
    let missing = dir.path().join("missing-claude");
    assert_eq!(
        harness_path(&missing).unwrap_err(),
        ConfigError::HarnessNotFound(missing.to_string_lossy().into_owned())
    );
}

#[test]
fn adapter_version_is_read_from_its_package() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"version":"0.81.1"}"#).unwrap();
    std::fs::write(dir.path().join("dist/index.js"), "").unwrap();
    assert_eq!(adapter_version(&dir.path().join("dist/index.js")), "0.81.1");
    assert_eq!(adapter_version(&dir.path().join("nope.js")), "unknown");
}

/// The adapter's tree gets the daemon's `PATH`: `process::spawn` clears the
/// environment, and a Claude without `PATH` cannot run `git`, `ls` or `mkdir`
/// for the Planner (found in the Phase B run: every shell tool failed with
/// exit 127). The value is passed by name, never the whole environment.
#[test]
fn the_adapter_inherits_path_by_name_and_names_its_claude() {
    let adapter = shadows_agent::claude::ClaudeAdapter::new(
        "/usr/bin/node".into(),
        "/opt/adapter/index.js".into(),
        "/opt/claude".into(),
        "t".into(),
        "t".into(),
    );
    let spec = adapter.process_spec(std::path::Path::new("/tmp"));
    let get = |key: &str| {
        spec.env
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("PATH"), std::env::var("PATH").ok().as_deref());
    assert!(get("PATH").is_some(), "the test process itself has a PATH");
    assert_eq!(get("CLAUDE_CODE_EXECUTABLE"), Some("/opt/claude"));
    assert!(
        spec.env.iter().all(
            |(k, _)| ["PATH", "HOME", "TMPDIR", "LANG", "CLAUDE_CODE_EXECUTABLE"]
                .contains(&k.as_str())
        ),
        "only named variables are passed: {:?}",
        spec.env.iter().map(|(k, _)| k).collect::<Vec<_>>()
    );
}
