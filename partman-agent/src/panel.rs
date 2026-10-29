// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The agent panel: an agent at work, its replies, thoughts, tool calls
//! and plan as they come, what it's used, its questions, and the user's
//! answers and messages. The app gives what's its own in a `Setup`: what
//! it calls itself, what it tells the agent it is, and the tools (MCP
//! servers) it gives the agent, which a grant lets it use.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use egui::{Color32, RichText};

use crate::acp::{self, AgentInfo, Client, ConfigOption, Conn, Event, McpServer, RpcError, Started};
use crate::profiles::{self, AgentProfile};
use crate::task::{Poll, Task};

/// The colours the panel is drawn in, from the app's theme or egui's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub text: Color32,
    /// Notes, hints and what's said in passing
    pub text_dim: Color32,
    /// The plan, tool output and instructions given
    pub text_label: Color32,
    /// The user's messages' background
    pub inset: Color32,
    /// The questions' cards' background
    pub card: Color32,
    pub accent: Color32,
    pub good: Color32,
    pub bad: Color32,
    pub caution: Color32,
}

impl Colors {
    pub fn from_visuals(v: &egui::Visuals) -> Colors {
        let dark = v.dark_mode;
        Colors {
            text: v.strong_text_color(),
            text_dim: v.weak_text_color(),
            text_label: v.text_color(),
            inset: v.faint_bg_color,
            card: v.extreme_bg_color,
            accent: v.selection.bg_fill,
            good: if dark { Color32::from_rgb(0x4c, 0xaf, 0x50) } else { Color32::from_rgb(0x2e, 0x7d, 0x32) },
            bad: if dark { Color32::from_rgb(0xe0, 0x4a, 0x3f) } else { Color32::from_rgb(0xc6, 0x28, 0x28) },
            caution: v.warn_fg_color,
        }
    }
}

/// What's the app's own: what it tells the agent it is, what it calls
/// itself to the user ("the pin planner"), and the tools it gives every
/// session, which a grant lets the agent use without asking.
#[derive(Clone, Debug)]
pub struct Setup {
    pub client: Client,
    pub app: String,
    pub tools: Vec<McpServer>,
}

/// A scroll area reported to partman-scroll, as the toolbox's are.
fn scroll<R>(ui: &mut egui::Ui, area: egui::ScrollArea, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let shown = area.show(ui, add);
    partman_scroll::area(ui.ctx(), &shown);
    shown.inner
}

/// A card for a question, set apart from the transcript.
fn card(ui: &mut egui::Ui, c: &Colors, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(c.card).corner_radius(6).inner_margin(14).show(ui, add);
}

/// Whether the context going from `was` to `now` tokens is the agent
/// compacting: down to under half, from enough that it would need to.
/// A small drop is the agent trimming a tool's output, not this.
fn compacted(was: u64, now: u64) -> bool {
    was >= 8_000 && now < was / 2
}

/// A token count as shown: 950, 13.7k, 200k, 1.2M.
fn tokens(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..100_000 => format!("{:.1}k", n as f64 / 1e3),
        100_000..1_000_000 => format!("{}k", n / 1_000),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

fn money(amount: f64, currency: &str) -> String {
    if currency == "USD" { format!("${amount:.2}") } else { format!("{amount:.2} {currency}") }
}

/// One line of an agent session, as shown.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entry {
    User(String),
    Agent(String),
    Thought(String),
    Tool { id: String, title: String, kind: String, status: String, detail: String },
    /// (what, status) of each step the agent plans
    Plan(Vec<(String, String)>),
    /// From the app itself: starting, errors, the agent quitting
    Note(String),
}

/// What the agent asks permission for, waiting on the user.
pub struct Asking {
    id: serde_json::Value,
    title: String,
    /// (optionId, name, kind)
    options: Vec<(String, String, String)>,
    /// The tool call's id, for its kind and paths as told before
    call: String,
    /// ACP's tool kind, if the request says: "read", "edit", "execute", ...
    kind: String,
    /// The paths it touches, as far as the request says
    paths: Vec<PathBuf>,
}

/// What the user allowed a task's agent to do without asking: files in
/// the folder it works in, the app's tools, fetching and thinking, and
/// shell commands only if said. Anything else is asked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Grant {
    pub commands: bool,
}

/// (the agent, the session, whether it's the one asked to resume)
type StartResult = Result<(AgentInfo, Started, bool), RpcError>;

/// A project's agent session as kept in its folder, so reopening the
/// project brings it back: which agent, its session to resume, whether its
/// panel was shown, and what was said.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Saved {
    pub agent: String,
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub shown: bool,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

impl Saved {
    pub const FILE: &str = "agent.json";

    pub fn load(dir: &std::path::Path) -> Option<Saved> {
        serde_json::from_slice(&std::fs::read(dir.join(Self::FILE)).ok()?).ok()
    }

