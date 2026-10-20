CREATE TABLE aylul_person (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT,
    aylul_calls_me TEXT,
    language TEXT,
    theme TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
