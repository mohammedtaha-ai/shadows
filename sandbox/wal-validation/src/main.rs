//! Throwaway probe for the §6.23 OPEN question in
//! docs/superpowers/specs/2026-09-21-sqlite-schema-design.md.
//!
//! NOT production code. See ./README.md.
//!
//! Q1: does deferred BEGIN + busy_timeout avoid SQLITE_BUSY lock-upgrade
//!     failures under concurrent read-then-write transactions? Does
//!     BEGIN IMMEDIATE eliminate them?
//! Q2: does durable_event.seq (INTEGER PRIMARY KEY AUTOINCREMENT) assignment
//!     order always match commit order / visibility order under contention?

use rand::Rng;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Executor, Row};
use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone, Copy)]
struct ScenarioConfig {
    name: &'static str,
    writers: usize,
    txns_per_writer: usize,
    begin_immediate: bool,
    busy_timeout_ms: u64,
    pool_size: u32,
    synchronous: SyncMode,
    concurrent_readers: usize,
    /// microseconds of simulated "read then think" work inside the txn,
    /// between the read and the writes. This is what widens the window in
    /// which another writer can steal the write lock before this txn's
    /// deferred BEGIN tries to upgrade.
    think_micros_max: u64,
    /// If true, all writers share ONE dedicated write connection (pool
    /// max_connections=1) instead of contending for write locks across
    /// many pooled connections. Readers still use the normal shared pool.
    /// This is the "single write connection" serialization strategy from
    /// Q1, as an alternative to BEGIN IMMEDIATE fan-out.
    single_writer_connection: bool,
}

#[derive(Clone, Copy)]
enum SyncMode {
    Normal,
    Full,
}

impl SyncMode {
    fn as_pragma(self) -> SqliteSynchronous {
        match self {
            SyncMode::Normal => SqliteSynchronous::Normal,
            SyncMode::Full => SqliteSynchronous::Full,
        }
    }
    fn label(self) -> &'static str {
        match self {
            SyncMode::Normal => "NORMAL",
            SyncMode::Full => "FULL",
        }
    }
}

#[derive(Default)]
struct ScenarioOutcome {
    attempted: u64,
    succeeded: u64,
    failed: u64,
    // (primary_code_or_message, count)
    errors: std::collections::BTreeMap<String, u64>,
    elapsed: Duration,
    // Q2 bookkeeping
    assign_order: Vec<(i64, u64)>, // (seq, global assign-order counter value)
    commit_order: Vec<(i64, u64)>, // (seq, global commit-order counter value)
    reader_anomalies: u64,
    reader_polls: u64,
    reader_blocked_errors: u64,
    rolled_back_gaps_observed: u64,
}

