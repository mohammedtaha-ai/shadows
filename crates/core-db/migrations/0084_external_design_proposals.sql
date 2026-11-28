-- Keep conversation provenance for Planner proposals and grant provenance for external proposals.
ALTER TABLE design_vision_proposal RENAME TO design_vision_proposal_legacy;
CREATE TABLE design_vision_proposal (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id     TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id  TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id      TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    action        TEXT NOT NULL CHECK (action IN ('add','edit','remove')),
    field         TEXT NOT NULL,
    item_id       TEXT NULL,
    text          TEXT NULL,
    old_text      TEXT NULL,
    expected_item_revision INTEGER NULL,
    status        TEXT NOT NULL CHECK (status IN ('Pending','Approved','Superseded')),
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    created_at    TEXT NOT NULL,
    seq           INTEGER NOT NULL,
    approved_principal TEXT NULL,
    approved_agent TEXT NULL,
    approved_request_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    reserved_item_id TEXT NULL,
    grant_id TEXT NULL REFERENCES mcp_grant(id),
    UNIQUE (project_id, seq),
    CHECK ((thread_id IS NULL) <> (grant_id IS NULL)),
    CHECK ((thread_id IS NULL) = (operation_id IS NULL)),
    CHECK ((thread_id IS NULL) = (entry_id IS NULL))
);
INSERT INTO design_vision_proposal (
    id, project_id, thread_id, operation_id, entry_id, action,
    field, item_id, text, old_text, expected_item_revision, status,
    standards_version, standards_additions_version_id, created_at, seq, approved_principal, approved_agent,
    approved_request_entry, reserved_item_id
)
SELECT
    id, project_id, thread_id, operation_id, entry_id, action,
    field, item_id, text, old_text, expected_item_revision, status,
    standards_version, standards_additions_version_id, created_at, seq, approved_principal, approved_agent,
    approved_request_entry, reserved_item_id
FROM design_vision_proposal_legacy;
DROP TABLE design_vision_proposal_legacy;
CREATE INDEX vision_proposal_item ON design_vision_proposal(project_id,item_id,status);

ALTER TABLE design_map_proposal RENAME TO design_map_proposal_legacy;
CREATE TABLE design_map_proposal (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (action IN ('first','add','change')),
    part_id TEXT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    vision_revision INTEGER NOT NULL,
    map_revision INTEGER NOT NULL,
    expected_part_revision INTEGER NULL,
    expected_items_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(expected_items_json)),
    status TEXT NOT NULL CHECK (status IN ('Pending','Approved','Superseded')),
    standards_version INTEGER NOT NULL,
    standards_additions_version_id TEXT NULL REFERENCES standards_additions_version(id),
    created_at TEXT NOT NULL,
    seq INTEGER NOT NULL,
    approved_principal TEXT NULL,
    approved_agent TEXT NULL,
    approved_request_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    grant_id TEXT NULL REFERENCES mcp_grant(id),
    UNIQUE(project_id,seq),
    CHECK ((thread_id IS NULL) <> (grant_id IS NULL)),
    CHECK ((thread_id IS NULL) = (operation_id IS NULL)),
    CHECK ((thread_id IS NULL) = (entry_id IS NULL))
);
INSERT INTO design_map_proposal (
    id, project_id, thread_id, operation_id, entry_id, action,
    part_id, payload_json, vision_revision, map_revision, expected_part_revision, expected_items_json,
    status, standards_version, standards_additions_version_id, created_at, seq, approved_principal,
    approved_agent, approved_request_entry
)
SELECT
    id, project_id, thread_id, operation_id, entry_id, action,
    part_id, payload_json, vision_revision, map_revision, expected_part_revision, expected_items_json,
    status, standards_version, standards_additions_version_id, created_at, seq, approved_principal,
    approved_agent, approved_request_entry
FROM design_map_proposal_legacy;
DROP TABLE design_map_proposal_legacy;
CREATE INDEX map_proposal_part ON design_map_proposal(project_id,part_id,status);

ALTER TABLE design_roadmap_proposal RENAME TO design_roadmap_proposal_legacy;
CREATE TABLE design_roadmap_proposal (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id TEXT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id TEXT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (action IN ('first','add','change','remove')),
    outcome_id TEXT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    roadmap_revision INTEGER NOT NULL,
    -- The named outcome's parent and content when proposed (change/remove).
    expected_outcome_json TEXT NULL CHECK (expected_outcome_json IS NULL
        OR json_valid(expected_outcome_json)),
    -- Part id -> that part's content when proposed; plan links do not count.
    expected_parts_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(expected_parts_json)),
    status TEXT NOT NULL CHECK (status IN ('Pending','Approved','Superseded')),
    approved_principal TEXT NULL,
    approved_agent TEXT NULL,
    approved_request_entry TEXT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    seq INTEGER NOT NULL,
    grant_id TEXT NULL REFERENCES mcp_grant(id),
    UNIQUE(project_id,seq),
    CHECK ((thread_id IS NULL) <> (grant_id IS NULL)),
    CHECK ((thread_id IS NULL) = (operation_id IS NULL)),
    CHECK ((thread_id IS NULL) = (entry_id IS NULL))
);
INSERT INTO design_roadmap_proposal (
    id, project_id, thread_id, operation_id, entry_id, action,
    outcome_id, payload_json, roadmap_revision, expected_outcome_json, expected_parts_json, status,
    approved_principal, approved_agent, approved_request_entry, created_at, seq
)
SELECT
    id, project_id, thread_id, operation_id, entry_id, action,
    outcome_id, payload_json, roadmap_revision, expected_outcome_json, expected_parts_json, status,
    approved_principal, approved_agent, approved_request_entry, created_at, seq
FROM design_roadmap_proposal_legacy;
DROP TABLE design_roadmap_proposal_legacy;
CREATE INDEX roadmap_proposal_outcome
    ON design_roadmap_proposal(project_id,outcome_id,status);
