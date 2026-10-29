// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! An agent's panel with nothing behind it: made-up settings, usage and a
//! transcript, laid out as the pin planner's is, for seeing the sidebar
//! and what an app puts in it. Its controls do something small and local.

use egui::{Color32, RichText, Ui};
use partman_widgets::{SendBox, blank_row, check};

/// A tool call's state, as an agent reports it.
#[derive(Clone, Copy, PartialEq)]
enum Status {
    Running,
    Done,
    Failed,
}

#[derive(Clone)]
enum Entry {
    /// Said by the app, not the agent or the user
    Note(String),
    User(String),
    /// The agent's text: plain, and `code` between backticks
    Agent(String),
    /// Its plan's steps, and whether each is done
    Plan(Vec<(String, bool)>),
    Tool { title: String, status: Status, detail: String },
    /// A table the agent drew, its header first
    Table(Vec<Vec<String>>),
    /// The agent asks to do something, to be allowed or not
    Asks(String),
    /// What it may do without asking, asked once per task
    Permission,
}

pub struct Agent {
    entries: Vec<Entry>,
    model: usize,
    effort: usize,
    busy: bool,
    shell_too: bool,
    input: String,
    /// The input's height, as the user dragged it, kept between runs
    pub input_height: Option<f32>,
    /// The tool calls shown opened, by their place in the transcript
    opened: std::collections::BTreeSet<usize>,
}

const MODELS: [&str; 3] = ["Small", "Medium", "Large"];
const EFFORTS: [&str; 3] = ["Low", "Medium", "High"];

impl Agent {
    pub fn new(input_height: Option<f32>) -> Agent {
        let tool = |title: &str, status, detail: &str| Entry::Tool { title: title.into(), status, detail: detail.into() };
        Agent {
            entries: vec![
                Entry::Note("Starting Fake agent…".into()),
                Entry::Note("Fake agent 1.0 is ready.".into()),
                Entry::User("Find the documents for the STM32F103RGT6: the datasheet, the reference manual and the errata.".into()),
                Entry::Agent("Working on it: first what the part already has attached, then ST's site for the rest.".into()),
                Entry::Plan(vec![("Look at what's attached".into(), true), ("Find the datasheet".into(), true), ("Find the reference manual".into(), false), ("Find the errata".into(), false)]),
                tool("pinplanner part 1", Status::Done, "1 attachment: ds.pdf (Datasheet), 2.1 MB"),
                tool("curl -sL https://www.st.com/resource/en/reference_manual/rm0008.pdf -o rm0008.pdf", Status::Failed, "curl: (22) The requested URL returned error: 403"),
                tool("Fetch https://www.st.com/en/microcontrollers-microprocessors/stm32f103rg.html", Status::Done, "Documentation: DS5792 (datasheet), RM0008 (reference manual), ES0340 (errata)"),
                Entry::Agent("The datasheet attached is `DS5792` rev 13, the current one. The reference manual is `RM0008`; ST refused the direct download, so it came through the product page instead.".into()),
                Entry::Table(vec![
                    vec!["Doc".into(), "Number".into(), "Source".into()],
                    vec!["Datasheet".into(), "DS5792".into(), "attached".into()],
                    vec!["Reference manual".into(), "RM0008".into(), "st.com".into()],
                    vec!["Errata".into(), "ES0340".into(), "st.com".into()],
                ]),
                Entry::Note("Fake agent compacted the conversation to fit its context: what it read before now survives only as its summary, so details from it, such as a pin table, may be lost.".into()),
                tool("pinplanner attach 1 rm0008.pdf", Status::Running, "Uploading 13.0 MB…"),
                Entry::Permission,
                Entry::Asks("Run pinplanner attach 1 es0340.pdf".into()),
            ],
            model: 0,
            effort: 0,
            busy: true,
            shell_too: false,
            input: String::new(),
            input_height,
            opened: Default::default(),
        }
    }

    /// Whether it's waiting on the user, for the sidebar's control to say so.
    pub fn asks(&self) -> bool {
        self.entries.iter().any(|e| matches!(e, Entry::Asks(_) | Entry::Permission))
    }

