//! One agent hook call: read the event, carry it to AYLUL's sidecar, return
//! the sidecar's answer. Every failure returns `None`, so Claude Code behaves
//! as if AYLUL were absent.
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

pub struct Endpoint {
    pub pipe: String,
    pub token: String,
}

/// Reads `hook.json`. `None` when it is missing or malformed.
pub fn read_endpoint(path: &Path) -> Option<Endpoint> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    Some(Endpoint {
        pipe: value.get("pipe")?.as_str()?.to_owned(),
        token: value.get("token")?.as_str()?.to_owned(),
    })
}

/// How long one event may wait for the sidecar; shorter than Claude Code's
/// own hook timeout for that event.
pub fn deadline(event: &Value) -> Duration {
    match event.get("hook_event_name").and_then(Value::as_str) {
        Some("PermissionRequest") => Duration::from_secs(590),
        Some("PermissionDenied") => Duration::from_secs(125),
        _ => Duration::from_secs(3),
    }
}

/// Windows `ERROR_PIPE_BUSY`: every instance of the pipe is serving another hook.
const ERROR_PIPE_BUSY: i32 = 231;

/// Opens the pipe, retrying while it is busy. Any other error returns at once.
pub fn connect_retrying<S>(
    mut open: impl FnMut() -> io::Result<S>,
    tries: u32,
    pause: Duration,
) -> io::Result<S> {
    let mut left = tries;
    loop {
        match open() {
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) && left > 1 => {
                left -= 1;
                thread::sleep(pause);
            }
            result => return result,
        }
    }
}

/// Sends one request line and returns the reply's `output`, if any.
pub fn exchange<S: Read + Write>(
    mut stream: S,
    token: &str,
    project_dir: Option<&str>,
    event: Value,
) -> Option<String> {
    let mut request = json!({ "v": 1, "token": token, "event": event });
    if let Some(dir) = project_dir {
        request["project_dir"] = Value::from(dir);
    }
    let mut line = request.to_string();
    line.push('\n');
    stream.write_all(line.as_bytes()).ok()?;
    stream.flush().ok()?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply).ok()?;
    let reply: Value = serde_json::from_str(reply.trim()).ok()?;
    match reply.get("output") {
        Some(output) if output.is_object() => Some(output.to_string()),
        _ => None,
    }
}

/// The whole call, cut at `wait`.
pub fn run_within<S, C>(
    input: &str,
    endpoint: Option<Endpoint>,
    project_dir: Option<String>,
    connect: C,
    wait: Duration,
) -> Option<String>
where
    S: Read + Write + Send + 'static,
    C: FnOnce(&str) -> io::Result<S> + Send + 'static,
{
    let event: Value = serde_json::from_str(input).ok()?;
    let endpoint = endpoint?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = connect(&endpoint.pipe)
            .ok()
            .and_then(|stream| exchange(stream, &endpoint.token, project_dir.as_deref(), event));
        let _ = tx.send(result);
    });
    rx.recv_timeout(wait).ok().flatten()
}

