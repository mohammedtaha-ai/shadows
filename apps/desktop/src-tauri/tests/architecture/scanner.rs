use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use quote::ToTokens;
use syn::visit::{self, Visit};

use super::Findings;

// Longest module prefix wins; private account primitives remain in Account.
// Preferences needs its own prefix because MainWindow may depend on it.
const OWNERS: &[(&str, &str)] = &[
    ("core_connection", "CoreConnection"),
    ("presence", "Companion"),
    ("windows::companion", "CompanionWindow"),
    ("windows::preferences", "DesktopPreferences"),
    ("windows::preferences_tests", "DesktopPreferences"),
    ("windows", "MainWindow"),
    ("navigation", "Navigation"),
    ("auth", "Account"),
    ("assistant", "Account"),
    ("native_login", "Account"),
    ("sidecar", "Activity"),
    ("app_paths", "AppPaths"),
    ("commands", "commands"),
    ("app", "composition"),
    ("", "composition"),
];

const EDGES: &[(&str, &str)] = &[
    ("commands", "composition"),
    // Window lookups remain ratcheted as managed_state until their owner moves.
    ("MainWindow", "composition"),
    ("CompanionWindow", "composition"),
    ("Companion", "CoreConnection"),
    ("Companion", "CompanionWindow"),
    ("Companion", "Navigation"),
    ("Navigation", "MainWindow"),
    ("MainWindow", "DesktopPreferences"),
];

// Lifecycle owners and OS adapters are composition dependencies; DTOs are ordinary values.
const SERVICES: &[&str] = &[
    "core_connection::Connection",
    "presence::Presence",
    "presence::watcher::PresenceWatcher",
    "windows::companion::Store",
    "windows::preferences::Store",
    "windows::preferences::PluginLoginItem",
    "auth::AuthController",
    "navigation::Navigation",
    "app_paths::AppPaths",
    "sidecar::SidecarState",
    "sidecar::Sidecar",
    "auth::service::AuthService",
];

pub(super) fn owner(module: &str) -> Option<&'static str> {
    OWNERS
        .iter()
        .filter(|(prefix, _)| {
            module == *prefix || (!prefix.is_empty() && module.starts_with(&format!("{prefix}::")))
        })
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, owner)| *owner)
}

fn boundary(module: &str) -> &str {
    OWNERS
        .iter()
        .filter(|(prefix, _)| {
            module == *prefix || (!prefix.is_empty() && module.starts_with(&format!("{prefix}::")))
        })
        .max_by_key(|(prefix, _)| prefix.len())
        .map_or("", |(prefix, _)| *prefix)
}

