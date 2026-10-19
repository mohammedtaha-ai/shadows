//! Bounded request/response ownership of one Node assistant child.
use serde_json::{Value, json};
use shadows_process::{ChildIn, ChildOut, ProcessHandle, ProcessSpec};
use std::{path::Path, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

pub struct Bridge {
    process: ProcessHandle,
    input: ChildIn,
    output: BufReader<ChildOut>,
    next_id: u64,
    failed: bool,
}

impl Bridge {
    pub async fn start(node: &Path, entry: &Path, config: &Path) -> anyhow::Result<(Self, String)> {
        let mut process = shadows_process::spawn(ProcessSpec {
            executable: node.to_owned(),
            args: vec![
                entry.to_string_lossy().into_owned(),
                "--config".into(),
                config.to_string_lossy().into_owned(),
            ],
            cwd: entry
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid assistant entry"))?
                .to_owned(),
            env: [
                "PATH",
                "HOME",
                "USERPROFILE",
                "LOCALAPPDATA",
                "APPDATA",
                "TEMP",
                "TMP",
            ]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok().map(|value| (name.into(), value)))
            .collect(),
            capture_stdout: true,
            pipe_stdin: true,
        })?;
        let (input, output, mut stderr) = process
            .take_stdio()
            .ok_or_else(|| anyhow::anyhow!("assistant stdio unavailable"))?;
        // Drain without retaining child diagnostics, which can include operator paths.
        tokio::spawn(async move {
            let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
        });
        let mut bridge = Self {
            process,
            input,
            output: BufReader::new(output),
            next_id: 0,
            failed: false,
        };
        let hello = tokio::time::timeout(Duration::from_secs(10), bridge.read_frame()).await??;
        let installation = hello
            .get("result")
            .and_then(|result| result.get("installationId"))
            .and_then(Value::as_str)
            .filter(|value| uuid::Uuid::parse_str(value).is_ok())
            .ok_or_else(|| anyhow::anyhow!("assistant readiness refused"))?
            .to_owned();
        if hello["id"] != 0 || hello["ok"] != true {
            anyhow::bail!("assistant readiness refused");
        }
        Ok((bridge, installation))
    }

    async fn read_frame(&mut self) -> anyhow::Result<Value> {
        let mut frame = Vec::new();
        let count = (&mut self.output)
            .take(131_073)
            .read_until(b'\n', &mut frame)
            .await?;
        if count == 0 || count > 131_072 || frame.last() != Some(&b'\n') {
            anyhow::bail!("assistant frame unavailable");
        }
        Ok(serde_json::from_slice(&frame)?)
    }

    async fn exchange(&mut self, request: Value) -> anyhow::Result<Value> {
        self.next_id += 1;
        let mut line = serde_json::to_vec(&json!({"id":self.next_id,"request":request}))?;
        if line.len() > 65_535 {
            anyhow::bail!("assistant input too large");
        }
        line.push(b'\n');
        self.input.write_all(&line).await?;
        self.input.flush().await?;
        let answer = self.read_frame().await?;
        if answer["id"] != self.next_id {
            anyhow::bail!("assistant response identity mismatch");
        }
        Ok(answer)
    }

    pub async fn call(&mut self, request: Value) -> Result<Value, &'static str> {
        if self.failed || self.process.has_exited() {
            return Err("unavailable");
        }
        match tokio::time::timeout(Duration::from_secs(40), self.exchange(request)).await {
            Ok(Ok(answer)) => Ok(answer),
            _ => {
                self.failed = true;
                let _ = self.process.terminate_tree();
                Err("outcome_unknown")
            }
        }
    }

    pub fn stop(&mut self) {
        self.failed = true;
        let _ = self.process.terminate_tree();
    }
}