    pub fn text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// An agent at work: its process, the session, and what it has said.
pub struct AgentRun {
    pub profile: AgentProfile,
    setup: Setup,
    pub cwd: PathBuf,
    conn: Arc<Conn>,
    events: Receiver<Event>,
    start: Option<Task<StartResult>>,
    turn: Option<Task<Result<acp::Ended, RpcError>>>,
    pub info: Option<AgentInfo>,
    pub session: Option<String>,
    pub config: Vec<ConfigOption>,
    /// The context as the agent last said, where it says
    pub context: Option<acp::Context>,
    /// Tokens in and out over the session's turns, and the last turn's
    pub spent: Option<acp::Usage>,
    last: Option<acp::Usage>,
    /// Times the agent compacted what was said, to fit its context
    pub compacted: u32,
    /// Stopped on compacting rather than let go on (`Job::exact`)
    exact: bool,
    /// How the last turn ended: the agent's stop reason ("end_turn",
    /// "cancelled", ...), or "error"
    pub last_stop: Option<String>,
    /// What's allowed without asking, once the user has said
    pub grant: Option<Grant>,
    /// The task's question of what to allow is still shown
    pub granting: bool,
    /// Shell commands ticked on that question, to start it so next time
    pub commands: bool,
    /// The choice of commands just made, for the settings to keep
    pub commands_picked: Option<bool>,
    /// The message box was dragged to another height and let go, for the
    /// settings to keep
    pub input_resized: bool,
    /// Each tool call's kind and paths, as its updates told: a permission
    /// request may carry neither (OpenCode's has no paths)
    calls: HashMap<String, (String, Vec<PathBuf>)>,
    /// Requests answered within the grant
    pub allowed: usize,
    config_task: Option<Task<Result<Vec<ConfigOption>, RpcError>>>,
    auth_task: Option<Task<Result<(), RpcError>>>,
    /// Sign-in methods offered when the agent won't start a session
    pub needs_auth: bool,
    pub entries: Vec<Entry>,
    asking: VecDeque<Asking>,
    /// Sent once the session is up
    first: Option<String>,
    /// A session from before, to take up
    resume: Option<String>,
    /// A session setting the user just picked, for the settings to keep
    pub picked: Option<(String, String)>,
    markdown: egui_commonmark::CommonMarkCache,
    /// The agent's replay of a resumed session is under way: it isn't
    /// shown, as what was said is kept (and shown) already
    replaying: bool,
    pub input: String,
    pub exited: bool,
}

impl AgentRun {
    /// Starts the agent in cwd, and gives it `task` once the session is up.
    /// Exact, it's stopped if it compacts what was said (`Job::exact`).
    pub fn start(ctx: &egui::Context, setup: &Setup, profile: &AgentProfile, cwd: PathBuf, task: String, exact: bool) -> Result<AgentRun, String> {
        let mut run = Self::spawn(ctx, setup, profile, cwd, exact)?;
        run.first = Some(task);
        // Asked once, up front, rather than at each tool call
        run.granting = true;
        run.open_session(ctx);
        Ok(run)
    }

    fn spawn(ctx: &egui::Context, setup: &Setup, profile: &AgentProfile, cwd: PathBuf, exact: bool) -> Result<AgentRun, String> {
        std::fs::create_dir_all(&cwd).map_err(|e| format!("{}: {e}", cwd.display()))?;
        let wake = ctx.clone();
        let (env, notes) = crate::servers::agent_env(profile);
        let mut with = profile.clone();
        with.env.extend(env);
        // OpenCode then fails a request that doesn't fit rather than
        // compacting; other agents are stopped on it being seen
        if exact && profiles::is_opencode(&profile.command) {
            with.env.insert("OPENCODE_DISABLE_AUTOCOMPACT".into(), "1".into());
        }
        let (conn, events) = Conn::spawn(&with, &cwd, move || wake.request_repaint())?;
        Ok(Self::connected(setup, profile, cwd, conn, events, exact, notes))
    }

    /// An agent already connected, its process or a thread: the run, not
    /// yet asked for a session.
    pub(crate) fn connected(setup: &Setup, profile: &AgentProfile, cwd: PathBuf, conn: Arc<Conn>, events: Receiver<Event>, exact: bool, notes: Vec<String>) -> AgentRun {
        AgentRun {
            profile: profile.clone(),
            setup: setup.clone(),
            cwd,
            conn,
            events,
            start: None,
            turn: None,
            info: None,
            session: None,
            config: vec![],
            context: None,
            spent: None,
            last: None,
            compacted: 0,
            exact,
            last_stop: None,
            grant: None,
            granting: false,
            commands: false,
            commands_picked: None,
            input_resized: false,
            calls: HashMap::new(),
            allowed: 0,
            config_task: None,
            auth_task: None,
            needs_auth: false,
            entries: std::iter::once(format!("Starting {}…", profile.name)).chain(notes).map(Entry::Note).collect(),
            asking: VecDeque::new(),
            first: None,
            resume: None,
            picked: None,
            markdown: Default::default(),
            replaying: false,
            input: String::new(),
            exited: false,
        }
    }

    /// Starts the agent again on a session kept from before: the agent
    /// takes it up if it can, else starts afresh below what was said.
    pub fn resume(ctx: &egui::Context, setup: &Setup, profile: &AgentProfile, cwd: PathBuf, saved: Saved) -> Result<AgentRun, String> {
        let mut run = Self::spawn(ctx, setup, profile, cwd, false)?;
        run.take_up(ctx, saved);
        Ok(run)
    }

    /// Starts a session below what was said before, asking first what the
    /// agent may do without asking, as a task's agent does.
    pub(crate) fn begin(&mut self, ctx: &egui::Context, before: Vec<Entry>) {
        let notes = std::mem::take(&mut self.entries);
        self.entries = before;
        self.entries.extend(notes);
        self.granting = true;
        self.open_session(ctx);
    }

