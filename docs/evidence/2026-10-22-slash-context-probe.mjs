// Drives claude-agent-acp the way Core does, to test slash commands.
// usage: node acp-probe.mjs <api|sub> <repo> <workdir>
import { spawn } from "node:child_process";
import { readFileSync, mkdirSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";

const [mode, repo, work] = process.argv.slice(2);
const adapter = join(repo, "harness/claude/node_modules/@agentclientprotocol/claude-agent-acp/dist/index.js");
const cwd = join(work, "cwd-" + mode);
mkdirSync(cwd, { recursive: true });

const env = {};
for (const k of ["PATH", "Path", "HOME", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP", "SystemRoot", "SYSTEMROOT", "ComSpec"]) {
  if (process.env[k]) env[k] = process.env[k];
}
env.CLAUDE_CODE_EXECUTABLE = "C:\\Users\\Mohammed\\.local\\bin\\claude.exe";
const MODEL = mode === "api" ? "nvidia/nemotron-3.5-super-vl:free" : "claude-haiku-5-5";
if (mode === "api") {
  // Read the key without printing it: one line at a time.
  let key = "";
  for (const line of readFileSync(join(repo, ".env"), "utf8").split(/\r?\n/)) {
    const i = line.indexOf("=");
    if (i > 0 && line.slice(0, i).trim() === "OPENROUTER") key = line.slice(i + 1).trim().replace(/^["']|["']$/g, "");
  }
  if (!key) throw new Error("OPENROUTER missing");
  const home = join(work, "home-api");
  mkdirSync(home, { recursive: true });
  Object.assign(env, {
    CLAUDE_CONFIG_DIR: home,
    ANTHROPIC_BASE_URL: "https://openrouter.ai/api",
    ANTHROPIC_API_KEY: "",
    ANTHROPIC_AUTH_TOKEN: key,
    ANTHROPIC_MODEL: MODEL,
    ANTHROPIC_DEFAULT_OPUS_MODEL: MODEL,
    ANTHROPIC_DEFAULT_SONNET_MODEL: MODEL,
    ANTHROPIC_DEFAULT_HAIKU_MODEL: MODEL,
    CLAUDE_CODE_MAX_CONTEXT_TOKENS: "1000000",
    CLAUDE_CODE_AUTO_COMPACT_WINDOW: "1000000",
  });
} else {
  env.ANTHROPIC_MODEL = MODEL;
}

const child = spawn(process.execPath, [adapter], { cwd, env, stdio: ["pipe", "pipe", "pipe"] });
let stderr = "";
child.stderr.on("data", (d) => (stderr += d));
let buf = "";
let id = 0;
const pending = new Map();
let sink = null;
child.stdout.on("data", (d) => {
  buf += d;
  let n;
  while ((n = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, n);
    buf = buf.slice(n + 1);
    if (!line.trim()) continue;
    const msg = JSON.parse(line);
    if (msg.id !== undefined && pending.has(msg.id) && msg.method === undefined) {
      pending.get(msg.id)(msg);
      pending.delete(msg.id);
    } else if (msg.method === "session/update") {
      sink?.(msg.params.update);
    } else if (msg.method && msg.id !== undefined) {
      // A request to the client (permission): refuse it.
      child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: msg.id, result: { outcome: { outcome: "cancelled" } } }) + "\n");
    }
  }
});
function call(method, params, ms = 240000) {
  const my = ++id;
  child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: my, method, params }) + "\n");
  return new Promise((res, rej) => {
    const t = setTimeout(() => rej(new Error(method + " timed out")), ms);
    pending.set(my, (m) => { clearTimeout(t); m.error ? rej(new Error(JSON.stringify(m.error))) : res(m.result); });
  });
}

const log = { mode, model: MODEL, adapter: "claude-agent-acp 0.81.1", claude: "2.1.306", started: new Date().toISOString(), sessions: [] };
let current;
async function turn(label, blocks) {
  let text = "";
  const kinds = {};
  const usage = [];
  sink = (u) => {
    kinds[u.sessionUpdate] = (kinds[u.sessionUpdate] || 0) + 1;
    if (u.sessionUpdate === "agent_message_chunk" && u.content?.type === "text") text += u.content.text;
    if (u.sessionUpdate === "usage_update") usage.push({ used: u.used, size: u.size });
  };
  const t0 = Date.now();
  const r = await call("session/prompt", { sessionId: current.id, prompt: blocks.map((t) => ({ type: "text", text: t })) });
  const row = { label, blocks, ms: Date.now() - t0, stop: r.stopReason, updates: kinds, usage, text: text.slice(0, 600) };
  current.turns.push(row);
  console.log(`[${current.name}] ${label}: ${row.ms} ms, usage ${usage.map((u) => u.used + "/" + u.size).join(" ")}, text: ${text.replace(/\s+/g, " ").slice(0, 120)}`);
}
async function session(name, steps) {
  const id = (await call("session/new", { cwd, mcpServers: [] })).sessionId;
  current = { name, id, turns: [] };
  log.sessions.push(current);
  for (const [label, blocks] of steps) await turn(label, blocks);
}
// Per-call usage as Claude Code itself recorded it in the session transcript.
function transcriptUsage(id) {
  const roots = [env.CLAUDE_CONFIG_DIR, join(process.env.USERPROFILE || "", ".claude")].filter(Boolean);
  for (const root of roots) {
    const dir = join(root, "projects");
    let found;
    try {
      for (const d of readdirSync(dir)) { const f = join(dir, d, id + ".jsonl"); if (existsSync(f)) found = f; }
    } catch {}
    if (!found) continue;
    const calls = [];
    const seen = new Set();
    for (const line of readFileSync(found, "utf8").split("\n")) {
      if (!line.trim()) continue;
      const e = JSON.parse(line);
      if (e.type === "assistant" && e.message?.usage && !seen.has(e.message.id)) {
        seen.add(e.message.id);
        const u = e.message.usage;
        calls.push({ at: e.timestamp, input: u.input_tokens, cache_read: u.cache_read_input_tokens || 0, cache_write: u.cache_creation_input_tokens || 0, output: u.output_tokens });
      }
      if (e.type === "system" && e.subtype === "local_command") calls.push({ at: e.timestamp, local_command: true });
    }
    return calls;
  }
  return null;
}

const HELLO = ["hello", ["Reply with exactly: hello"]];
const OK = ["ok", ["Reply with exactly: ok"]];
const READ = ["/context", ["/context"]];
try {
  await call("initialize", { protocolVersion: 1, clientCapabilities: {} });
  console.log("mode", mode, "model", MODEL);
  await session("readings", [HELLO, READ, READ, OK]);
  await session("control", [HELLO, OK]);
  await session("command-with-block", [HELLO, ["/context + block", ["/context", "[Shadows] Stage: idea."]],
    ["what it saw", ["Without using tools: list every user message you have received in this conversation so far, verbatim, one per line."]]]);
  for (const s of log.sessions) s.transcript = transcriptUsage(s.id);
} catch (e) {
  log.error = e.message;
  console.log("ERROR", e.message, "\nstderr tail:", stderr.slice(-3000));
} finally {
  child.kill();
  writeFileSync(join(work, `result-${mode}.json`), JSON.stringify(log, null, 1));
  console.log("wrote", `result-${mode}.json`);
}
