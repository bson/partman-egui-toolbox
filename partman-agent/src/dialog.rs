// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The agents dialog: the user's agent profiles, added from presets or by
//! hand, edited, and one picked as the default. The app keeps them, in
//! its settings, from what Save gives back.

use std::collections::HashMap;

use egui::{RichText, TextEdit};

use crate::panel::Colors;
use crate::profiles::{self, AgentProfile, ModelServer};
use crate::servers::{self, Model};
use crate::task::{Poll, Task};

/// What came of the dialog.
#[derive(Clone, Debug, PartialEq)]
pub enum Closed {
    /// Saved: the profiles, and the default's name
    Saved(Vec<AgentProfile>, Option<String>),
    Cancelled,
}

/// The dialog's working copy; settings change only on Save.
pub struct AgentsDialog {
    profiles: Vec<AgentProfile>,
    /// Which profile is the default, by place so renaming keeps it
    default: Option<usize>,
    selected: Option<usize>,
    /// The selected profile's command line and environment as typed
    line: String,
    env: String,
    error: Option<String>,
    /// The presets listed to add from
    adding: bool,
    /// What each model server was found to offer, asked once it's typed
    checks: HashMap<ModelServer, Check>,
}

enum Check {
    Asking(Task<Result<Vec<Model>, String>>),
    Done(Result<Vec<Model>, String>),
}

impl AgentsDialog {
    /// The dialog on the app's profiles, the one named `default` its
    /// default.
    pub fn new(profiles: &[AgentProfile], default: Option<&str>) -> AgentsDialog {
        let default = default.and_then(|n| profiles.iter().position(|p| p.name == n));
        let mut d = AgentsDialog { profiles: profiles.to_vec(), default, selected: None, line: String::new(), env: String::new(), error: None, adding: false, checks: HashMap::new() };
        d.select(default.or((!d.profiles.is_empty()).then_some(0)));
        d
    }

    fn select(&mut self, i: Option<usize>) {
        self.selected = i;
        if let Some(p) = i.and_then(|i| self.profiles.get(i)) {
            self.line = p.command_line();
            self.env = profiles::env_text(&p.env);
        }
    }

    /// Takes in what's typed for the selected profile; false (and an
    /// error said) if it doesn't read.
    fn commit(&mut self) -> bool {
        let Some(p) = self.selected.and_then(|i| self.profiles.get_mut(i)) else { return true };
        let mut words = profiles::split_words(&self.line);
        if words.is_empty() {
            self.error = Some(format!("{}: give the command that starts the agent", p.name));
            return false;
        }
        match profiles::parse_env(&self.env) {
            Ok(env) => p.env = env,
            Err(e) => {
                self.error = Some(format!("{}: {e}", p.name));
                return false;
            }
        }
        p.command = words.remove(0);
        p.args = words;
        self.error = None;
        true
    }

    fn unique_name(&self, base: &str) -> String {
        let mut name = base.to_string();
        let mut n = 2;
        while self.profiles.iter().any(|p| p.name == name) {
            name = format!("{base} {n}");
            n += 1;
        }
        name
    }

    fn add(&mut self, mut p: AgentProfile) {
        if !self.commit() {
            return;
        }
        p.name = self.unique_name(&p.name);
        if self.default.is_none() {
            self.default = Some(self.profiles.len());
        }
        self.profiles.push(p);
        self.select(Some(self.profiles.len() - 1));
    }