async fn run_scenario(cfg: ScenarioConfig, db_path: &str) -> ScenarioOutcome {
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(format!("{db_path}-wal"));
    let _ = std::fs::remove_file(format!("{db_path}-shm"));

    let connect_opts = SqliteConnectOptions::from_str(&format!("sqlite://{db_path}"))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(cfg.synchronous.as_pragma())
        .busy_timeout(Duration::from_millis(cfg.busy_timeout_ms))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(cfg.pool_size)
        .connect_with(connect_opts.clone())
        .await
        .expect("connect");

    // When single_writer_connection is set, writers use a SEPARATE pool
    // capped at 1 connection, so writes are serialized at the application
    // level rather than relying on SQLite lock-upgrade retries. Readers
    // still use the normal `pool` above (same file, WAL readers are
    // independent of the writer connection).
    let write_pool = if cfg.single_writer_connection {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_opts)
            .await
            .expect("connect write_pool")
    } else {
        pool.clone()
    };

    // Schema mirrors the real shape: an entity table plus durable_event,
    // written together in one read-then-write transaction.
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS task_like (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            state TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .await
    .unwrap();
    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS durable_event (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT NOT NULL UNIQUE,
            payload TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .await
    .unwrap();
    // seed one row so the "read current state" step has something to read
    pool.execute("INSERT INTO task_like (state, updated_at) VALUES ('seed', '0');")
        .await
        .unwrap();

    let outcome = Arc::new(Mutex::new(ScenarioOutcome::default()));
    let assign_counter = Arc::new(AtomicU64::new(0));
    let commit_counter = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let start = Instant::now();

    // Concurrent readers: poll durable_event ordered by seq, watch for
    // visibility inversion (a lower seq becoming visible strictly after a
    // higher one was already observed).
    let mut reader_handles = Vec::new();
    for _ in 0..cfg.concurrent_readers {
        let pool = pool.clone();
        let outcome = outcome.clone();
        let stop = stop.clone();
        reader_handles.push(tokio::spawn(async move {
            let mut seen: BTreeSet<i64> = BTreeSet::new();
            let mut max_seen: i64 = -1;
            let mut local_polls = 0u64;
            let mut local_anomalies = 0u64;
            let mut local_blocked = 0u64;
            while !stop.load(Ordering::Relaxed) {
                match sqlx::query("SELECT seq FROM durable_event ORDER BY seq ASC")
                    .fetch_all(&pool)
                    .await
                {
                    Ok(rows) => {
                        local_polls += 1;
                        for r in rows {
                            let seq: i64 = r.get(0);
                            if !seen.contains(&seq) {
                                if seq < max_seen {
                                    local_anomalies += 1;
                                }
                                seen.insert(seq);
                                if seq > max_seen {
                                    max_seen = seq;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        local_blocked += 1;
                    }
                }
                tokio::time::sleep(Duration::from_micros(500)).await;
            }
            let mut o = outcome.lock().await;
            o.reader_polls += local_polls;
            o.reader_anomalies += local_anomalies;
            o.reader_blocked_errors += local_blocked;
        }));
    }

    let mut writer_handles = Vec::new();
    for w in 0..cfg.writers {
        let pool = write_pool.clone();
        let outcome = outcome.clone();
        let assign_counter = assign_counter.clone();
        let commit_counter = commit_counter.clone();
        writer_handles.push(tokio::spawn(async move {
            for i in 0..cfg.txns_per_writer {
                let attempted;
                let mut succeeded = false;
                let mut err_label: Option<String> = None;
                let mut this_seq: Option<i64> = None;
                let mut this_assign_order: Option<u64> = None;

                attempted = true;
                let begin_sql = if cfg.begin_immediate {
                    "BEGIN IMMEDIATE"
                } else {
                    "BEGIN DEFERRED"
                };

                // Acquire the connection OUTSIDE the fallible block so that,
                // on any failure partway through, we can send a best-effort
                // ROLLBACK before the connection goes back to the pool.
                // (A pooled raw connection with an unclosed transaction would
                // otherwise poison the next borrower with "cannot start a
                // transaction within a transaction" -- a harness bug, not a
                // SQLite finding, if left unhandled.)
                let mut conn = pool.acquire().await.expect("acquire connection from pool");
                let mut began = false;

                let result: Result<(), sqlx::Error> = async {
                    conn.execute(begin_sql).await?;
                    began = true;

                    // READ current state (this is the deferred-BEGIN trap:
                    // the read acquires only a SHARED lock).
                    let row = sqlx::query("SELECT state FROM task_like ORDER BY id DESC LIMIT 1")
                        .fetch_one(&mut *conn)
                        .await?;
                    let _current: String = row.get(0);

                    // simulate "think time" between read and write, widening
                    // the window for another writer to grab the write lock
                    // first under DEFERRED mode.
                    if cfg.think_micros_max > 0 {
                        let micros = rand::thread_rng().gen_range(0..cfg.think_micros_max);
                        tokio::time::sleep(Duration::from_micros(micros)).await;
                    }

                    // WRITE: entity row + durable_event row, same txn.
                    sqlx::query(
                        "INSERT INTO task_like (state, updated_at) VALUES (?, ?)",
                    )
                    .bind(format!("w{w}-{i}"))
                    .bind(format!("{w}-{i}"))
                    .execute(&mut *conn)
                    .await?;

                    let seq: i64 = sqlx::query(
                        "INSERT INTO durable_event (event_id, payload, created_at) VALUES (?, ?, ?) RETURNING seq",
                    )
                    .bind(format!("evt-{w}-{i}"))
                    .bind("{}")
                    .bind(format!("{w}-{i}"))
                    .fetch_one(&mut *conn)
                    .await?
                    .get(0);

                    this_seq = Some(seq);
                    this_assign_order = Some(assign_counter.fetch_add(1, Ordering::SeqCst));

                    conn.execute("COMMIT").await?;
                    began = false;
                    Ok(())
                }
                .await;

                match result {
                    Ok(()) => {
                        succeeded = true;
                    }
                    Err(e) => {
                        err_label = Some(classify_sqlite_error(&e));
                        if began {
                            // best-effort: ignore rollback's own error
                            let _ = conn.execute("ROLLBACK").await;
                        }
                    }
                }
                drop(conn);

                let mut o = outcome.lock().await;
                if attempted {
                    o.attempted += 1;
                }
                if succeeded {
                    o.succeeded += 1;
                    if let (Some(seq), Some(order)) = (this_seq, this_assign_order) {
                        o.assign_order.push((seq, order));
                        let c = commit_counter.fetch_add(1, Ordering::SeqCst);
                        o.commit_order.push((seq, c));
                    }
                } else {
                    o.failed += 1;
                    *o.errors.entry(err_label.unwrap_or_else(|| "unknown".into())).or_insert(0) += 1;
                }
            }
        }));
    }

    for h in writer_handles {
        let _ = h.await;
    }
    stop.store(true, Ordering::Relaxed);
    for h in reader_handles {
        let _ = h.await;
    }

    let elapsed = start.elapsed();

    // Check for rollback gaps: with N successful commits assigning seqs,
    // are there numeric gaps in the seq sequence (expected if attempts
    // failed/rolled back after assignment -- shouldn't happen with our
    // pattern since assignment happens right before COMMIT and failures
    // happen before INSERT, but AUTOINCREMENT can still skip on internal
    // retries). Report actual final max(seq) vs count of successful rows.
    let max_seq: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM durable_event")
        .fetch_one(&pool)
        .await
        .unwrap_or(None);
    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let mut o = Arc::try_unwrap(outcome).unwrap_or_else(|_| panic!("outcome still shared")).into_inner();
    o.elapsed = elapsed;
    if let Some(m) = max_seq {
        if m != row_count {
            o.rolled_back_gaps_observed = (m - row_count).max(0) as u64;
        }
    }

    if cfg.single_writer_connection {
        write_pool.close().await;
    }
    pool.close().await;
    o
}

fn classify_sqlite_error(e: &sqlx::Error) -> String {
    if let sqlx::Error::Database(db_err) = e {
        let code = db_err.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into());
        // sqlx's SqliteError Display includes the extended result code text
        // sqlite itself produced (e.g. "database is locked", "database is
        // deadlocked"), which is more informative than the bare code.
        format!("code={} msg={}", code, db_err.message())
    } else {
        format!("non-db-error: {e}")
    }
}

fn print_outcome(cfg: &ScenarioConfig, o: &ScenarioOutcome) {
    println!("=== Scenario: {} ===", cfg.name);
    println!(
        "  writers={} txns_per_writer={} begin_immediate={} busy_timeout_ms={} pool_size={} synchronous={} readers={} think_micros_max={}",
        cfg.writers, cfg.txns_per_writer, cfg.begin_immediate, cfg.busy_timeout_ms, cfg.pool_size,
        cfg.synchronous.label(), cfg.concurrent_readers, cfg.think_micros_max
    );
    println!(
        "  attempted={} succeeded={} failed={} elapsed={:?}",
        o.attempted, o.succeeded, o.failed, o.elapsed
    );
    if !o.errors.is_empty() {
        println!("  errors:");
        for (k, v) in &o.errors {
            println!("    {v:>6}x  {k}");
        }
    }
    println!(
        "  rolled_back/skipped seq gaps observed (max_seq - row_count): {}",
        o.rolled_back_gaps_observed
    );

    // Q2: does assignment order == commit order?
    let mut by_assign = o.assign_order.clone();
    by_assign.sort_by_key(|(_, ord)| *ord);
    let mut by_commit = o.commit_order.clone();
    by_commit.sort_by_key(|(_, ord)| *ord);
    let assign_seq_sequence: Vec<i64> = by_assign.iter().map(|(s, _)| *s).collect();
    let commit_seq_sequence: Vec<i64> = by_commit.iter().map(|(s, _)| *s).collect();
    let assign_is_sorted = assign_seq_sequence.windows(2).all(|w| w[0] < w[1]);
    let commit_is_sorted = commit_seq_sequence.windows(2).all(|w| w[0] < w[1]);
    let assign_matches_commit = assign_seq_sequence == commit_seq_sequence;
    println!(
        "  Q2: assign-order-monotonic-in-seq={} commit-order-monotonic-in-seq={} assign_order==commit_order={}",
        assign_is_sorted, commit_is_sorted, assign_matches_commit
    );
    if cfg.concurrent_readers > 0 {
        println!(
            "  Q2 reader: polls={} visibility_inversions_observed={} reader_errors={}",
            o.reader_polls, o.reader_anomalies, o.reader_blocked_errors
        );
    }
    println!();
}

#[tokio::main]
async fn main() {
    let tmp_dir = std::env::temp_dir().join("wal-validation-probe");
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let scenarios = vec![
        ScenarioConfig {
            name: "A: deferred BEGIN, 8 writers, busy_timeout=5000 (spec default), no think time",
            writers: 8,
            txns_per_writer: 150,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 16,
            synchronous: SyncMode::Normal,
            concurrent_readers: 0,
            think_micros_max: 0,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "B: deferred BEGIN, 8 writers, busy_timeout=5000, WITH think time (read-then-write window)",
            writers: 8,
            txns_per_writer: 150,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 16,
            synchronous: SyncMode::Normal,
            concurrent_readers: 0,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "C: deferred BEGIN, 16 writers, busy_timeout=5000, WITH think time",
            writers: 16,
            txns_per_writer: 100,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 32,
            synchronous: SyncMode::Normal,
            concurrent_readers: 0,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "D: BEGIN IMMEDIATE, 16 writers, busy_timeout=5000, WITH think time",
            writers: 16,
            txns_per_writer: 100,
            begin_immediate: true,
            busy_timeout_ms: 5000,
            pool_size: 32,
            synchronous: SyncMode::Normal,
            concurrent_readers: 0,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "E: BEGIN IMMEDIATE, 16 writers, busy_timeout=0 (no tolerance), WITH think time",
            writers: 16,
            txns_per_writer: 100,
            begin_immediate: true,
            busy_timeout_ms: 0,
            pool_size: 32,
            synchronous: SyncMode::Normal,
            concurrent_readers: 0,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "F: deferred BEGIN, 8 writers, busy_timeout=0 (isolate upgrade failure from timeout tolerance), WITH think time",
            writers: 8,
            txns_per_writer: 150,
            begin_immediate: false,
            busy_timeout_ms: 0,
            pool_size: 16,
            synchronous: SyncMode::Normal,
            concurrent_readers: 0,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "G: BEGIN IMMEDIATE, 8 writers + 4 concurrent readers, busy_timeout=5000, WITH think time (readers not blocked?)",
            writers: 8,
            txns_per_writer: 150,
            begin_immediate: true,
            busy_timeout_ms: 5000,
            pool_size: 16,
            synchronous: SyncMode::Normal,
            concurrent_readers: 4,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "H: deferred BEGIN, 8 writers + 4 concurrent readers, busy_timeout=5000, WITH think time (Q2 visibility-inversion probe)",
            writers: 8,
            txns_per_writer: 150,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 16,
            synchronous: SyncMode::Normal,
            concurrent_readers: 4,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "I: BEGIN IMMEDIATE, 32 writers (high contention), busy_timeout=5000, WITH think time",
            writers: 32,
            txns_per_writer: 60,
            begin_immediate: true,
            busy_timeout_ms: 5000,
            pool_size: 64,
            synchronous: SyncMode::Normal,
            concurrent_readers: 4,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "J: deferred BEGIN, 32 writers (high contention), busy_timeout=5000, WITH think time",
            writers: 32,
            txns_per_writer: 60,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 64,
            synchronous: SyncMode::Normal,
            concurrent_readers: 4,
            think_micros_max: 3000,
            single_writer_connection: false,
        },
        ScenarioConfig {
            name: "K: single write connection, deferred BEGIN, 16 logical writers + 4 concurrent readers, busy_timeout=5000",
            writers: 16,
            txns_per_writer: 100,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 16,
            synchronous: SyncMode::Normal,
            concurrent_readers: 4,
            think_micros_max: 3000,
            single_writer_connection: true,
        },
        ScenarioConfig {
            name: "L: single write connection, deferred BEGIN, 32 logical writers (high contention) + 4 concurrent readers, busy_timeout=5000",
            writers: 32,
            txns_per_writer: 60,
            begin_immediate: false,
            busy_timeout_ms: 5000,
            pool_size: 64,
            synchronous: SyncMode::Normal,
            concurrent_readers: 4,
            think_micros_max: 3000,
            single_writer_connection: true,
        },
    ];

    for (idx, cfg) in scenarios.iter().enumerate() {
        let db_path = tmp_dir.join(format!("scenario_{idx}.sqlite3"));
        let db_path_str = db_path.to_str().unwrap().replace('\\', "/");
        let o = run_scenario(*cfg, &db_path_str).await;
        print_outcome(cfg, &o);
    }

    println!("(sqlite db files left in {:?} for inspection; delete manually)", tmp_dir);

    run_rollback_gap_probe(&tmp_dir).await;
}

/// Direct, non-contention test of the "gaps from rolled-back transactions"
/// sub-question in Q2: insert a durable_event row, note its seq, explicitly
/// ROLLBACK, insert again, and check whether the seq number was reused or
/// skipped.
async fn run_rollback_gap_probe(tmp_dir: &std::path::Path) {
    println!("=== Rollback-gap probe (direct, single connection, no contention) ===");
    let db_path = tmp_dir.join("rollback_gap_probe.sqlite3");
    let db_path_str = db_path.to_str().unwrap().replace('\\', "/");
    let _ = std::fs::remove_file(&db_path_str);
    let _ = std::fs::remove_file(format!("{db_path_str}-wal"));
    let _ = std::fs::remove_file(format!("{db_path_str}-shm"));

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{db_path_str}"))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_millis(5000));
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    pool.execute(
        "CREATE TABLE durable_event (seq INTEGER PRIMARY KEY AUTOINCREMENT, event_id TEXT NOT NULL UNIQUE, payload TEXT NOT NULL, created_at TEXT NOT NULL);",
    )
    .await
    .unwrap();

    let mut conn = pool.acquire().await.unwrap();

    // 1) committed insert -> seq A
    conn.execute("BEGIN IMMEDIATE").await.unwrap();
    let seq_a: i64 = sqlx::query("INSERT INTO durable_event (event_id, payload, created_at) VALUES ('a','{}','t') RETURNING seq")
        .fetch_one(&mut *conn).await.unwrap().get(0);
    conn.execute("COMMIT").await.unwrap();

    // 2) insert then ROLLBACK -> seq B (never committed)
    conn.execute("BEGIN IMMEDIATE").await.unwrap();
    let seq_b: i64 = sqlx::query("INSERT INTO durable_event (event_id, payload, created_at) VALUES ('b','{}','t') RETURNING seq")
        .fetch_one(&mut *conn).await.unwrap().get(0);
    conn.execute("ROLLBACK").await.unwrap();

    // 3) committed insert -> seq C
    conn.execute("BEGIN IMMEDIATE").await.unwrap();
    let seq_c: i64 = sqlx::query("INSERT INTO durable_event (event_id, payload, created_at) VALUES ('c','{}','t') RETURNING seq")
        .fetch_one(&mut *conn).await.unwrap().get(0);
    conn.execute("COMMIT").await.unwrap();

    let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event").fetch_one(&mut *conn).await.unwrap();

    println!("  seq_a (committed) = {seq_a}");
    println!("  seq_b (rolled back, never committed) = {seq_b}");
    println!("  seq_c (committed, after rollback) = {seq_c}");
    println!("  row_count in table = {row_count}");
    println!(
        "  seq_c reused seq_b? {}   (AUTOINCREMENT should NOT reuse -> expect seq_c > seq_b, and seq_b never appears as a row)",
        seq_c == seq_b
    );
    println!(
        "  gap in visible seq numbers left by the rollback: {} (seq_c - seq_a - 1, i.e. how many numbers were consumed and skipped)",
        seq_c - seq_a - 1
    );

    // The pool has max_connections(1) and `conn` still holds it. `pool.close()`
    // waits for every connection to be returned, and `conn` is not returned
    // until it is dropped, so closing before the drop deadlocks. Drop first.
    drop(conn);
    pool.close().await;
}
