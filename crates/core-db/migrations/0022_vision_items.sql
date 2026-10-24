-- §23.3: the vision's five fields become lists of items with stable ids, and
-- every vision change is a stored version. Text becomes one item per field.
CREATE TABLE design_vision_version (
    project_id   TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    number       INTEGER NOT NULL CHECK (number >= 1),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    -- The approved proposal this version applied; NULL for a hand edit.
    proposal_id  TEXT NULL,
    created_at   TEXT NOT NULL,
    PRIMARY KEY (project_id, number)
);

-- Match Rust Unicode White_Space when classifying empty legacy fields.
UPDATE design_workspace SET vision_content = json_object(
    'purpose', CASE WHEN trim(coalesce(json_extract(vision_content, '$.purpose'), ''),
        char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288)) = ''
        THEN json_array() ELSE json_array(json_object('id', 'PU1', 'revision', 1,
        'text', json_extract(vision_content, '$.purpose'), 'approved_from', NULL)) END,
    'users', CASE WHEN trim(coalesce(json_extract(vision_content, '$.users'), ''),
        char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288)) = ''
        THEN json_array() ELSE json_array(json_object('id', 'U1', 'revision', 1,
        'text', json_extract(vision_content, '$.users'), 'approved_from', NULL)) END,
    'goals', CASE WHEN trim(coalesce(json_extract(vision_content, '$.goals'), ''),
        char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288)) = ''
        THEN json_array() ELSE json_array(json_object('id', 'G1', 'revision', 1,
        'text', json_extract(vision_content, '$.goals'), 'approved_from', NULL)) END,
    'boundaries', CASE WHEN trim(coalesce(json_extract(vision_content, '$.boundaries'), ''),
        char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288)) = ''
        THEN json_array() ELSE json_array(json_object('id', 'B1', 'revision', 1,
        'text', json_extract(vision_content, '$.boundaries'), 'approved_from', NULL)) END,
    'technical_direction', CASE
        WHEN trim(coalesce(json_extract(vision_content, '$.technical_direction'), ''),
            char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288)) = ''
        THEN json_array() ELSE json_array(json_object('id', 'T1', 'revision', 1,
        'text', json_extract(vision_content, '$.technical_direction'),
        'approved_from', NULL)) END
);

-- One past the highest id each field holds after the conversion: 1 or 0.
ALTER TABLE design_workspace ADD COLUMN vision_counters TEXT NOT NULL DEFAULT
    '{"purpose":0,"users":0,"goals":0,"boundaries":0,"technical_direction":0}'
    CHECK (json_valid(vision_counters));
UPDATE design_workspace SET vision_counters = json_object(
    'purpose', json_array_length(vision_content, '$.purpose'),
    'users', json_array_length(vision_content, '$.users'),
    'goals', json_array_length(vision_content, '$.goals'),
    'boundaries', json_array_length(vision_content, '$.boundaries'),
    'technical_direction', json_array_length(vision_content, '$.technical_direction')
);

INSERT INTO design_vision_version (project_id, number, content_json, proposal_id, created_at)
SELECT project_id, vision_revision, vision_content, NULL, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
  FROM design_workspace WHERE vision_revision >= 1;

CREATE TABLE design_vision_proposal (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    thread_id     TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    operation_id  TEXT NOT NULL REFERENCES operation(id) ON DELETE RESTRICT,
    entry_id      TEXT NOT NULL REFERENCES thread_entry(id) ON DELETE RESTRICT,
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
    UNIQUE (project_id, seq)
);
CREATE INDEX vision_proposal_item ON design_vision_proposal(project_id,item_id,status);
