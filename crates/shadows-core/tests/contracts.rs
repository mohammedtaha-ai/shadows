//! Spec §14.7: a service contract that no longer matches its code fails here.
//! It cannot check that a rule's prose is true; the reviewer owns that.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use yaml_rust2::{Yaml, YamlLoader};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for e in read {
        let p = e.unwrap().path();
        if p.is_dir() {
            rs_files(&p, out)
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p)
        }
    }
}

fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let p = a.path();
        p.is_ident("test") || (p.segments.len() == 2 && p.segments[1].ident == "test")
    })
}

fn collect_tests(items: &[syn::Item], out: &mut BTreeSet<String>) {
    for item in items {
        match item {
            syn::Item::Fn(f) if is_test(&f.attrs) => {
                out.insert(f.sig.ident.to_string());
            }
            syn::Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    collect_tests(inner, out)
                }
            }
            _ => {}
        }
    }
}

/// Every `#[test]` and `#[tokio::test]` function under `crates/*/tests` and
/// `crates/*/src`, by name.
fn all_tests() -> BTreeSet<String> {
    let mut files = Vec::new();
    for c in std::fs::read_dir(workspace().join("crates")).unwrap() {
        let c = c.unwrap().path();
        rs_files(&c.join("tests"), &mut files);
        rs_files(&c.join("src"), &mut files);
    }
    let mut out = BTreeSet::new();
    for f in files {
        if let Ok(file) = syn::parse_file(&std::fs::read_to_string(&f).unwrap()) {
            collect_tests(&file.items, &mut out)
        }
    }
    out
}

