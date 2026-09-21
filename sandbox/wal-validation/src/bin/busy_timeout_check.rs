// Diagnostic: confirm PRAGMA busy_timeout is actually applied and actually
// makes a blocked writer wait, rather than failing instantly.
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Executor, Row};
use std::str::FromStr;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    let path = std::env::temp_dir().join("busy_timeout_check.sqlite3");
    let path_str = path.to_str().unwrap().replace('\\', "/");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{path_str}-wal"));
    let _ = std::fs::remove_file(format!("{path_str}-shm"));

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path_str}"))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_millis(5000));

    let pool = SqlitePoolOptions::new().max_connections(4).connect_with(opts).await.unwrap();
    pool.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)").await.unwrap();

    // confirm the pragma value on a fresh connection
    let mut c = pool.acquire().await.unwrap();
    let row = sqlx::query("PRAGMA busy_timeout").fetch_one(&mut *c).await.unwrap();
    let bt: i64 = row.get(0);
    println!("PRAGMA busy_timeout reports: {bt}");
    drop(c);

    // Connection A: BEGIN IMMEDIATE, hold the write lock for 2s, then commit.
    let pool_a = pool.clone();
    let holder = tokio::spawn(async move {
        let mut conn = pool_a.acquire().await.unwrap();
        let t0 = Instant::now();
        conn.execute("BEGIN IMMEDIATE").await.unwrap();
        conn.execute("INSERT INTO t (v) VALUES ('a')").await.unwrap();
        println!("[holder] got write lock at {:?}, sleeping 2s", t0.elapsed());
        tokio::time::sleep(Duration::from_secs(2)).await;
        conn.execute("COMMIT").await.unwrap();
        println!("[holder] committed at {:?}", t0.elapsed());
    });

    // give the holder a head start to actually acquire the lock first
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connection B: try BEGIN IMMEDIATE while A holds the lock. Time how
    // long it blocks before either succeeding or failing.
    let pool_b = pool.clone();
    let waiter = tokio::spawn(async move {
        let mut conn = pool_b.acquire().await.unwrap();
        let t0 = Instant::now();
        let result = conn.execute("BEGIN IMMEDIATE").await;
        println!("[waiter] BEGIN IMMEDIATE result={:?} after {:?}", result.is_ok(), t0.elapsed());
        if result.is_ok() {
            let _ = conn.execute("ROLLBACK").await;
        }
    });

    let _ = holder.await;
    let _ = waiter.await;

    // Also check the DEFERRED upgrade case explicitly.
    let pool_c = pool.clone();
    let holder2 = tokio::spawn(async move {
        let mut conn = pool_c.acquire().await.unwrap();
        let t0 = Instant::now();
        conn.execute("BEGIN IMMEDIATE").await.unwrap();
        conn.execute("INSERT INTO t (v) VALUES ('c')").await.unwrap();
        println!("[holder2] got write lock at {:?}, sleeping 2s", t0.elapsed());
        tokio::time::sleep(Duration::from_secs(2)).await;
        conn.execute("COMMIT").await.unwrap();
        println!("[holder2] committed at {:?}", t0.elapsed());
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pool_d = pool.clone();
    let waiter2 = tokio::spawn(async move {
        let mut conn = pool_d.acquire().await.unwrap();
        let t0 = Instant::now();
        conn.execute("BEGIN DEFERRED").await.unwrap();
        // read first (this is the deferred trap: only a SHARED lock so far)
        let _row = sqlx::query("SELECT COUNT(*) FROM t").fetch_one(&mut *conn).await.unwrap();
        println!("[waiter2] read done at {:?}, now attempting write (upgrade)", t0.elapsed());
        let result = conn.execute("INSERT INTO t (v) VALUES ('d')").await;
        println!("[waiter2] upgrade-write result={:?} after {:?}", result.is_ok(), t0.elapsed());
        if let Err(e) = &result {
            if let sqlx::Error::Database(db) = e {
                println!("[waiter2] error code={:?} msg={}", db.code(), db.message());
            }
        }
        if result.is_ok() {
            let _ = conn.execute("COMMIT").await;
        } else {
            let _ = conn.execute("ROLLBACK").await;
        }
    });
    let _ = holder2.await;
    let _ = waiter2.await;
}
