-- §20: a message the person wrote while a turn ran, waiting to be sent.
CREATE TABLE queued_message (
    id          TEXT PRIMARY KEY,
    thread_id   TEXT NOT NULL REFERENCES planning_thread(id),
    position    INTEGER NOT NULL,
    prompt      TEXT NOT NULL,
    model       TEXT NOT NULL,
    mode        TEXT NOT NULL,
    effort      TEXT,
    focus_json  TEXT,
    plan_id     TEXT,
    last_error  TEXT,
    created_at  TEXT NOT NULL,
    UNIQUE (thread_id, position)
);
