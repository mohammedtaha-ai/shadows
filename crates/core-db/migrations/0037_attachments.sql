CREATE TABLE attachment (
    id TEXT PRIMARY KEY NOT NULL,
    media_type TEXT NOT NULL,
    byte_len INTEGER NOT NULL,
    sha256 BLOB NOT NULL,
    name TEXT,
    bytes BLOB NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE thread_attachment (
    thread_id TEXT NOT NULL REFERENCES planning_thread(id),
    attachment_id TEXT NOT NULL REFERENCES attachment(id),
    PRIMARY KEY (thread_id, attachment_id)
);
CREATE INDEX thread_attachment_image ON thread_attachment(attachment_id);
ALTER TABLE thread_entry ADD COLUMN attachments_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE queued_message ADD COLUMN attachments_json TEXT NOT NULL DEFAULT '[]';
