use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use super::{POOL_METHODS, Recorder, Source, all_nodes, children, name};

pub(super) struct Index {
    types: BTreeMap<String, String>,
    aliases: BTreeMap<String, String>,
    fields: BTreeMap<(String, String), String>,
    storage_methods: BTreeMap<String, BTreeSet<String>>,
    storage_owners: BTreeMap<(String, String), BTreeSet<String>>,
    constructors: BTreeSet<String>,
    returns: BTreeMap<(String, String), String>,
    exports: BTreeSet<String>,
}

fn uses(tree: &syn::UseTree, prefix: &str, out: &mut Vec<(String, String)>) {
    match tree {
        syn::UseTree::Path(path) => uses(&path.tree, &format!("{prefix}{}::", path.ident), out),
        syn::UseTree::Name(name) => {
            let local = if name.ident == "self" {
                prefix
                    .trim_end_matches("::")
                    .rsplit("::")
                    .next()
                    .unwrap()
                    .to_owned()
            } else {
                name.ident.to_string()
            };
            let target = if name.ident == "self" {
                prefix.trim_end_matches("::").to_owned()
            } else {
                format!("{prefix}{}", name.ident)
            };
            out.push((local, target));
        }
        syn::UseTree::Rename(rename) => out.push((
            rename.rename.to_string(),
            format!("{prefix}{}", rename.ident),
        )),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                uses(item, prefix, out);
            }
        }
        syn::UseTree::Glob(_) => {
            out.push(("*".to_owned(), prefix.trim_end_matches("::").to_owned()))
        }
    }
}

fn imports(source: &Source, container: Node<'_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for child in children(container)
        .into_iter()
        .filter(|child| child.kind() == "use_declaration")
    {
        let item = syn::parse_str::<syn::ItemUse>(source.text(child)).expect("parsed Rust use");
        uses(&item.tree, "", &mut out);
    }
    out
}

fn absolute(source: &Source, path: &str) -> String {
    let krate = source.module.split("::").next().unwrap();
    if let Some(rest) = path.strip_prefix("crate::") {
        return format!("{krate}::{rest}");
    }
    if let Some(rest) = path.strip_prefix("self::") {
        return format!("{}::{rest}", source.module);
    }
    if path.starts_with("super::") {
        let mut module = source.module.clone();
        let mut rest = path;
        while let Some(next) = rest.strip_prefix("super::") {
            module = module
                .rsplit_once("::")
                .map(|(parent, _)| parent)
                .unwrap_or(&module)
                .to_owned();
            rest = next;
        }
        return format!("{module}::{rest}");
    }
    let first = path.split("::").next().unwrap_or(path);
    if children(source.tree.root_node())
        .iter()
        .any(|node| node.kind() == "mod_item" && name(source, *node) == first)
    {
        return format!("{}::{path}", source.module);
    }
    path.trim_start_matches("::").to_owned()
}

fn without_generics(path: &str) -> String {
    let mut depth = 0_usize;
    let mut out = String::new();
    for ch in path.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ch if depth == 0 && !ch.is_whitespace() => out.push(ch),
            _ => {}
        }
    }
    while out.contains("::::") {
        out = out.replace("::::", "::");
    }
    out.trim_end_matches("::").to_owned()
}

fn query_argument(
    source: &Source,
    argument: Option<Node<'_>>,
    recorder: &mut Recorder,
    call: Node<'_>,
) {
    match argument {
        Some(argument) if ["string_literal", "raw_string_literal"].contains(&argument.kind()) => {
            let literal =
                syn::parse_str::<syn::LitStr>(source.text(argument)).expect("query string literal");
            if !super::sql::statement(&literal.value()) {
                recorder.at(
                    "ownership",
                    source,
                    call,
                    "SQL review: unclassified literal query",
                );
            }
        }
        _ => recorder.at(
            "ownership",
            source,
            call,
            "SQL review: dynamic or indirect query",
        ),
    }
}