/// Every symbol the folder declares, and the public methods of its own
/// `mod.rs`: the service's.
fn symbols(dir: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut all, mut public) = (BTreeSet::new(), BTreeSet::new());
    let mut files = Vec::new();
    rs_files(dir, &mut files);
    for f in files {
        let Ok(file) = syn::parse_file(&std::fs::read_to_string(&f).unwrap()) else {
            continue;
        };
        for item in &file.items {
            match item {
                syn::Item::Fn(x) => {
                    all.insert(x.sig.ident.to_string());
                }
                syn::Item::Struct(x) => {
                    all.insert(x.ident.to_string());
                }
                syn::Item::Enum(x) => {
                    all.insert(x.ident.to_string());
                }
                syn::Item::Const(x) => {
                    all.insert(x.ident.to_string());
                }
                syn::Item::Type(x) => {
                    all.insert(x.ident.to_string());
                }
                syn::Item::Impl(i) => {
                    for it in &i.items {
                        if let syn::ImplItem::Fn(m) = it {
                            all.insert(m.sig.ident.to_string());
                            // The service's own `mod.rs` only: `Path::ends_with` would also
                            // take `store/mod.rs`, whose `impl Storage` methods are not
                            // the service's.
                            if matches!(m.vis, syn::Visibility::Public(_))
                                && i.trait_.is_none()
                                && f == dir.join("mod.rs")
                            {
                                public.insert(m.sig.ident.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    (all, public)
}

/// `pub(crate)`, exactly: neither `pub(super)` nor `pub(in …)` nor `pub`.
fn crate_visible(vis: &syn::Visibility) -> bool {
    matches!(
        vis,
        syn::Visibility::Restricted(r) if r.in_token.is_none() && r.path.is_ident("crate")
    )
}

/// Rule 9: the `pub(crate)` functions of a service's `store` (`store.rs` or `store/`), free
/// or a method of one of its `impl` blocks, which are exactly the ones another service may
/// call inside one write (§14.6).
fn shared(dir: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    rs_files(&dir.join("store"), &mut files);
    if dir.join("store.rs").exists() {
        files.push(dir.join("store.rs"))
    }
    let mut out = BTreeSet::new();
    for f in files {
        let Ok(file) = syn::parse_file(&std::fs::read_to_string(&f).unwrap()) else {
            continue;
        };
        for item in &file.items {
            match item {
                syn::Item::Fn(x) if crate_visible(&x.vis) => {
                    out.insert(x.sig.ident.to_string());
                }
                syn::Item::Impl(i) => {
                    for it in &i.items {
                        if let syn::ImplItem::Fn(m) = it
                            && crate_visible(&m.vis)
                        {
                            out.insert(m.sig.ident.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// The names a section lists: a key whose value is a signature string, or a
/// map holding a `signature`. Any other map is a group, read recursively.
fn listed(y: &Yaml, out: &mut BTreeSet<String>) {
    if let Yaml::Hash(h) = y {
        for (k, v) in h {
            let named = matches!(v, Yaml::String(_))
                || matches!(
                    v,
                    Yaml::Hash(inner) if inner.contains_key(&Yaml::String("signature".into()))
                );
            if named {
                if let Yaml::String(k) = k {
                    out.insert(k.clone());
                }
            } else {
                listed(v, out)
            }
        }
    }
}

/// The services: every `pub fn name(&self) -> &Service` of `impl AppCore`.
fn accessors(app: &Path) -> BTreeSet<String> {
    let file = syn::parse_file(&std::fs::read_to_string(app).unwrap()).unwrap();
    let mut out = BTreeSet::new();
    for item in &file.items {
        let syn::Item::Impl(i) = item else { continue };
        if i.trait_.is_some()
            || !matches!(&*i.self_ty, syn::Type::Path(p) if p.path.is_ident("AppCore"))
        {
            continue;
        }
        for it in &i.items {
            if let syn::ImplItem::Fn(m) = it
                && matches!(m.vis, syn::Visibility::Public(_))
                && m.sig.inputs.len() == 1
                && matches!(
                    &m.sig.output,
                    syn::ReturnType::Type(_, t) if matches!(**t, syn::Type::Reference(_))
                )
            {
                out.insert(m.sig.ident.to_string());
            }
        }
    }
    out
}

#[test]
fn every_contract_matches_its_service() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let tests = all_tests();
    let mut problems = Vec::new();
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(&src).unwrap() {
        let dir = entry.unwrap().path();
        let path = dir.join("contract.yaml");
        if !path.exists() {
            continue;
        }
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        found.insert(name.clone());
        let docs = match YamlLoader::load_from_str(&std::fs::read_to_string(&path).unwrap()) {
            Ok(d) => d,
            Err(e) => {
                problems.push(format!("{name}: not valid YAML: {e}"));
                continue;
            }
        };
        let Some(doc) = docs.first() else {
            problems.push(format!("{name}: an empty contract"));
            continue;
        };
        let Some(source) = doc["source"].as_str() else {
            problems.push(format!("{name}: no source"));
            continue;
        };
        let source_dir = workspace().join(source);
        if !source_dir.exists() {
            problems.push(format!("{name}: source {source} does not exist"));
            continue;
        }
        let (declared, public) = symbols(&source_dir);
        let mut fns = BTreeSet::new();
        listed(&doc["functions"], &mut fns);
        for f in &fns {
            if !declared.contains(f) {
                problems.push(format!("{name}: `{f}` is not in {source}"))
            }
        }
        // Rule 9's calls (§14.6): a store method another service calls outside
        // its own write is declared, and the declared name exists here.
        let mut calls = BTreeSet::new();
        listed(&doc["called_by_other_services"], &mut calls);
        for r in &calls {
            if !declared.contains(r) {
                problems.push(format!(
                    "{name}: called_by_other_services names `{r}`, which is not in {source}"
                ))
            }
        }
        let mut gap_at = BTreeSet::new();
        if let Some(gaps) = doc["gaps"].as_vec() {
            for g in gaps {
                if let Some(at) = g["at"].as_str() {
                    gap_at.insert(at.to_string());
                    if !declared.contains(at) {
                        problems.push(format!("{name}: gap at `{at}` is not in {source}"))
                    }
                }
            }
        }
        // Rule 7: a shape is a type this service declares.
        if let Yaml::Hash(h) = &doc["shapes"] {
            for k in h.keys() {
                if let Some(k) = k.as_str()
                    && !declared.contains(k)
                {
                    problems.push(format!("{name}: shape `{k}` is not in {source}"))
                }
            }
        }
        // Rules 7 and 8: an agreement names real symbols, and a broken one is also a gap.
        if let Some(ags) = doc["agreements"].as_vec() {
            for a in ags {
                let between: Vec<&str> = a["between"]
                    .as_vec()
                    .map(|v| v.iter().filter_map(Yaml::as_str).collect())
                    .unwrap_or_default();
                if between.len() < 2 {
                    problems.push(format!("{name}: an agreement names fewer than two paths"))
                }
                // A path in another service is written `harness::change_model` and looked up
                // in that folder.
                for s in &between {
                    let found = match s.split_once("::") {
                        Some((svc, sym)) => symbols(&src.join(svc)).0.contains(sym),
                        None => declared.contains(*s),
                    };
                    if !found {
                        problems.push(format!("{name}: agreement path `{s}` does not exist"))
                    }
                }
                if a["holds"].as_bool() == Some(false)
                    && !between.iter().any(|s| gap_at.contains(*s))
                {
                    problems.push(format!(
                        "{name}: agreement {between:?} does not hold and no gap names it"
                    ))
                }
            }
        }
        let mut named = Vec::new();
        // The template's `agreements` carry `tested_by` too; rule 4 covers every one.
        for section in ["obligations", "agreements"] {
            if let Some(obs) = doc[section].as_vec() {
                for o in obs {
                    match &o["tested_by"] {
                        Yaml::String(s) => named.push(s.clone()),
                        Yaml::Array(a) => {
                            named.extend(a.iter().filter_map(|t| t.as_str().map(str::to_string)))
                        }
                        _ => {}
                    }
                    let t = o["tested_by"].as_str().unwrap_or("");
                    if (t == "none" || t == "unknown") && o["note"].as_str().is_none() {
                        problems.push(format!(
                            "{name}: an entry of {section} says tested_by: {t} without a note"
                        ))
                    }
                }
            }
        }
        if let Some(ts) = doc["tests"].as_vec() {
            for t in ts {
                if let Some(n) = t["name"].as_str() {
                    named.push(n.to_string())
                }
            }
        }
        for n in named {
            let bare = n.trim_start_matches("integration: ").to_string();
            if bare != "none" && bare != "unknown" && !tests.contains(&bare) {
                problems.push(format!("{name}: test `{bare}` does not exist"))
            }
        }
        for m in &public {
            if !fns.contains(m) {
                problems.push(format!(
                    "{name}: public method `{m}` has no entry in functions"
                ))
            }
        }
        // Rule 9: what a store shares inside one write is declared, and only that.
        let actual = shared(&source_dir);
        let mut declared_shared = BTreeSet::new();
        listed(&doc["shared_in_transaction"], &mut declared_shared);
        for s in actual.difference(&declared_shared) {
            problems.push(format!(
                "{name}: store function `{s}` is pub(crate) but not in shared_in_transaction"
            ))
        }
        for s in declared_shared.difference(&actual) {
            problems.push(format!(
                "{name}: shared_in_transaction names `{s}`, which is not a pub(crate) \
                 store function"
            ))
        }
    }
    // §14.7: every service has a contract. A folder without one is skipped above, so the
    // services are read from the code: `AppCore`'s accessors (`core.plans()`, §14.4), each
    // named after its folder. A ninth service is required the day its accessor exists.
    let services = accessors(&src.join("app.rs"));
    if services.is_empty() {
        problems.push("app.rs: no `AppCore` accessor found, so no service is checked".into())
    }
    for s in &services {
        if !found.contains(s) {
            problems.push(format!("{s}: the service has no contract.yaml"))
        }
    }
    assert!(
        problems.is_empty(),
        "contracts out of date (spec §14.7):\n{}",
        problems.join("\n")
    );
}
