// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Model servers an agent is given: any speaking OpenAI's API (Ollama,
//! LM Studio, llama.cpp, vLLM, or a service such as OpenRouter). Their
//! models are listed afresh each time the agent starts, so what's on a
//! server is what the agent offers, with no agent config written by hand.

use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::profiles::{AgentProfile, ModelServer};

/// A model a server offers, with what the server says of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub id: String,
    /// None when the server doesn't say
    pub tools: Option<bool>,
    pub reasoning: bool,
    pub context: Option<u64>,
}

/// The server's OpenAI API root: http:// unless another scheme is given,
/// and /v1 under a bare host and port, where Ollama, LM Studio and
/// llama.cpp have it.
pub fn base_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    let u = if u.contains("://") { u.to_string() } else { format!("http://{u}") };
    let rest = &u[u.find("://").map_or(0, |i| i + 3)..];
    if rest.contains('/') { u } else { format!("{u}/v1") }
}

/// A short name for the server, its host's first label ("barley"), that
/// its models are told apart by in the agent.
pub fn name(url: &str) -> String {
    let b = base_url(url);
    let host = b[b.find("://").map_or(0, |i| i + 3)..].split(['/', ':']).next().unwrap_or_default();
    let label: String = host.split('.').next().unwrap_or_default().chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    if label.is_empty() { "server".into() } else { label.to_lowercase() }
}

fn agent() -> ureq::Agent {
    // Short: the agent waits on this to start, and a server that's off
    // shouldn't hold it up for long
    ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(2))).timeout_global(Some(Duration::from_secs(5))).build().into()
}

/// The models the server offers. Ollama and LM Studio say more of each
/// in their own APIs; models only for images or embeddings are left out,
/// being no use to an agent.
pub fn models(s: &ModelServer) -> Result<Vec<Model>, String> {
    let base = base_url(&s.url);
    let a = agent();
    let mut req = a.get(format!("{base}/models"));
    if !s.key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", s.key));
    }
    let v: Value = req.call().map_err(|e| e.to_string())?.body_mut().read_json().map_err(|e| format!("not a model server's answer: {e}"))?;
    let list = v["data"].as_array().ok_or("not a model server's answer: no list of models")?;
    let root = base.strip_suffix("/v1").unwrap_or(&base);
    // Asked only of a server that's Ollama, as it takes a request a model
    // and a service may have hundreds
    let ollama = a.get(format!("{root}/api/version")).call().ok().and_then(|mut r| r.body_mut().read_json::<Value>().ok()).is_some_and(|v| v["version"].is_string());
    let studio = if ollama { None } else { lm_studio(&a, root) };
    // The context Ollama runs a loaded model with, which is its server's
    // setting and often far less than the model was trained for
    let loaded: Vec<Value> = if ollama { a.get(format!("{root}/api/ps")).call().ok().and_then(|mut r| r.body_mut().read_json::<Value>().ok()).and_then(|v| v["models"].as_array().cloned()).unwrap_or_default() } else { vec![] };
    let mut out = vec![];
    for id in list.iter().filter_map(|m| m["id"].as_str()) {
        let mut m = Model { id: id.into(), tools: None, reasoning: false, context: None };
        if ollama && let Some(show) = a.post(format!("{root}/api/show")).send_json(json!({ "model": id })).ok().and_then(|mut r| r.body_mut().read_json::<Value>().ok()) {
            let caps: Vec<&str> = show["capabilities"].as_array().map(|c| c.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            if !caps.is_empty() && !caps.contains(&"completion") {
                continue;
            }
            m.tools = (!caps.is_empty()).then(|| caps.contains(&"tools"));
            m.reasoning = caps.contains(&"thinking");
            m.context = ollama_context(&show, loaded.iter().find(|l| l["name"] == id || l["model"] == id));
        } else if let Some(info) = studio.as_ref().and_then(|s| s.iter().find(|i| i["id"] == id)) {
            if info["type"] == "embeddings" {
                continue;
            }
            m.tools = info["capabilities"].as_array().map(|c| c.iter().any(|c| c == "tool_use"));
            m.context = info["max_context_length"].as_u64();
        }
        out.push(m);
    }
    Ok(out)
}

/// The context Ollama gives a model: as it's loaded with, else as its
/// Modelfile sets (num_ctx), at most what it was trained for. Unknown
/// otherwise, as the server's default isn't to be had: the trained
/// length would be wrong, and the agent then sends more than is read,
/// Ollama dropping the start of it unsaid.
fn ollama_context(show: &Value, loaded: Option<&Value>) -> Option<u64> {
    let trained = show["model_info"].as_object().and_then(|i| i.iter().find(|(k, _)| k.ends_with(".context_length")).and_then(|(_, v)| v.as_u64()));
    let set = show["parameters"].as_str().and_then(|p| p.lines().find_map(|l| l.trim().strip_prefix("num_ctx")?.trim().parse().ok()));
    let runs = loaded.and_then(|l| l["context_length"].as_u64()).or(set)?;
    Some(trained.map_or(runs, |t| t.min(runs)))
}