impl Index {
    pub(super) fn new(sources: &[Source]) -> Self {
        let mut index = Self {
            types: BTreeMap::new(),
            aliases: BTreeMap::new(),
            fields: BTreeMap::new(),
            storage_methods: BTreeMap::new(),
            storage_owners: BTreeMap::new(),
            constructors: BTreeSet::new(),
            returns: BTreeMap::new(),
            exports: BTreeSet::new(),
        };
        for source in sources {
            if source.module == source.module.split("::").next().unwrap() {
                for node in children(source.tree.root_node())
                    .into_iter()
                    .filter(|node| node.kind() == "use_declaration")
                {
                    let item = syn::parse_str::<syn::ItemUse>(source.text(node)).expect("root use");
                    if matches!(item.vis, syn::Visibility::Public(_)) {
                        let mut exported = Vec::new();
                        uses(&item.tree, "", &mut exported);
                        for (local, _) in exported {
                            index.exports.insert(format!("{}::{local}", source.module));
                        }
                    }
                }
            }
            for (local, path) in imports(source, source.tree.root_node()) {
                if local != "*" {
                    index.aliases.insert(
                        format!("{}::{local}", source.module),
                        absolute(source, &path),
                    );
                }
            }
            for node in all_nodes(source.tree.root_node()) {
                if matches!(node.kind(), "struct_item" | "enum_item" | "type_item") {
                    index.types.insert(
                        format!("{}::{}", source.module, name(source, node)),
                        source.owner.clone(),
                    );
                }
            }
        }
        for source in sources {
            for node in all_nodes(source.tree.root_node())
                .into_iter()
                .filter(|node| node.kind() == "type_item")
            {
                if let Some(target) = node
                    .child_by_field_name("type")
                    .and_then(|ty| index.type_of(source, ty))
                {
                    let alias = format!("{}::{}", source.module, name(source, node));
                    if alias != target {
                        index.aliases.insert(alias, target);
                    }
                }
            }
        }
        for source in sources {
            for node in all_nodes(source.tree.root_node()) {
                if node.kind() == "struct_item" {
                    let ty = format!("{}::{}", source.module, name(source, node));
                    if let Some(body) = node.child_by_field_name("body") {
                        for field in children(body)
                            .into_iter()
                            .filter(|field| field.kind() == "field_declaration")
                        {
                            if let Some(field_type) = field.child_by_field_name("type")
                                && let Some(target) = index.type_of(source, field_type)
                            {
                                index
                                    .fields
                                    .insert((ty.clone(), name(source, field)), target);
                            }
                        }
                    }
                }
                if node.kind() == "impl_item" {
                    let Some(ty) = node
                        .child_by_field_name("type")
                        .and_then(|ty| index.type_of(source, ty))
                    else {
                        continue;
                    };
                    let Some(body) = node.child_by_field_name("body") else {
                        continue;
                    };
                    for method in children(body)
                        .into_iter()
                        .filter(|child| child.kind() == "function_item")
                    {
                        let method_name = name(source, method);
                        let has_receiver =
                            method
                                .child_by_field_name("parameters")
                                .is_some_and(|parameters| {
                                    children(parameters)
                                        .iter()
                                        .any(|node| node.kind() == "self_parameter")
                                });
                        if ty.ends_with("::Storage") && has_receiver {
                            index
                                .storage_owners
                                .entry((ty.clone(), method_name.clone()))
                                .or_default()
                                .insert(source.owner.clone());
                            index
                                .storage_methods
                                .entry(method_name.clone())
                                .or_default()
                                .insert(source.owner.clone());
                        }
                        if let Some(result) = method
                            .child_by_field_name("return_type")
                            .and_then(|result| index.type_of(source, result))
                        {
                            index
                                .returns
                                .insert((ty.clone(), method_name.clone()), result);
                        }
                        if source.service
                            && ["new", "start"].contains(&method_name.as_str())
                            && (source.path
                                == format!("{}{}{}", super::CORE, source.owner, "/mod.rs")
                                || source.path
                                    == format!(
                                        "crates/core-{}/src/lib.rs",
                                        source.owner.replace('_', "-")
                                    ))
                        {
                            index.constructors.insert(ty.clone());
                        }
                    }
                }
            }
        }
        index
    }

    fn sensitive(&self, path: &str) -> bool {
        let leaf = path.rsplit("::").next().unwrap_or(path);
        let registry = [
            "Storage",
            "Sessions",
            "LiveHandles",
            "ThreadStopper",
            "Runtime",
            "CoreParts",
        ]
        .contains(&leaf);
        let exported_value = self.types.contains_key(path)
            && !registry
            && !leaf.ends_with("Store")
            && self
                .exports
                .iter()
                .any(|export| self.expand(export.clone()) == path);
        sensitive(path) && !exported_value
    }

