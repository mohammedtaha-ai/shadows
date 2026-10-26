import { DatabaseSync } from 'node:sqlite';
import { mkdirSync } from 'node:fs';
import { dirname } from 'node:path';

const SCHEMA = `
CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, body TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS steps (seq INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS decisions (seq INTEGER PRIMARY KEY AUTOINCREMENT, item_id TEXT NOT NULL, session_id TEXT NOT NULL,
  kind TEXT NOT NULL, tool TEXT NOT NULL, target TEXT NOT NULL, outcome TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS rules (id TEXT PRIMARY KEY, project TEXT NOT NULL, rule TEXT NOT NULL, backup_path TEXT NOT NULL,
  approved_by TEXT NOT NULL, at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS grant_uses (seq INTEGER PRIMARY KEY AUTOINCREMENT, rule_id TEXT NOT NULL, session_id TEXT NOT NULL,
  tool_use_id TEXT NOT NULL, at TEXT NOT NULL);
`;

/** Composition opens one connection shared by the private domain stores. */
export function openDatabase(file: string): DatabaseSync {
  if (file !== ':memory:') mkdirSync(dirname(file), { recursive: true });
  const db = new DatabaseSync(file);
  db.exec(SCHEMA);
  return db;
}
