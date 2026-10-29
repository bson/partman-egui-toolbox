// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The AI agents a user can run from the window: any program speaking the
//! Agent Client Protocol (ACP) over its stdin and stdout. Which agent, and
//! so which model and whose service, is each user's own choice, kept in
//! their settings; the presets are only starting points.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentProfile {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    /// Set for the agent alone, such as an API key or a model choice
    pub env: BTreeMap<String, String>,
    /// The session settings last picked (configOption id to value), such
    /// as effort or fast mode: each session starts with the agent's own
    /// defaults, so they're put back on every new one
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, String>,
    /// Model servers whose models the agent's offered, for agents that
    /// can be given them (takes_servers)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<ModelServer>,
}

/// A server speaking OpenAI's API: Ollama, LM Studio, llama.cpp, vLLM, or
/// a service such as OpenRouter.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelServer {
    pub url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub key: String,
}

/// (name, command line, what it needs). Taken from ACP's registry
/// (agentclientprotocol.com/registry): its npm packages are run through
/// npx unpinned, so they stay current; its downloads are assumed
/// installed on the PATH.
const PRESETS: &[(&str, &str, &str)] = &[
    ("Claude", "npx -y @agentclientprotocol/claude-agent-acp", "Claude Code's login, or ANTHROPIC_API_KEY"),
    ("Codex", "npx -y @agentclientprotocol/codex-acp", "Codex's login, or OPENAI_API_KEY"),
    ("Gemini CLI", "npx -y @google/gemini-cli --acp", "Gemini CLI's login, or GEMINI_API_KEY"),
    ("GitHub Copilot", "npx -y @github/copilot --acp", "a GitHub Copilot subscription"),
    ("Qwen Code", "npx -y @qwen-code/qwen-code --acp", "Qwen Code's login or an API key"),
    ("Cline", "npx -y cline --acp", "Cline's provider settings"),
    ("goose", "goose acp", "goose installed; any provider, local models through Ollama"),
    ("OpenCode", "opencode acp", "OpenCode installed; any provider, or model servers given here"),
    ("Mistral Vibe", "vibe-acp", "Mistral Vibe installed, and a Mistral API key"),
    ("Cursor", "cursor-agent acp", "Cursor's agent CLI installed and signed in"),
];

/// What a fresh install is offered: OpenCode with one of its free models,
/// DeepSeek V4.1 Flash, which wrote sound pin info for an MSP430F5510 in a
/// trial where local models couldn't, at high effort, as pin info is
/// careful cross-reading of tables. A model OpenCode has since retired is
/// let be when the session starts, leaving OpenCode's own choice.
pub const STARTER: &str = "OpenCode";
const STARTER_OPTIONS: &[(&str, &str)] = &[("model", "opencode/deepseek-v4.1-flash-free"), ("effort", "high")];

/// The starter agent, if OpenCode is installed: on the PATH, or where its
/// installer puts it, which an app started from the desktop may not have
/// on its PATH.
pub fn starter() -> Option<AgentProfile> {
    let p = presets().into_iter().find(|p| p.profile.name == STARTER)?.profile;
    let installed = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".opencode/bin/opencode")).filter(|p| p.is_file());
    let program = p.program().or(installed)?;
    Some(starter_at(p, program))
}

fn starter_at(mut p: AgentProfile, program: PathBuf) -> AgentProfile {
    p.command = program.display().to_string();
    p.options = STARTER_OPTIONS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    p
}

pub struct Preset {
    pub profile: AgentProfile,
    pub needs: &'static str,
}

pub fn presets() -> Vec<Preset> {
    PRESETS
        .iter()
        .map(|(name, line, needs)| {
            let mut words = split_words(line);
            let command = words.remove(0);
            Preset { profile: AgentProfile { name: name.to_string(), command, args: words, env: BTreeMap::new(), ..Default::default() }, needs }
        })
        .collect()
}

impl AgentProfile {
    /// Where the command is: as given if it's a path, else on the PATH.
    pub fn program(&self) -> Option<PathBuf> {
        find_program(&self.command)
    }

    /// Whether the agent can be given model servers: OpenCode reads them
    /// from its environment, as config of its own
    pub fn takes_servers(&self) -> bool {
        takes_servers(&self.command)
    }