    fn expand(&self, mut path: String) -> String {
        for _ in 0..16 {
            let parts = path.split("::").collect::<Vec<_>>();
            let mut replacement = None;
            for length in (1..=parts.len()).rev() {
                let prefix = parts[..length].join("::");
                if let Some(target) = self.aliases.get(&prefix) {
                    let suffix = parts[length..].join("::");
                    replacement = Some(if suffix.is_empty() {
                        target.clone()
                    } else {
                        format!("{target}::{suffix}")
                    });
                    break;
                }
            }
            match replacement {
                Some(next) if next != path => path = next,
                _ => break,
            }
        }
        path
    }

    fn resolve(&self, source: &Source, context: Node<'_>, path: &str) -> String {
        let normalized = without_generics(path);
        let path = normalized.as_str();
        let first = path.split("::").next().unwrap_or(path);
        let mut scope = Some(context);
        while let Some(container) = scope {
            for (local, target) in imports(source, container) {
                if local == first {
                    let suffix = &path[first.len()..];
                    return self.expand(format!("{}{suffix}", absolute(source, &target)));
                }
            }
            scope = container.parent();
        }
        let direct = self.expand(absolute(source, path));
        if direct != path
            || self.types.contains_key(&direct)
            || path.starts_with("shadows_")
            || path.starts_with("core_")
        {
            return direct;
        }
        let mut module = source.module.as_str();
        loop {
            let candidate = self.expand(format!("{module}::{path}"));
            if self.types.contains_key(&candidate) || candidate != format!("{module}::{path}") {
                return candidate;
            }
            match module.rsplit_once("::") {
                Some((parent, _)) => module = parent,
                None => break,
            }
        }
        direct
    }

    fn type_of(&self, source: &Source, node: Node<'_>) -> Option<String> {
        if matches!(node.kind(), "type_identifier" | "scoped_type_identifier") {
            return Some(self.resolve(source, node, source.text(node)));
        }
        if node.kind() == "generic_type" {
            if let Some(ty) = node.child_by_field_name("type") {
                let resolved = self.resolve(source, node, source.text(ty));
                if resolved == "sqlx::QueryBuilder" {
                    return Some(resolved);
                }
            }
            let arguments = node.child_by_field_name("type_arguments")?;
            return children(arguments)
                .into_iter()
                .find_map(|child| self.type_of(source, child));
        }
        children(node)
            .into_iter()
            .find_map(|child| self.type_of(source, child))
    }

    fn receiver(&self, source: &Source, node: Node<'_>, depth: usize) -> Option<String> {
        if depth > 12 {
            return None;
        }
        match node.kind() {
            "self" => {
                let mut parent = node.parent();
                while let Some(item) = parent {
                    if item.kind() == "impl_item" {
                        return self.type_of(source, item.child_by_field_name("type")?);
                    }
                    parent = item.parent();
                }
                None
            }
            "identifier" => {
                let identifier = source.text(node);
                let mut scope = node.parent();
                while let Some(container) = scope {
                    if container.kind() == "function_item"
                        && let Some(parameters) = container.child_by_field_name("parameters")
                    {
                        for parameter in children(parameters) {
                            if parameter
                                .child_by_field_name("pattern")
                                .is_some_and(|pattern| source.text(pattern) == identifier)
                            {
                                return parameter
                                    .child_by_field_name("type")
                                    .and_then(|ty| self.type_of(source, ty));
                            }
                        }
                    }
                    for binding in children(container).into_iter().rev().filter(|binding| {
                        binding.kind() == "let_declaration"
                            && binding.start_byte() < node.start_byte()
                    }) {
                        if binding
                            .child_by_field_name("pattern")
                            .is_some_and(|pattern| {
                                source.text(pattern) == identifier
                                    || source.text(pattern) == format!("mut {identifier}")
                            })
                        {
                            return binding
                                .child_by_field_name("type")
                                .and_then(|ty| self.type_of(source, ty))
                                .or_else(|| {
                                    binding
                                        .child_by_field_name("value")
                                        .and_then(|value| self.receiver(source, value, depth + 1))
                                });
                        }
                    }
                    scope = container.parent();
                }
                None
            }
            "field_expression" => {
                let value = node.child_by_field_name("value")?;
                let field = node.child_by_field_name("field")?;
                let ty = self.receiver(source, value, depth + 1)?;
                self.fields
                    .get(&(ty, source.text(field).to_owned()))
                    .cloned()
            }
            "call_expression" => {
                let function = node.child_by_field_name("function")?;
                if function.kind() == "field_expression" {
                    let method = function.child_by_field_name("field")?;
                    if ["clone", "as_ref", "as_mut"].contains(&source.text(method)) {
                        return self.receiver(
                            source,
                            function.child_by_field_name("value")?,
                            depth + 1,
                        );
                    }
                    let receiver =
                        self.receiver(source, function.child_by_field_name("value")?, depth + 1)?;
                    if receiver == "sqlx::QueryBuilder"
                        && ["push", "push_bind"].contains(&source.text(method))
                    {
                        return Some(receiver);
                    }
                    return self
                        .returns
                        .get(&(receiver, source.text(method).to_owned()))
                        .cloned();
                } else {
                    let function = without_generics(source.text(function));
                    if let Some((ty, method)) = function.rsplit_once("::")
                        && ["new", "start", "open", "default", "clone"].contains(&method)
                    {
                        let resolved = self.resolve(source, node, ty);
                        if self.types.contains_key(&resolved) || resolved == "sqlx::QueryBuilder" {
                            return Some(resolved);
                        }
                        if ["Arc", "std::sync::Arc", "Box"].contains(&ty) {
                            return node
                                .child_by_field_name("arguments")
                                .and_then(|args| children(args).first().copied())
                                .and_then(|arg| self.receiver(source, arg, depth + 1));
                        }
                    }
                }
                None
            }
            "reference_expression"
            | "parenthesized_expression"
            | "try_expression"
            | "await_expression" => children(node)
                .into_iter()
                .find_map(|child| self.receiver(source, child, depth + 1)),
            _ => None,
        }
    }
}