    /// Shows what was said before, and asks the agent to take up its
    /// session, or start a new one below it.
    pub(crate) fn take_up(&mut self, ctx: &egui::Context, saved: Saved) {
        self.entries = saved.entries;
        self.note(format!("Starting {} again…", self.profile.name));
        self.replaying = saved.session.is_some();
        self.resume = saved.session;
        self.open_session(ctx);
    }

    /// The session as it's to be kept.
    pub fn saved(&self, shown: bool) -> Saved {
        Saved { agent: self.profile.name.clone(), session: self.session.clone().or_else(|| self.resume.clone()), shown, entries: self.entries.clone() }
    }

    fn open_session(&mut self, ctx: &egui::Context) {
        let (conn, cwd) = (self.conn.clone(), self.cwd.clone());
        let known = self.info.clone();
        let resume = self.resume.clone();
        let options = self.profile.options.clone();
        let (client, tools) = (self.setup.client.clone(), self.setup.tools.clone());
        self.start = Some(Task::spawn(ctx, move || {
            let info = match known {
                Some(i) => i,
                None => acp::initialize(&conn, &client)?,
            };
            if let Some(id) = resume.filter(|_| info.load_session)
                && let Ok(mut started) = acp::load_session(&conn, &id, &cwd, &tools)
            {
                put_back(&conn, &mut started, &options);
                return Ok((info, started, true));
            }
            let mut started = acp::new_session(&conn, &cwd, &tools)?;
            // Before the task's sent, so it's done as the user set it up
            put_back(&conn, &mut started, &options);
            Ok((info, started, false))
        }));
    }

    /// What the agent is waiting for the user to allow, if anything.
    pub fn asking(&self) -> Option<&str> {
        self.asking.front().map(|a| a.title.as_str())
    }

    pub fn busy(&self) -> bool {
        self.start.is_some() || self.turn.is_some() || self.auth_task.is_some()
    }

    fn note(&mut self, text: impl Into<String>) {
        self.entries.push(Entry::Note(text.into()));
    }

    pub fn send(&mut self, ctx: &egui::Context, text: String) {
        let Some(session) = self.session.clone() else { return };
        self.entries.push(Entry::User(text.clone()));
        let conn = self.conn.clone();
        self.turn = Some(Task::spawn(ctx, move || acp::prompt(&conn, &session, &text)));
    }

    /// Stops the turn under way; a permission still asked for is refused.
    pub fn cancel(&mut self) {
        if let Some(s) = &self.session {
            acp::cancel(&self.conn, s);
        }
        for a in self.asking.drain(..) {
            self.conn.reply(a.id, Ok(serde_json::json!({ "outcome": { "outcome": "cancelled" } })));
        }
    }

    pub fn stop(&mut self) {
        self.cancel();
        self.conn.stop();
    }

    /// Whether the agent said or did anything since what it was last sent;
    /// thinking alone doesn't count.
    fn answered(&self) -> bool {
        self.entries.iter().rev().take_while(|e| !matches!(e, Entry::User(_))).any(|e| matches!(e, Entry::Agent(_) | Entry::Tool { .. }))
    }

    /// Takes in what the agent and the tasks have to say.
    pub fn poll(&mut self, ctx: &egui::Context) {
        self.take_events();
        if let Some(t) = &self.start {
            match t.poll() {
                Poll::Pending => {}
                Poll::Ready(r) => {
                    self.start = None;
                    // The replay all came before the answer: what's left
                    // of it is dropped too
                    self.take_events();
                    self.replaying = false;
                    match r {
                        Ok((info, started, resumed)) => {
                            self.note(format!("{} {} is ready.", info.title, info.version).trim_end().to_string());
                            match self.resume.take() {
                                Some(_) if resumed => self.note("It took up the session where it left off."),
                                Some(_) => self.note("It can't take up the earlier session, so this is a new one: it doesn't know what was said above."),
                                None => {}
                            }
                            self.info = Some(info);
                            self.session = Some(started.id);
                            self.config = started.config;
                            if let Some(task) = self.first.take() {
                                self.send(ctx, task);
                            }
                        }
                        Err(e) if e.code == acp::AUTH_REQUIRED => {
                            self.needs_auth = true;
                            self.note(format!("{} needs you to sign in: {}", self.profile.name, e.message));
                        }
                        Err(e) => {
                            let tail = self.conn.stderr_tail(8);
                            self.note(if tail.is_empty() { format!("The agent didn't start: {e}") } else { format!("The agent didn't start: {e}\n{tail}") });
                        }
                    }
                }
                Poll::Lost => self.start = None,
            }
        }
        if let Some(t) = &self.turn {
            match t.poll() {
                Poll::Pending => {}
                Poll::Ready(r) => {
                    self.turn = None;
                    self.last_stop = Some(r.as_ref().map_or("error", |e| e.stop.as_str()).to_string());
                    if let Ok(acp::Ended { usage: Some(u), .. }) = &r {
                        let s = self.spent.get_or_insert_default();
                        s.input += u.input;
                        s.output += u.output;
                        self.last = Some(*u);
                    }
                    match r.as_ref().map(|e| e.stop.as_str()) {
                        Ok("end_turn") if self.answered() => {}
                        // Else the panel looks as if it's still waiting
                        Ok("end_turn") => self.note(format!(
                            "{} ended its turn without replying or using a tool. A local model often does this when its context is too small for the task and the tools' descriptions, which are then cut off: give it a larger context, or pick a model that can use tools.",
                            self.profile.name
                        )),
                        Ok("cancelled") => self.note("Stopped."),
                        Ok("refusal") => self.note("The agent refused."),
                        Ok(other) => self.note(format!("The agent stopped: {}", other.replace('_', " "))),
                        Err(e) => self.note(format!("The agent failed: {e}")),
                    }
                }
                Poll::Lost => self.turn = None,
            }
        }
        if let Some(t) = &self.config_task
            && let Poll::Ready(r) = t.poll()
        {
            self.config_task = None;
            match r {
                Ok(c) if !c.is_empty() => self.config = c,
                Ok(_) => {}
                Err(e) => self.note(format!("The setting wasn't changed: {e}")),
            }
        }
        if let Some(t) = &self.auth_task
            && let Poll::Ready(r) = t.poll()
        {
            self.auth_task = None;
            match r {
                Ok(()) => {
                    self.needs_auth = false;
                    self.open_session(ctx);
                }
                Err(e) => self.note(format!("Signing in failed: {e}")),
            }
        }
    }

