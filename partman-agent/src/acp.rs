// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A client of the Agent Client Protocol (agentclientprotocol.com): the
//! agent runs as a child process, and JSON-RPC 2.0 messages go one per
//! line over its stdin and stdout; or, for one made up (`crate::fake`),
//! over a pair of pipes to a thread. Messages are kept as JSON values rather
//! than typed: agents add fields and update kinds faster than the schema
//! settles, and what isn't understood is shown rather than refused.
//!
//! A reader thread routes the agent's answers to the calls waiting for
//! them, and everything else (session updates, its requests of us) to an
//! event channel the UI drains.

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::profiles::AgentProfile;

pub const PROTOCOL_VERSION: u64 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// ACP's code for a session the agent won't start until signed in.
pub const AUTH_REQUIRED: i64 = -32000;
/// The connection itself failed: the agent quit, or wrote nonsense.
const BROKEN: i64 = -1;

#[derive(Debug)]
pub enum Event {
    /// session/update's `update`
    Update(Value),
    /// session/request_permission: answer with `Conn::respond`
    Permission { id: Value, params: Value },
    /// The agent's process ended: how, and the end of what it wrote to
    /// stderr
    Exited(String),
}

type Waiting = Arc<Mutex<HashMap<u64, Sender<Result<Value, RpcError>>>>>;

/// What the client says of itself to the agent: the app's name for
/// programs, its title for people, and its version.
#[derive(Clone, Debug)]
pub struct Client {
    pub name: String,
    pub title: String,
    pub version: String,
}

pub struct Conn {
    /// Where messages to the agent go: its stdin, or a pipe
    input: Mutex<Option<Box<dyn Write + Send>>>,
    /// Its process, when it's one
    child: Mutex<Option<Child>>,
    waiting: Waiting,
    next: AtomicU64,
    stderr: Arc<Mutex<VecDeque<String>>>,
}

const STDERR_LINES: usize = 200;

