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
    // A definition whose line is > 200 chars, of multi-byte Arabic: its signature is cut.
    let long = format!("    const GREETING: &str = \"{}\";", "سلام ".repeat(60));
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
    // Lines: 1 struct, 2 impl, 3 fn open, 4 the long const, 5–7 the body's end, 8 const, 9 main, 10 helper.
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
    // Every signature respects the limit; the long definition's is cut at exactly
    // 200 whole characters, a prefix of its trimmed line.
    assert!(
        tags.iter()
            .filter_map(|t| t.signature.as_ref())
            .all(|s| s.chars().count() <= 200)
    );
    let greeting = tags
        .iter()
        .find(|t| t.name == "GREETING" && t.role == Role::Definition && t.line == 4)
        .expect("the long const is a definition");
    let sig = greeting.signature.as_deref().unwrap();
    assert_eq!(sig.chars().count(), 200);
    assert!(long.trim().starts_with(sig), "{sig}");
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
