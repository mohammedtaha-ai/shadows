use core_standards::{EffectiveStandards, Presence, Standards};

fn base_only() -> EffectiveStandards {
    EffectiveStandards {
        base: Standards::base().clone(),
        additions: None,
    }
}

#[test]
fn the_base_standards_parse_with_five_kinds_and_rules_s1_to_s9() {
    let b = Standards::base();
    assert_eq!(b.version, 2);
    let kinds: Vec<_> = b
        .parts
        .iter()
        .map(|p| (p.name.as_str(), p.presence))
        .collect();
    assert_eq!(
        kinds,
        [
            ("backend", Presence::Required),
            ("database", Presence::Required),
            ("api", Presence::Waivable),
            ("frontend", Presence::Waivable),
            ("integration", Presence::Optional),
        ]
    );
    let rules: Vec<_> = b.rules.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        rules,
        ["S1", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9"]
    );
    for rule in &b.rules {
        assert!(rule.id.starts_with('S'));
        for part in &rule.parts {
            assert!(
                b.parts.iter().any(|p| &p.name == part),
                "{} names {part}",
                rule.id
            );
        }
    }
}

#[test]
fn v2_names_required_backend_layers() {
    let standards = base_only();
    assert_eq!(standards.base.version, 2);
    let backend = standards
        .base
        .parts
        .iter()
        .find(|p| p.name == "backend")
        .expect("backend");
    let required: Vec<_> = backend
        .layers
        .iter()
        .filter(|l| l.required)
        .map(|l| l.id.as_str())
        .collect();
    assert_eq!(
        required,
        [
            "domain",
            "application",
            "interface",
            "infrastructure",
            "composition"
        ]
    );
    assert_eq!(standards.layers("backend").len(), 5);
    assert!(standards.layers("unknown").is_empty());
}

#[test]
fn every_may_use_points_at_a_known_layer() {
    let b = Standards::base();
    for kind in &b.parts {
        for layer in &kind.layers {
            for reference in &layer.may_use {
                let target = reference.part_kind.as_deref().unwrap_or(kind.name.as_str());
                let other = b
                    .parts
                    .iter()
                    .find(|p| p.name == target)
                    .unwrap_or_else(|| panic!("{} names unknown kind {target}", layer.id));
                assert!(
                    other.layers.iter().any(|l| l.id == reference.layer),
                    "{} names unknown layer {target}.{}",
                    layer.id,
                    reference.layer
                );
            }
        }
    }
}

#[test]
fn api_contract_has_no_implementation_dependency() {
    let b = Standards::base();
    let api = b.parts.iter().find(|p| p.name == "api").expect("api");
    let contract = api
        .layers
        .iter()
        .find(|l| l.id == "contract")
        .expect("contract");
    assert!(contract.may_use.is_empty());
    for kind in &b.parts {
        for layer in &kind.layers {
            if layer.id == "domain" {
                assert!(layer.may_use.is_empty(), "{} may use something", kind.name);
            }
        }
    }
}

#[test]
fn integration_is_optional_and_not_mandatory() {
    let standards = base_only();
    assert_eq!(
        standards.mandatory_parts(),
        ["backend", "database", "api", "frontend"]
    );
}