pub(super) fn read_sources(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else if path.extension().is_some_and(|v| v == "rs") {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace('\\', "/"),
                    std::fs::read_to_string(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn module(file: &str) -> String {
    match file {
        "lib.rs" | "main.rs" => String::new(),
        _ => file
            .trim_end_matches(".rs")
            .trim_end_matches("/mod")
            .replace('/', "::"),
    }
}

fn modules(sources: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    fn join(base: &str, name: &str) -> String {
        let joined = format!("{base}/{name}");
        let mut parts = Vec::new();
        for part in joined.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    assert!(parts.pop().is_some(), "unscanned path module: {joined}");
                }
                part => parts.push(part),
            }
        }
        parts.join("/")
    }
    fn walk(
        file: &str,
        current: &str,
        dir: &str,
        items: &[syn::Item],
        sources: &BTreeMap<String, String>,
        out: &mut BTreeMap<String, String>,
    ) {
        for item in items {
            let syn::Item::Mod(m) = item else {
                continue;
            };
            let name = m.ident.to_string();
            let child = format!("{current}::{name}")
                .trim_start_matches("::")
                .to_owned();
            if let Some((_, items)) = &m.content {
                walk(file, &child, &join(dir, &name), items, sources, out);
                continue;
            }
            let explicit = m.attrs.iter().find_map(|attr| {
                if attr.path().is_ident("path") {
                    if let syn::Meta::NameValue(n) = &attr.meta {
                        if let syn::Expr::Lit(v) = &n.value {
                            if let syn::Lit::Str(v) = &v.lit {
                                return Some(v.value());
                            }
                        }
                    }
                }
                None
            });
            let parent = file.rsplit_once('/').map_or("", |v| v.0);
            let has_explicit_path = explicit.is_some();
            let candidates = explicit.map_or_else(
                || {
                    vec![
                        join(dir, &format!("{name}.rs")),
                        join(dir, &format!("{name}/mod.rs")),
                    ]
                },
                |p| {
                    vec![join(
                        if out.get(file).is_some_and(|m| m == current) {
                            parent
                        } else {
                            dir
                        },
                        &p,
                    )]
                },
            );
            let found = candidates.into_iter().find(|p| sources.contains_key(p));
            assert!(
                !has_explicit_path || found.is_some(),
                "unscanned path module: {file}::{child}"
            );
            if let Some(path) = found {
                assert!(!out.contains_key(&path), "multiple module mounts: {path}");
                if out.insert(path.clone(), child.clone()).is_none() {
                    let source = syn::parse_file(&sources[&path]).unwrap();
                    let child_dir = if path.ends_with("/mod.rs") {
                        path.trim_end_matches("/mod.rs")
                    } else {
                        path.trim_end_matches(".rs")
                    };
                    walk(&path, &child, child_dir, &source.items, sources, out);
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    if let Some(lib) = sources.get("lib.rs") {
        out.insert("lib.rs".into(), String::new());
        walk(
            "lib.rs",
            "",
            "",
            &syn::parse_file(lib).unwrap().items,
            sources,
            &mut out,
        );
    }
    for file in sources.keys() {
        out.entry(file.clone()).or_insert_with(|| module(file));
    }
    out
}

pub(super) fn uses(tree: &syn::UseTree, prefix: &str, out: &mut Vec<(String, String)>) {
    match tree {
        syn::UseTree::Path(p) => uses(&p.tree, &format!("{prefix}{}::", p.ident), out),
        syn::UseTree::Group(g) => {
            for t in &g.items {
                uses(t, prefix, out);
            }
        }
        syn::UseTree::Name(n) => {
            let target = if n.ident == "self" {
                prefix.trim_end_matches("::").to_owned()
            } else {
                format!("{prefix}{}", n.ident)
            };
            out.push((target.rsplit("::").next().unwrap().to_owned(), target));
        }
        syn::UseTree::Rename(r) => out.push((r.rename.to_string(), format!("{prefix}{}", r.ident))),
        syn::UseTree::Glob(_) => out.push(("*".into(), prefix.trim_end_matches("::").into())),
    }
}

type Imports = BTreeMap<String, String>;

pub(super) fn resolve(raw: &str, module: &str, imports: &Imports, exports: &Imports) -> String {
    let mut path = raw.to_owned();
    let mut rooted = false;
    for _ in 0..32 {
        let next = if path == "crate" {
            String::new()
        } else if path == "self" {
            module.to_owned()
        } else if path == "super" {
            module.rsplit_once("::").map_or("", |v| v.0).to_owned()
        } else if let Some(rest) = path.strip_prefix("crate::") {
            rooted = true;
            rest.to_owned()
        } else if let Some(rest) = path.strip_prefix("self::") {
            rooted = true;
            format!("{module}::{rest}").trim_start_matches("::").into()
        } else if path.starts_with("super::") {
            rooted = true;
            let mut parent = module.to_owned();
            let mut rest = path.as_str();
            while let Some(tail) = rest.strip_prefix("super::") {
                parent = parent.rsplit_once("::").map_or("", |v| v.0).to_owned();
                rest = tail;
            }
            format!("{parent}::{rest}").trim_start_matches("::").into()
        } else {
            let (head, tail) = path.split_once("::").unwrap_or((&path, ""));
            if let Some(target) = imports.get(head).filter(|_| !rooted) {
                format!("{target}{}{tail}", if tail.is_empty() { "" } else { "::" })
            } else {
                path.clone()
            }
        };
        let mut expanded = next.clone();
        // Resolve the longest exported prefix, including module reexports.
        let mut prefix = next.as_str();
        loop {
            if let Some(target) = exports.get(prefix) {
                rooted = true;
                expanded = format!("{target}{}", &next[prefix.len()..]);
                break;
            }
            let Some((parent, _)) = prefix.rsplit_once("::") else {
                break;
            };
            prefix = parent;
        }
        if expanded == path {
            break;
        }
        path = expanded;
    }
    path
}

fn path_text(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|p| p.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn anchored(target: String) -> String {
    if owner(&target).is_some() && !target.is_empty() {
        format!("crate::{target}")
    } else {
        target
    }
}

struct Checker<'a> {
    file: &'a str,
    module: String,
    imports: Imports,
    exports: &'a Imports,
    findings: &'a mut Findings,
    occurrences: BTreeMap<(String, String), usize>,
    symbols: &'a BTreeSet<String>,
    edges: &'a mut BTreeSet<(String, String, String)>,
    factories: &'a mut BTreeSet<String>,
}

impl Checker<'_> {
    fn composition(&self) -> bool {
        (self.file == "lib.rs" && self.module.is_empty())
            || (self.file == "app.rs" && self.module == "app")
    }
    fn record(&mut self, rule: &str, target: &str) {
        let occurrence = self
            .occurrences
            .entry((rule.into(), target.into()))
            .or_default();
        *occurrence += 1;
        self.findings
            .insert(format!("{rule} :: {} :: {target} #{occurrence}", self.file));
    }
    fn resolved(&self, path: &syn::Path) -> String {
        resolve(&path_text(path), &self.module, &self.imports, self.exports)
    }
    fn reference_surface(&mut self, raw: &str) {
        let surface = resolve(raw, &self.module, &self.imports, &Imports::new());
        self.private_access(&surface);
    }
    fn private_access(&mut self, target: &str) {
        // Composition must name the lifetime values it holds and exposes.
        if self.file == "app.rs" && self.module == "app" && SERVICES.contains(&target) {
            return;
        }
        if owner(target).is_none() || owner(target) == owner(&self.module) {
            return;
        }
        let rest = target
            .strip_prefix(boundary(target))
            .unwrap()
            .trim_start_matches("::");
        let submodule = rest
            .split("::")
            .next()
            .is_some_and(|s| s.chars().next().is_some_and(char::is_lowercase))
            && rest.contains("::");
        let module_import = self.symbols.contains(target)
            && !rest.is_empty()
            && rest.chars().next().is_some_and(char::is_lowercase);
        if submodule || module_import || (SERVICES.contains(&target) && target.ends_with("::Store"))
        {
            self.record("private_access", target);
        }
    }
    fn reference(&mut self, target: &str) {
        if target.is_empty() {
            return;
        }
        let from = owner(&self.module);
        if let Some(to) = owner(target) {
            if from != Some(to) {
                if let Some(from) = from.filter(|f| *f != "composition") {
                    self.edges
                        .insert((from.into(), to.into(), self.file.into()));
                }
                if from != Some("composition") && !from.is_some_and(|f| EDGES.contains(&(f, to))) {
                    self.record("owner_edge", target);
                }
            }
        }
        let http_owner = matches!(from, Some("Companion" | "CoreConnection" | "Account"));
        if (target == "reqwest" || target.starts_with("reqwest::")) && !http_owner {
            self.record("core_http", target);
        }
        if target == "tauri::State" && !self.composition() {
            self.record("managed_state", target);
        }
    }
    fn construction(&mut self, target: &str) {
        if self.composition() {
            return;
        }
        let (ty, method) = target.rsplit_once("::").unwrap_or((target, ""));
        if SERVICES.contains(&target)
            || self.factories.contains(target)
            || (SERVICES.contains(&ty) && matches!(method, "new" | "default" | "start" | "resolve"))
            || target == "sidecar::start_default"
        {
            self.record("construction", target);
        }
    }
}

impl<'ast> Visit<'ast> for Checker<'_> {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let saved = self.imports.clone();
        if let syn::Type::Path(ty) = item.self_ty.as_ref() {
            let service = self.resolved(&ty.path);
            if SERVICES.contains(&service.as_str()) {
                struct ReturnsService<'a> {
                    name: &'a str,
                    found: bool,
                }
                impl<'ast> Visit<'ast> for ReturnsService<'_> {
                    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
                        if ty
                            .path
                            .segments
                            .last()
                            .is_some_and(|p| p.ident == "Self" || p.ident == self.name)
                        {
                            self.found = true;
                        }
                        visit::visit_type_path(self, ty);
                    }
                }
                for member in &item.items {
                    if let syn::ImplItem::Fn(method) = member {
                        if method.sig.receiver().is_none() {
                            let mut returns = ReturnsService {
                                name: service.rsplit("::").next().unwrap(),
                                found: false,
                            };
                            returns.visit_return_type(&method.sig.output);
                            if returns.found {
                                self.factories
                                    .insert(format!("{service}::{}", method.sig.ident));
                            }
                        }
                    }
                }
            }
            self.imports
                .insert("Self".into(), anchored(self.resolved(&ty.path)));
        }
        visit::visit_item_impl(self, item);
        self.imports = saved;
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if let Some((_, items)) = &item.content {
            let previous = self.module.clone();
            let imports = std::mem::take(&mut self.imports);
            self.module = format!("{}::{}", self.module, item.ident)
                .trim_start_matches("::")
                .into();
            if owner(&self.module).is_none() {
                self.record("coverage", &format!("unknown owner {}", self.module));
            }
            self.visit_items(items);
            self.module = previous;
            self.imports = imports;
        }
    }
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let saved = self.imports.clone();
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Item(syn::Item::Use(item)) => self.add_use(item),
                syn::Stmt::Item(syn::Item::Type(item)) => {
                    if let syn::Type::Path(ty) = item.ty.as_ref() {
                        let surface = resolve(
                            &path_text(&ty.path),
                            &self.module,
                            &self.imports,
                            &Imports::new(),
                        );
                        self.imports
                            .insert(item.ident.to_string(), anchored(surface));
                    }
                }
                _ => {}
            }
        }
        visit::visit_block(self, block);
        self.imports = saved;
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        let mut entries = Vec::new();
        uses(&item.tree, "", &mut entries);
        for (_, raw) in entries {
            let target = resolve(&raw, &self.module, &self.imports, self.exports);
            self.reference_surface(&raw);
            self.reference(&target);
        }
    }
    fn visit_path(&mut self, path: &'ast syn::Path) {
        let target = self.resolved(path);
        self.reference_surface(&path_text(path));
        self.reference(&target);
        visit::visit_path(self, path);
    }
    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        let target = if let Some(qself) = &expr.qself {
            if let syn::Type::Path(ty) = qself.ty.as_ref() {
                format!(
                    "{}::{}",
                    self.resolved(&ty.path),
                    expr.path.segments.last().unwrap().ident
                )
            } else {
                self.resolved(&expr.path)
            }
        } else {
            self.resolved(&expr.path)
        };
        // Taking a constructor as a value must not make an indirect call invisible.
        if !expr.path.is_ident("Self") {
            self.construction(&target);
        }
        let lookup = self.resolved(&expr.path);
        if matches!(
            lookup.as_str(),
            "tauri::Manager::state" | "tauri::Manager::try_state"
        ) && !self.composition()
        {
            self.record("managed_state", &lookup);
        }
        // A single lowercase value can shadow a module. Imports already carry
        // its dependency; qualified calls and constructor calls are checked separately.
        if expr.path.segments.len() == 1
            && expr.path.segments[0]
                .ident
                .to_string()
                .chars()
                .next()
                .is_some_and(char::is_lowercase)
            && self
                .imports
                .get(&expr.path.segments[0].ident.to_string())
                .is_none_or(|target| boundary(target) == target)
        {
            for segment in &expr.path.segments {
                self.visit_path_arguments(&segment.arguments);
            }
        } else {
            visit::visit_expr_path(self, expr);
        }
    }
    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        // A factory must initialize its private fields; calls to that factory
        // (including Self::new) are the lifetime construction boundary.
        if !expr.path.is_ident("Self") {
            self.construction(&self.resolved(&expr.path));
        }
        visit::visit_expr_struct(self, expr);
    }
    fn visit_expr_method_call(&mut self, expr: &'ast syn::ExprMethodCall) {
        if matches!(expr.method.to_string().as_str(), "state" | "try_state") && !self.composition()
        {
            let generics = expr
                .turbofish
                .to_token_stream()
                .to_string()
                .replace(" :: ", "::");
            self.record("managed_state", &format!("{}{}", expr.method, generics));
        }
        visit::visit_expr_method_call(self, expr);
    }
    fn visit_lit_str(&mut self, lit: &'ast syn::LitStr) {
        if lit.value().contains("/api/")
            && !matches!(owner(&self.module), Some("Companion" | "CoreConnection"))
        {
            self.record("core_http", &lit.value());
        }
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.visit_path(&mac.path);
        // Macro arguments are token streams; parse expressions so format!/json!/assert!
        // do not hide qualified calls, state lookups or endpoint literals.
        use syn::parse::Parser;
        let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        if let Ok(args) = parser.parse2(mac.tokens.clone()) {
            for expr in args {
                self.visit_expr(&expr);
            }
        } else {
            self.macro_tokens(mac.tokens.clone());
        }
    }
}