    fn take_events(&mut self) {
        while let Ok(e) = self.events.try_recv() {
            match e {
                Event::Update(_) if self.replaying => {}
                Event::Update(u) => self.update(u),
                Event::Permission { id, params } => {
                    let a = asking(id, &params);
                    if !self.allow(&a) {
                        self.asking.push_back(a);
                    }
                }
                Event::Exited(how) => {
                    self.exited = true;
                    self.note(how);
                }
            }
        }
    }

    fn update(&mut self, u: serde_json::Value) {
        let s = |k: &str| u.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        match u.get("sessionUpdate").and_then(|v| v.as_str()).unwrap_or("") {
            "agent_message_chunk" => self.append(content_text(u.get("content")), false),
            "agent_thought_chunk" => self.append(content_text(u.get("content")), true),
            "tool_call" => {
                self.track(&u);
                self.entries.push(Entry::Tool { id: s("toolCallId"), title: s("title"), kind: s("kind"), status: s("status"), detail: tool_detail(&u) })
            }
            "tool_call_update" => {
                self.track(&u);
                let id = s("toolCallId");
                let detail = tool_detail(&u);
                if let Some(Entry::Tool { title, kind, status, detail: d, .. }) = self.entries.iter_mut().rev().find(|e| matches!(e, Entry::Tool { id: i, .. } if *i == id)) {
                    for (field, key) in [(title, "title"), (kind, "kind"), (status, "status")] {
                        if let Some(v) = u.get(key).and_then(|v| v.as_str()) {
                            *field = v.to_string();
                        }
                    }
                    if !detail.is_empty() {
                        *d = detail;
                    }
                }
            }
            "plan" => {
                let steps = u.get("entries").and_then(|e| e.as_array()).map(|a| {
                    a.iter().map(|e| (e.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(), e.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string())).collect()
                });
                let steps = steps.unwrap_or_default();
                // A plan replaces the one before it, where it stands
                match self.entries.iter_mut().rev().find(|e| matches!(e, Entry::Plan(_))) {
                    Some(Entry::Plan(p)) => *p = steps,
                    _ => self.entries.push(Entry::Plan(steps)),
                }
            }
            "config_option_update" => {
                let c = acp::config_options(&u);
                if !c.is_empty() {
                    self.config = c;
                }
            }
            "usage_update" => {
                if let Some(c) = acp::context(&u) {
                    // ACP has no word for compacting: it shows as the context
                    // dropping to a fraction of what it held. Only seen
                    // mid-turn from an agent reporting usage as it goes;
                    // OpenCode reports at a turn's end, so exact tasks
                    // turn its compacting off instead
                    if self.context.as_ref().is_some_and(|was| compacted(was.used, c.used)) {
                        self.compacted += 1;
                        if self.exact && self.turn.is_some() {
                            self.cancel();
                            self.note(format!(
                                "Stopped: {} compacted the conversation to fit its context, and what it would write from its summary of the documents can't be trusted. Use a model with a larger context.",
                                self.profile.name
                            ));
                        } else {
                            self.note(format!(
                            "{} compacted the conversation to fit its context: what it read before now survives only as its summary, so details from it, such as a pin table, may be lost. Check what it writes against the documents.",
                            self.profile.name
                        ));
                        }
                    }
                    self.context = Some(c);
                }
            }
            _ => {}
        }
    }

    fn append(&mut self, text: String, thought: bool) {
        match self.entries.last_mut() {
            Some(Entry::Agent(t)) if !thought => t.push_str(&text),
            Some(Entry::Thought(t)) if thought => t.push_str(&text),
            _ if thought => self.entries.push(Entry::Thought(text)),
            _ => self.entries.push(Entry::Agent(text)),
        }
    }

