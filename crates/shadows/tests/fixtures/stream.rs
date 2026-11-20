//! Reading a project's event stream frame by frame, as a client does.

use std::time::Duration;

use axum::body::BodyDataStream;
use serde_json::Value;
use tokio_stream::StreamExt;

/// The next `event:`/`data:` frame, waiting up to ten seconds for it.
pub async fn next_frame(body: &mut BodyDataStream, pending: &mut String) -> (String, Value) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(end) = pending.find("\n\n") {
                let frame: String = pending.drain(..end + 2).collect();
                let event = frame.lines().find_map(|l| l.strip_prefix("event: "));
                let data = frame.lines().find_map(|l| l.strip_prefix("data: "));
                if let (Some(event), Some(data)) = (event, data) {
                    return (event.to_owned(), serde_json::from_str(data).unwrap());
                }
            } else {
                let chunk = body.next().await.expect("stream stays open").unwrap();
                pending.push_str(std::str::from_utf8(&chunk).unwrap());
            }
        }
    })
    .await
    .expect("the committed event reaches the stream")
}
