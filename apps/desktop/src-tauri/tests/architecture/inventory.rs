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
    let mut findings = check(
        &std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        &std::fs::read_to_string(root.join("build.rs")).unwrap(),
        &permissions,
    );
    let capabilities = std::fs::read_dir(root.join("capabilities"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(path).unwrap(),
            )
        })
        .collect();
    let output = std::process::Command::new("node")
        .arg(root.join("../tests/native-acl.test.mjs"))
        .arg("--wrapper-inventory")
        .output()
        .expect("Node is required for the TypeScript wrapper AST inventory");
    assert!(
        output.status.success(),
        "wrapper inventory: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wrappers = serde_json::from_slice(&output.stdout).expect("wrapper inventory JSON");
    findings.extend(surface_check(
        &std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        &scanner::read_sources(&root.join("src")),
        &permissions,
        &capabilities,
        &wrappers,
    ));
    findings
}

// Only allow lists confer authority; deny metadata remains in the name inventory.
fn grants(
    permissions: &BTreeMap<String, String>,
    capabilities: &BTreeMap<String, String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut allows = BTreeMap::<String, BTreeSet<String>>::new();
    for text in permissions.values() {
        let value: toml::Value = toml::from_str(text).unwrap();
        for permission in value
            .get("permission")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(id) = permission.get("identifier").and_then(toml::Value::as_str) else {
                continue;
            };
            let commands = permission.get("commands").and_then(|v| v.get("allow"));
            allows.entry(id.to_owned()).or_default().extend(
                commands
                    .and_then(toml::Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|v| v.as_str().unwrap().to_owned()),
            );
        }
    }
    let mut result = BTreeMap::<String, BTreeSet<String>>::new();
    for text in capabilities.values() {
        let cap: serde_json::Value = serde_json::from_str(text).unwrap();
        if cap.get("local").and_then(|v| v.as_bool()) == Some(false) {
            continue;
        }
        for window in cap["windows"].as_array().expect("capability windows") {
            for permission in cap["permissions"]
                .as_array()
                .expect("capability permissions")
            {
                let id = permission
                    .as_str()
                    .or_else(|| permission["identifier"].as_str())
                    .unwrap();
                if id.contains(':') {
                    continue;
                }
                let commands = allows
                    .get(id)
                    .unwrap_or_else(|| panic!("unresolved app permission {id}"));
                for command in commands {
                    result
                        .entry(command.clone())
                        .or_default()
                        .insert(window.as_str().unwrap().to_owned());
                }
            }
        }
    }
    result
}