impl Conn {
    /// Starts the agent in cwd; wake is called whenever an event arrives.
    pub fn spawn(profile: &AgentProfile, cwd: &std::path::Path, wake: impl Fn() + Send + Sync + 'static) -> Result<(Arc<Conn>, Receiver<Event>), String> {
        let program = profile.program().ok_or_else(|| format!("{}: \"{}\" isn't found; check the agent's command under Agents", profile.name, profile.command))?;
        let mut cmd = Command::new(&program);
        cmd.args(&profile.args).envs(&profile.env).current_dir(cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| format!("starting {}: {e}", program.display()))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let stderr = child.stderr.take().ok_or("no stderr")?;
        let conn = Self::over(stdout, stdin, Some(child), wake);
        // stderr is read all along: a full pipe would stall the agent
        let tail = conn.0.stderr.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
                if t.len() == STDERR_LINES {
                    t.pop_front();
                }
                t.push_back(line);
            }
        });
        Ok(conn)
    }

    /// Talks to an agent over what it writes and what it reads, its
    /// process if it's one; wake is called whenever an event arrives.
    pub fn over(output: impl Read + Send + 'static, input: impl Write + Send + 'static, child: Option<Child>, wake: impl Fn() + Send + Sync + 'static) -> (Arc<Conn>, Receiver<Event>) {
        let conn = Arc::new(Conn { input: Mutex::new(Some(Box::new(input))), child: Mutex::new(child), waiting: Arc::default(), next: AtomicU64::new(1), stderr: Arc::default() });
        let (tx, rx) = mpsc::channel();
        let reader = Arc::downgrade(&conn);
        let waiting = conn.waiting.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                let Some(conn) = reader.upgrade() else { return };
                conn.route(msg, &tx);
                drop(conn);
                wake();
            }
            // Calls still waiting would wait forever
            for (_, w) in waiting.lock().unwrap_or_else(|e| e.into_inner()).drain() {
                let _ = w.send(Err(RpcError { code: BROKEN, message: "the agent stopped".into() }));
            }
            let how = match reader.upgrade() {
                Some(c) => c.exit_text(),
                None => String::new(),
            };
            let _ = tx.send(Event::Exited(how));
            wake();
        });
        (conn, rx)
    }

    fn route(&self, msg: Value, tx: &Sender<Event>) {
        let method = msg.get("method").and_then(Value::as_str);
        let id = msg.get("id").cloned();
        match (method, id) {
            // An answer to one of our calls
            (None, Some(id)) => {
                let Some(n) = id.as_u64() else { return };
                let Some(w) = self.waiting.lock().unwrap_or_else(|e| e.into_inner()).remove(&n) else { return };
                let r = match msg.get("error") {
                    Some(e) => Err(RpcError {
                        code: e.get("code").and_then(Value::as_i64).unwrap_or(0),
                        message: e.get("message").and_then(Value::as_str).unwrap_or("the agent refused").to_string(),
                    }),
                    None => Ok(msg.get("result").cloned().unwrap_or(Value::Null)),
                };
                let _ = w.send(r);
            }
            (Some(m), id) => {
                let params = msg.get("params").cloned().unwrap_or(Value::Null);
                match (m, id) {
                    ("session/update", None) => {
                        let _ = tx.send(Event::Update(params.get("update").cloned().unwrap_or(Value::Null)));
                    }
                    ("session/request_permission", Some(id)) => {
                        let _ = tx.send(Event::Permission { id, params });
                    }
                    // Asked of us but not offered (files, terminals): the
                    // agent must hear back, or it waits forever
                    (m, Some(id)) => self.reply(id, Err(RpcError { code: -32601, message: format!("{m} isn't supported by this client") })),
                    // Extensions (an agent's account status and the like)
                    (_, None) => {}
                }
            }
            (None, None) => {}
        }
    }

    fn send(&self, msg: &Value) -> Result<(), RpcError> {
        let broken = |e: String| RpcError { code: BROKEN, message: e };
        let mut input = self.input.lock().unwrap_or_else(|e| e.into_inner());
        let w = input.as_mut().ok_or_else(|| broken("the agent was stopped".into()))?;
        let mut line = serde_json::to_vec(msg).map_err(|e| broken(e.to_string()))?;
        line.push(b'\n');
        w.write_all(&line).and_then(|_| w.flush()).map_err(|e| broken(format!("the agent stopped listening: {e}")))
    }

    /// Calls a method and waits for the answer, which can take as long as
    /// the agent works (a prompt lasts its whole turn).
    pub fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        self.waiting.lock().unwrap_or_else(|e| e.into_inner()).insert(id, tx);
        if let Err(e) = self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })) {
            self.waiting.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
            return Err(e);
        }
        rx.recv().unwrap_or_else(|_| Err(RpcError { code: BROKEN, message: "the agent stopped".into() }))
    }

    pub fn notify(&self, method: &str, params: Value) {
        let _ = self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// Answers a request the agent made of us.
    pub fn reply(&self, id: Value, result: Result<Value, RpcError>) {
        let msg = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": e.code, "message": e.message } }),
        };
        let _ = self.send(&msg);
    }

    /// The end of what the agent wrote to stderr, for when it fails.
    pub fn stderr_tail(&self, lines: usize) -> String {
        let t = self.stderr.lock().unwrap_or_else(|e| e.into_inner());
        t.iter().skip(t.len().saturating_sub(lines)).cloned().collect::<Vec<_>>().join("\n")
    }

    /// Whether the process has ended, and how; None while it runs, or
    /// for an agent that isn't a process.
    fn ended(&self) -> Option<std::process::ExitStatus> {
        self.child.lock().unwrap_or_else(|e| e.into_inner()).as_mut().and_then(|c| c.try_wait().ok().flatten())
    }

    fn exit_text(&self) -> String {
        if self.child.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
            return "The agent finished.".to_string();
        }
        // The reader saw EOF; give the process a moment to be reaped
        let mut status = None;
        for _ in 0..20 {
            if let Some(s) = self.ended() {
                status = Some(s);
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let mut how = match status {
            Some(s) if s.success() => "The agent finished.".to_string(),
            Some(s) => format!("The agent stopped ({s})."),
            None => "The agent closed its output.".to_string(),
        };
        let tail = self.stderr_tail(12);
        if !tail.is_empty() && status.is_none_or(|s| !s.success()) {
            how = format!("{how}\n{tail}");
        }
        how
    }

    /// Asks the agent to quit by closing its input, as agents do on EOF,
    /// and kills it if it hasn't shortly after.
    pub fn stop(&self) {
        self.input.lock().unwrap_or_else(|e| e.into_inner()).take();
        if self.child.lock().unwrap_or_else(|e| e.into_inner()).is_none() {
            return;
        }
        for _ in 0..20 {
            if self.ended().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if let Some(c) = self.child.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        self.stop();
    }
}

/// What the agent said of itself on initialize.
#[derive(Clone, Debug, Default)]
pub struct AgentInfo {
    pub title: String,
    pub version: String,
    /// (id, name, description) of each way to sign in
    pub auth_methods: Vec<(String, String, String)>,
    /// It can take up a session started before (`session/load`)
    pub load_session: bool,
}

pub fn initialize(conn: &Conn, client: &Client) -> Result<AgentInfo, RpcError> {
    let r = conn.call(
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            // Files and terminals are the agent's own business here: it
            // works in a folder of its own
            "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false }, "terminal": false },
            "clientInfo": { "name": client.name, "title": client.title, "version": client.version },
        }),
    )?;
    let version = r.get("protocolVersion").and_then(Value::as_u64).unwrap_or(0);
    if version != PROTOCOL_VERSION {
        return Err(RpcError { code: BROKEN, message: format!("the agent speaks ACP version {version}, and {} {PROTOCOL_VERSION}", client.title) });
    }
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let info = r.get("agentInfo").cloned().unwrap_or(Value::Null);
    let auth_methods = r.get("authMethods").and_then(Value::as_array).map(|a| a.iter().map(|m| (s(m, "id"), s(m, "name"), s(m, "description"))).collect()).unwrap_or_default();
    let load_session = r.pointer("/agentCapabilities/loadSession").and_then(Value::as_bool).unwrap_or(false);
    Ok(AgentInfo { title: if s(&info, "title").is_empty() { s(&info, "name") } else { s(&info, "title") }, version: s(&info, "version"), auth_methods, load_session })
}

