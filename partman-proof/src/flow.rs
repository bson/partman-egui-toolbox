// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Two workflows, as the style guide has them: the app's own, its steps
//! in a bar under the top one and the board of cards the first of them,
//! kept between runs; and a modal one, a dialog with its own step bar
//! ending in the action the whole workflow does.

use std::collections::BTreeSet;

use egui::{RichText, Ui};
use partman_table::{Col, Select, SortBy, Table};
use partman_workflow::{Advance, Workflow};
use serde::{Deserialize, Serialize};

/// The made-up parts, by reference: value, package, quantity.
pub type Parts = [(String, String, &'static str, u32)];

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Step {
    #[default]
    Cards,
    Choose,
    Details,
    Summary,
}

const STEPS: [Step; 4] = [Step::Cards, Step::Choose, Step::Details, Step::Summary];

/// The app's workflow: an order of parts, made up. Saved with the app.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Flow {
    pub at: Step,
    pub chosen: BTreeSet<String>,
    pub name: String,
    pub quantity: u32,
    pub alternates: bool,
}

impl Workflow for Flow {
    type Step = Step;

    fn steps(&self) -> Vec<Step> {
        STEPS.to_vec()
    }

    fn current(&self) -> Step {
        self.at
    }

    fn label(&self, step: Step) -> String {
        match step {
            Step::Cards => "Cards",
            Step::Choose => "Choose parts",
            Step::Details => "Details",
            Step::Summary => "Summary",
        }
        .into()
    }

    fn enabled(&self, step: Step) -> bool {
        match step {
            Step::Cards | Step::Choose => true,
            Step::Details => !self.chosen.is_empty(),
            Step::Summary => !self.chosen.is_empty() && !self.name.trim().is_empty(),
        }
    }

    fn go(&mut self, step: Step) {
        self.at = step;
    }

    fn why_not(&self, step: Step) -> String {
        if self.chosen.is_empty() {
            "Choose some parts first".into()
        } else if step == Step::Summary {
            "Give the order a name first, in Details".into()
        } else {
            String::new()
        }
    }

    /// Pressed with nothing chosen or named, it says what's missing
    /// rather than being greyed, as the guide has it: pressing checks
    fn advance(&mut self, _ctx: &egui::Context, to: Step) -> Advance {
        if self.enabled(to) { Advance::Go } else { Advance::Refused(self.why_not(to)) }
    }
}

impl Flow {
    /// A step but the first, in a card work area: what it asks for, then
    /// the advance button at its bottom right, or on the last step Place
    /// order, which starts over. What to say is returned: why it couldn't
    /// go on, or that the order was placed.
    pub fn step_ui(&mut self, ui: &mut Ui, parts: &Parts, keys: &[String]) -> Option<String> {
        partman_workflow::card_area(ui, partman_workflow::Width { max: 900.0, min: 480.0 }, |ui| {
            // A card, as the guide's work area has one: the panel's colour
            // on the canvas
            egui::Frame::new().fill(ui.visuals().panel_fill).corner_radius(6).inner_margin(14).show(ui, |ui| {
                ui.set_width(ui.available_width());
                match self.at {
                    Step::Cards => {}
                    Step::Choose => {
                        ui.strong("Choose parts");
                        ui.label("The parts the order is for. Details is unlocked once some are chosen; pressing Details → with none says so.");
                        // The table takes what's left of the work area, less
                        // its header, the blank rows around it and before
                        // the advance row, its own row of actions and the
                        // advance row, and the card's margin
                        let row = partman_widgets::row_height(ui);
                        let s = ui.spacing();
                        let below = (row + 6.0) + 3.0 * (1.5 * row + 2.0 * s.item_spacing.y) + 2.0 * (s.interact_size.y + s.item_spacing.y) + 2.0 * 14.0;
                        let fit = partman_workflow::fit_height(ui, below, 5.0 * (row + 6.0));
                        Table::new("proof-flow-choose", keys)
                            .select(Select::Many)
                            .selection(&mut self.chosen)
                            .column(Col::new("Reference").width(120.0).sort(SortBy::Text(parts.iter().map(|p| p.0.clone()).collect())))
                            .column(Col::new("Value").width(120.0).sort(SortBy::Text(parts.iter().map(|p| p.1.clone()).collect())))
                            .column(Col::new("Package").sort(SortBy::Text(parts.iter().map(|p| p.2.to_string()).collect())))
                            .scroll(fit.is_some())
                            .max_height(fit.unwrap_or(f32::INFINITY))
                            .show(
                                ui,
                                |r, i| {
                                    r.col(|ui| {
                                        ui.label(&parts[i].0);
                                    });
                                    r.col(|ui| {
                                        ui.label(&parts[i].1);
                                    });
                                    r.col(|ui| {
                                        ui.label(parts[i].2);
                                    });
                                },
                                |ui, sel| {
                                    ui.label(format!("{} chosen", sel.len()));
                                },
                            );
                    }
                    Step::Details => {
                        ui.strong("Details");
                        ui.label("Summary is unlocked once the order has a name.");
                        partman_widgets::blank_row(ui);
                        egui::Grid::new("proof-flow-details").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                            ui.label("Name");
                            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("What the order's called").desired_width(240.0)).on_hover_text("The order's name, as it's listed");
                            ui.end_row();
                            ui.label("Quantity");
                            ui.add(egui::DragValue::new(&mut self.quantity).range(1..=1000)).on_hover_text("How many of each part; drag or type");
                            ui.end_row();
                        });
                        partman_widgets::check(ui, &mut self.alternates, "Allow alternates").on_hover_text("Let parts with the same value and package stand in for those chosen");
                    }
                    Step::Summary => {
                        ui.strong("Summary");
                        ui.label(format!("{}: {}, {} of each{}.", self.name.trim(), parts_count(self.chosen.len()), self.quantity.max(1), if self.alternates { ", alternates allowed" } else { "" }));
                        ui.label(RichText::new("The last step: nothing comes after it, so its button does what the workflow is for, and cycles back to the start. Its state is kept with the app between runs.").weak());
                    }
                }
                partman_widgets::blank_row(ui);
                if self.at != Step::Summary {
                    return partman_workflow::advance_row(ui, self, |_| {});
                }
                // The workflow's action, placing the order, then a new one
                let placed = ui.horizontal(|ui| ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| partman_workflow::finish_button(ui, "Place order", "Place the order, and start a new one from the beginning", Ok(()))).inner).inner;
                placed.then(|| {
                    let said = format!("Placed {}", self.name.trim());
                    *self = Flow::default();
                    said
                })
            })
            .inner
        })
    }
}

