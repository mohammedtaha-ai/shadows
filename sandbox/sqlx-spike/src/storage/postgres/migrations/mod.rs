//! Postgres migrations.

pub const V1_INIT_SQL: &str = "
CREATE TABLE IF NOT EXISTS project (
    id UUID PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS operation (
    id UUID PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    thread_id UUID,
    workflow_id UUID,
    task_id UUID,
    runtime_instance_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    durable_seq BIGINT NOT NULL,
    outcome TEXT
);
CREATE TABLE IF NOT EXISTS durable_event (
    id UUID PRIMARY KEY NOT NULL,
    durable_seq BIGINT NOT NULL,
    kind TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    operation_id UUID,
    payload JSONB NOT NULL
);
CREATE TABLE IF NOT EXISTS command_record (
    command_id UUID PRIMARY KEY NOT NULL,
    principal UUID NOT NULL,
    scope TEXT NOT NULL,
    operation_id UUID NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS research_artifact (
    id UUID PRIMARY KEY NOT NULL,
    project_id UUID NOT NULL,
    title TEXT NOT NULL,
    source TEXT,
    summary TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    tsv tsvector
);
CREATE TABLE IF NOT EXISTS durable_seq_counter (n BIGINT PRIMARY KEY);
INSERT INTO durable_seq_counter(n) VALUES (0) ON CONFLICT DO NOTHING;
";

pub const V2_TSV_TRIGGER_SQL: &str = "
CREATE OR REPLACE FUNCTION research_tsv_update() RETURNS trigger AS $$
BEGIN
    NEW.tsv := to_tsvector('english', coalesce(NEW.title,'') || ' ' || coalesce(NEW.summary,''));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS research_tsv_trigger ON research_artifact;
CREATE TRIGGER research_tsv_trigger BEFORE INSERT OR UPDATE
    ON research_artifact FOR EACH ROW EXECUTE FUNCTION research_tsv_update();
CREATE INDEX IF NOT EXISTS research_tsv_idx ON research_artifact USING GIN(tsv);
";

pub const EXTRA_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS workflow_status (
    workflow_id UUID PRIMARY KEY NOT NULL,
    status TEXT NOT NULL,
    set_at TIMESTAMPTZ NOT NULL
);
";

pub async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _shadows_migrations (\
            version INTEGER PRIMARY KEY NOT NULL,\
            applied_at TIMESTAMPTZ NOT NULL DEFAULT now()\
        )",
    )
    .execute(pool)
    .await?;

    let v1_applied: Option<(i32,)> =
        sqlx::query_as("SELECT version FROM _shadows_migrations WHERE version = 1")
            .fetch_optional(pool)
            .await?;
    if v1_applied.is_none() {
        // Use execute (not query) so multi-statement strings are allowed.
        sqlx::raw_sql(V1_INIT_SQL).execute(pool).await?;
        sqlx::query("INSERT INTO _shadows_migrations(version) VALUES (1)")
            .execute(pool)
            .await?;
    }

    let v2_applied: Option<(i32,)> =
        sqlx::query_as("SELECT version FROM _shadows_migrations WHERE version = 2")
            .fetch_optional(pool)
            .await?;
    if v2_applied.is_none() {
        sqlx::raw_sql(V2_TSV_TRIGGER_SQL).execute(pool).await?;
        sqlx::query("INSERT INTO _shadows_migrations(version) VALUES (2) ON CONFLICT DO NOTHING")
            .execute(pool)
            .await?;
    }

    sqlx::raw_sql(EXTRA_SCHEMA).execute(pool).await?;

    sqlx::query(
        "UPDATE research_artifact SET tsv = to_tsvector('english', coalesce(title,'') || ' ' || coalesce(summary,'')) WHERE tsv IS NULL"
    )
    .execute(pool)
    .await
    .ok();

    Ok(())
}