/// An MCP server for the agent to start, over stdio.
#[derive(Clone, Debug)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// A session started: its id, and its settings (configOptions).
pub struct Started {
    pub id: String,
    pub config: Vec<ConfigOption>,
}

fn servers(mcp: &[McpServer]) -> Vec<Value> {
    mcp.iter()
        .map(|m| json!({ "name": m.name, "command": m.command, "args": m.args, "env": m.env.iter().map(|(k, v)| json!({ "name": k, "value": v })).collect::<Vec<_>>() }))
        .collect()
}

pub fn new_session(conn: &Conn, cwd: &std::path::Path, mcp: &[McpServer]) -> Result<Started, RpcError> {
    let r = conn.call("session/new", json!({ "cwd": cwd, "mcpServers": servers(mcp) }))?;
    let id = r.get("sessionId").and_then(Value::as_str).ok_or(RpcError { code: BROKEN, message: "the agent started no session".into() })?.to_string();
    Ok(Started { id, config: config_options(&r) })
}

/// Takes up a session from before, for an agent that can. The agent
/// replays the session as updates before it answers.
pub fn load_session(conn: &Conn, id: &str, cwd: &std::path::Path, mcp: &[McpServer]) -> Result<Started, RpcError> {
    let r = conn.call("session/load", json!({ "sessionId": id, "cwd": cwd, "mcpServers": servers(mcp) }))?;
    Ok(Started { id: id.to_string(), config: config_options(&r) })
}

pub fn authenticate(conn: &Conn, method: &str) -> Result<(), RpcError> {
    conn.call("authenticate", json!({ "methodId": method })).map(|_| ())
}

/// Sends the user's words and waits out the agent's turn; why it ended
/// ("end_turn", "cancelled", ...).
/// Tokens a turn took in and gave out, as the agent says at its end.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
}

/// How a turn ended, and what it used if the agent says.
#[derive(Clone, Debug, PartialEq)]
pub struct Ended {
    pub stop: String,
    pub usage: Option<Usage>,
}

pub fn prompt(conn: &Conn, session: &str, text: &str) -> Result<Ended, RpcError> {
    let r = conn.call("session/prompt", json!({ "sessionId": session, "prompt": [{ "type": "text", "text": text }] }))?;
    let stop = r.get("stopReason").and_then(Value::as_str).unwrap_or("end_turn").to_string();
    let n = |k: &str| r["usage"].get(k).and_then(Value::as_u64);
    let usage = n("inputTokens").or(n("outputTokens")).map(|_| Usage { input: n("inputTokens").unwrap_or(0), output: n("outputTokens").unwrap_or(0) });
    Ok(Ended { stop, usage })
}

/// The session's context as the agent last said (a usage_update): tokens
/// in it, its size, and what the session has cost so far, if it says.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub used: u64,
    pub size: u64,
    pub cost: Option<(f64, String)>,
}

pub fn context(u: &Value) -> Option<Context> {
    let cost = u["cost"]["amount"].as_f64().map(|a| (a, u["cost"]["currency"].as_str().unwrap_or("USD").to_string()));
    Some(Context { used: u["used"].as_u64()?, size: u["size"].as_u64()?, cost })
}

