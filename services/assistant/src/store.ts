import {DatabaseSync} from 'node:sqlite';
import {randomUUID} from 'node:crypto';
import {mkdirSync,chmodSync} from 'node:fs';
import {dirname} from 'node:path';
export type Conversation={id:string;command_id:string;session_id:string;title:string;created_at:string;updated_at:string};
export type Turn={id:string;conversation_id:string;command_id:string;fingerprint:string;input:string;output:string;run_id:string|null;state:string;created_at:string;updated_at:string};
export class Store {
  private db:DatabaseSync;
  readonly installationId:string;
  constructor(path:string){
    mkdirSync(dirname(path),{recursive:true,mode:0o700});
    this.db=new DatabaseSync(path);chmodSync(path,0o600);
    this.db.exec('PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=3000;');
    const version=this.db.prepare('PRAGMA user_version').get()?.user_version;
    if(version!==0&&version!==1){this.db.close();throw new Error('unavailable');}
    this.db.exec(`BEGIN IMMEDIATE;
      CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS conversations (id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, session_id TEXT NOT NULL UNIQUE, title TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS turns (id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id), command_id TEXT NOT NULL UNIQUE, fingerprint TEXT NOT NULL, input TEXT NOT NULL, output TEXT NOT NULL, run_id TEXT, state TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, ordinal INTEGER NOT NULL, UNIQUE(conversation_id,ordinal));
      CREATE UNIQUE INDEX IF NOT EXISTS active_turn ON turns(conversation_id) WHERE state IN ('submitting','running','unknown','stopping');
      PRAGMA user_version=1; COMMIT;`);
    this.db.exec('PRAGMA foreign_keys=ON');
    this.db.prepare("INSERT OR IGNORE INTO metadata VALUES ('installation',?)").run(randomUUID());
    this.installationId=String(this.db.prepare("SELECT value FROM metadata WHERE key='installation'").get()!.value);
    // The old process might have submitted these writes. Never replay them at boot.
    this.db.prepare("UPDATE turns SET state='unknown' WHERE state='submitting'").run();
  }
  transaction<T>(work:()=>T):T {this.db.exec('BEGIN IMMEDIATE');try{const result=work();this.db.exec('COMMIT');return result;}catch(error){this.db.exec('ROLLBACK');throw error;}}
  close(){this.db.close();}
  list(){return this.db.prepare('SELECT * FROM conversations ORDER BY updated_at DESC,id LIMIT 100').all() as unknown as Conversation[];}
  conversation(id:string){const row=this.db.prepare('SELECT * FROM conversations WHERE id=?').get(id) as unknown as Conversation|undefined;if(!row)throw new Error('not_found');return row;}
  byCommand(id:string){return this.db.prepare('SELECT * FROM conversations WHERE command_id=?').get(id) as unknown as Conversation|undefined;}
  create(row:Conversation){this.db.prepare('INSERT INTO conversations VALUES (?,?,?,?,?,?)').run(row.id,row.command_id,row.session_id,row.title,row.created_at,row.updated_at);}
  turns(id:string){return this.db.prepare('SELECT * FROM turns WHERE conversation_id=? ORDER BY ordinal DESC LIMIT 31').all(id) as unknown as Turn[];}
  active(id:string){return this.db.prepare("SELECT id FROM turns WHERE conversation_id=? AND state IN ('submitting','running','unknown','stopping') LIMIT 1").get(id);}
  turn(id:string,conversationId:string){const row=this.db.prepare('SELECT * FROM turns WHERE id=? AND conversation_id=?').get(id,conversationId) as unknown as Turn|undefined;if(!row)throw new Error('not_found');return row;}
  turnCommand(id:string){return this.db.prepare('SELECT * FROM turns WHERE command_id=?').get(id) as unknown as Turn|undefined;}
  insert(row:Turn){this.db.prepare('INSERT INTO turns VALUES (?,?,?,?,?,?,?,?,?,?,(SELECT COALESCE(MAX(ordinal),0)+1 FROM turns WHERE conversation_id=?))').run(row.id,row.conversation_id,row.command_id,row.fingerprint,row.input,row.output,row.run_id,row.state,row.created_at,row.updated_at,row.conversation_id);this.db.prepare('UPDATE conversations SET updated_at=? WHERE id=?').run(row.updated_at,row.conversation_id);}
  update(id:string,state:string,now:string,runId?:string,output?:string){this.db.prepare('UPDATE turns SET state=?,updated_at=?,run_id=COALESCE(?,run_id),output=COALESCE(?,output) WHERE id=?').run(state,now,runId??null,output??null,id);}
}
