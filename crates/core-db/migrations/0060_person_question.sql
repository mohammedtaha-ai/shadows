-- A question the Planner asked the person with `person_ask`. Owned by Turns.
-- `options` is a JSON list of `{ "label" }`; `chosen` is the index the person
-- picked, `NULL` until they answer. `entry_id` is the conversation entry whose
-- card draws the question, rewritten when it is answered.
CREATE TABLE person_question (
    id          TEXT PRIMARY KEY,
    thread_id   TEXT NOT NULL,
    entry_id    TEXT NOT NULL,
    question    TEXT NOT NULL,
    options     TEXT NOT NULL,
    chosen      INTEGER,
    asked_at    TEXT NOT NULL,
    answered_at TEXT
);
