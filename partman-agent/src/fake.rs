// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A made-up agent, for seeing the panel without a real one: it speaks
//! ACP over a pair of pipes to a thread in the app, so the panel works as
//! it does with any agent. It introduces itself, offers a model and an
//! effort to pick, and answers each message with a thought, a plan, a tool
//! call it asks permission for, then a reply and what it used.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

use crate::acp::Conn;
use crate::panel::{AgentRun, Entry, Setup};
use crate::profiles::AgentProfile;

/// The made-up agent at work in cwd, below `before`, a transcript to start
/// from, and asking what it may do without asking, as a task's agent does.
pub fn start(ctx: &egui::Context, setup: &Setup, cwd: PathBuf, before: Vec<Entry>) -> std::io::Result<AgentRun> {
    let (from_app, to_agent) = std::io::pipe()?;
    let (from_agent, to_app) = std::io::pipe()?;
    std::thread::spawn(move || serve(BufReader::new(from_app), to_app));
    let wake = ctx.clone();
    let (conn, events) = Conn::over(from_agent, to_agent, None, move || wake.request_repaint());
    let profile = AgentProfile { name: "Fake agent".into(), command: "fake".into(), ..Default::default() };
    let mut run = AgentRun::connected(setup, &profile, cwd, conn, events, false, vec![]);
    run.begin(ctx, before);
    Ok(run)
}

/// The agent's settings, as it offers them.
struct Settings {
    model: String,
    effort: String,
}

impl Settings {
    fn options(&self) -> Value {
        let select = |id: &str, name: &str, current: &str, choices: &[&str]| {
            json!({ "id": id, "name": name, "type": "select", "currentValue": current,
                    "options": choices.iter().map(|c| json!({ "value": c.to_lowercase(), "name": c })).collect::<Vec<_>>() })
        };
        json!([select("model", "Model", &self.model, &["Small", "Medium", "Large"]), select("effort", "Effort", &self.effort, &["Low", "Medium", "High"])])
    }
}

/// A turn under way: the prompt's request id, what it was asked, and the
/// permission request it waits on.
struct Turn {
    id: Value,
    text: String,
    asked: u64,
}

/// The plan's update, its first `done` steps done and the next under way.
fn plan(done: usize) -> Value {
    let steps = ["Read the message", "Look at the files", "Answer"];
    let status = |i: usize| if i < done { "completed" } else if i == done { "in_progress" } else { "pending" };
    json!({ "sessionUpdate": "plan", "entries": steps.iter().enumerate().map(|(i, s)| json!({ "content": s, "status": status(i) })).collect::<Vec<_>>() })
}

