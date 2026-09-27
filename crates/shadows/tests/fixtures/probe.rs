//! Where `tree_probe` is: `shadows-process`'s test binary, which
//! `CARGO_BIN_EXE_*` names only inside that package (spec §14.3).
pub fn tree_probe_path() -> std::path::PathBuf {
    static PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        escargot::CargoBuild::new()
            .package("shadows-process")
            .bin("tree_probe")
            .features("test-support")
            .current_release()
            .run()
            .expect("tree_probe builds")
            .path()
            .to_path_buf()
    })
    .clone()
}