    pub fn command_line(&self) -> String {
        std::iter::once(&self.command).chain(&self.args).map(|w| quote_word(w)).collect::<Vec<_>>().join(" ")
    }
}

pub fn takes_servers(command: &str) -> bool {
    is_opencode(command)
}

pub fn is_opencode(command: &str) -> bool {
    std::path::Path::new(command).file_stem().is_some_and(|s| s == "opencode")
}

pub fn find_program(command: &str) -> Option<PathBuf> {
    if command.is_empty() {
        return None;
    }
    let p = PathBuf::from(command);
    if p.components().count() > 1 {
        return p.is_file().then_some(p);
    }
    let path = std::env::var_os("PATH")?;
    let exts: &[&str] = if cfg!(windows) { &["", ".exe", ".cmd", ".bat"] } else { &[""] };
    std::env::split_paths(&path).flat_map(|d| exts.iter().map(move |e| d.join(format!("{command}{e}")))).find(|p| p.is_file())
}

/// A command line split into words as a shell would for plain quoting:
/// spaces separate, and '…' or "…" keep spaces in a word.
pub fn split_words(line: &str) -> Vec<String> {
    let mut words = vec![];
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                started = false;
            }
            (None, c) => word.push(c),
        }
    }
    if started || !word.is_empty() {
        words.push(word);
    }
    words
}

pub fn quote_word(w: &str) -> String {
    if !w.is_empty() && !w.contains(|c: char| c.is_whitespace() || c == '\'' || c == '"') {
        return w.to_string();
    }
    if w.contains('\'') { format!("\"{w}\"") } else { format!("'{w}'") }
}

/// Environment entries as KEY=VALUE lines, for editing.
pub fn env_text(env: &BTreeMap<String, String>) -> String {
    env.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
}

/// KEY=VALUE lines back into entries; an error names a line that isn't.
pub fn parse_env(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut env = BTreeMap::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (k, v) = line.split_once('=').ok_or_else(|| format!("\"{line}\" isn't NAME=value"))?;
        let k = k.trim();
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("\"{k}\" isn't a variable name"));
        }
        env.insert(k.to_string(), v.to_string());
    }
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_quotes_command_lines() {
        assert_eq!(split_words("npx -y  pkg --acp"), ["npx", "-y", "pkg", "--acp"]);
        assert_eq!(split_words("a 'b c' \"d e\" ''"), ["a", "b c", "d e", ""]);
        let p = AgentProfile { command: "/opt/my agent/run".into(), args: vec!["--acp".into(), "".into()], ..Default::default() };
        assert_eq!(p.command_line(), "'/opt/my agent/run' --acp ''");
        assert_eq!(split_words(&p.command_line()), ["/opt/my agent/run", "--acp", ""]);
    }

    #[test]
    fn presets_have_commands() {
        let p = presets();
        assert!(p.iter().all(|p| !p.profile.command.is_empty() && !p.profile.name.is_empty()));
        assert_eq!(p[0].profile.args, ["-y", "@agentclientprotocol/claude-agent-acp"]);
    }

    #[test]
    fn reads_environment_lines() {
        let env = parse_env("A=1\n\n# note\nB_2 = x=y\n").unwrap();
        assert_eq!(env.get("A").map(String::as_str), Some("1"));
        assert_eq!(env.get("B_2").map(String::as_str), Some(" x=y"));
        assert!(parse_env("nope").is_err());
        assert!(parse_env("A B=1").is_err());
        assert_eq!(parse_env(&env_text(&env)).unwrap(), env);
    }

    #[test]
    fn starter_runs_deepseek_at_high_effort() {
        let p = presets().into_iter().find(|p| p.profile.name == STARTER).unwrap().profile;
        let s = starter_at(p, PathBuf::from("/home/u/.opencode/bin/opencode"));
        assert_eq!(s.command_line(), "/home/u/.opencode/bin/opencode acp");
        assert_eq!(s.options.get("model").map(String::as_str), Some("opencode/deepseek-v4.1-flash-free"));
        assert_eq!(s.options.get("effort").map(String::as_str), Some("high"));
        assert!(s.takes_servers());
    }
}
