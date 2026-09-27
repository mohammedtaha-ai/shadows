//! One job: turn the source tree into the list of declarations a caller outside
//! the file could reach.
//!
//! What counts as reachable is `syn`'s visibility, not a guess: anything whose
//! visibility is `Inherited` (no `pub`) is private to its module and is left
//! out. Inline `mod` blocks are walked, because a `pub fn` inside a private
//! inline module is not reachable and must not appear.
//!
//! One macro is expanded rather than skipped: `newtype_id!` (`id.rs`), which
//! declares every domain id and its constructors. Left unexpanded, `ThreadId`
//! and its siblings would be missing from a document that claims to list every
//! reachable declaration — the very types a caller most often needs the shape
//! of. The expansion is the macro's own single rule with `$name` substituted,
//! read from the tree, so the map follows the macro when it changes. Any other
//! macro invocation is still invisible; a second item-declaring macro is the
//! trigger to generalise this.

use std::path::{Path, PathBuf};

use proc_macro2::{Group, TokenStream, TokenTree};

/// A declaration worth listing, with the impl block it belongs to when it has
/// one. `owner` is the printed self type, so `Storage` groups its methods.
pub enum Decl {
    Free(syn::Item),
    Method {
        owner: String,
        vis: syn::Visibility,
        sig: syn::Signature,
    },
}

pub struct FileEntry {
    /// Forward-slashed and relative to the workspace root
    /// (`crates/shadows/src/agent/acp.rs`), so the generated file is
    /// byte-identical on Windows and Linux. CI runs both.
    pub path: String,
    pub lines: usize,
    pub decls: Vec<Decl>,
}

/// Every workspace crate's `src/` directory as `(crate folder, path)`, sorted
/// by folder name. A folder under `crates/` without a `src/` is not a crate
/// the map describes.
pub fn crate_sources(workspace: &Path) -> Vec<(String, PathBuf)> {
    let crates = workspace.join("crates");
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("reading {}: {e}", crates.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.join("src").is_dir())
        .map(|path| {
            let name = path
                .file_name()
                .expect("a named entry")
                .to_string_lossy()
                .to_string();
            (name, path.join("src"))
        })
        .collect();
    out.sort();
    out
}

