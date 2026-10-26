//! Exact exceptions make ownership migration measurable without permitting new debt.
mod fixtures;
mod inventory;
mod scanner;

use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type Findings = BTreeSet<String>;

fn compare(actual: &Findings, baseline: &Findings) -> Vec<String> {
    actual
        .difference(baseline)
        .map(|v| format!("NEW {v}"))
        .chain(baseline.difference(actual).map(|v| format!("STALE {v}")))
        .collect()
}

fn baseline(text: &str) -> Findings {
    let mut result = Findings::new();
    for line in text
        .lines()
        .filter(|v| !v.is_empty() && !v.starts_with('#'))
    {
        assert_eq!(line.split(" :: ").count(), 3, "invalid baseline: {line}");
        assert!(result.insert(line.to_owned()), "duplicate baseline: {line}");
    }
    result
}

#[test]
fn native_architecture_ratchet() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources = scanner::read_sources(&root.join("src"));
    let mut actual = scanner::scan(&sources);
    actual.extend(inventory::scan(root));
    actual.extend(scanner::coverage(root, &sources));
    let errors = compare(&actual, &baseline(include_str!("baseline.txt")));
    assert!(
        errors.is_empty(),
        "native architecture ratchet:\n{}",
        errors.join("\n")
    );
}

#[test]
fn desktop_registers_one_private_composition_root() {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Registration {
        managed: usize,
        starts: usize,
    }
    impl<'ast> Visit<'ast> for Registration {
        fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
            if call.method == "manage" {
                self.managed += 1;
            }
            syn::visit::visit_expr_method_call(self, call);
        }
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref() {
                let names: Vec<_> = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                if names.ends_with(&["DesktopApp".to_owned(), "start".to_owned()]) {
                    self.starts += 1;
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let sources = scanner::read_sources(&root);
    let mut registration = Registration::default();
    for source in sources.values() {
        registration.visit_file(&syn::parse_file(source).unwrap());
    }
    assert_eq!(registration.managed, 1, "register only the desktop root");
    assert_eq!(registration.starts, 1, "construct the desktop root once");
    assert_desktop_owners(&sources["app.rs"]);
}

fn expected_desktop_owners() -> BTreeMap<String, String> {
    include_str!("README.md")
        .lines()
        .filter_map(|line| {
            let columns: Vec<_> = line.split('|').map(str::trim).collect();
            if columns.len() == 4 && columns[1].starts_with('`') && columns[2].starts_with('`') {
                Some((
                    columns[1].trim_matches('`').to_owned(),
                    columns[2].trim_matches('`').to_owned(),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn assert_desktop_owners(source: &str) {
    let app = syn::parse_file(source).unwrap();
    let fields = app
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "DesktopApp" => Some(&item.fields),
            _ => None,
        })
        .expect("DesktopApp owns native lifetime");
    assert!(
        fields
            .iter()
            .all(|field| matches!(field.vis, syn::Visibility::Inherited)),
        "owners are exposed through typed accessors"
    );
    let actual: BTreeMap<_, _> = fields
        .iter()
        .map(|field| {
            (
                field
                    .ident
                    .as_ref()
                    .expect("owners have field names")
                    .to_string(),
                field.ty.to_token_stream().to_string().replace(' ', ""),
            )
        })
        .collect();
    let expected = expected_desktop_owners();
    assert_eq!(
        expected.len(),
        9,
        "the owner table declares D-0's nine owners"
    );
    assert_eq!(
        actual, expected,
        "DesktopApp fields must equal the documented owner identities"
    );
}

#[test]
fn composition_inventory_rejects_renamed_extra_and_wrong_type_fields() {
    let fields = expected_desktop_owners()
        .iter()
        .map(|(name, ty)| format!("{name}: {ty},"))
        .collect::<String>();
    let valid = format!("struct DesktopApp {{ {fields} }}");
    assert_desktop_owners(&valid);
    for invalid in [
        valid.replace("core_connection:", "connection:"),
        format!("struct DesktopApp {{ {fields} watcher: usize }}"),
        valid.replace("main_window: windows::MainWindow", "main_window: usize"),
    ] {
        assert!(
            std::panic::catch_unwind(|| assert_desktop_owners(&invalid)).is_err(),
            "a renamed, extra or mistyped owner must fail: {invalid}"
        );
    }
}
