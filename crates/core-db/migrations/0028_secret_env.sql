CREATE TABLE secret_env (
    profile TEXT NOT NULL,
    name TEXT NOT NULL,
    format INTEGER NOT NULL,
    nonce BLOB NOT NULL,
    ciphertext BLOB NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (profile, name)
);

CREATE TABLE secret_env_import (
    profile TEXT PRIMARY KEY NOT NULL,
    imported_at TEXT NOT NULL,
    stored INTEGER NOT NULL,
    skipped TEXT NOT NULL
);