impl Checker<'_> {
    fn macro_tokens(&mut self, tokens: proc_macro2::TokenStream) {
        use syn::parse::Parser;
        if let Ok(stmts) = syn::Block::parse_within.parse2(tokens.clone()) {
            self.visit_block(&syn::Block {
                brace_token: Default::default(),
                stmts,
            });
            return;
        }
        // DSL separators (such as =>) need not be Rust expressions. Check the
        // longest expression at each token, then descend into unmatched groups.
        let tokens: Vec<_> = tokens.into_iter().collect();
        let mut start = 0;
        while start < tokens.len() {
            let expression = ((start + 1)..=tokens.len()).rev().find_map(|end| {
                let stream = tokens[start..end].iter().cloned().collect();
                syn::parse2::<syn::Expr>(stream)
                    .ok()
                    .map(|expr| (end, expr))
            });
            if let Some((end, expr)) = expression {
                self.visit_expr(&expr);
                start = end;
            } else {
                if let proc_macro2::TokenTree::Group(group) = &tokens[start] {
                    self.macro_tokens(group.stream());
                }
                start += 1;
            }
        }
    }
    fn add_use(&mut self, item: &syn::ItemUse) {
        let mut entries = Vec::new();
        uses(&item.tree, "", &mut entries);
        for (alias, target) in entries {
            let resolved = resolve(&target, &self.module, &self.imports, self.exports);
            if alias == "*" {
                if resolved == "tauri" {
                    for name in ["State", "Manager", "generate_handler"] {
                        self.imports.insert(name.into(), format!("tauri::{name}"));
                    }
                }
                // A foreign glob hides the named dependency surface, so require review.
                if owner(&resolved).is_some() && owner(&resolved) != owner(&self.module) {
                    self.record("unresolved_import", &format!("{resolved}::*"));
                }
                for symbol in self.symbols {
                    if let Some((parent, name)) = symbol.rsplit_once("::") {
                        if parent == resolved {
                            self.imports.insert(name.into(), anchored(symbol.clone()));
                        }
                    }
                }
            } else {
                let surface = resolve(&target, &self.module, &self.imports, &Imports::new());
                self.imports.insert(alias, anchored(surface));
            }
        }
    }
    fn visit_items(&mut self, items: &[syn::Item]) {
        let saved = self.imports.clone();
        for symbol in self.symbols {
            if let Some((parent, name)) = symbol.rsplit_once("::") {
                if parent == self.module {
                    self.imports.insert(name.into(), anchored(symbol.clone()));
                }
            }
        }
        for item in items {
            match item {
                syn::Item::Use(u) => self.add_use(u),
                syn::Item::Type(t) => {
                    if let syn::Type::Path(p) = t.ty.as_ref() {
                        let surface = resolve(
                            &path_text(&p.path),
                            &self.module,
                            &self.imports,
                            &Imports::new(),
                        );
                        self.imports.insert(t.ident.to_string(), anchored(surface));
                    }
                }
                _ => {}
            }
        }
        for item in items {
            self.visit_item(item);
        }
        self.imports = saved;
    }
}