    /// How full the context is, the tokens the session has taken in and
    /// given out, and its cost, as far as the agent says.
    fn usage_ui(&self, ui: &mut egui::Ui, t: &Colors) {
        let dim = |s: String| RichText::new(s).small().color(t.text_dim);
        if self.context.is_none() && self.spent.is_none() {
            // Else a long first turn looks as if usage were broken
            if self.turn.is_some() {
                ui.label(dim("Token use isn't reported yet: some agents, OpenCode among them, report it only when a turn ends".into()));
            }
            return;
        }
        let r = ui.horizontal(|ui| {
            if let Some(c) = &self.context {
                let full = if c.size == 0 { 0.0 } else { (c.used as f32 / c.size as f32).min(1.0) };
                ui.label(dim("Context".into()));
                // Nearly full is when the agent compacts what was said, or
                // the model loses its start
                let fill = if full >= 0.8 { t.caution } else { t.accent };
                ui.add(egui::ProgressBar::new(full).desired_width(80.0).desired_height(6.0).fill(fill));
                ui.label(dim(format!("{} of {}", tokens(c.used), tokens(c.size))));
            }
            if let Some(s) = &self.spent {
                ui.label(dim(format!("· {} in, {} out", tokens(s.input), tokens(s.output))));
            }
            if let Some((amount, currency)) = self.context.as_ref().and_then(|c| c.cost.as_ref()) {
                ui.label(dim(format!("· {}", money(*amount, currency))));
            }
            match self.compacted {
                0 => {}
                1 => {
                    ui.label(RichText::new("· compacted").small().color(t.caution));
                }
                n => {
                    ui.label(RichText::new(format!("· compacted {n}×")).small().color(t.caution));
                }
            }
        });
        let mut tip = vec![];
        if let Some(c) = &self.context {
            tip.push(format!("The context holds {} of the model's {} tokens", c.used, c.size));
        }
        if let Some(s) = &self.spent {
            tip.push(format!("This session: {} tokens in, {} out, over its turns", s.input, s.output));
        }
        if let Some(l) = &self.last {
            tip.push(format!("The last turn: {} in, {} out", l.input, l.output));
        }
        if self.compacted > 0 {
            tip.push("Compacted: what was said was replaced by a summary to fit the context, so earlier details may be lost".into());
        }
        r.response.on_hover_text(tip.join("\n"));
    }

    fn set_config(&mut self, ctx: &egui::Context, id: String, value: String) {
        let Some(session) = self.session.clone() else { return };
        self.profile.options.insert(id.clone(), value.clone());
        self.picked = Some((id.clone(), value.clone()));
        // An agent such as OpenCode settles these for each message, so the
        // turn under way carries on as it began while the choice shows the
        // new one
        if self.turn.is_some() {
            let name = self.config.iter().find(|c| c.id == id).map_or(id.as_str(), |c| c.name.as_str()).to_lowercase();
            self.note(format!("The new {name} is used from the next message; the one under way goes on as it began"));
        }
        let conn = self.conn.clone();
        self.config_task = Some(Task::spawn(ctx, move || acp::set_config(&conn, &session, &id, &value)));
    }

    fn sign_in(&mut self, ctx: &egui::Context, method: String) {
        let conn = self.conn.clone();
        self.auth_task = Some(Task::spawn(ctx, move || acp::authenticate(&conn, &method)));
    }

    /// Keeps a tool call's kind and paths, for a permission request on it.
    fn track(&mut self, u: &serde_json::Value) {
        let Some(id) = u.get("toolCallId").and_then(|v| v.as_str()) else { return };
        let e = self.calls.entry(id.to_string()).or_default();
        if let Some(k) = u.get("kind").and_then(|v| v.as_str()) {
            e.0 = k.to_string();
        }
        for p in call_paths(u) {
            if !e.1.contains(&p) {
                e.1.push(p);
            }
        }
    }

    /// Answers a request the grant covers with the agent's allow-once
    /// option; false if it's for the user. Never "always": what's allowed
    /// lasts only as long as this run.
    fn allow(&mut self, a: &Asking) -> bool {
        let Some(grant) = self.grant else { return false };
        let (kind, mut paths) = self.calls.get(&a.call).cloned().unwrap_or_default();
        let kind = if a.kind.is_empty() { kind } else { a.kind.clone() };
        paths.extend(a.paths.iter().cloned());
        if !in_scope(&kind, &a.title, &paths, &self.cwd, &self.setup.tools, grant) {
            return false;
        }
        let Some((option, _, _)) = a.options.iter().find(|(_, _, k)| k == "allow_once") else { return false };
        self.conn.reply(a.id.clone(), Ok(serde_json::json!({ "outcome": { "outcome": "selected", "optionId": option } })));
        self.allowed += 1;
        true
    }

    /// The task's question answered: allowed within its scope, with shell
    /// commands or not, and what was waiting answered by it.
    pub fn set_grant(&mut self, grant: Option<Grant>) {
        self.granting = false;
        self.grant = grant;
        let waiting: Vec<Asking> = self.asking.drain(..).collect();
        for a in waiting {
            if !self.allow(&a) {
                self.asking.push_back(a);
            }
        }
    }

    fn answer(&mut self, option: Option<String>) {
        let Some(a) = self.asking.pop_front() else { return };
        let outcome = match option {
            Some(o) => serde_json::json!({ "outcome": { "outcome": "selected", "optionId": o } }),
            None => serde_json::json!({ "outcome": { "outcome": "cancelled" } }),
        };
        self.conn.reply(a.id, Ok(outcome));
    }