/// Stops the turn under way; its prompt call then ends "cancelled".
pub fn cancel(conn: &Conn, session: &str) {
    conn.notify("session/cancel", json!({ "sessionId": session }));
}

pub fn set_config(conn: &Conn, session: &str, id: &str, value: &str) -> Result<Vec<ConfigOption>, RpcError> {
    let r = conn.call("session/set_config_option", json!({ "sessionId": session, "configId": id, "value": value }))?;
    Ok(config_options(&r))
}

/// A session setting the agent offers: model, mode, effort and the like.
/// Only choices from a list are kept; on/off ones are rare and left to
/// the agent's defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigOption {
    pub id: String,
    pub name: String,
    pub category: String,
    pub current: String,
    /// (value, name)
    pub choices: Vec<(String, String)>,
}

pub fn config_options(v: &Value) -> Vec<ConfigOption> {
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let Some(list) = v.get("configOptions").and_then(Value::as_array) else { return vec![] };
    list.iter()
        .filter(|o| o.get("type").and_then(Value::as_str) == Some("select"))
        .map(|o| {
            let mut choices = vec![];
            for c in o.get("options").and_then(Value::as_array).into_iter().flatten() {
                // Grouped choices hold their own lists
                match c.get("options").and_then(Value::as_array) {
                    Some(inner) => choices.extend(inner.iter().map(|c| (s(c, "value"), s(c, "name")))),
                    None => choices.push((s(c, "value"), s(c, "name"))),
                }
            }
            ConfigOption { id: s(o, "id"), name: s(o, "name"), category: s(o, "category"), current: s(o, "currentValue"), choices }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_config_options() {
        let v = json!({ "configOptions": [
            { "id": "model", "name": "Model", "category": "model", "type": "select", "currentValue": "b",
              "options": [{ "value": "a", "name": "A" }, { "value": "b", "name": "B" }] },
            { "id": "think", "name": "Think", "type": "boolean", "currentValue": true },
            { "id": "g", "name": "Grouped", "type": "select", "currentValue": "x",
              "options": [{ "group": "1", "name": "One", "options": [{ "value": "x", "name": "X" }] }] },
        ] });
        let c = config_options(&v);
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].current.as_str(), c[0].choices.len()), ("b", 2));
        assert_eq!(c[1].choices, [("x".to_string(), "X".to_string())]);
    }
}

/// Against a real agent, asked for a word: PARTMAN_AGENT names its command
/// line, such as "npx -y @agentclientprotocol/claude-agent-acp". It spends
/// a little of the agent's account, so it's run by hand:
/// `cargo test -p partman-agent real_agent -- --ignored`.
#[cfg(test)]
mod real {
    use super::*;

    #[test]
    #[ignore]
    fn real_agent() {
        let line = std::env::var("PARTMAN_AGENT").expect("set PARTMAN_AGENT to the agent's command line");
        let mut words = crate::profiles::split_words(&line);
        let profile = AgentProfile { name: "real".into(), command: words.remove(0), args: words, ..Default::default() };
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/real-agent");
        std::fs::create_dir_all(&dir).unwrap();
        let (conn, events) = Conn::spawn(&profile, &dir, || {}).unwrap();
        let client = Client { name: "partman-agent".into(), title: "partman-agent's test".into(), version: "0".into() };
        let info = initialize(&conn, &client).unwrap();
        eprintln!("agent: {} {}", info.title, info.version);
        let s = new_session(&conn, &dir, &[]).unwrap();
        eprintln!("settings: {:?}", s.config.iter().map(|c| (&c.name, &c.current)).collect::<Vec<_>>());
        let turn = {
            let conn = conn.clone();
            let id = s.id.clone();
            std::thread::spawn(move || prompt(&conn, &id, "Reply with only the word ready, and nothing else."))
        };
        let mut said = String::new();
        while !turn.is_finished() {
            if let Ok(Event::Update(u)) = events.recv_timeout(Duration::from_millis(100))
                && u["sessionUpdate"] == "agent_message_chunk"
            {
                said += u["content"]["text"].as_str().unwrap_or("");
            }
        }
        let stop = turn.join().unwrap().unwrap();
        while let Ok(Event::Update(u)) = events.try_recv() {
            if u["sessionUpdate"] == "agent_message_chunk" {
                said += u["content"]["text"].as_str().unwrap_or("");
            }
        }
        eprintln!("stop: {}; usage: {:?}; said: {said}", stop.stop, stop.usage);
        assert_eq!(stop.stop, "end_turn");
        assert!(said.to_lowercase().contains("ready"), "{said}");
        conn.stop();
    }
}
