CREATE TABLE design_outcome (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES design_workspace(project_id) ON DELETE RESTRICT,
    parent_id TEXT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    content_json TEXT NOT NULL CHECK (json_valid(content_json)),
    UNIQUE (project_id,id),
    FOREIGN KEY (project_id,parent_id) REFERENCES design_outcome(project_id,id) ON DELETE RESTRICT,
    CHECK (parent_id IS NULL OR parent_id <> id)
);
CREATE INDEX design_outcome_children ON design_outcome(project_id,parent_id,ordinal,id);
CREATE TABLE design_outcome_part (
    project_id TEXT NOT NULL, outcome_id TEXT NOT NULL, part_id TEXT NOT NULL,
    PRIMARY KEY(project_id,outcome_id,part_id),
    FOREIGN KEY(project_id,outcome_id) REFERENCES design_outcome(project_id,id) ON DELETE RESTRICT,
    FOREIGN KEY(project_id,part_id) REFERENCES design_part(project_id,id) ON DELETE RESTRICT
);
CREATE TABLE design_outcome_plan (
    project_id TEXT NOT NULL, outcome_id TEXT NOT NULL, plan_id TEXT NOT NULL,
    PRIMARY KEY(project_id,outcome_id,plan_id),
    FOREIGN KEY(project_id,outcome_id) REFERENCES design_outcome(project_id,id) ON DELETE RESTRICT,
    FOREIGN KEY(project_id,plan_id) REFERENCES plan(project_id,id) ON DELETE RESTRICT
);