fn serve(input: impl BufRead, mut out: impl Write) {
    let mut send = |msg: Value| {
        let mut line = serde_json::to_vec(&msg).unwrap_or_default();
        line.push(b'\n');
        out.write_all(&line).and_then(|_| out.flush()).is_ok()
    };
    let update = |send: &mut dyn FnMut(Value) -> bool, u: Value| send(json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "fake", "update": u } }));
    // Shown as it comes, as a real agent's is
    let pause = || std::thread::sleep(Duration::from_millis(250));
    let mut settings = Settings { model: "small".into(), effort: "low".into() };
    let mut turn: Option<Turn> = None;
    let mut used = 3_000u64;
    let mut next = 1_000u64;
    for line in input.lines() {
        let Ok(line) = line else { return };
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let ok = |id: Value, result: Value| json!({ "jsonrpc": "2.0", "id": id, "result": result });
        match (msg.get("method").and_then(Value::as_str), id) {
            (Some("initialize"), Some(id)) => {
                send(ok(id, json!({ "protocolVersion": crate::acp::PROTOCOL_VERSION, "agentInfo": { "name": "fake", "title": "Fake agent", "version": "1.0" }, "agentCapabilities": { "loadSession": false } })));
            }
            (Some("session/new"), Some(id)) => {
                send(ok(id, json!({ "sessionId": "fake", "configOptions": settings.options() })));
            }
            (Some("session/set_config_option"), Some(id)) => {
                let value = params["value"].as_str().unwrap_or("").to_string();
                match params["configId"].as_str() {
                    Some("model") => settings.model = value,
                    Some("effort") => settings.effort = value,
                    _ => {}
                }
                send(ok(id, json!({ "configOptions": settings.options() })));
            }
            (Some("session/prompt"), Some(id)) => {
                let text = params["prompt"][0]["text"].as_str().unwrap_or("").to_string();
                update(&mut send, json!({ "sessionUpdate": "agent_thought_chunk", "content": { "type": "text", "text": format!("Asked: {text}. Nothing to look up, but the steps are shown anyway.") } }));
                pause();
                update(&mut send, plan(1));
                pause();
                next += 1;
                let call = format!("call-{next}");
                update(&mut send, json!({ "sessionUpdate": "tool_call", "toolCallId": call, "title": "ls -la", "kind": "execute", "status": "pending", "rawInput": { "command": "ls -la" } }));
                // A command: asked about unless shell commands are allowed
                send(json!({ "jsonrpc": "2.0", "id": next, "method": "session/request_permission", "params": {
                    "sessionId": "fake",
                    "toolCall": { "toolCallId": call, "title": "ls -la", "kind": "execute" },
                    "options": [{ "optionId": "allow", "name": "Allow", "kind": "allow_once" }, { "optionId": "deny", "name": "Deny", "kind": "reject_once" }],
                } }));
                turn = Some(Turn { id, text, asked: next });
            }
            (Some("session/cancel"), None) => {
                if let Some(t) = turn.take() {
                    send(ok(t.id, json!({ "stopReason": "cancelled" })));
                }
            }
            // The user's answer to the permission asked
            (None, Some(id)) if turn.as_ref().is_some_and(|t| id.as_u64() == Some(t.asked)) => {
                let Some(t) = turn.take() else { continue };
                let allowed = msg["result"]["outcome"]["optionId"].as_str() == Some("allow");
                let call = format!("call-{}", t.asked);
                if allowed {
                    update(&mut send, json!({ "sessionUpdate": "tool_call_update", "toolCallId": call, "status": "in_progress" }));
                    pause();
                    update(&mut send, json!({ "sessionUpdate": "tool_call_update", "toolCallId": call, "status": "completed",
                        "content": [{ "type": "content", "content": { "type": "text", "text": "total 8\ndrwxr-xr-x 2 me me 4096 .\ndrwxr-xr-x 9 me me 4096 .." } }] }));
                } else {
                    update(&mut send, json!({ "sessionUpdate": "tool_call_update", "toolCallId": call, "status": "failed" }));
                }
                pause();
                update(&mut send, plan(3));
                let looked = if allowed { "The folder is empty" } else { "I wasn't let look in the folder" };
                for chunk in [format!("A made-up answer to *{}*: ", t.text.trim()), format!("{looked}, and there's no agent behind this panel. "), format!("Model `{}`, effort `{}`.", settings.model, settings.effort)] {
                    update(&mut send, json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": chunk } }));
                    pause();
                }
                used += 2_400;
                update(&mut send, json!({ "sessionUpdate": "usage_update", "used": used, "size": 200_000, "cost": { "amount": used as f64 * 0.000_002, "currency": "USD" } }));
                send(ok(t.id, json!({ "stopReason": "end_turn", "usage": { "inputTokens": used, "outputTokens": 120 } })));
            }
            (Some(m), Some(id)) => {
                send(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("{m} isn't something the fake agent does") } }));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    struct S {
        run: AgentRun,
        height: Option<f32>,
    }

    fn wait(h: &mut egui_kittest::Harness<'_, S>, what: &str, done: impl Fn(&AgentRun) -> bool) {
        let t = std::time::Instant::now();
        while !done(&h.state().run) {
            assert!(t.elapsed() < Duration::from_secs(10), "never: {what}");
            std::thread::sleep(Duration::from_millis(20));
            h.step();
        }
        h.run_steps(2);
    }

    /// The made-up agent through the panel, as a real one would go: it
    /// starts below the transcript given, the grant is asked, a message is
    /// sent, the command it asks to run allowed, and its answer comes.
    #[test]
    fn talks_through_the_panel() {
        let setup = Setup { client: crate::acp::Client { name: "test".into(), title: "the test".into(), version: "0".into() }, app: "the test".into(), tools: vec![] };
        let ctx = egui::Context::default();
        let run = start(&ctx, &setup, std::env::temp_dir(), vec![Entry::User("Before".into()), Entry::Agent("Said before".into())]).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 800.0)).build_ui_state(
            |ui, s: &mut S| {
                let colors = crate::Colors::from_visuals(ui.visuals());
                s.run.ui(ui, &colors, &mut s.height);
            },
            S { run, height: None },
        );
        wait(&mut h, "the session", |r| r.session.is_some());
        assert!(matches!(&h.state().run.entries[0], Entry::User(t) if t == "Before"), "below what was said");
        assert!(h.query_by_label("Fake agent 1.0 is ready.").is_some());
        h.get_by_label("Ask each time").click();
        h.run_steps(2);
        h.get_by_label("Message the agent").focus();
        h.run_steps(2);
        h.get_by_label("Message the agent").type_text("What's here?");
        h.run_steps(2);
        h.key_press(egui::Key::Enter);
        wait(&mut h, "the question", |r| r.asking().is_some());
        assert_eq!(h.state().run.asking(), Some("ls -la"));
        h.get_by_label("Allow").click();
        wait(&mut h, "the answer", |r| !r.busy());
        let said: String = h.state().run.entries.iter().filter_map(|e| if let Entry::Agent(t) = e { Some(t.as_str()) } else { None }).collect();
        assert!(said.contains("A made-up answer to *What's here?*"), "{said}");
        assert_eq!(h.state().run.last_stop.as_deref(), Some("end_turn"));
        assert!(h.state().run.context.is_some(), "its usage");
    }
}