pub(super) fn scan(sources: &BTreeMap<String, String>) -> Findings {
    let modules = modules(sources);
    let parsed: Vec<_> = sources
        .iter()
        .map(|(file, source)| {
            (
                file,
                syn::parse_file(source).unwrap_or_else(|e| panic!("{file}: {e}")),
            )
        })
        .collect();
    let mut exports = Imports::new();
    fn collect(items: &[syn::Item], module: &str, exports: &mut Imports) {
        let mut imports = Imports::new();
        for item in items {
            match item {
                syn::Item::Use(u) => {
                    let mut entries = Vec::new();
                    uses(&u.tree, "", &mut entries);
                    imports.extend(entries);
                }
                syn::Item::Type(t) => {
                    if let syn::Type::Path(ty) = t.ty.as_ref() {
                        imports.insert(t.ident.to_string(), path_text(&ty.path));
                    }
                }
                syn::Item::Struct(t) => {
                    imports.insert(
                        t.ident.to_string(),
                        format!("crate::{module}::{}", t.ident).replace("crate::::", "crate::"),
                    );
                }
                syn::Item::Mod(t) => {
                    imports.insert(
                        t.ident.to_string(),
                        format!("crate::{module}::{}", t.ident).replace("crate::::", "crate::"),
                    );
                }
                _ => {}
            }
        }
        for item in items {
            match item {
                syn::Item::Use(u) => {
                    let mut entries = Vec::new();
                    uses(&u.tree, "", &mut entries);
                    for (alias, raw) in entries {
                        let target = resolve(&raw, module, &imports, &Imports::new());
                        exports.insert(
                            format!("{module}::{alias}").trim_start_matches("::").into(),
                            target,
                        );
                    }
                }
                syn::Item::Type(t) => {
                    if let syn::Type::Path(ty) = t.ty.as_ref() {
                        exports.insert(
                            format!("{module}::{}", t.ident)
                                .trim_start_matches("::")
                                .into(),
                            resolve(&path_text(&ty.path), module, &imports, &Imports::new()),
                        );
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, items)) = &m.content {
                        collect(
                            items,
                            format!("{module}::{}", m.ident).trim_start_matches("::"),
                            exports,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    for (file, tree) in &parsed {
        collect(&tree.items, &modules[*file], &mut exports);
    }
    let mut symbols = BTreeSet::new();
    fn declarations(items: &[syn::Item], module: &str, symbols: &mut BTreeSet<String>) {
        for item in items {
            let name = match item {
                syn::Item::Struct(v) => Some(&v.ident),
                syn::Item::Enum(v) => Some(&v.ident),
                syn::Item::Type(v) => Some(&v.ident),
                syn::Item::Mod(v) => Some(&v.ident),
                _ => None,
            };
            if let Some(name) = name {
                symbols.insert(format!("{module}::{name}").trim_start_matches("::").into());
            }
            if let syn::Item::Mod(m) = item {
                if let Some((_, items)) = &m.content {
                    declarations(
                        items,
                        format!("{module}::{}", m.ident).trim_start_matches("::"),
                        symbols,
                    );
                }
            }
        }
    }
    for (file, tree) in &parsed {
        declarations(&tree.items, &modules[*file], &mut symbols);
    }
    let mut findings = Findings::new();
    let mut edges = BTreeSet::new();
    let mut factories = BTreeSet::new();
    // Discover signatures before checking calls, including calls in earlier files.
    for _ in 0..2 {
        findings.clear();
        edges.clear();
        for (file, tree) in &parsed {
            if owner(&modules[*file]).is_none() {
                findings.insert(format!("coverage :: {file} :: unknown owner"));
            }
            let current = modules[*file].clone();
            let imports = symbols
                .iter()
                .filter_map(|symbol| {
                    let (parent, name) = symbol.rsplit_once("::")?;
                    (parent == current).then(|| (name.to_owned(), symbol.clone()))
                })
                .collect();
            let mut checker = Checker {
                file,
                module: current,
                imports,
                exports: &exports,
                findings: &mut findings,
                occurrences: BTreeMap::new(),
                symbols: &symbols,
                edges: &mut edges,
                factories: &mut factories,
            };
            checker.visit_items(&tree.items);
        }
    }
    for (from, to, file) in &edges {
        fn reaches(
            edges: &BTreeSet<(String, String, String)>,
            start: &str,
            target: &str,
            seen: &mut BTreeSet<String>,
        ) -> bool {
            start == target
                || (seen.insert(start.into())
                    && edges
                        .iter()
                        .filter(|(a, _, _)| a == start)
                        .any(|(_, b, _)| reaches(edges, b, target, seen)))
        }
        if reaches(&edges, to, from, &mut BTreeSet::new()) {
            findings.insert(format!(
                "owner_cycle :: {file} :: {from} -> {to} (cycle edge)"
            ));
        }
    }
    findings
}

pub(super) fn check_coverage(
    sources: &BTreeMap<String, String>,
    contracts: &BTreeSet<String>,
    map: &str,
) -> Findings {
    let mut result = Findings::new();
    for file in sources.keys() {
        let module = module(file);
        let prefix = boundary(&module);
        if prefix.is_empty() {
            if !map.contains(&format!("`apps/desktop/src-tauri/src/{file}`")) {
                result.insert(format!("coverage :: {file} :: missing composition map"));
            }
            continue;
        }
        let directory = prefix.replace("::", "/");
        let standalone = sources.contains_key(&format!("{directory}.rs"));
        if !standalone && !contracts.contains(&format!("{directory}/contract.yaml")) {
            result.insert(format!(
                "coverage :: {file} :: missing {directory}/contract.yaml"
            ));
        }
        let mapped = format!(
            "apps/desktop/src-tauri/src/{directory}{}",
            if standalone { ".rs" } else { "/" }
        );
        if !map.contains(&format!("`{mapped}`")) {
            result.insert(format!("coverage :: {file} :: missing map {mapped}"));
        }
    }
    result
}

pub(super) fn coverage(root: &Path, sources: &BTreeMap<String, String>) -> Findings {
    let map = std::fs::read_to_string(root.join("../../../docs/aylul/codebase/README.md")).unwrap();
    let contracts = OWNERS
        .iter()
        .map(|(prefix, _)| format!("{}/contract.yaml", prefix.replace("::", "/")))
        .filter(|file| root.join("src").join(file).exists())
        .collect();
    check_coverage(sources, &contracts, &map)
}