    pub fn show(&mut self, ui: &mut Ui, said: &mut String) {
        ui.heading("Agent");
        ui.horizontal(|ui| {
            ui.label("Fake agent");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("End").on_hover_text("End the agent's session").clicked() {
                    self.busy = false;
                    self.entries.push(Entry::Note("The session ended.".into()));
                }
                if self.busy {
                    if ui.button("Stop").on_hover_text("Stop what the agent's doing, keeping the session").clicked() {
                        self.busy = false;
                        self.entries.push(Entry::Note("Stopped.".into()));
                    }
                    ui.spinner().on_hover_text("The agent is working");
                }
            });
        });
        ui.horizontal(|ui| {
            pick(ui, "Model", &MODELS, &mut self.model, "The model the agent uses, larger being slower and dearer");
            pick(ui, "Effort", &EFFORTS, &mut self.effort, "How hard the agent thinks before it answers");
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Context").small());
            ui.add(egui::ProgressBar::new(0.015).desired_width(80.0).desired_height(6.0)).on_hover_text("3.0k of the model's 200k tokens of context used");
            ui.label(RichText::new("3.0k of 200k · 27.4k in, 4 out · $0.00").small().weak());
            ui.label(RichText::new("· compacted").small().color(Color32::from_rgb(0xe0, 0xa0, 0x30)));
        });
        ui.label(RichText::new("Working in /tmp/proof-agent").small().weak());
        ui.separator();

        // The input at the bottom, the transcript taking what it leaves
        egui::Panel::bottom("proof-agent-input").frame(egui::Frame::NONE).show(ui, |ui| {
            ui.add_space(4.0);
            let shown = SendBox::new("proof-agent-input", &mut self.input, &mut self.input_height).hint("Answer or ask the agent…").show(ui);
            if shown.sent {
                let text = std::mem::take(&mut self.input);
                *said = format!("Sent the agent: {}", text.trim());
                self.entries.push(Entry::User(text.trim().to_string()));
                self.entries.push(Entry::Agent("A made-up answer: there's no agent behind this panel.".into()));
            }
        });
        egui::ScrollArea::vertical().id_salt("proof-agent-log").stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            let mut answer = None;
            for (i, e) in self.entries.iter().enumerate() {
                match e {
                    Entry::Note(t) => {
                        ui.label(RichText::new(t).small().italics().weak());
                    }
                    Entry::User(t) => {
                        egui::Frame::new().fill(ui.visuals().faint_bg_color).inner_margin(6).corner_radius(3).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(RichText::new(t).strong());
                        });
                    }
                    Entry::Agent(t) => {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            for (n, part) in t.split('`').enumerate() {
                                if n % 2 == 1 {
                                    ui.label(RichText::new(part).code());
                                } else {
                                    ui.label(part);
                                }
                            }
                        });
                    }
                    Entry::Plan(steps) => {
                        for (step, done) in steps {
                            let mark = if *done { "☑" } else { "☐" };
                            ui.label(RichText::new(format!("{mark} {step}")).small());
                        }
                    }
                    Entry::Tool { title, status, detail } => {
                        let (mark, color) = match status {
                            Status::Done => ("✔", Color32::from_rgb(0x4c, 0xaf, 0x50)),
                            Status::Failed => ("×", Color32::from_rgb(0xe0, 0x4a, 0x3f)),
                            Status::Running => ("•", ui.visuals().weak_text_color()),
                        };
                        let open = self.opened.contains(&i);
                        let text = RichText::new(format!("{} {mark} {title}", if open { "▼" } else { "▶" })).small().color(color);
                        let tip = if open { "Hide what the tool gave back" } else { "Show what the tool gave back" };
                        if ui.add(egui::Label::new(text).truncate().sense(egui::Sense::click())).on_hover_text(tip).clicked() {
                            answer = Some(Answer::Toggle(i));
                        }
                        if open {
                            ui.indent(("tool", i), |ui| {
                                ui.label(RichText::new(detail).small().monospace());
                            });
                        }
                    }
                    Entry::Table(rows) => {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            egui::Grid::new(("proof-agent-table", i)).striped(true).show(ui, |ui| {
                                for (n, row) in rows.iter().enumerate() {
                                    for cell in row {
                                        ui.label(if n == 0 { RichText::new(cell).strong() } else { RichText::new(cell) });
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                    }
                    Entry::Permission => {
                        blank_row(ui);
                        ui.indent("permission", |ui| {
                            ui.label("What may the agent do for this task without asking?");
                            ui.label(RichText::new("Read, write and delete files in /tmp/proof-agent, use the app's tools, and fetch from the web. Anything else, it asks about.").small().weak());
                            check(ui, &mut self.shell_too, "Shell commands too").on_hover_text("Let it run shell commands without asking as well");
                            ui.horizontal(|ui| {
                                if ui.button("Allow for this task").on_hover_text("Let it do these without asking, until the task ends").clicked() {
                                    answer = Some(Answer::Permission(true));
                                }
                                if ui.button("Ask each time").on_hover_text("Have it ask before each of these").clicked() {
                                    answer = Some(Answer::Permission(false));
                                }
                            });
                        });
                    }
                    Entry::Asks(what) => {
                        blank_row(ui);
                        ui.indent("asks", |ui| {
                            ui.label(RichText::new(format!("The agent asks: {what}")).color(Color32::from_rgb(0xe0, 0xa0, 0x30)));
                            ui.horizontal(|ui| {
                                if ui.button("Allow").on_hover_text("Let it do this, this once").clicked() {
                                    answer = Some(Answer::Asks(true));
                                }
                                if ui.button("Deny").on_hover_text("Don't let it; it's told so and carries on").clicked() {
                                    answer = Some(Answer::Asks(false));
                                }
                            });
                        });
                    }
                }
                ui.add_space(4.0);
            }
            match answer {
                Some(Answer::Toggle(i)) => {
                    if !self.opened.remove(&i) {
                        self.opened.insert(i);
                    }
                }
                Some(Answer::Permission(yes)) => {
                    self.entries.retain(|e| !matches!(e, Entry::Permission));
                    self.entries.push(Entry::Note(if yes { "Allowed for this task." } else { "It asks each time." }.into()));
                }
                Some(Answer::Asks(yes)) => {
                    self.entries.retain(|e| !matches!(e, Entry::Asks(_)));
                    self.entries.push(Entry::Note(if yes { "Allowed." } else { "Denied." }.into()));
                }
                None => {}
            }
        });
    }
}

enum Answer {
    Toggle(usize),
    Permission(bool),
    Asks(bool),
}

/// A drop-down of choices, its label before the one chosen.
fn pick(ui: &mut Ui, label: &str, choices: &[&str], at: &mut usize, tip: &str) {
    egui::ComboBox::from_id_salt(("proof-agent", label))
        .selected_text(format!("{label}: {}", choices[*at]))
        .show_ui(ui, |ui| {
            for (i, c) in choices.iter().enumerate() {
                ui.selectable_value(at, i, *c);
            }
        })
        .response
        .on_hover_text(tip);
}