/// LM Studio's own list of its models, if it is LM Studio.
fn lm_studio(a: &ureq::Agent, root: &str) -> Option<Vec<Value>> {
    let v: Value = a.get(format!("{root}/api/v0/models")).call().ok()?.body_mut().read_json().ok()?;
    v["data"].as_array().cloned()
}

/// OpenCode's config for the servers and the models each was found to
/// have, as a provider for each, named as `name` says.
pub fn opencode_config(found: &[(&ModelServer, Vec<Model>)]) -> Value {
    let mut providers = Map::new();
    for (s, models) in found {
        let mut id = name(&s.url);
        let mut n = 2;
        while providers.contains_key(&id) {
            id = format!("{}-{n}", name(&s.url));
            n += 1;
        }
        let mut options = json!({ "baseURL": base_url(&s.url) });
        if !s.key.is_empty() {
            options["apiKey"] = s.key.clone().into();
        }
        let mut list = Map::new();
        for m in models {
            let mut e = json!({ "name": m.id });
            if let Some(t) = m.tools {
                e["tool_call"] = t.into();
            }
            if m.reasoning {
                e["reasoning"] = true.into();
            }
            if let Some(c) = m.context {
                e["limit"] = json!({ "context": c, "output": c / 4 });
            }
            list.insert(m.id.clone(), e);
        }
        providers.insert(id.clone(), json!({ "npm": "@ai-sdk/openai-compatible", "name": id, "options": options, "models": list }));
    }
    json!({ "provider": providers })
}

