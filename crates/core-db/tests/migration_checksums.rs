//! Pin the SQLx catalog independently of the current migration sources.

#[test]
fn embedded_migrations_match_the_base_catalog() {
    let baseline = include_str!("fixtures/base_migrations.sha384");
    let expected: Vec<_> = baseline.lines().collect();
    let embedded = sqlx::migrate!("./migrations");
    assert_eq!(embedded.iter().count(), expected.len());
    assert_eq!(expected.len(), 39);
    for (migration, record) in embedded.iter().zip(expected) {
        let (checksum, filename) = record.split_once(' ').unwrap();
        let (version, description) = filename
            .strip_suffix(".sql")
            .unwrap()
            .split_once('_')
            .unwrap();
        assert_eq!(migration.version, version.parse::<i64>().unwrap());
        assert_eq!(migration.description, description.replace('_', " "));
        let actual: String = migration
            .checksum
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(actual, checksum, "SQLx checksum changed for {filename}");
    }
}
