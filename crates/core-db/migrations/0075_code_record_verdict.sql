CREATE TABLE code_record_verdict (
    attempt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    verdict_json TEXT NOT NULL,
    verified_at TEXT NOT NULL
);

CREATE INDEX code_record_verdict_project ON code_record_verdict (project_id);
