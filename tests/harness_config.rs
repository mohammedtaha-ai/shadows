//! Spec §1.4's one rule, and how the pinned adapter's version is read: the
//! configuration a harness is started from, not anything it answers.

use shadows::config::{ConfigError, adapter_version, harness_path};

/// Spec §1.4: the harness is resolved from explicit configuration, never from
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
