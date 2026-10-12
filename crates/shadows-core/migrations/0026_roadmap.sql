-- §23.5 roadmap proposals: the Planner proposes outcomes and the parts they
-- reference; an approved proposal bumps roadmap_revision, which opens plan work.
ALTER TABLE design_workspace ADD COLUMN roadmap_revision INTEGER NOT NULL DEFAULT 0
    CHECK (roadmap_revision >= 0);

CREATE TABLE design_roadmap_proposal (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id TEXT NOT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id TEXT NOT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (action IN ('first','add','change','remove')),
    outcome_id TEXT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    roadmap_revision INTEGER NOT NULL,
    expected_outcome_revision INTEGER NULL,
    expected_parts_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(expected_parts_json)),
    status TEXT NOT NULL CHECK (status IN ('Pending','Approved','Superseded')),
    approved_principal TEXT NULL,
    approved_agent TEXT NULL,
    approved_request_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    seq INTEGER NOT NULL,
    UNIQUE(project_id,seq)
);
CREATE INDEX roadmap_proposal_outcome
    ON design_roadmap_proposal(project_id,outcome_id,status);
