use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use syn::visit::{self, Visit};

use super::{scanner, Findings};

#[derive(Default)]
struct Commands {
    handlers: BTreeSet<String>,
    build: BTreeSet<String>,
    imports: BTreeMap<String, String>,
    registration_depth: usize,
}

fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || (attr.path().is_ident("cfg")
                && attr
                    .parse_args::<syn::Path>()
                    .is_ok_and(|path| path.is_ident("test")))
    })
}

impl Commands {
    fn add_import(&mut self, item: &syn::ItemUse) {
        let mut entries = Vec::new();
        scanner::uses(&item.tree, "", &mut entries);
        self.imports.extend(entries);
    }
}

impl<'ast> Visit<'ast> for Commands {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.imports.clear();
        for item in &file.items {
            if let syn::Item::Use(item) = item {
                self.add_import(item);
            }
        }
        visit::visit_file(self, file);
    }
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !test_only(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if test_only(&item.attrs) {
            return;
        }
        let saved = std::mem::take(&mut self.imports);
        if let Some((_, items)) = &item.content {
            for item in items {
                if let syn::Item::Use(item) = item {
                    self.add_import(item);
                }
            }
        }
        visit::visit_item_mod(self, item);
        self.imports = saved;
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let saved = self.imports.clone();
        for stmt in &block.stmts {
            if let syn::Stmt::Item(syn::Item::Use(item)) = stmt {
                self.add_import(item);
            }
        }
        visit::visit_block(self, block);
        self.imports = saved;
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.registration_depth == 0 {
            return;
        }
        let path = mac
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if scanner::resolve(&path, "", &self.imports, &BTreeMap::new()) == "tauri::generate_handler"
        {
            use syn::parse::Parser;
            let paths = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
                .expect("handler paths");
            for path in paths {
                let path = path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let path = scanner::resolve(&path, "", &self.imports, &BTreeMap::new());
                self.handlers
                    .insert(path.rsplit("::").next().unwrap().to_owned());
            }
        }
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "invoke_handler" {
            self.visit_expr(&call.receiver);
            self.registration_depth += 1;
            for arg in &call.args {
                self.visit_expr(arg);
            }
            self.registration_depth -= 1;
            return;
        }
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
    let mut registration = Commands::default();
    registration.visit_file(&syn::parse_file(lib).expect("lib.rs parses"));
    let mut metadata = Commands::default();
    metadata.visit_file(&syn::parse_file(build).expect("build.rs parses"));
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
        ("lib.rs", &registration.handlers),
        ("build.rs", &metadata.build),
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