fn secret_read(ty: &str, method: &str) -> bool {
    ty.ends_with("::Secrets") && matches!(method, "resolve_env" | "resolve_provider_key")
}

fn sensitive(path: &str) -> bool {
    path.rsplit_once("::")
        .is_some_and(|(ty, method)| secret_read(ty, method))
        || path.contains("::store::")
        || path.ends_with("::store")
        || ((path.contains("::db::") || path.ends_with("::db") || path.starts_with("core_db::"))
            && !path.contains("::StorageError"))
        || [
            "Storage",
            "Sessions",
            "LiveHandles",
            "ThreadStopper",
            "Runtime",
            "CoreParts",
        ]
        .iter()
        .any(|name| path.ends_with(&format!("::{name}")))
}

pub(super) fn check(source: &Source, node: Node<'_>, index: &Index, recorder: &mut Recorder) {
    let composition = source.path == "crates/shadows-core/src/app.rs";
    if !composition
        && node.kind() == "struct_expression"
        && let Some(ty) = node
            .child_by_field_name("name")
            .and_then(|ty| index.type_of(source, ty))
        && index.constructors.contains(&ty)
    {
        recorder.at(
            "ownership",
            source,
            node,
            &format!("construct {ty} outside composition root"),
        );
    }
    if !composition
        && node.kind() == "call_expression"
        && let Some(function) = node.child_by_field_name("function")
        && function.kind() != "field_expression"
    {
        let path = index.resolve(source, node, source.text(function));
        if let Some((ty, method)) = path.rsplit_once("::")
            && ["new", "start", "default"].contains(&method)
            && index.constructors.contains(ty)
        {
            recorder.at(
                "ownership",
                source,
                node,
                &format!("construct {ty} outside composition root"),
            );
        }
    }
    if !(source.service || source.adapter) {
        return;
    }
    if node.kind() == "macro_invocation"
        && let Some(mac) = node.child_by_field_name("macro")
    {
        let path = index.resolve(source, node, source.text(mac));
        if path.starts_with("sqlx::query") {
            let first = children(node)
                .into_iter()
                .find(|child| child.kind() == "token_tree")
                .and_then(|tree| children(tree).first().copied());
            query_argument(source, first, recorder, node);
        }
    }
    if ["type_identifier", "scoped_type_identifier"].contains(&node.kind())
        && !node
            .parent()
            .is_some_and(|parent| parent.kind() == "scoped_type_identifier")
    {
        let path = index.resolve(source, node, source.text(node));
        if source.service && path.ends_with("::AppCore") {
            recorder.at("ownership", source, node, "service depends on AppCore");
        }
        if source.adapter && index.sensitive(&path) {
            recorder.at(
                "ownership",
                source,
                node,
                &format!("adapter accesses {path}"),
            );
        }
    }
    if node.kind() == "use_declaration"
        && let Ok(item) = syn::parse_str::<syn::ItemUse>(source.text(node))
    {
        let mut paths = Vec::new();
        uses(&item.tree, "", &mut paths);
        for (_, path) in paths {
            let path = index.expand(absolute(source, &path));
            if source.adapter && index.sensitive(&path) {
                recorder.at(
                    "ownership",
                    source,
                    node,
                    &format!("adapter imports {path}"),
                );
            }
            if source.service && path.ends_with("::AppCore") {
                recorder.at("ownership", source, node, "service imports AppCore");
            }
            if path.ends_with("::*") || source.text(node).contains("::*") {
                recorder.at(
                    "ownership",
                    source,
                    node,
                    "name resolution review: glob import",
                );
            }
        }
    }
    if node.kind() != "call_expression" {
        return;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    if function.kind() == "field_expression" {
        let Some(method) = function.child_by_field_name("field") else {
            return;
        };
        let method = source.text(method);
        if method == "push"
            && function
                .child_by_field_name("value")
                .and_then(|value| index.receiver(source, value, 0))
                .is_some_and(|ty| ty == "sqlx::QueryBuilder")
        {
            let argument = node
                .child_by_field_name("arguments")
                .and_then(|args| children(args).first().copied());
            query_argument(source, argument, recorder, node);
        }
        if let Some(owners) = index.storage_methods.get(method)
            && !POOL_METHODS.contains(&method)
        {
            let receiver = function
                .child_by_field_name("value")
                .and_then(|value| index.receiver(source, value, 0));
            let owners = receiver
                .as_ref()
                .and_then(|ty| index.storage_owners.get(&(ty.clone(), method.to_owned())))
                .unwrap_or(owners);
            match receiver {
                Some(ty)
                    if ty.ends_with("::Storage")
                        && (!owners.contains(&source.owner) || source.adapter) =>
                {
                    recorder.at(
                        "ownership",
                        source,
                        node,
                        &format!(
                            "Storage::{method} belongs to {}",
                            owners.iter().cloned().collect::<Vec<_>>().join(",")
                        ),
                    );
                }
                None => recorder.at(
                    "ownership",
                    source,
                    node,
                    &format!("call review: unresolved receiver for Storage::{method}"),
                ),
                Some(ty) if !index.types.contains_key(&ty) => recorder.at(
                    "ownership",
                    source,
                    node,
                    &format!("call review: unclassified type {ty} for Storage::{method}"),
                ),
                _ => {}
            }
        }
        if source.adapter
            && let Some(ty) = function
                .child_by_field_name("value")
                .and_then(|value| index.receiver(source, value, 0))
            && (index.sensitive(&ty) || secret_read(&ty, method))
        {
            recorder.at(
                "ownership",
                source,
                node,
                &format!("adapter calls {ty}::{method}"),
            );
        }
    } else {
        let call = source.text(function);
        let path = index.resolve(source, node, call);
        if let Some((ty, method)) = path.rsplit_once("::")
            && ty.ends_with("::Storage")
            && index.storage_methods.contains_key(method)
            && !POOL_METHODS.contains(&method)
        {
            let owners = index
                .storage_owners
                .get(&(ty.to_owned(), method.to_owned()))
                .unwrap_or(&index.storage_methods[method]);
            if !owners.contains(&source.owner) || source.adapter {
                recorder.at(
                    "ownership",
                    source,
                    node,
                    &format!("Storage::{method} cross-owner associated call"),
                );
            }
        }
        if path.contains("::store::") {
            let mut parts = path.split("::");
            let krate = parts.next().unwrap_or("");
            let owner = krate
                .strip_prefix("core_")
                .unwrap_or_else(|| parts.next().unwrap_or(""));
            if source.adapter || (source.service && owner != source.owner) {
                recorder.at(
                    "ownership",
                    source,
                    node,
                    &format!("cross-owner store call {path}"),
                );
            }
        }
        if path.starts_with("sqlx::query")
            || path.starts_with("sqlx::QueryBuilder::")
            || path.starts_with("sqlx::raw_sql")
        {
            let argument = node
                .child_by_field_name("arguments")
                .and_then(|args| children(args).first().copied());
            query_argument(source, argument, recorder, node);
        }
        if source.adapter
            && path
                .rsplit_once("::")
                .is_some_and(|(ty, method)| index.sensitive(ty) || secret_read(ty, method))
        {
            recorder.at(
                "ownership",
                source,
                node,
                &format!("adapter accesses {path}"),
            );
        }
    }
}
