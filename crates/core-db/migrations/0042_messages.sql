CREATE TABLE message_group (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES project(id),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE message_group_member (
    group_id TEXT NOT NULL REFERENCES message_group(id),
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    thread_id TEXT NOT NULL UNIQUE REFERENCES planning_thread(id),
    delivered_seq INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (group_id, slug)
);

CREATE TABLE message (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL REFERENCES message_group(id),
    seq INTEGER NOT NULL,
    speaker_json TEXT NOT NULL,
    to_json TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('chat', 'notice')),
    body TEXT NOT NULL,
    cause_seq INTEGER,
    operation_id TEXT UNIQUE REFERENCES operation(id),
    created_at TEXT NOT NULL,
    UNIQUE (group_id, seq)
);

CREATE TABLE message_delivery (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL,
    slug TEXT NOT NULL,
    cause_seq INTEGER NOT NULL,
    round INTEGER NOT NULL CHECK (round >= 1),
    trigger_seq INTEGER NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('waiting', 'starting', 'running', 'answered', 'failed')),
    prompt TEXT,
    model TEXT,
    mode TEXT,
    operation_id TEXT UNIQUE REFERENCES operation(id),
    FOREIGN KEY (group_id, slug) REFERENCES message_group_member(group_id, slug),
    FOREIGN KEY (group_id, cause_seq) REFERENCES message(group_id, seq),
    FOREIGN KEY (group_id, trigger_seq) REFERENCES message(group_id, seq),
    UNIQUE (group_id, slug, trigger_seq)
);

CREATE UNIQUE INDEX message_one_active_delivery ON message_delivery(group_id, slug)
    WHERE state IN ('starting', 'running');

CREATE TABLE message_cursor (
    name TEXT PRIMARY KEY,
    seq INTEGER NOT NULL
);
