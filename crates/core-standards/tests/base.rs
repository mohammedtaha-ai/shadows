use core_standards::Standards;

#[test]
fn the_base_standards_parse_with_the_four_mandatory_parts() {
    let b = Standards::base();
    assert!(b.version >= 1);
    let parts: Vec<_> = b
        .parts
        .iter()
        .map(|p| (p.name.as_str(), p.waivable))
        .collect();
    assert_eq!(
        parts,
        [
            ("backend", false),
            ("database", false),
            ("api", true),
            ("frontend", true)
        ]
    );
    assert!(b.rules.len() >= 5);
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
    assert!(
        b.contract_template
            .shape
            .contains(&"obligations".to_owned())
    );
    assert!(!b.contract_template.rules.is_empty());
}
