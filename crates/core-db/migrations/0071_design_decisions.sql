CREATE TABLE design_decisions (
    project_id TEXT NOT NULL,
    decision_id TEXT NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    title TEXT NOT NULL,
    decision TEXT NOT NULL,
    rationale TEXT NOT NULL,
    source TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('proposed','accepted','superseded')),
    proposed_by TEXT NOT NULL,
    approved_by_json TEXT CHECK (approved_by_json IS NULL OR json_valid(approved_by_json)),
    created_at TEXT NOT NULL,
    accepted_at TEXT,
    PRIMARY KEY (project_id, decision_id, version)
);
