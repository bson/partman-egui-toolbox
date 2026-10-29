// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The agent sidebar's contents: partman-agent's panel, with its made-up
//! agent at work below a made-up transcript, to see the panel as an app
//! shows it without a real agent behind it.

use partman_agent::{AgentRun, Colors, Entry, Setup};

pub struct Agent {
    /// The made-up agent, once the first frame gives it a context; none
    /// once its session is ended
    run: Option<AgentRun>,
    started: bool,
    /// The input's height, as the user dragged it, kept between runs
    pub input_height: Option<f32>,
}

impl Agent {
    pub fn new(input_height: Option<f32>) -> Agent {
        Agent { run: None, started: false, input_height }
    }

    /// Whether it waits on the user, for the sidebar's control to say so.
    pub fn asks(&self) -> bool {
        self.run.as_ref().is_some_and(|r| r.asking().is_some() || r.granting)
    }

    pub fn show(&mut self, ui: &mut egui::Ui, said: &mut String) {
        if !self.started {
            self.started = true;
            self.start(ui.ctx(), said);
        }
        ui.heading("Agent");
        let Some(run) = &mut self.run else {
            ui.label("The made-up agent's session was ended.");
            if ui.button("Start it again").on_hover_text("Start the made-up agent, below the same made-up transcript").clicked() {
                self.start(ui.ctx(), said);
            }
            return;
        };
        if !run.ui(ui, &Colors::from_visuals(ui.visuals()), &mut self.input_height) {
            self.run = None;
            *said = "Ended the made-up agent's session".into();
        }
    }

    fn start(&mut self, ctx: &egui::Context, said: &mut String) {
        let setup = Setup {
            client: partman_agent::acp::Client { name: "partman-proof".into(), title: "the proof sheet".into(), version: env!("CARGO_PKG_VERSION").into() },
            app: "the proof sheet".into(),
            tools: vec![],
        };
        match partman_agent::fake::start(ctx, &setup, std::env::temp_dir().join("proof-agent"), transcript()) {
            Ok(run) => self.run = Some(run),
            Err(e) => *said = format!("The made-up agent didn't start: {e}"),
        }
    }
}

/// What was said before the made-up agent starts: a task, a reply in
/// Markdown, a plan, tool calls that passed and failed, a table, and a
/// note, so every kind of entry is there to see. The agent's own notes,
/// starting, come below it.
fn transcript() -> Vec<Entry> {
    let tool = |id: &str, title: &str, kind: &str, status: &str, detail: &str| Entry::Tool { id: id.into(), title: title.into(), kind: kind.into(), status: status.into(), detail: detail.into() };
    let plan = |steps: &[(&str, &str)]| Entry::Plan(steps.iter().map(|(s, st)| (s.to_string(), st.to_string())).collect());
    vec![
        Entry::User("Find the documents for the STM32F103RGT6: the datasheet, the reference manual and the errata.".into()),
        Entry::Thought("The part's attachments first, then ST's product page for what's missing.".into()),
        Entry::Agent("Working on it: first what the part already has attached, then ST's site for the rest.".into()),
        plan(&[("Look at what's attached", "completed"), ("Find the datasheet", "completed"), ("Find the reference manual", "in_progress"), ("Find the errata", "pending")]),
        tool("t1", "pinplanner part 1", "execute", "completed", "1 attachment: ds.pdf (Datasheet), 2.1 MB"),
        tool("t2", "curl -sL https://www.st.com/resource/en/reference_manual/rm0008.pdf -o rm0008.pdf", "execute", "failed", "curl: (22) The requested URL returned error: 403"),
        tool("t3", "Fetch https://www.st.com/en/microcontrollers-microprocessors/stm32f103rg.html", "fetch", "completed", "Documentation: DS5792 (datasheet), RM0008 (reference manual), ES0340 (errata)"),
        Entry::Agent(
            "The datasheet attached is `DS5792` rev 13, the current one. The reference manual is `RM0008`; ST refused the direct download, so it came through the product page instead.\n\n\
             | Doc | Number | Source |\n|---|---|---|\n| Datasheet | DS5792 | attached |\n| Reference manual | RM0008 | st.com |\n| Errata | ES0340 | st.com |"
                .into(),
        ),
        Entry::Note("Fake agent compacted the conversation to fit its context: what it read before now survives only as its summary, so details from it, such as a pin table, may be lost.".into()),
    ]
}