    /// The session in the agent panel; false when the user ends it.
    pub fn ui(&mut self, ui: &mut egui::Ui, t: &Colors, box_height: &mut Option<f32>) -> bool {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        let mut open = true;
        ui.horizontal(|ui| {
            ui.label(RichText::new(&self.profile.name).strong().color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("End").on_hover_text("Stop the agent, ending the session").clicked() {
                    open = false;
                }
                if self.turn.is_some() && ui.button("Stop").on_hover_text("Stop what the agent is doing, keeping the session").clicked() {
                    self.cancel();
                }
                if self.busy() {
                    ui.spinner();
                }
            });
        });
        let mut change = None;
        if !self.config.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for c in &self.config {
                    let shown = c.choices.iter().find(|(v, _)| *v == c.current).map_or(c.current.as_str(), |(_, n)| n.as_str());
                    egui::ComboBox::from_id_salt(("agent-config", &c.id))
                        .selected_text(format!("{}: {shown}", c.name))
                        .show_ui(ui, |ui| {
                            for (v, n) in &c.choices {
                                if ui.selectable_label(*v == c.current, n).clicked() && *v != c.current {
                                    change = Some((c.id.clone(), v.clone()));
                                }
                            }
                        })
                        .response
                        .on_hover_text(format!("The agent's {}, as it offers them", c.name.to_lowercase()));
                }
            });
        }
        if let Some((id, v)) = change {
            self.set_config(&ctx, id, v);
        }
        self.usage_ui(ui, t);
        ui.label(RichText::new(format!("Working in {}", self.cwd.display())).small().color(t.text_dim));
        if let Some(g) = self.grant {
            let answered = if self.allowed > 0 { format!(" · {} answered so", self.allowed) } else { String::new() };
            let commands = if g.commands { ", shell commands" } else { "" };
            let tools = if self.setup.tools.is_empty() { String::new() } else { format!(", {}'s tools", self.setup.app) };
            ui.label(RichText::new(format!("Allowed without asking: files here{tools}, fetching{commands}{answered}")).small().color(t.text_dim));
        }
        ui.separator();

        let mut answer = None;
        let mut grant_pick: Option<Option<Grant>> = None;
        let mut sign_in = None;
        // What the message box and Send took last frame, left below the
        // log: a guess the first time, then measured, as its rows' height
        // depends on the font and the spacing on the theme
        let input_id = ui.id().with("agent-input-height");
        let input_height = ui.data(|d| d.get_temp::<f32>(input_id)).unwrap_or(160.0);
        scroll(ui, egui::ScrollArea::vertical().id_salt("agent-log").stick_to_bottom(true).auto_shrink([false, false]).max_height((ui.available_height() - input_height).max(80.0)), |ui| {
            for (i, e) in self.entries.iter().enumerate() {
                entry_ui(ui, t, i, e, &mut self.markdown);
            }
            if self.needs_auth
                && let Some(info) = &self.info
            {
                ui.add_space(6.0);
                for (id, name, desc) in &info.auth_methods {
                    if ui.button(format!("Sign in: {name}")).on_hover_text(desc).clicked() {
                        sign_in = Some(id.clone());
                    }
                }
            }
            if self.granting && !self.exited {
                ui.add_space(6.0);
                card(ui, t, |ui| {
                    ui.label(RichText::new("What may the agent do for this task without asking?").strong());
                    ui.label(
                        RichText::new(format!(
                            "Read, write and delete files in {}, {}and fetch from the web. Anything else, it asks about.",
                            self.cwd.display(),
                            if self.setup.tools.is_empty() { String::new() } else { format!("use {}'s tools, ", self.setup.app) }
                        ))
                            .small()
                            .color(t.text_dim),
                    );
                    if ui.checkbox(&mut self.commands, "Shell commands too").on_hover_text("A command can do anything you can, so they're asked about unless this is ticked").changed() {
                        self.commands_picked = Some(self.commands);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(RichText::new("Allow for this task").strong()).on_hover_text("Answer what's within this for the agent, until it ends").clicked() {
                            grant_pick = Some(Some(Grant { commands: self.commands }));
                        }
                        if ui.button("Ask each time").on_hover_text("Ask about every tool call the agent asks to make").clicked() {
                            grant_pick = Some(None);
                        }
                    });
                });
            }
            if let Some(a) = self.asking.front() {
                ui.add_space(6.0);
                card(ui, t, |ui| {
                    ui.label(RichText::new(format!("The agent asks: {}", a.title)).color(t.caution));
                    ui.horizontal_wrapped(|ui| {
                        for (id, name, _) in &a.options {
                            if ui.button(name).on_hover_text("Answer the agent so").clicked() {
                                answer = Some(Some(id.clone()));
                            }
                        }
                    });
                });
            }
        });
        if let Some(a) = answer {
            self.answer(a);
        }
        if let Some(g) = grant_pick {
            self.set_grant(g);
        }
        if let Some(m) = sign_in {
            self.sign_in(&ctx, m);
        }

        let input_top = ui.cursor().top();
        ui.separator();
        let ready = self.session.is_some() && self.turn.is_none() && !self.exited;
        let shown = ui.add_enabled_ui(!self.exited, |ui| partman_widgets::SendBox::new("agent-input", &mut self.input, box_height).hint("Answer or ask the agent…").enabled(ready).show(ui)).inner;
        shown.response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Message the agent"));
        self.input_resized |= shown.resized;
        if shown.sent {
            let text = std::mem::take(&mut self.input).trim().to_string();
            self.send(&ctx, text);
        }
        let took = ui.min_rect().bottom().max(ui.cursor().top()) - input_top + ui.spacing().item_spacing.y;
        if ui.data(|d| d.get_temp::<f32>(input_id)) != Some(took) {
            ui.data_mut(|d| d.insert_temp(input_id, took));
            ctx.request_repaint();
        }
        open
    }
}

