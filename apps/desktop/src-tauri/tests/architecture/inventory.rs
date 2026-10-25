use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use syn::visit::{self, Visit};

use super::Findings;

#[derive(Default)]
struct Commands {
    handlers: BTreeSet<String>,
    build: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for Commands {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "generate_handler")
        {
            use syn::parse::Parser;
            let paths = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
                .expect("handler paths");
            for path in paths {
                self.handlers
                    .insert(path.segments.last().unwrap().ident.to_string());
            }
        }
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "commands" {
            struct Names(BTreeSet<String>);
            impl<'ast> Visit<'ast> for Names {
                fn visit_lit_str(&mut self, s: &'ast syn::LitStr) {
                    self.0.insert(s.value());
                }
            }
            let mut names = Names(BTreeSet::new());
            for arg in &call.args {
                names.visit_expr(arg);
            }
            self.build.extend(names.0);
        }
        visit::visit_expr_method_call(self, call);
    }
}

pub(super) fn check(lib: &str, build: &str, permissions: &BTreeMap<String, String>) -> Findings {
    let mut commands = Commands::default();
    commands.visit_file(&syn::parse_file(lib).expect("lib.rs parses"));
    commands.visit_file(&syn::parse_file(build).expect("build.rs parses"));
    let mut permitted = BTreeSet::new();
    for (file, text) in permissions {
        let value: toml::Value = toml::from_str(text).unwrap_or_else(|e| panic!("{file}: {e}"));
        for permission in value
            .get("permission")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(table) = permission.get("commands") {
                for key in ["allow", "deny"] {
                    for command in table
                        .get(key)
                        .and_then(toml::Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        permitted.insert(command.as_str().expect("command name").to_owned());
                    }
                }
            }
        }
    }
    let inventories = [
        ("lib.rs", &commands.handlers),
        ("build.rs", &commands.build),
        ("permissions/", &permitted),
    ];
    let all: BTreeSet<_> = inventories
        .iter()
        .flat_map(|(_, names)| names.iter())
        .collect();
    let mut findings = Findings::new();
    for name in all {
        for (file, names) in inventories {
            if !names.contains(name) {
                findings.insert(format!("command_inventory :: {file} :: missing {name}"));
            }
        }
    }
    findings
}

pub(super) fn scan(root: &Path) -> Findings {
    fn walk(dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|v| v == "toml") {
                out.insert(
                    path.display().to_string(),
                    std::fs::read_to_string(path).unwrap(),
                );
            }
        }
    }
    let mut permissions = BTreeMap::new();
    walk(&root.join("permissions"), &mut permissions);
    check(
        &std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        &std::fs::read_to_string(root.join("build.rs")).unwrap(),
        &permissions,
    )
}
