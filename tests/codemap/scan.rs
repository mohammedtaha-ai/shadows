//! One job: turn the source tree into the list of declarations a caller outside
//! the file could reach.
//!
//! What counts as reachable is `syn`'s visibility, not a guess: anything whose
//! visibility is `Inherited` (no `pub`) is private to its module and is left
//! out. Inline `mod` blocks are walked, because a `pub fn` inside a private
//! inline module is not reachable and must not appear.

use std::path::Path;

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
    /// Forward-slashed and relative to the crate root, so the generated file is
    /// byte-identical on Windows and Linux. CI runs both.
    pub path: String,
    pub lines: usize,
    pub decls: Vec<Decl>,
}

/// Every `.rs` file under `root`, sorted by path. Sorting is what makes the
/// output stable: directory iteration order is a filesystem detail, and letting
/// it through would make the generated file differ between machines while
/// nothing had actually changed.
pub fn scan(root: &Path) -> Vec<FileEntry> {
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort();

    files
        .iter()
        .map(|abs| {
            let source = std::fs::read_to_string(abs)
                .unwrap_or_else(|e| panic!("reading {}: {e}", abs.display()));
            let parsed = syn::parse_file(&source)
                .unwrap_or_else(|e| panic!("parsing {}: {e}", abs.display()));

            let mut decls = Vec::new();
            walk(&parsed.items, &mut decls);

            FileEntry {
                path: relative(root, abs),
                lines: source.lines().count(),
                decls,
            }
        })
        .collect()
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
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

fn relative(root: &Path, abs: &Path) -> String {
    let root_parent = root.parent().unwrap_or(root);
    abs.strip_prefix(root_parent)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn walk(items: &[syn::Item], out: &mut Vec<Decl>) {
    for item in items {
        match item {
            // An inline module that is not itself public cannot expose
            // anything, so its contents are skipped rather than flattened.
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content
                    && is_reachable(&m.vis)
                {
                    walk(inner, out);
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