/// Sets the session's settings to those the user picked before, where the
/// agent still offers them, in the order the agent lists its settings (a
/// model's choice can change what effort offers). One that fails is let
/// be: the session's still good, at the agent's default.
fn put_back(conn: &Conn, started: &mut Started, options: &std::collections::BTreeMap<String, String>) {
    // Taken from the list as it stands after each: setting one can bring
    // others, as a reasoning model brings its effort, which the session's
    // first list, for the agent's default model, may not have had
    let mut done = std::collections::BTreeSet::new();
    while let Some(c) = started.config.iter().find(|c| !done.contains(&c.id) && options.contains_key(&c.id)).cloned() {
        done.insert(c.id.clone());
        let want = &options[&c.id];
        if c.current == *want || !c.choices.iter().any(|(v, _)| v == want) {
            continue;
        }
        if let Ok(config) = acp::set_config(conn, &started.id, &c.id, want)
            && !config.is_empty()
        {
            started.config = config;
        }
    }
}

fn asking(id: serde_json::Value, params: &serde_json::Value) -> Asking {
    let s = |v: &serde_json::Value, k: &str| v.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let call = params.get("toolCall").cloned().unwrap_or_default();
    let mut title = s(&call, "title");
    if title.is_empty() {
        title = "to go ahead".into();
    }
    let options = params.get("options").and_then(|o| o.as_array()).map(|a| a.iter().map(|o| (s(o, "optionId"), s(o, "name"), s(o, "kind"))).collect()).unwrap_or_default();
    Asking { id, title, options, call: s(&call, "toolCallId"), kind: s(&call, "kind"), paths: call_paths(&call) }
}

/// The paths a tool call says it touches: its locations, and the path
/// fields agents put in its input. Not "cwd", where a command runs, which
/// says nothing of what it touches.
fn call_paths(c: &serde_json::Value) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = c.get("locations").and_then(|l| l.as_array()).into_iter().flatten().filter_map(|l| l.get("path").and_then(|p| p.as_str())).map(PathBuf::from).collect();
    if let Some(i) = c.get("rawInput") {
        for k in ["filePath", "file_path", "path", "filename", "source", "destination", "notebook_path"] {
            if let Some(p) = i.get(k).and_then(|v| v.as_str()) {
                out.push(PathBuf::from(p));
            }
        }
    }
    out
}

/// Whether a tool call is within what the user allowed: files only where
/// every path is known and in the folder; commands only if allowed, as
/// one can't be judged by its words; the app's own tools, by their MCP
/// server's name; and fetching and thinking, which change nothing here.
fn in_scope(kind: &str, title: &str, paths: &[PathBuf], folder: &std::path::Path, tools: &[McpServer], grant: Grant) -> bool {
    match kind {
        "read" | "search" | "edit" | "delete" | "move" => !paths.is_empty() && paths.iter().all(|p| inside(folder, p)),
        "execute" => grant.commands,
        "fetch" | "think" | "switch_mode" => true,
        // OpenCode names an MCP server's tools server_tool, Claude Code
        // mcp__server__tool
        _ => tools.iter().any(|m| title.starts_with(&format!("{}_", m.name)) || title.starts_with(&format!("mcp__{}__", m.name))),
    }
}

/// Whether path is in folder, after resolving ".." and the links of the
/// part that exists, so "folder/../x" or a link out of it isn't taken
/// for inside.
fn inside(folder: &std::path::Path, path: &std::path::Path) -> bool {
    use std::path::Component;
    let real = |p: &std::path::Path| -> PathBuf {
        let mut norm = PathBuf::new();
        for c in p.components() {
            match c {
                Component::ParentDir => {
                    norm.pop();
                }
                Component::CurDir => {}
                c => norm.push(c),
            }
        }
        // The longest part that exists, with its links resolved, and the rest
        let mut head = norm.clone();
        let mut tail = vec![];
        while !head.exists() {
            match (head.file_name().map(|n| n.to_owned()), head.parent().map(|p| p.to_path_buf())) {
                (Some(n), Some(p)) => {
                    tail.push(n);
                    head = p;
                }
                _ => break,
            }
        }
        let mut r = head.canonicalize().unwrap_or(head);
        for n in tail.into_iter().rev() {
            r.push(n);
        }
        r
    };
    let folder = real(folder);
    let p = if path.is_absolute() { path.to_path_buf() } else { folder.join(path) };
    real(&p).starts_with(&folder)
}

/// The text of a content block; other kinds by what they are.
fn content_text(c: Option<&serde_json::Value>) -> String {
    let Some(c) = c else { return String::new() };
    match c.get("type").and_then(|v| v.as_str()) {
        Some("text") => c.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        Some("resource_link") => format!("[{}]", c.get("name").and_then(|v| v.as_str()).unwrap_or("link")),
        Some(other) => format!("[{other}]"),
        None => String::new(),
    }
}

/// What a tool call shows when opened: its input, then what it produced.
fn tool_detail(u: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(i) = u.get("rawInput").filter(|v| !v.is_null()) {
        out += &match i.get("command").and_then(|c| c.as_str()) {
            Some(c) => c.to_string(),
            None => serde_json::to_string_pretty(i).unwrap_or_default(),
        };
    }
    for c in u.get("content").and_then(|c| c.as_array()).into_iter().flatten() {
        let text = match c.get("type").and_then(|v| v.as_str()) {
            Some("content") => content_text(c.get("content")),
            Some("diff") => format!("edited {}", c.get("path").and_then(|v| v.as_str()).unwrap_or("")),
            _ => String::new(),
        };
        if !text.is_empty() {
            if !out.is_empty() {
                out += "\n";
            }
            out += &text;
        }
    }
    // A long output is cut short: the panel is for following along
    if out.chars().count() > 4000 {
        out = out.chars().take(4000).collect::<String>() + "\n…";
    }
    out
}

