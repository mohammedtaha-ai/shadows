ALTER TABLE agent_invocation ADD COLUMN usage_agent_slug TEXT;
ALTER TABLE agent_invocation ADD COLUMN usage_provider_slug TEXT;
ALTER TABLE agent_invocation ADD COLUMN usage_provider_name TEXT;
ALTER TABLE agent_invocation ADD COLUMN usage_label TEXT;
ALTER TABLE agent_invocation ADD COLUMN usage_preset TEXT;
CREATE TABLE aylul_usage (
    operation_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL, agent_slug TEXT NOT NULL,
    provider_slug TEXT NOT NULL, provider_name TEXT NOT NULL, label TEXT, preset TEXT,
    model TEXT NOT NULL,
    input_tokens INTEGER CHECK(input_tokens >= 0),
    output_tokens INTEGER CHECK(output_tokens >= 0),
    cached_read_tokens INTEGER CHECK(cached_read_tokens >= 0),
    cached_write_tokens INTEGER CHECK(cached_write_tokens >= 0),
    total_tokens INTEGER CHECK(total_tokens >= 0),
    cost REAL, currency TEXT,
    recorded_at TEXT NOT NULL,
    CHECK ((cost IS NULL AND currency IS NULL) OR
        (cost IS NOT NULL AND cost >= 0 AND cost <= 1.7976931348623157e308
         AND currency IS NOT NULL AND currency = 'USD'))
);
CREATE INDEX aylul_usage_period ON aylul_usage
    (recorded_at, provider_slug, model, operation_id);
CREATE INDEX aylul_usage_timestamp ON aylul_usage(julianday(recorded_at));
