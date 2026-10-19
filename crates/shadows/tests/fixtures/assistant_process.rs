//! Test-only stdio peer; ordinary product builds do not include this target.
use serde_json::{Value, json};
use std::io::{self, BufRead};
fn main() {
    println!(
        "{}",
        json!({"id":0,"ok":true,"result":{"installationId":"00000000-0000-4000-8000-000000000001"}})
    );
    for line in io::stdin().lock().lines() {
        let frame: Value =
            serde_json::from_str(&line.expect("fixture input")).expect("fixture JSON");
        if frame["request"]["action"] == "crash" {
            std::process::exit(1);
        }
        println!(
            "{}",
            json!({"id":frame["id"],"ok":true,"result":{"state":"ready"}})
        );
    }
}