/// The environment the agent starts with to have its servers' models,
/// and a note for each server that couldn't be listed. The servers are
/// asked at once, so one that's off costs one timeout, not one each.
pub fn agent_env(p: &AgentProfile) -> (Vec<(String, String)>, Vec<String>) {
    let servers: Vec<&ModelServer> = p.servers.iter().filter(|s| !s.url.trim().is_empty()).collect();
    if !p.takes_servers() || servers.is_empty() {
        return (vec![], vec![]);
    }
    let listed: Vec<Result<Vec<Model>, String>> = std::thread::scope(|sc| {
        let asks: Vec<_> = servers.iter().map(|s| sc.spawn(move || models(s))).collect();
        asks.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("listing its models failed".into()))).collect()
    });
    let mut found = vec![];
    let mut notes = vec![];
    for (s, r) in servers.into_iter().zip(listed) {
        match r {
            Ok(m) => found.push((s, m)),
            Err(e) => notes.push(format!("{}'s models aren't offered: {e}", s.url.trim())),
        }
    }
    if found.is_empty() {
        return (vec![], notes);
    }
    (vec![("OPENCODE_CONFIG_CONTENT".into(), opencode_config(&found).to_string())], notes)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    #[test]
    fn urls_as_given_or_filled_in() {
        assert_eq!(base_url("barley:11434"), "http://barley:11434/v1");
        assert_eq!(base_url("http://barley:11434/"), "http://barley:11434/v1");
        assert_eq!(base_url("https://openrouter.ai/api/v1"), "https://openrouter.ai/api/v1");
        assert_eq!(name("http://barley.rockgarden.net:11434"), "barley");
        assert_eq!(name("https://openrouter.ai/api/v1"), "openrouter");
    }

    /// A server answering each path with its body, as Ollama would.
    pub fn serving(answers: &'static [(&'static str, &'static str)]) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        std::thread::spawn(move || {
            for s in l.incoming() {
                let Ok(mut s) = s else { return };
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                r.read_line(&mut line).unwrap();
                let path = line.split_whitespace().nth(1).unwrap_or_default().to_string();
                let mut len = 0;
                loop {
                    let mut h = String::new();
                    r.read_line(&mut h).unwrap();
                    if h == "\r\n" || h.is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                // A quoted model name answers /api/show for that model
                let found = answers.iter().find(|(p, _)| *p == path || (path == "/api/show" && p.starts_with('"') && body.contains(p)));
                let (status, text) = found.map_or(("404 Not Found", ""), |(_, t)| ("200 OK", *t));
                write!(s, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len()).unwrap();
            }
        });
        url
    }

    /// Ollama's models with what it says of them; one only for images
    /// left out.
    #[test]
    fn ollamas_models() {
        let url = serving(&[
            ("/v1/models", r#"{"data":[{"id":"qwen3.8:latest"},{"id":"x/flux2-klein:9b"}]}"#),
            ("/api/version", r#"{"version":"0.12.0"}"#),
            ("/api/ps", r#"{"models":[{"name":"qwen3.8:latest","context_length":32768}]}"#),
            ("\"qwen3.8:latest\"", r#"{"capabilities":["completion","tools","thinking"],"model_info":{"qwen3_5.context_length":262144}}"#),
            ("\"x/flux2-klein:9b\"", r#"{"capabilities":["image"]}"#),
        ]);
        let s = ModelServer { url, key: String::new() };
        let m = models(&s).unwrap();
        assert_eq!(m, vec![Model { id: "qwen3.8:latest".into(), tools: Some(true), reasoning: true, context: Some(32768) }]);
        let c = opencode_config(&[(&s, m)]);
        let p = &c["provider"]["127"];
        assert_eq!(p["options"]["baseURL"], format!("{}/v1", s.url));
        assert_eq!(p["models"]["qwen3.8:latest"]["limit"]["output"], 8192);
        assert_eq!(p["models"]["qwen3.8:latest"]["tool_call"], true);
    }

    /// Ollama's context as it runs a model, not as the model was trained.
    #[test]
    fn ollamas_context() {
        let show = json!({ "model_info": { "q.context_length": 262144 }, "parameters": "temperature 1\nnum_ctx 131072" });
        assert_eq!(ollama_context(&show, None), Some(131072));
        assert_eq!(ollama_context(&show, Some(&json!({ "context_length": 32768 }))), Some(32768));
        assert_eq!(ollama_context(&json!({ "model_info": { "q.context_length": 8192 } }), Some(&json!({ "context_length": 32768 }))), Some(8192));
        assert_eq!(ollama_context(&json!({ "model_info": { "q.context_length": 262144 } }), None), None);
    }

    /// Any other server's models as listed, and one that's off said so.
    #[test]
    fn other_servers() {
        let url = serving(&[("/v1/models", r#"{"data":[{"id":"some/model"}]}"#)]);
        let s = ModelServer { url, key: "k".into() };
        assert_eq!(models(&s).unwrap(), vec![Model { id: "some/model".into(), tools: None, reasoning: false, context: None }]);
        assert_eq!(opencode_config(&[(&s, vec![])])["provider"]["127"]["options"]["apiKey"], "k");
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        let p = AgentProfile { command: "opencode".into(), servers: vec![ModelServer { url: format!("http://{free}"), key: String::new() }], ..Default::default() };
        let (env, notes) = agent_env(&p);
        assert!(env.is_empty() && notes.len() == 1, "{notes:?}");
        let other = AgentProfile { command: "goose".into(), ..p };
        assert_eq!(agent_env(&other), (vec![], vec![]));
    }

    /// A real agent given a real server offers its models: PARTMAN_AGENT
    /// names the agent's command line ("opencode acp"), and
    /// PARTMAN_MODEL_SERVER the server. Spends nothing, as nothing's asked
    /// of a model: `cargo test -p partman-agent real_server -- --ignored`.
    #[test]
    #[ignore]
    fn real_server() {
        let line = std::env::var("PARTMAN_AGENT").expect("set PARTMAN_AGENT to the agent's command line");
        let url = std::env::var("PARTMAN_MODEL_SERVER").expect("set PARTMAN_MODEL_SERVER to a model server");
        let mut words = crate::profiles::split_words(&line);
        let mut p = AgentProfile { name: "real".into(), command: words.remove(0), args: words, servers: vec![ModelServer { url: url.clone(), key: String::new() }], ..Default::default() };
        let (env, notes) = agent_env(&p);
        assert!(notes.is_empty(), "{notes:?}");
        p.env.extend(env);
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/real-server");
        std::fs::create_dir_all(&dir).unwrap();
        let (conn, _events) = crate::acp::Conn::spawn(&p, &dir, || {}).unwrap();
        let client = crate::acp::Client { name: "partman-agent".into(), title: "partman-agent's test".into(), version: "0".into() };
        crate::acp::initialize(&conn, &client).unwrap();
        let s = crate::acp::new_session(&conn, &dir, &[]).unwrap();
        let model = s.config.iter().find(|c| c.id == "model").expect("no model setting");
        let ours: Vec<&String> = model.choices.iter().map(|(v, _)| v).filter(|v| v.starts_with(&format!("{}/", name(&url)))).collect();
        eprintln!("offered: {ours:?}");
        assert!(!ours.is_empty(), "none of {}'s models among {:?}", url, model.choices);
    }
}
