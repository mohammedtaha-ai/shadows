//! Exact exceptions make ownership migration measurable without permitting new debt.
mod fixtures;
mod inventory;
mod scanner;

use std::collections::BTreeSet;
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
    let app = syn::parse_file(&sources["app.rs"]).unwrap();
    let fields = app
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == "DesktopApp" => Some(&item.fields),
            _ => None,
        })
        .expect("DesktopApp owns native lifetime");
    assert_eq!(fields.len(), 9, "retain the nine existing native values");
    assert!(
        fields
            .iter()
            .all(|field| matches!(field.vis, syn::Visibility::Inherited)),
        "owners are exposed through typed accessors"
    );
}