fn expression(stmt: &syn::Stmt) -> Option<&syn::Expr> {
    match stmt {
        syn::Stmt::Expr(expr, _) => Some(expr),
        _ => None,
    }
}
fn string(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(value),
        ..
    }) = expr
    {
        Some(value.value())
    } else {
        None
    }
}
fn value(expr: &syn::Expr, label: &str, command: &str) -> Option<String> {
    string(expr).or_else(|| match expr {
        syn::Expr::Path(path) if path.path.is_ident("label") => Some(label.into()),
        syn::Expr::Path(path) if path.path.is_ident("command") || path.path.is_ident("name") => {
            Some(command.into())
        }
        syn::Expr::MethodCall(call) if call.method == "label" => Some(label.into()),
        _ => None,
    })
}
fn pattern(pat: &syn::Pat, text: &str) -> Option<bool> {
    match pat {
        syn::Pat::Wild(_) => Some(true),
        syn::Pat::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => Some(s.value() == text),
            _ => None,
        },
        syn::Pat::Or(pat) => {
            let values = pat
                .cases
                .iter()
                .map(|p| pattern(p, text))
                .collect::<Option<Vec<_>>>()?;
            Some(values.contains(&true))
        }
        _ => None,
    }
}
fn boolean(
    expr: &syn::Expr,
    label: &str,
    command: &str,
    functions: &BTreeMap<String, syn::ItemFn>,
) -> Option<bool> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(v),
            ..
        }) => Some(v.value),
        syn::Expr::Unary(v) if matches!(v.op, syn::UnOp::Not(_)) => {
            Some(!boolean(&v.expr, label, command, functions)?)
        }
        syn::Expr::Paren(v) => boolean(&v.expr, label, command, functions),
        syn::Expr::Binary(v) => match v.op {
            syn::BinOp::Eq(_) => {
                Some(value(&v.left, label, command)? == value(&v.right, label, command)?)
            }
            syn::BinOp::Ne(_) => {
                Some(value(&v.left, label, command)? != value(&v.right, label, command)?)
            }
            syn::BinOp::Or(_) => Some(
                boolean(&v.left, label, command, functions)?
                    || boolean(&v.right, label, command, functions)?,
            ),
            syn::BinOp::And(_) => Some(
                boolean(&v.left, label, command, functions)?
                    && boolean(&v.right, label, command, functions)?,
            ),
            _ => None,
        },
        syn::Expr::Match(v) => {
            let input = value(&v.expr, label, command)?;
            for arm in &v.arms {
                if arm.guard.is_some() {
                    return None;
                }
                if pattern(&arm.pat, &input)? {
                    return boolean(&arm.body, label, command, functions);
                }
            }
            None
        }
        syn::Expr::Macro(v) if v.mac.path.is_ident("matches") => {
            use syn::parse::Parser;
            let (expr, pat) = (|input: syn::parse::ParseStream| {
                let expr = input.parse::<syn::Expr>()?;
                input.parse::<syn::Token![,]>()?;
                Ok((expr, syn::Pat::parse_multi(input)?))
            })
            .parse2(v.mac.tokens.clone())
            .ok()?;
            pattern(&pat, &value(&expr, label, command)?)
        }
        syn::Expr::Call(v) => {
            let syn::Expr::Path(path) = v.func.as_ref() else {
                return None;
            };
            let name = path.path.segments.last()?.ident.to_string();
            if name != "allowed_caller" {
                return None;
            }
            let command = value(v.args.last()?, label, command)?;
            let function = functions.get("commands::companion::allowed_caller")?;
            boolean(
                expression(function.block.stmts.first()?)?,
                label,
                &command,
                functions,
            )
        }
        _ => None,
    }
}
fn result_ok(expr: &syn::Expr) -> Option<bool> {
    match expr {
        syn::Expr::Return(v) => result_ok(v.expr.as_deref()?),
        syn::Expr::Block(v) => result_ok(expression(v.block.stmts.last()?)?),
        syn::Expr::Call(v) => {
            let syn::Expr::Path(path) = v.func.as_ref() else {
                return None;
            };
            match path.path.get_ident()?.to_string().as_str() {
                "Ok" => Some(true),
                "Err" => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

// Admission comes from the first executed statement, never the helper's policy table alone.
fn admission(
    function: &syn::ItemFn,
    module: &str,
    label: &str,
    command: &str,
    functions: &BTreeMap<String, syn::ItemFn>,
    depth: usize,
) -> Option<bool> {
    if depth > 8 {
        return None;
    }
    let Some(first) = function.block.stmts.first().and_then(expression) else {
        return Some(true);
    };
    if let syn::Expr::If(v) = first {
        let condition = boolean(&v.cond, label, command, functions)?;
        let then = result_ok(expression(v.then_branch.stmts.last()?)?)?;
        let otherwise = v
            .else_branch
            .as_ref()
            .map(|(_, e)| result_ok(e))
            .unwrap_or(Some(true))?;
        return Some(if condition { then } else { otherwise });
    }
    if let syn::Expr::Try(v) = first {
        let syn::Expr::Call(call) = v.expr.as_ref() else {
            return None;
        };
        let syn::Expr::Path(path) = call.func.as_ref() else {
            return None;
        };
        let mut parts: Vec<_> = module.split("::").map(str::to_owned).collect();
        for segment in &path.path.segments {
            match segment.ident.to_string().as_str() {
                "crate" => parts.clear(),
                "super" => {
                    parts.pop();
                }
                "self" => {}
                name => parts.push(name.into()),
            }
        }
        let target = parts.join("::");
        let Some(helper) = functions.get(&target) else {
            return Some(true);
        };
        let command = call
            .args
            .iter()
            .find_map(string)
            .unwrap_or_else(|| command.into());
        return admission(
            helper,
            target.rsplit_once("::")?.0,
            label,
            &command,
            functions,
            depth + 1,
        );
    }
    Some(true)
}

pub(super) fn surface_check(
    lib: &str,
    sources: &BTreeMap<String, String>,
    permissions: &BTreeMap<String, String>,
    capabilities: &BTreeMap<String, String>,
    wrappers: &BTreeMap<String, BTreeSet<String>>,
) -> Findings {
    let mut registration = Commands::default();
    registration.visit_file(&syn::parse_file(lib).unwrap());
    let grants = grants(permissions, capabilities);
    let mut functions = BTreeMap::new();
    fn collect(items: &[syn::Item], module: &str, functions: &mut BTreeMap<String, syn::ItemFn>) {
        for item in items {
            match item {
                syn::Item::Fn(f) if !test_only(&f.attrs) => {
                    functions.insert(format!("{module}::{}", f.sig.ident), f.clone());
                }
                syn::Item::Mod(m) if !test_only(&m.attrs) => {
                    if let Some((_, items)) = &m.content {
                        collect(items, &format!("{module}::{}", m.ident), functions);
                    }
                }
                _ => {}
            }
        }
    }
    for (file, source) in sources {
        let module = file
            .strip_suffix(".rs")
            .unwrap()
            .trim_end_matches("/mod")
            .replace('/', "::");
        collect(
            &syn::parse_file(source).unwrap().items,
            &module,
            &mut functions,
        );
    }
    let mut findings = Findings::new();
    let all: BTreeSet<_> = registration
        .handlers
        .iter()
        .chain(grants.keys())
        .chain(wrappers.keys())
        .collect();
    for command in all {
        let granted = grants.get(command).cloned().unwrap_or_default();
        let wrapped = wrappers.get(command).cloned().unwrap_or_default();
        if !registration.handlers.contains(command) {
            findings.insert(format!("command_surface :: {command} :: not registered"));
        }
        if granted != wrapped {
            findings.insert(format!(
                "command_grant_wrapper :: {command} :: grant={} wrapper={}",
                labels(&granted),
                labels(&wrapped)
            ));
        }
        if !registration.handlers.contains(command) {
            continue;
        }
        let definitions: Vec<_> = functions
            .iter()
            .filter(|(_, f)| {
                f.sig.ident == command
                    && f.attrs.iter().any(|a| {
                        a.path()
                            .segments
                            .last()
                            .is_some_and(|s| s.ident == "command")
                    })
            })
            .collect();
        let [(path, function)] = definitions.as_slice() else {
            findings.insert(format!(
                "command_guard :: {command} :: expected one command definition"
            ));
            continue;
        };
        // Literal candidates include labels admitted only by guards, without an ACL or wrapper.
        struct Literals(BTreeSet<String>);
        impl<'ast> Visit<'ast> for Literals {
            fn visit_lit_str(&mut self, value: &'ast syn::LitStr) {
                self.0.insert(value.value());
            }
            fn visit_macro(&mut self, mac: &'ast syn::Macro) {
                fn tokens(stream: proc_macro2::TokenStream, out: &mut BTreeSet<String>) {
                    for token in stream {
                        match token {
                            proc_macro2::TokenTree::Group(group) => tokens(group.stream(), out),
                            proc_macro2::TokenTree::Literal(literal) => {
                                if let Ok(value) =
                                    syn::parse_str::<syn::LitStr>(&literal.to_string())
                                {
                                    out.insert(value.value());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                tokens(mac.tokens.clone(), &mut self.0);
            }
        }
        let mut literals = Literals(BTreeSet::new());
        for function in functions.values() {
            literals.visit_item_fn(function);
        }
        // The extra label distinguishes an unrestricted function from a finite policy.
        let universe: BTreeSet<_> = granted
            .iter()
            .chain(wrapped.iter())
            .cloned()
            .chain(literals.0)
            .chain(["main".into(), "companion".into(), "<other>".into()])
            .collect();
        let mut admitted = BTreeSet::new();
        let mut known = true;
        for label in universe {
            match admission(
                function,
                path.rsplit_once("::").unwrap().0,
                &label,
                command,
                &functions,
                0,
            ) {
                Some(true) => {
                    admitted.insert(label);
                }
                Some(false) => {}
                None => {
                    known = false;
                }
            }
        }
        if !known {
            findings.insert(format!(
                "command_guard :: {command} :: unsupported first guard"
            ));
            continue;
        }
        if admitted != granted {
            findings.insert(format!(
                "command_guard_grant :: {command} :: guard={} grant={}",
                labels(&admitted),
                labels(&granted)
            ));
        }
        if admitted != wrapped {
            findings.insert(format!(
                "command_guard_wrapper :: {command} :: guard={} wrapper={}",
                labels(&admitted),
                labels(&wrapped)
            ));
        }
    }
    findings
}
fn labels(labels: &BTreeSet<String>) -> String {
    if labels.contains("<other>") {
        "any".into()
    } else if labels.is_empty() {
        "none".into()
    } else {
        labels.iter().cloned().collect::<Vec<_>>().join(",")
    }
}
