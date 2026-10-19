CREATE TABLE aylul_provider (
    slug TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL,
    models TEXT NOT NULL, context_window INTEGER NOT NULL,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE aylul_agent (
    slug TEXT PRIMARY KEY, name TEXT NOT NULL,
    provider TEXT REFERENCES aylul_provider(slug) ON DELETE SET NULL,
    default_model TEXT NOT NULL, deleted_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    CHECK (deleted_at IS NOT NULL OR provider IS NOT NULL)
);
ALTER TABLE planning_thread ADD COLUMN profile TEXT;
UPDATE planning_thread SET profile = 'aylul' WHERE harness_kind = 'aylul';
CREATE INDEX planning_thread_profile ON planning_thread(profile, created_at, id);
