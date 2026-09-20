pub mod m20240101_000001_init;
pub mod m20240101_000002_event_seq_unique;

pub struct Migrator;

#[async_trait::async_trait]
impl sea_orm_migration::MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_init::Migration),
            Box::new(m20240101_000002_event_seq_unique::Migration),
        ]
    }
}