/// One entry; its place in the list names its widgets, as neither text
/// nor an agent's tool call ids are sure to be unique.
fn entry_ui(ui: &mut egui::Ui, t: &Colors, i: usize, e: &Entry, markdown: &mut egui_commonmark::CommonMarkCache) {
    match e {
        Entry::User(text) => {
            ui.add_space(6.0);
            egui::Frame::new().fill(t.inset).corner_radius(4).inner_margin(6).show(ui, |ui| {
                ui.set_width(ui.available_width().max(0.0));
                // A task comes with its instructions: its first paragraph
                // says what it is
                match text.split_once("\n\n") {
                    Some((head, _)) if text.len() > 600 => {
                        ui.label(RichText::new(head).color(t.text));
                        egui::CollapsingHeader::new(RichText::new("The instructions given").small().color(t.text_dim)).id_salt(("task", i)).show(ui, |ui| {
                            ui.label(RichText::new(text).small().color(t.text_label));
                        });
                    }
                    _ => {
                        ui.label(RichText::new(text).color(t.text));
                    }
                }
            });
        }
        Entry::Agent(text) => {
            ui.add_space(4.0);
            // Agents answer in Markdown
            ui.push_id(("reply", i), |ui| {
                egui_commonmark::CommonMarkViewer::new().show(ui, markdown, text.trim());
            });
        }
        Entry::Thought(text) => {
            ui.add_space(2.0);
            egui::CollapsingHeader::new(RichText::new("Thinking").small().color(t.text_dim)).id_salt(("thought", i)).show(ui, |ui| {
                ui.label(RichText::new(text.trim()).small().color(t.text_dim));
            });
        }
        Entry::Tool { title, kind, status, detail, .. } => {
            let (mark, color) = match status.as_str() {
                "completed" => ("✔", t.good),
                "failed" => ("✖", t.bad),
                "in_progress" => ("…", t.accent),
                _ => ("•", t.text_dim),
            };
            let name = if title.is_empty() { kind } else { title };
            let head = RichText::new(format!("{mark} {}", name.lines().next().unwrap_or(""))).small().color(color);
            // A title can be a whole command line: kept to one line, as
            // egui's CollapsingHeader never wraps its title and would
            // widen the panel past the window
            let full = if name.contains('\n') || name.len() > 60 { format!("{name}\n\n{detail}") } else { detail.clone() };
            if full.trim().is_empty() {
                ui.add(egui::Label::new(head).truncate());
            } else {
                use egui::collapsing_header::{CollapsingState, paint_default_icon};
                let mut state = CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id(("tool", i)), false);
                let header = ui.horizontal(|ui| {
                    state.show_toggle_button(ui, paint_default_icon);
                    ui.add(egui::Label::new(head).truncate().sense(egui::Sense::click())).on_hover_text("Show or hide what the agent gave the tool")
                });
                if header.inner.clicked() {
                    state.toggle(ui);
                }
                state.show_body_indented(&header.response, ui, |ui| {
                    ui.label(RichText::new(full.trim()).small().monospace().color(t.text_label));
                });
            }
        }
        Entry::Plan(steps) => {
            ui.add_space(4.0);
            for (what, status) in steps {
                let mark = match status.as_str() {
                    "completed" => "☑",
                    "in_progress" => "▶",
                    _ => "☐",
                };
                ui.label(RichText::new(format!("{mark} {what}")).small().color(t.text_label));
            }
        }
        Entry::Note(text) => {
            ui.add_space(4.0);
            ui.label(RichText::new(text).small().italics().color(t.text_dim));
        }
    }
}

impl Drop for AgentRun {
    /// An agent left running would go on spending, unseen
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// In the folder means after "..", and through links, not by its words.
    #[test]
    fn inside_the_folder() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/inside-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scratch")).unwrap();
        assert!(inside(&dir, &dir.join("scratch/ds.txt")));
        assert!(inside(&dir, Path::new("scratch/new/page.png")), "relative, and not made yet");
        assert!(!inside(&dir, &dir.join("../elsewhere.txt")));
        assert!(!inside(&dir, Path::new("/tmp/ds.txt")));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/tmp", dir.join("out")).unwrap();
            assert!(!inside(&dir, &dir.join("out/ds.txt")), "a link out of it");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_a_grant_covers() {
        let dir = Path::new("/p/proj");
        let here = [PathBuf::from("/p/proj/x.json")];
        let away = [PathBuf::from("/home/u/.ssh/id_rsa")];
        let g = Grant { commands: false };
        // The app's own tools, by their server's name
        let tools = [McpServer { name: "pinplanner".into(), command: String::new(), args: vec![], env: vec![] }];
        assert!(in_scope("edit", "", &here, dir, &tools, g) && in_scope("read", "", &here, dir, &tools, g));
        assert!(!in_scope("read", "", &away, dir, &tools, g) && !in_scope("read", "", &[], dir, &tools, g), "unknown paths are asked");
        assert!(!in_scope("execute", "ls -la", &here, dir, &tools, g) && in_scope("execute", "ls -la", &here, dir, &tools, Grant { commands: true }));
        assert!(in_scope("other", "pinplanner_attach", &[], dir, &tools, g) && in_scope("other", "mcp__pinplanner__attach", &[], dir, &tools, g));
        assert!(!in_scope("other", "github_create_issue", &[], dir, &tools, g) && in_scope("fetch", "", &[], dir, &tools, g));
    }
}

