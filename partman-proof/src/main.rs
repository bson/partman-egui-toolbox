// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A proof sheet for the toolbox: its tables, buttons, text and agent
//! buttons on a board of cards, each card made to show some of how they
//! look and behave, to be seen in light and dark without an app.

use std::collections::BTreeSet;

use egui::{RichText, Ui};
use partman_card::{Card, Colors, Layout};
use partman_table::{Col, Select, SortBy, Table};
use partman_widgets::{AgentButton, Icon, IconButton, blank_row, check, explanation, radio};

/// The cards, by key and tab title.
const CARDS: [(&str, &str); 6] = [
    ("many", "Selecting many"),
    ("one", "Selecting one"),
    ("none", "No selection"),
    ("page", "A page"),
    ("buttons", "Buttons and text"),
    ("agents", "Agent tasks"),
];

/// The tables in one stack, the rest in another.
const DEFAULT: [&[&str]; 2] = [&["many", "one", "none", "page"], &["buttons", "agents"]];

const LAYOUT: &str = "layout";

/// Parts, made up, for rows: reference, value, package, quantity.
fn parts(n: usize) -> Vec<(String, String, &'static str, u32)> {
    const VALUES: [&str; 8] = ["10k", "4.7k", "100nF", "1uF", "22pF", "0R", "BAT54", "LED red"];
    const PACKAGES: [&str; 5] = ["0402", "0603", "0805", "SOD-323", "SOT-23"];
    (0..n)
        .map(|i| {
            let kind = if i % 3 == 0 { "C" } else if i % 5 == 0 { "D" } else { "R" };
            (format!("{kind}{}", i + 1), VALUES[i * 7 % VALUES.len()].to_string(), PACKAGES[i * 3 % PACKAGES.len()], (i * 37 % 50 + 1) as u32)
        })
        .collect()
}

