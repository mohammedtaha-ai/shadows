-- Milestone 2 (§13.15). workflow/task/task_parent did not exist before this.

-- Domain §4.2: ThreadEntryKind is now an enum; refuse to migrate a database
-- holding any other kind. SQLite has no ASSERT, so a CHECK on a temp table does it.
CREATE TEMP TABLE entry_kind_check (kind TEXT NOT NULL CHECK (kind IN
    ('UserMessage','AgentMessage','PermissionRefused','PlanView','PlanApproved')));
INSERT INTO entry_kind_check SELECT DISTINCT kind FROM thread_entry;
DROP TABLE entry_kind_check;

-- A grant's thread and project cannot disagree (§13.15).
CREATE UNIQUE INDEX planning_thread_id_project ON planning_thread(id, project_id);

-- §6.8 with §13.2's version, revision, title and goal. At most one Draft per
-- thread is enforced by the command that creates a draft, in its transaction.
CREATE TABLE workflow (
    id                  TEXT PRIMARY KEY,
    thread_id           TEXT NOT NULL REFERENCES planning_thread(id) ON DELETE RESTRICT,
    state               TEXT NOT NULL CHECK (state IN ('Draft','Approved','Frozen','Running','Completed','Failed')),
    previous_version_id TEXT NULL,
    source_plan_json    TEXT NULL,
    version             INTEGER NOT NULL CHECK (version >= 1),
    revision            INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0),
    title               TEXT NOT NULL,
    goal                TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    frozen_at           TEXT NULL,
    UNIQUE (id, thread_id),
    UNIQUE (previous_version_id),
    UNIQUE (thread_id, version),
    FOREIGN KEY (previous_version_id, thread_id) REFERENCES workflow(id, thread_id),
    CHECK (state NOT IN ('Frozen','Running','Completed','Failed') OR frozen_at IS NOT NULL)
);

-- §6.9 with §13.3's number: title, goal and acceptance in contract_json; the
-- declared scope (reads, writes) in scope_json.
CREATE TABLE task (
    id            TEXT PRIMARY KEY,
    workflow_id   TEXT NOT NULL REFERENCES workflow(id) ON DELETE RESTRICT,
    number        INTEGER NOT NULL CHECK (number >= 1),
    contract_json TEXT NOT NULL,
    scope_json    TEXT NOT NULL,
    state         TEXT NOT NULL DEFAULT 'Pending'
                  CHECK (state IN ('Pending','Ready','InProgress','Completed','Failed','Blocked')),
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (id, workflow_id),
    UNIQUE (workflow_id, number)
);

-- §6.10 with §13.3's link kind, label and waiting items: one row per link,
-- both ends inside one version.
CREATE TABLE task_parent (
    workflow_id   TEXT NOT NULL,
    task_id       TEXT NOT NULL,
    parent_id     TEXT NOT NULL,
    kind          TEXT NOT NULL CHECK (kind IN ('needs','completes_after')),
    label         TEXT NOT NULL,
    waiting_items TEXT NULL,
    PRIMARY KEY (workflow_id, task_id, parent_id, kind),
    FOREIGN KEY (task_id, workflow_id) REFERENCES task(id, workflow_id) ON DELETE RESTRICT,
    FOREIGN KEY (parent_id, workflow_id) REFERENCES task(id, workflow_id) ON DELETE RESTRICT,
    CHECK (task_id != parent_id),
    CHECK ((kind = 'completes_after') = (waiting_items IS NOT NULL))
);
CREATE INDEX task_parent_by_parent ON task_parent(workflow_id, parent_id);

CREATE TABLE planner_instructions_version (
    id         TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id),
    number     INTEGER NOT NULL CHECK (number >= 1),
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (project_id, number)
);

-- §13.7: the token is stored only as its hash.
CREATE TABLE mcp_grant (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL CHECK (kind IN ('thread','project')),
    thread_id  TEXT NULL,
    project_id TEXT NOT NULL REFERENCES project(id),
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    revoked_at TEXT NULL,
    UNIQUE (id, project_id),
    CHECK ((kind = 'thread') = (thread_id IS NOT NULL)),
    FOREIGN KEY (thread_id, project_id) REFERENCES planning_thread(id, project_id)
);

-- §13.5: a plan an external agent intends to start. Its plan is in its
-- grant's project; that pair crosses two tables, so the command that sets
-- workflow_id checks it inside its transaction.
CREATE TABLE draft_intent (
    draft_ref   TEXT PRIMARY KEY,
    grant_id    TEXT NOT NULL REFERENCES mcp_grant(id),
    workflow_id TEXT NULL REFERENCES workflow(id),
    created_at  TEXT NOT NULL,
    expires_at  TEXT NOT NULL
);

ALTER TABLE agent_invocation ADD COLUMN prompt_version TEXT NULL;
ALTER TABLE agent_invocation ADD COLUMN planner_instructions_version_id TEXT NULL
    REFERENCES planner_instructions_version(id);