/// The whole call with the event's own deadline.
pub fn run<S, C>(
    input: &str,
    endpoint: Option<Endpoint>,
    project_dir: Option<String>,
    connect: C,
) -> Option<String>
where
    S: Read + Write + Send + 'static,
    C: FnOnce(&str) -> io::Result<S> + Send + 'static,
{
    let wait = serde_json::from_str::<Value>(input)
        .map(|event| deadline(&event))
        .unwrap_or(Duration::from_secs(3));
    run_within(input, endpoint, project_dir, connect, wait)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Fake {
        reply: io::Cursor<Vec<u8>>,
        sent: Arc<Mutex<Vec<u8>>>,
    }
    impl Read for Fake {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.reply.read(buf)
        }
    }
    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.sent.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    const EVENT: &str = r#"{"session_id":"s1","hook_event_name":"Stop"}"#;

    fn endpoint() -> Option<Endpoint> {
        Some(Endpoint {
            pipe: r"\\.\pipe\x".into(),
            token: "t0k".into(),
        })
    }

    fn fake(reply: &str) -> (Fake, Arc<Mutex<Vec<u8>>>) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        (
            Fake {
                reply: io::Cursor::new(reply.as_bytes().to_vec()),
                sent: sent.clone(),
            },
            sent,
        )
    }

    #[test]
    fn no_endpoint_gives_no_output() {
        let (stream, _) = fake("{\"output\":{\"a\":1}}\n");
        assert_eq!(run(EVENT, None, None, move |_| Ok(stream)), None);
    }

    #[test]
    fn malformed_input_gives_no_output() {
        let (stream, _) = fake("{\"output\":{\"a\":1}}\n");
        assert_eq!(run("not json", endpoint(), None, move |_| Ok(stream)), None);
    }

    #[test]
    fn connect_error_gives_no_output() {
        let failing =
            |_: &str| -> io::Result<Fake> { Err(io::Error::from(io::ErrorKind::NotFound)) };
        assert_eq!(run(EVENT, endpoint(), None, failing), None);
    }

    #[test]
    fn reply_output_is_returned() {
        let reply = r#"{"output":{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}}"#;
        let (stream, _) = fake(&format!("{reply}\n"));
        let out = run(EVENT, endpoint(), None, move |_| Ok(stream)).expect("output");
        let expected: Value = serde_json::from_str(reply).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&out).unwrap(),
            expected["output"]
        );
    }

    #[test]
    fn empty_reply_gives_no_output() {
        let (stream, _) = fake("{}\n");
        assert_eq!(run(EVENT, endpoint(), None, move |_| Ok(stream)), None);
    }

    #[test]
    fn request_carries_version_token_and_event() {
        let (stream, sent) = fake("{}\n");
        run(EVENT, endpoint(), Some(r"C:\work\proj".into()), move |_| {
            Ok(stream)
        });
        let line = String::from_utf8(sent.lock().unwrap().clone()).unwrap();
        assert!(line.ends_with('\n'));
        let request: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(request["v"], 1);
        assert_eq!(request["token"], "t0k");
        assert_eq!(request["project_dir"], r"C:\work\proj");
        assert_eq!(request["event"]["hook_event_name"], "Stop");
    }

    #[test]
    fn no_project_dir_is_left_out() {
        let (stream, sent) = fake("{}\n");
        run(EVENT, endpoint(), None, move |_| Ok(stream));
        let line = String::from_utf8(sent.lock().unwrap().clone()).unwrap();
        let request: Value = serde_json::from_str(line.trim()).unwrap();
        assert!(request.get("project_dir").is_none());
    }

    #[test]
    fn a_busy_pipe_is_retried_and_other_errors_are_not() {
        let mut busy = 2;
        let opened = connect_retrying(
            || {
                if busy > 0 {
                    busy -= 1;
                    Err(io::Error::from_raw_os_error(ERROR_PIPE_BUSY))
                } else {
                    Ok(())
                }
            },
            50,
            Duration::from_millis(1),
        );
        assert!(opened.is_ok());
        let mut calls = 0;
        let missing = connect_retrying(
            || -> io::Result<()> {
                calls += 1;
                Err(io::Error::from(io::ErrorKind::NotFound))
            },
            50,
            Duration::from_millis(1),
        );
        assert!(missing.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn busy_code_is_231_and_a_forever_busy_pipe_is_tried_exactly_three_times() {
        assert_eq!(ERROR_PIPE_BUSY, 231);
        let mut calls = 0;
        let result: io::Result<()> = connect_retrying(
            || {
                calls += 1;
                Err(io::Error::from_raw_os_error(231))
            },
            3,
            Duration::from_millis(1),
        );
        assert_eq!(result.unwrap_err().raw_os_error(), Some(231));
        assert_eq!(calls, 3);
    }

    #[test]
    fn stalled_sidecar_is_cut_at_the_deadline() {
        let stalled = |_: &str| -> io::Result<Fake> {
            thread::sleep(Duration::from_secs(5));
            Err(io::Error::from(io::ErrorKind::TimedOut))
        };
        let started = std::time::Instant::now();
        assert_eq!(
            run_within(EVENT, endpoint(), None, stalled, Duration::from_millis(100)),
            None
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn deadlines_follow_the_event() {
        let of = |name: &str| deadline(&json!({ "hook_event_name": name }));
        assert_eq!(of("PermissionRequest"), Duration::from_secs(590));
        assert_eq!(of("PermissionDenied"), Duration::from_secs(125));
        assert_eq!(of("PostToolUse"), Duration::from_secs(3));
    }

    #[test]
    fn endpoint_file_is_read_or_ignored() {
        let dir = std::env::temp_dir().join(format!("aylul-hook-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let good = dir.join("good.json");
        std::fs::write(&good, r#"{"pipe":"\\\\.\\pipe\\p","token":"t"}"#).unwrap();
        let bad = dir.join("bad.json");
        std::fs::write(&bad, "{").unwrap();
        let read = read_endpoint(&good).expect("endpoint");
        assert_eq!(read.pipe, r"\\.\pipe\p");
        assert_eq!(read.token, "t");
        assert!(read_endpoint(&bad).is_none());
        assert!(read_endpoint(&dir.join("missing.json")).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