struct Proof {
    layout: Layout,
    many: Vec<(String, String, &'static str, u32)>,
    many_keys: Vec<String>,
    /// The single selection, the app's, so it can select a row itself
    one: BTreeSet<String>,
    /// Which row "Select a row" picks next
    next: usize,
    one_height: f32,
    /// The page's rows, the app's selection and sort, and the row opened
    page_sel: BTreeSet<String>,
    /// The column the app sorts the page by, and whether ascending
    page_sort: (String, bool),
    opened: Option<String>,
    said: String,
    toggles: [bool; 2],
    choice: usize,
}

impl Proof {
    fn new(layout: Option<Layout>) -> Proof {
        let many = parts(60);
        let many_keys = many.iter().map(|p| p.0.clone()).collect();
        Proof {
            layout: layout.unwrap_or_default(),
            many,
            many_keys,
            one: BTreeSet::new(),
            next: 0,
            one_height: 200.0,
            page_sel: BTreeSet::new(),
            page_sort: ("Item".into(), true),
            opened: None,
            said: String::new(),
            toggles: [true, false],
            choice: 0,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        egui::Panel::top("bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Toolbox proof");
                ui.separator();
                // egui's own switch says nothing on a hover
                let now = ui.ctx().options(|o| o.theme_preference);
                for (pref, label, tip) in [
                    (egui::ThemePreference::System, "System", "Follow the system's light or dark theme"),
                    (egui::ThemePreference::Dark, "Dark", "Show everything in the dark theme"),
                    (egui::ThemePreference::Light, "Light", "Show everything in the light theme"),
                ] {
                    if ui.selectable_label(now == pref, label).on_hover_text(tip).clicked() {
                        ui.ctx().set_theme(pref);
                    }
                }
                if !self.said.is_empty() {
                    ui.separator();
                    ui.label(&self.said);
                }
            });
        });
        let colors = Colors::from_visuals(ui.visuals());
        egui::CentralPanel::default().frame(egui::Frame::new().fill(colors.canvas).inner_margin(12)).show(ui, |ui| {
            let cards: Vec<Card> = CARDS.iter().map(|(key, title)| Card { key, title, loading: false, close: false }).collect();
            let mut layout = std::mem::take(&mut self.layout);
            partman_card::board(ui, &colors, &mut layout, &cards, &DEFAULT, |ui, key| match key {
                "many" => self.many_card(ui),
                "one" => self.one_card(ui),
                "none" => none_card(ui),
                "page" => self.page_card(ui),
                "buttons" => self.buttons_card(ui),
                "agents" => self.agents_card(ui),
                _ => {}
            });
            self.layout = layout;
        });
    }

    /// Many selected, the table keeping them: sorting, ranges, select all,
    /// actions and an agent's below, and the selection kept in sight.
    fn many_card(&mut self, ui: &mut Ui) {
        ui.label("Click, Shift-click and Ctrl-click rows; sort by a header. Select rows far apart, switch to another tab and back: the first selected is centred, and carets mark those out of view.");
        let rows = &self.many;
        // Two closures say what was pressed, so it's in a cell of its own
        let said = std::cell::RefCell::new(None);
        Table::new("proof-many", &self.many_keys)
            .select(Select::Many)
            .column(Col::new("Reference").width(110.0).sort(SortBy::Text(rows.iter().map(|p| p.0.clone()).collect())))
            .column(Col::new("Value").width(110.0).sort(SortBy::Text(rows.iter().map(|p| p.1.clone()).collect())))
            .column(Col::new("Package").width(110.0).sort(SortBy::Text(rows.iter().map(|p| p.2.to_string()).collect())))
            .column(Col::new("Quantity").sort(SortBy::Number(rows.iter().map(|p| p.3 as f64).collect())))
            .max_height(220.0)
            .reload()
            .show_with_agents(
                ui,
                |r, i| {
                    let p = &rows[i];
                    r.col(|ui| {
                        ui.label(&p.0);
                    });
                    r.col(|ui| {
                        ui.label(&p.1);
                    });
                    r.col(|ui| {
                        ui.label(p.2);
                    });
                    r.col(|ui| {
                        ui.label(p.3.to_string());
                    });
                },
                |ui, sel| {
                    if IconButton::new(Icon::Delete).name("Delete the selected rows").enabled(!sel.is_empty()).show(ui).on_hover_text(needs(sel, "Delete the selected rows")).clicked() {
                        *said.borrow_mut() = Some(format!("Delete {}", sel.join(", ")));
                    }
                    if IconButton::new(Icon::Download).enabled(!sel.is_empty()).show(ui).on_hover_text(needs(sel, "Download the selected rows")).clicked() {
                        *said.borrow_mut() = Some(format!("Download {}", sel.join(", ")));
                    }
                },
                |ui, sel| {
                    if AgentButton::new("Review the selection").enabled(!sel.is_empty()).show(ui).on_hover_text(needs(sel, "Have an agent review the selected rows")).clicked() {
                        *said.borrow_mut() = Some(format!("An agent reviews {}", sel.join(", ")));
                    }
                },
            );
        if let Some(s) = said.into_inner() {
            self.said = s;
        }
    }

    /// One at a time, the app's selection: the app selecting a row from
    /// none scrolls to it, and resizing keeps it in view.
    fn one_card(&mut self, ui: &mut Ui) {
        ui.label("\"Select a row\" selects one as the app would, from none: the table scrolls to centre it. Change the height: the selected row stays centred.");
        ui.horizontal(|ui| {
            if ui.button("Select a row").on_hover_text("Clear the selection and select another row, as the app").clicked() {
                // Far apart, so each is out of sight of the last
                const PICKS: [usize; 4] = [42, 7, 55, 23];
                self.one = BTreeSet::from([self.many_keys[PICKS[self.next % PICKS.len()]].clone()]);
                self.next += 1;
            }
            if ui.button("Clear").on_hover_text("Select no row").clicked() {
                self.one.clear();
            }
            ui.add(egui::Slider::new(&mut self.one_height, 60.0..=400.0).text("Height")).on_hover_text("The most the table's rows take before they scroll");
        });
        let rows = &self.many;
        let mut said = None;
        Table::new("proof-one", &self.many_keys)
            .select(Select::One)
            .selection(&mut self.one)
            .column(Col::new("Reference").width(120.0).sort(SortBy::Text(rows.iter().map(|p| p.0.clone()).collect())))
            .column(Col::new("Value").sort(SortBy::Text(rows.iter().map(|p| p.1.clone()).collect())))
            .max_height(self.one_height)
            .show(
                ui,
                |r, i| {
                    r.col(|ui| {
                        ui.label(&rows[i].0);
                    });
                    r.col(|ui| {
                        ui.label(&rows[i].1);
                    });
                },
                |ui, sel| {
                    let tip = needs(sel, "Use the selected row");
                    if ui.add_enabled(!sel.is_empty(), egui::Button::new("Use")).on_hover_text(&tip).on_disabled_hover_text(&tip).clicked() {
                        said = Some(format!("Use {}", sel.join(", ")));
                    }
                },
            );
        if let Some(s) = said {
            self.said = s;
        }
    }

    /// A page of a longer list: the app sorts it, a plain click opens a
    /// row, rows have heights of their own.
    fn page_card(&mut self, ui: &mut Ui) {
        ui.label("A plain click opens a row; Ctrl and Shift select. The app sorts it; the long rows are taller.");
        let mut rows: Vec<(String, String)> = (1..=12)
            .map(|i| {
                let note = if i % 4 == 0 { "A longer note, in a row taller than the others: rows can have heights of their own, and this wraps when the card is narrow." } else { "Short" };
                (format!("Item {i:02}"), note.to_string())
            })
            .collect();
        let (by, up) = &self.page_sort;
        if by == "Note" {
            rows.sort_by(|a, b| a.1.cmp(&b.1));
        }
        if !up {
            rows.reverse();
        }
        let keys: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
        let heights: Vec<f32> = rows.iter().map(|r| if r.1.len() > 10 { 44.0 } else { 24.0 }).collect();
        let shown = Table::new("proof-page", &keys)
            .select(Select::Many)
            .selection(&mut self.page_sel)
            .sorted("Item", true)
            .click_opens()
            .row_heights(&heights)
            .names(&keys)
            .column(Col::new("Item").width(110.0).sort(SortBy::App))
            .column(Col::new("Note").sort(SortBy::App))
            .max_height(260.0)
            .show(
                ui,
                |r, i| {
                    r.col(|ui| {
                        ui.label(&rows[i].0);
                    });
                    r.col(|ui| {
                        ui.add(egui::Label::new(&rows[i].1).wrap());
                    });
                },
                |ui, sel| {
                    ui.label(format!("{} selected", sel.len()));
                },
            );
        if let Some(sorted) = shown.sorted {
            self.page_sort = sorted;
        }
        if let Some(i) = shown.opened {
            self.opened = Some(keys[i].clone());
        }
        if let Some(o) = &self.opened {
            ui.label(format!("Opened {o}"));
        }
    }

    fn buttons_card(&mut self, ui: &mut Ui) {
        ui.heading("Heading");
        ui.label("Body text, as most of a card is written. A sentence or two long enough to wrap when the card is narrow, to see how the lines sit.");
        ui.label(RichText::new("Strong text").strong());
        ui.label(RichText::new("Weak text, for what's said in passing").weak());
        ui.label(RichText::new("Small text, for footnotes").small());
        ui.label(RichText::new("Monospace: STM32F103RGT6 PA9 USART1_TX").monospace());
        ui.horizontal(|ui| {
            ui.label("A link:");
            ui.hyperlink_to("the toolbox", "https://git.rockgarden.net/bson/partman-egui-toolbox").on_hover_text("Open the toolbox's repository in the browser");
        });
        blank_row(ui);
        ui.strong("Buttons");
        let mut said = None;
        let mut press = |r: egui::Response, tip: &str| {
            if r.on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
                said = Some(format!("Pressed: {tip}"));
            }
        };
        ui.horizontal_wrapped(|ui| {
            press(ui.button("Button"), "A plain button: says it was pressed");
            press(ui.add_enabled(false, egui::Button::new("Disabled")), "A disabled button: never does anything");
            press(ui.button(RichText::new("Strong").strong()), "A button whose text is strong: says it was pressed");
            press(ui.selectable_label(true, "Selectable, on"), "A selectable label that's on: says it was pressed");
            press(ui.selectable_label(false, "Selectable, off"), "A selectable label that's off: says it was pressed");
        });
        ui.horizontal_wrapped(|ui| {
            for icon in ICONS {
                press(IconButton::new(icon).show(ui), &format!("{}: says it was pressed", icon.name()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for icon in ICONS {
                press(IconButton::new(icon).enabled(false).show(ui), &format!("{}, disabled: never does anything", icon.name()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            press(IconButton::new(Icon::Open).text("Partman").show(ui), "Open, with its text after: says it was pressed");
            press(IconButton::new(Icon::Check).name("Mark verified").color(egui::Color32::from_rgb(0x4c, 0xaf, 0x50)).show(ui), "Check in a colour that says something: says it was pressed");
            press(IconButton::new(Icon::Edit).text("Edit").show(ui), "Edit, with its text after: says it was pressed");
        });
        blank_row(ui);
        ui.strong("Choices");
        for (i, name) in ["A checkbox, on", "A checkbox, off"].iter().enumerate() {
            let on = self.toggles[i];
            check(ui, &mut self.toggles[i], *name).on_hover_text(if on { "Turn it off" } else { "Turn it on" });
        }
        for (i, name) in ["A radio button", "Another"].iter().enumerate() {
            let tip = if self.choice == i { "The one chosen" } else { "Choose this one" };
            if radio(ui, self.choice == i, *name).on_hover_text(tip).clicked() {
                self.choice = i;
            }
        }
        explanation(ui, "An explanation comes a blank row before the buttons it explains, and says what they'll do.");
        ui.horizontal(|ui| {
            press(ui.button("Do it"), "Do what the explanation says: says it was pressed");
            press(ui.button("Don't"), "Leave it: says it was pressed");
        });
        if let Some(s) = said {
            self.said = s;
        }
    }

    fn agents_card(&mut self, ui: &mut Ui) {
        ui.label("Agent task buttons: taller than others, so on a line of their own below them.");
        let mut said = None;
        let mut press = |r: egui::Response, tip: &str| {
            if r.on_hover_text(tip).clicked() {
                said = Some(format!("Pressed: {tip}"));
            }
        };
        ui.horizontal_wrapped(|ui| {
            press(ui.button("A plain button"), "A plain button, to compare heights with: says it was pressed");
            press(IconButton::new(Icon::Reload).show(ui), "An icon button, to compare heights with: says it was pressed");
        });
        ui.horizontal_wrapped(|ui| {
            for task in ["Extract pin info and attach", "Review the selection", "Find the part's documents"] {
                press(AgentButton::new(task).show(ui), &format!("Give an agent the task \"{task}\": says it was pressed"));
            }
        });
        ui.horizontal_wrapped(|ui| {
            press(AgentButton::new("Disabled").enabled(false).show(ui), "An agent task that can't be given now: never does anything");
            press(AgentButton::new("Another colour").color(egui::Color32::from_rgb(0x3f, 0x8f, 0xe0)).show(ui), "An agent task with the A in another colour: says it was pressed");
        });
        if let Some(s) = said {
            self.said = s;
        }
    }
}

/// A hover text for an action on the selection, saying when there's none
/// that it needs one.
fn needs(sel: &[String], what: &str) -> String {
    if sel.is_empty() { format!("{what}; select some first") } else { what.to_string() }
}

const ICONS: [Icon; 8] = [Icon::Delete, Icon::Download, Icon::Open, Icon::Reload, Icon::MoveUp, Icon::MoveDown, Icon::Check, Icon::Edit];

/// Nothing to do on rows: no selection column, links in a column of
/// their own still working.
fn none_card(ui: &mut Ui) {
    ui.label("With nothing to do on its rows, a table has no selection column.");
    let rows = [("Datasheet", "ds.pdf"), ("Reference manual", "rm.pdf"), ("Errata", "errata.pdf"), ("Pin mux", "pinmux.xml")];
    let keys: Vec<String> = rows.iter().map(|r| r.1.to_string()).collect();
    Table::new("proof-none", &keys)
        .column(Col::new("Document").width(160.0).sort(SortBy::Text(rows.iter().map(|r| r.0.to_string()).collect())))
        .column(Col::new("File").sort(SortBy::Text(keys.clone())))
        .show(
        ui,
        |r, i| {
            r.col(|ui| {
                ui.label(rows[i].0);
            });
            r.col(|ui| {
                let _ = ui.link(rows[i].1).on_hover_text(format!("Open {} (here, it does nothing)", rows[i].1));
            });
        },
        |_, _| {},
    );
}

impl eframe::App for Proof {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        partman_scroll::drive(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, LAYOUT, &self.layout);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("Toolbox proof").with_app_id("partman-proof").with_inner_size([1100.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native("partman-proof", options, Box::new(|cc| Ok(Box::new(Proof::new(cc.storage.and_then(|s| eframe::get_value(s, LAYOUT)))))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    /// Every card's tab brings it up, with what it's to show.
    #[test]
    fn every_card_shows() {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_ui_state(|ui, p: &mut Proof| p.show(ui), Proof::new(None));
        h.run();
        for (title, shows) in [("Selecting many", "Agent: Review the selection"), ("Selecting one", "Select a row"), ("No selection", "Datasheet"), ("A page", "Select Item 01"), ("Buttons and text", "Heading"), ("Agent tasks", "Agent: Find the part's documents")] {
            h.get_by_label(title).click();
            // A fitted stack takes a few frames to its card's height
            h.run_steps(10);
            h.run();
            assert!(h.query_by_label(shows).is_some(), "{title} shows {shows}");
        }
    }

    /// The app selecting a row from none scrolls the table to it.
    #[test]
    fn selecting_a_row_scrolls_to_it() {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_ui_state(|ui, p: &mut Proof| p.show(ui), Proof::new(None));
        h.run();
        h.get_by_label("Selecting one").click();
        h.run();
        assert!(h.query_by_label("Select row 43").is_none(), "out of sight to start");
        h.get_by_label("Select a row").click();
        h.run();
        assert_eq!(h.state().one, BTreeSet::from([h.state().many_keys[42].clone()]));
        assert!(h.query_by_label("Select row 43").is_some(), "scrolled to");
    }

    /// Every card in dark and light, as PNGs in the toolbox's tmp/, to look
    /// at without a display; needs a GPU: `cargo test -p partman-proof --
    /// --ignored`.
    #[test]
    #[ignore]
    fn screenshots() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
        std::fs::create_dir_all(&dir).unwrap();
        for (theme, visuals) in [("dark", egui::Visuals::dark()), ("light", egui::Visuals::light())] {
            let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1100.0, 800.0)).wgpu().build_ui_state(|ui, p: &mut Proof| p.show(ui), Proof::new(None));
            h.ctx.set_visuals(visuals);
            h.run_steps(10);
            for (n, (_, title)) in CARDS.iter().enumerate() {
                h.get_by_label(title).click();
                h.run_steps(10);
                if *title == "Selecting many" {
                    // One row scrolled above the view and one below, for both carets
                    h.get_by_label("Select row 2").click();
                    h.run_steps(3);
                    h.get_by_label("Select row 4").click_modifiers(egui::Modifiers::COMMAND);
                    h.run_steps(3);
                    let at = h.get_by_label("Select row 2").rect().center() + egui::vec2(200.0, 0.0);
                    h.hover_at(at);
                    h.event(egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -70.0), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE });
                    h.run_steps(10);
                }
                h.render().unwrap().save(dir.join(format!("proof-{theme}-{n}.png"))).unwrap();
            }
        }
    }
}
