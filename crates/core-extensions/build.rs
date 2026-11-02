//! Embed the bundled library's source files in a deterministic order.
use std::path::Path;

fn files(root: &Path, path: &Path, out: &mut Vec<(String, String)>) {
    let mut entries = std::fs::read_dir(path)
        .expect("read bundled library")
        .map(|entry| entry.expect("bundled entry").path())
        .collect::<Vec<_>>();
    entries.sort();
    for entry in entries {
        let metadata = std::fs::symlink_metadata(&entry).expect("bundled metadata");
        assert!(!metadata.file_type().is_symlink(), "bundled symlink");
        if metadata.is_dir() {
            files(root, &entry, out);
        } else {
            assert!(metadata.is_file(), "bundled regular file");
            out.push((
                entry
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                std::fs::canonicalize(entry)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ));
        }
    }
}

fn main() {
    let root = Path::new("assets/library");
    println!("cargo:rerun-if-changed=assets/library");
    let mut entries = Vec::new();
    files(root, root, &mut entries);
    let mut generated = String::from("const FILES: &[(&str, &[u8])] = &[\n");
    for (relative, source) in entries {
        generated.push_str(&format!("({relative:?}, include_bytes!({source:?})),\n"));
    }
    generated.push_str("];\n");
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("bundled.rs"), generated).expect("write bundled index");
}
