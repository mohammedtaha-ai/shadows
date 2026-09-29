use shadows_index::{Role, Tag, extract, language_for};
use std::path::Path;

fn defs(tags: &[Tag]) -> Vec<(String, String, u32)> {
    tags.iter()
        .filter(|t| t.role == Role::Definition)
        .map(|t| (t.name.clone(), t.kind.clone(), t.line))
        .collect()
}

#[test]
fn rust_tags() {
    let long = format!("    let s = \"{}\";", "سلام ".repeat(60)); // > 200 chars, Arabic
    let src = format!(
        "pub struct Storage;\n\
         impl Storage {{\n    pub fn open(path: &str) -> Self {{\n{long}\n        Storage\n    }}\n}}\n\
         pub const LIMIT: u32 = 5;\n\
         fn main() {{ let _ = Storage::open(\"x\"); helper(); }}\n\
         fn helper() {{}}\n"
    );
    let lang = language_for(Path::new("a.rs")).expect("rust is in the table");
    let tags = extract(lang, &src);
    let d = defs(&tags);
    assert!(d.contains(&("Storage".into(), "class".into(), 1)), "{d:?}");
    assert!(d.contains(&("open".into(), "method".into(), 3)), "{d:?}");
    // Lines: 1 struct, 2 impl, 3 fn open, 4 the long line, 5–7 the body's end, 8 const, 9 main, 10 helper.
    assert!(
        d.iter().any(|(n, _, l)| n == "LIMIT" && *l == 8),
        "const is a definition: {d:?}"
    );
    let open = tags
        .iter()
        .find(|t| t.name == "open" && t.role == Role::Definition)
        .unwrap();
    assert_eq!(
        open.signature.as_deref(),
        Some("pub fn open(path: &str) -> Self {")
    );
    // A reference through a path, and a plain call.
    assert!(
        tags.iter()
            .any(|t| t.name == "open" && t.role == Role::Reference && t.line == 9)
    );
    assert!(
        tags.iter()
            .any(|t| t.name == "helper" && t.role == Role::Reference)
    );
    // The long line is no definition, but every signature respects the limit.
    assert!(
        tags.iter()
            .filter_map(|t| t.signature.as_ref())
            .all(|s| s.chars().count() <= 200)
    );
}

#[test]
fn typescript_and_tsx_tags() {
    let ts = "export interface Plan { id: string }\n\
              export function load(id: string): Plan { return get(id); }\n\
              export class Client { fetch() { return load('x'); } }\n";
    let d = defs(&extract(language_for(Path::new("a.ts")).unwrap(), ts));
    assert!(d.contains(&("Plan".into(), "interface".into(), 1)), "{d:?}");
    assert!(d.contains(&("load".into(), "function".into(), 2)), "{d:?}");
    assert!(d.contains(&("Client".into(), "class".into(), 3)), "{d:?}");
    let tsx = "export function Card() { return <div>{title()}</div>; }\n";
    let d = defs(&extract(language_for(Path::new("a.tsx")).unwrap(), tsx));
    assert!(d.contains(&("Card".into(), "function".into(), 1)), "{d:?}");
}

#[test]
fn javascript_tags_and_broken_code() {
    let js = "function start() { stop(); }\nclass Job {}\nfunction half( {\n";
    let tags = extract(language_for(Path::new("a.mjs")).unwrap(), js);
    let d = defs(&tags);
    assert!(d.contains(&("start".into(), "function".into(), 1)), "{d:?}");
    assert!(d.contains(&("Job".into(), "class".into(), 2)), "{d:?}");
    assert!(
        tags.iter()
            .any(|t| t.name == "stop" && t.role == Role::Reference)
    );
    assert!(language_for(Path::new("a.py")).is_none());
}