/// Every `.rs` file under every `crates/*/src`, sorted by path. Sorting is
/// what makes the output stable: directory iteration order is a filesystem
/// detail, and letting it through would make the generated file differ between
/// machines while nothing had actually changed.
pub fn scan(workspace: &Path) -> Vec<FileEntry> {
    let sources = crate_sources(workspace);
    let mut files = Vec::new();
    for (_, src) in &sources {
        collect(src, &mut files);
    }
    // Sorted as paths (component by component), not as strings, so `a/b.rs`
    // and `a.rs` keep the order they always had.
    files.sort();

    // `newtype_id!` lives in whichever crate's `id.rs` defines it. Reading one
    // fixed path would answer `None` once `id.rs` moved, silently dropping
    // every id from the map, so every crate is asked.
    let newtype_id = sources
        .iter()
        .find_map(|(_, src)| newtype_id_template(&src.join("id.rs")));

    files
        .iter()
        .map(|abs| {
            let source = std::fs::read_to_string(abs)
                .unwrap_or_else(|e| panic!("reading {}: {e}", abs.display()));
            let parsed = syn::parse_file(&source)
                .unwrap_or_else(|e| panic!("parsing {}: {e}", abs.display()));

            let mut decls = Vec::new();
            walk(&parsed.items, &newtype_id, &mut decls);

            FileEntry {
                path: relative(workspace, abs),
                lines: source.lines().count(),
                decls,
            }
        })
        .collect()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn relative(workspace: &Path, abs: &Path) -> String {
    abs.strip_prefix(workspace)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn walk(items: &[syn::Item], newtype_id: &Option<TokenStream>, out: &mut Vec<Decl>) {
    for item in items {
        match item {
            syn::Item::Macro(m) if m.mac.path.is_ident("newtype_id") => {
                let template = newtype_id
                    .as_ref()
                    .expect("`newtype_id!` is invoked, so some crates/*/src/id.rs must define it");
                let name = m
                    .mac
                    .tokens
                    .clone()
                    .into_iter()
                    .filter_map(|t| match t {
                        TokenTree::Ident(i) => Some(i),
                        _ => None,
                    })
                    .last()
                    .expect("`newtype_id!` names its type last");
                let expanded: syn::File = syn::parse2(substitute(template.clone(), &name))
                    .expect("`newtype_id!`'s expansion parses as items");
                walk(&expanded.items, newtype_id, out);
            }
            // An inline module that is not itself public cannot expose
            // anything, so its contents are skipped rather than flattened.
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content
                    && is_reachable(&m.vis)
                {
                    walk(inner, newtype_id, out);
                }
            }
            syn::Item::Impl(imp) => {
                // A trait impl (`impl Display for X`) adds no new name a caller
                // has to learn; its methods are reached through the trait.
                if imp.trait_.is_some() {
                    continue;
                }
                let owner = printed_type(&imp.self_ty);
                for member in &imp.items {
                    if let syn::ImplItem::Fn(f) = member
                        && is_reachable(&f.vis)
                    {
                        out.push(Decl::Method {
                            owner: owner.clone(),
                            vis: f.vis.clone(),
                            sig: f.sig.clone(),
                        });
                    }
                }
            }
            other => {
                if let Some(vis) = visibility(other)
                    && is_reachable(vis)
                {
                    out.push(Decl::Free(other.clone()));
                }
            }
        }
    }
}

/// The body of `newtype_id!`'s single rule, or `None` when the tree has no
/// such macro (then any invocation is a contradiction and `walk` says so).
fn newtype_id_template(id_rs: &Path) -> Option<TokenStream> {
    let source = std::fs::read_to_string(id_rs).ok()?;
    let parsed = syn::parse_file(&source).expect("parsing id.rs");
    parsed.items.iter().find_map(|item| match item {
        syn::Item::Macro(m) if m.ident.as_ref().is_some_and(|i| i == "newtype_id") => {
            // `(matcher) => { body }`: the body is the last brace group.
            m.mac
                .tokens
                .clone()
                .into_iter()
                .filter_map(|t| match t {
                    TokenTree::Group(g) if g.delimiter() == proc_macro2::Delimiter::Brace => {
                        Some(g.stream())
                    }
                    _ => None,
                })
                .last()
        }
        _ => None,
    })
}

/// Expands one rule body: `$name` becomes the invoked identifier, and the
/// `$( ... )*` repetition — the forwarded doc attributes — is dropped, since
/// the inventory strips attributes anyway.
fn substitute(body: TokenStream, name: &proc_macro2::Ident) -> TokenStream {
    let mut out = Vec::new();
    let mut trees = body.into_iter().peekable();
    while let Some(tree) = trees.next() {
        match tree {
            TokenTree::Punct(p) if p.as_char() == '$' => match trees.next() {
                Some(TokenTree::Ident(i)) if i == "name" => {
                    out.push(TokenTree::Ident(name.clone()))
                }
                Some(TokenTree::Group(_)) => {
                    if matches!(trees.peek(), Some(TokenTree::Punct(p)) if p.as_char() == '*') {
                        trees.next();
                    }
                }
                other => {
                    panic!("`newtype_id!` uses a metavariable the codemap cannot expand: {other:?}")
                }
            },
            TokenTree::Group(g) => {
                let mut inner = Group::new(g.delimiter(), substitute(g.stream(), name));
                inner.set_span(g.span());
                out.push(TokenTree::Group(inner));
            }
            other => out.push(other),
        }
    }
    out.into_iter().collect()
}

fn visibility(item: &syn::Item) -> Option<&syn::Visibility> {
    match item {
        syn::Item::Const(i) => Some(&i.vis),
        syn::Item::Enum(i) => Some(&i.vis),
        syn::Item::Fn(i) => Some(&i.vis),
        syn::Item::Static(i) => Some(&i.vis),
        syn::Item::Struct(i) => Some(&i.vis),
        syn::Item::Trait(i) => Some(&i.vis),
        syn::Item::Type(i) => Some(&i.vis),
        syn::Item::Use(i) => Some(&i.vis),
        _ => None,
    }
}

fn is_reachable(vis: &syn::Visibility) -> bool {
    !matches!(vis, syn::Visibility::Inherited)
}

/// `impl Storage` and `impl<'a> Foo<'a>` both need a short stable label. The
/// printed type is enough; generics are visible in the signatures below it.
fn printed_type(ty: &syn::Type) -> String {
    let file = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: vec![syn::Item::Type(syn::ItemType {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            type_token: Default::default(),
            ident: syn::Ident::new("T", syn::spanned::Spanned::span(ty)),
            generics: Default::default(),
            eq_token: Default::default(),
            ty: Box::new(ty.clone()),
            semi_token: Default::default(),
        })],
    };
    let printed = prettyplease::unparse(&file);
    printed
        .trim()
        .trim_start_matches("type T = ")
        .trim_end_matches(';')
        .trim()
        .to_string()
}