    /// Draws the dialog, `app` being what the app calls itself ("the pin
    /// planner"); what came of it once it's closed.
    pub fn show(&mut self, ctx: &egui::Context, t: &Colors, app: &str) -> Option<Closed> {
        let presets = profiles::presets();
        let mut pick = None;
        let mut add = None;
        let mut remove = false;
        let a = modal(ctx, t, |ui| {
            ui.label(RichText::new("Any agent speaking the Agent Client Protocol (ACP) can be used. Which one, and which model it runs, is up to you.").color(t.text_dim));
            ui.add_space(8.0);
            // A fixed height, so the dialog doesn't move as the list grows
            const BODY: f32 = 360.0;
            ui.horizontal_top(|ui| {
                ui.set_height(BODY);
                ui.vertical(|ui| {
                    ui.set_width(180.0);
                    scroll(ui, egui::ScrollArea::vertical().id_salt("agent-list").max_height(BODY).auto_shrink([false, false]), |ui| {
                        for (i, p) in self.profiles.iter().enumerate() {
                            let text = if self.default == Some(i) { RichText::new(format!("{} (default)", p.name)).strong() } else { RichText::new(&p.name) };
                            if ui.selectable_label(self.selected == Some(i), text).on_hover_text("Show this agent's settings").clicked() {
                                pick = Some(i);
                            }
                        }
                        if self.profiles.is_empty() {
                            ui.label(RichText::new("No agents yet").color(t.text_dim));
                        }
                        ui.add_space(6.0);
                        if ui.selectable_label(self.adding, "Add…").on_hover_text("Add an agent: one of those known, or another").clicked() {
                            self.adding = !self.adding;
                        }
                        if self.adding {
                            ui.indent("presets", |ui| {
                                for p in &presets {
                                    if ui.button(&p.profile.name).on_hover_text(format!("{}\nNeeds {}", p.profile.command_line(), p.needs)).clicked() {
                                        add = Some(p.profile.clone());
                                    }
                                }
                                if ui.button("Another agent").on_hover_text("An agent not listed: give its command yourself").clicked() {
                                    add = Some(AgentProfile { name: "Agent".into(), ..Default::default() });
                                }
                            });
                        }
                    });
                });
                ui.separator();
                ui.vertical(|ui| scroll(ui, egui::ScrollArea::vertical().id_salt("agent-settings").max_height(BODY).auto_shrink([false, false]), |ui| {
                    let Some(i) = self.selected.filter(|i| *i < self.profiles.len()) else {
                        ui.label(format!("Add an agent to use it from {app}."));
                        return;
                    };
                    ui.label("Name");
                    ui.add(TextEdit::singleline(&mut self.profiles[i].name).desired_width(f32::INFINITY));
                    ui.label("Command");
                    ui.add(TextEdit::singleline(&mut self.line).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                    let command = profiles::split_words(&self.line).into_iter().next().unwrap_or_default();
                    match profiles::find_program(&command) {
                        Some(p) => ui.label(RichText::new(format!("Found: {}", p.display())).small().color(t.good)),
                        None => ui.label(RichText::new(format!("\"{command}\" isn't found on the PATH")).small().color(t.caution)),
                    };
                    if let Some(p) = presets.iter().find(|p| p.profile.name == self.profiles[i].name) {
                        ui.label(RichText::new(format!("Needs {}", p.needs)).small().color(t.text_dim));
                    }
                    ui.add_space(4.0);
                    ui.label("Environment (NAME=value per line)");
                    ui.add(TextEdit::multiline(&mut self.env).desired_rows(3).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
                    ui.label(RichText::new("Kept in your settings file, readable by you alone.").small().color(t.text_dim));
                    if profiles::takes_servers(&command) {
                        ui.add_space(4.0);
                        ui.label("Model servers");
                        ui.label(RichText::new("Any speaking OpenAI's API: Ollama, LM Studio, llama.cpp, vLLM, or a service. Their models are offered each time the agent starts.").small().color(t.text_dim));
                        let mut drop = None;
                        for (k, sv) in self.profiles[i].servers.iter_mut().enumerate() {
                            let mut typing = false;
                            ui.horizontal(|ui| {
                                let url = ui.add(TextEdit::singleline(&mut sv.url).hint_text("http://host:port").desired_width(230.0).font(egui::TextStyle::Monospace));
                                let key = ui.add(TextEdit::singleline(&mut sv.key).hint_text("API key, if any").password(true).desired_width(120.0));
                                typing = url.has_focus() || key.has_focus();
                                if ui.button("Remove").on_hover_text("Stop offering this server's models").clicked() {
                                    drop = Some(k);
                                }
                            });
                            // Asked once it's typed, not on every key
                            if !typing && !sv.url.trim().is_empty() && !self.checks.contains_key(sv) {
                                let ask = sv.clone();
                                self.checks.insert(sv.clone(), Check::Asking(Task::spawn(ui.ctx(), move || servers::models(&ask))));
                            }
                            server_status(ui, t, self.checks.get_mut(sv));
                        }
                        if let Some(k) = drop {
                            self.profiles[i].servers.remove(k);
                        }
                        if ui.button("Add server").on_hover_text("Offer a model server's models to this agent").clicked() {
                            self.profiles[i].servers.push(ModelServer::default());
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let mut is_default = self.default == Some(i);
                        if ui.checkbox(&mut is_default, "Use by default").on_hover_text("The agent tasks run with unless one's chosen").changed() {
                            self.default = is_default.then_some(i);
                        }
                        if ui.button("Remove").on_hover_text("Remove this agent from the list").clicked() {
                            remove = true;
                        }
                    });
                }));
            });
            if let Some(e) = &self.error {
                ui.add_space(6.0);
                ui.label(RichText::new(e).color(t.caution));
            }
        });
        if let Some(i) = pick
            && self.commit()
        {
            self.select(Some(i));
        }
        if let Some(p) = add {
            self.adding = false;
            self.add(p);
        }
        if remove && let Some(i) = self.selected {
            self.profiles.remove(i);
            self.default = match self.default {
                Some(d) if d == i => None,
                Some(d) if d > i => Some(d - 1),
                d => d,
            };
            self.error = None;
            self.select((!self.profiles.is_empty()).then(|| i.min(self.profiles.len() - 1)));
        }
        if a.cancel {
            return Some(Closed::Cancelled);
        }
        if a.ok && self.commit() {
            let mut names: Vec<&str> = self.profiles.iter().map(|p| p.name.as_str()).collect();
            names.iter_mut().for_each(|n| *n = n.trim());
            names.sort();
            if names.windows(2).any(|w| w[0] == w[1]) || names.iter().any(|n| n.trim().is_empty()) {
                self.error = Some("Each agent needs a name of its own".into());
                return None;
            }
            let default = self.default.unwrap_or(0);
            let mut profiles = std::mem::take(&mut self.profiles);
            for p in &mut profiles {
                p.name = p.name.trim().to_string();
            }
            let default = profiles.get(default).map(|p| p.name.clone());
            return Some(Closed::Saved(profiles, default));
        }
        None
    }
}

/// What a model server was found to offer, under its row.
fn server_status(ui: &mut egui::Ui, t: &Colors, check: Option<&mut Check>) {
    let Some(c) = check else { return };
    if let Check::Asking(task) = c {
        match task.poll() {
            Poll::Pending => {
                ui.label(RichText::new("Asking for its models…").small().color(t.text_dim));
                return;
            }
            Poll::Ready(r) => *c = Check::Done(r),
            Poll::Lost => *c = Check::Done(Err("asking for its models failed".into())),
        }
    }
    let Check::Done(r) = c else { return };
    match r {
        Ok(m) if m.is_empty() => ui.label(RichText::new("It offers no models").small().color(t.caution)),
        Ok(m) => {
            let list: Vec<String> = m.iter().map(model_text).collect();
            ui.label(RichText::new(format!("Offers {}", list.join(", "))).small().color(t.good))
        }
        Err(e) => ui.label(RichText::new(e.as_str()).small().color(t.caution)),
    };
}

/// A model as listed: its context, when the server says, as it decides
/// how much of a datasheet the agent can take in at once.
fn model_text(m: &Model) -> String {
    match m.context {
        Some(c) => format!("{} ({}k context)", m.id, c / 1024),
        None => m.id.clone(),
    }
}


struct Answer {
    ok: bool,
    cancel: bool,
}

/// The dialog's window: its title, the body, then Save and Cancel (or
/// Escape).
fn modal(ctx: &egui::Context, t: &Colors, body: impl FnOnce(&mut egui::Ui)) -> Answer {
    let mut a = Answer { ok: false, cancel: false };
    egui::Modal::new(egui::Id::new("agents")).show(ctx, |ui| {
        ui.set_width(640.0);
        ui.label(RichText::new("Agents").size(16.0).strong().color(t.text));
        ui.add_space(8.0);
        body(ui);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            a.ok = ui.button("Save").on_hover_text("Keep these agents in your settings").clicked();
            a.cancel = ui.button("Cancel").on_hover_text("Close it, changing nothing (Escape)").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape));
        });
    });
    a
}

/// A scroll area reported to partman-scroll, as the toolbox's are.
fn scroll<R>(ui: &mut egui::Ui, area: egui::ScrollArea, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let shown = area.show(ui, add);
    partman_scroll::area(ui.ctx(), &shown);
    shown.inner
}