fn parts_count(n: usize) -> String {
    if n == 1 { "1 part".into() } else { format!("{n} parts") }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriceStep {
    Search,
    Review,
}

/// The dialog's workflow: prices checked for parts found, made up. Not
/// kept: cancelling it, or closing it with its action, ends it.
#[derive(Default)]
pub struct Prices {
    review: bool,
    query: String,
    picked: BTreeSet<String>,
    in_stock: bool,
}

impl Workflow for Prices {
    type Step = PriceStep;

    fn steps(&self) -> Vec<PriceStep> {
        vec![PriceStep::Search, PriceStep::Review]
    }

    fn current(&self) -> PriceStep {
        if self.review { PriceStep::Review } else { PriceStep::Search }
    }

    fn label(&self, step: PriceStep) -> String {
        format!("{step:?}")
    }

    fn enabled(&self, step: PriceStep) -> bool {
        step == PriceStep::Search || !self.picked.is_empty()
    }

    fn go(&mut self, step: PriceStep) {
        self.review = step == PriceStep::Review;
    }

    fn why_not(&self, _step: PriceStep) -> String {
        "Pick some parts found first".into()
    }
}

/// What came of the dialog this frame.
pub enum Outcome {
    Open,
    Cancelled,
    /// Its action done, for the app to say
    Done(String),
}

impl Prices {
    pub fn show(&mut self, ctx: &egui::Context, parts: &Parts) -> Outcome {
        let mut outcome = Outcome::Open;
        let shown = egui::Modal::new(egui::Id::new("proof-prices")).show(ctx, |ui| {
            ui.set_width(620.0);
            ui.strong("Check prices");
            ui.add_space(6.0);
            ui.horizontal(|ui| partman_workflow::step_bar(ui, &partman_workflow::Colors::from_visuals(ui.visuals()), self));
            partman_widgets::blank_row(ui);
            let q = self.query.trim().to_lowercase();
            let found: Vec<usize> = (0..parts.len()).filter(|&i| q.is_empty() || [parts[i].0.to_lowercase(), parts[i].1.to_lowercase(), parts[i].2.to_lowercase()].iter().any(|t| t.contains(&q))).collect();
            match self.current() {
                PriceStep::Search => {
                    ui.add(egui::TextEdit::singleline(&mut self.query).hint_text("Reference, value or package").desired_width(f32::INFINITY)).on_hover_text("Narrow the parts down to those matching this");
                    let keys: Vec<String> = found.iter().map(|&i| parts[i].0.clone()).collect();
                    Table::new("proof-prices-search", &keys)
                        .select(Select::Many)
                        .names(&keys)
                        .selection(&mut self.picked)
                        .column(Col::new("Reference").width(120.0).sort(SortBy::Text(found.iter().map(|&i| parts[i].0.clone()).collect())))
                        .column(Col::new("Value").sort(SortBy::Text(found.iter().map(|&i| parts[i].1.clone()).collect())))
                        .max_height(200.0)
                        .show(
                            ui,
                            |r, n| {
                                let i = found[n];
                                r.col(|ui| {
                                    ui.label(&parts[i].0);
                                });
                                r.col(|ui| {
                                    ui.label(&parts[i].1);
                                });
                            },
                            |ui, sel| {
                                ui.label(format!("{} picked", sel.len()));
                            },
                        );
                }
                PriceStep::Review => {
                    let keys: Vec<String> = self.picked.iter().cloned().collect();
                    // Made up, but steady: from the reference's number
                    let price = |k: &str| k.trim_start_matches(|c: char| c.is_ascii_alphabetic()).parse::<u32>().map_or(0.10, |n| 0.02 + (n % 17) as f64 * 0.013);
                    let prices: Vec<f64> = keys.iter().map(|k| price(k)).collect();
                    Table::new("proof-prices-review", &keys)
                        .column(Col::new("Reference").width(120.0).sort(SortBy::Text(keys.clone())))
                        .column(Col::new("Price").sort(SortBy::Number(prices.clone())))
                        .max_height(200.0)
                        .show(
                            ui,
                            |r, i| {
                                r.col(|ui| {
                                    ui.label(&keys[i]);
                                });
                                r.col(|ui| {
                                    ui.label(format!("${:.3}", prices[i]));
                                });
                            },
                            |_, _| {},
                        );
                    partman_widgets::check(ui, &mut self.in_stock, "Only distributors with stock").on_hover_text("Take prices only from distributors that have the parts in stock");
                }
            }
            partman_widgets::blank_row(ui);
            ui.horizontal(|ui| {
                if ui.button("Cancel").on_hover_text("Close it, changing nothing (Escape)").clicked() {
                    outcome = Outcome::Cancelled;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.current() == PriceStep::Review {
                        // The whole workflow's action, closing it
                        let ready = if self.picked.is_empty() { Err("Pick some parts first".to_string()) } else { Ok(()) };
                        if partman_workflow::finish_button(ui, "Update", "Update the parts' prices with these, and close", ready) {
                            outcome = Outcome::Done(format!("Updated the prices of {}", parts_count(self.picked.len())));
                        }
                    } else {
                        partman_workflow::advance_button(ui, self);
                    }
                });
            });
        });
        if shown.should_close() && matches!(outcome, Outcome::Open) {
            outcome = Outcome::Cancelled;
        }
        outcome
    }
}
