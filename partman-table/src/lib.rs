// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tables the way the toolbox's style guide (STYLE.md) has them: a header
//! over every column but a selection one, columns the user can adjust and
//! the last filling the room, rows selected by a checkbox or radio button
//! or by clicking anywhere in them that's inert, the actions on the
//! selection in a row of buttons below, and a blank row above and below.
//! Only the data rows scroll. Which rows are selected, the row last
//! clicked and where it's scrolled are kept in egui's memory, under the
//! table's id, so they hold across the app's steps and its runs: rows are
//! known by keys the app gives, which have to stay the same for a row.

use std::collections::BTreeSet;
use std::hash::Hash;

use egui::{Id, Layout, Modifiers, RichText, Sense, Ui, WidgetInfo, WidgetType};
use egui_extras::{Column, TableBuilder, TableRow};
use partman_widgets::{Icon, IconButton, blank_row, row_height};
use serde::{Deserialize, Serialize};

/// How rows are selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Select {
    /// They aren't: no selection column
    #[default]
    None,
    /// One at a time, by radio button
    One,
    /// Any number, by checkbox, with one in the header for all
    Many,
}

/// A column: its header, and how wide it starts and can be made.
#[derive(Clone, Debug, PartialEq)]
pub struct Col {
    title: String,
    width: f32,
    min: f32,
}

impl Col {
    pub fn new(title: impl Into<String>) -> Col {
        Col { title: title.into(), width: 120.0, min: 40.0 }
    }

    /// How wide it starts. The last column takes the rest of the room.
    pub fn width(mut self, width: f32) -> Col {
        self.width = width;
        self
    }

    /// How narrow it can be made.
    pub fn at_least(mut self, min: f32) -> Col {
        self.min = min;
        self
    }
}

/// What's kept of a table between frames and runs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    selected: BTreeSet<String>,
    /// The row last clicked, which a shift-click selects from
    anchor: Option<String>,
}

/// A table, drawn by `show`.
pub struct Table<'a> {
    id: Id,
    keys: &'a [String],
    cols: Vec<Col>,
    select: Select,
    max_height: f32,
    scroll: bool,
    row_height: Option<f32>,
    reload: bool,
    widths: Option<&'a mut Vec<f32>>,
}

/// What came of showing it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    /// The selected rows' keys, in the rows' order
    pub selected: Vec<String>,
    /// The reload button was pressed
    pub reload: bool,
    /// The user changed the widths given by `widths`, for the app to keep
    pub resized: bool,
}

impl<'a> Table<'a> {
    /// A table under id, whose rows are known by keys, in the order shown.
    pub fn new(id: impl Hash + std::fmt::Debug, keys: &'a [String]) -> Table<'a> {
        Table { id: Id::new(id), keys, cols: vec![], select: Select::None, max_height: 300.0, scroll: true, row_height: None, reload: false, widths: None }
    }

    pub fn select(mut self, select: Select) -> Table<'a> {
        self.select = select;
        self
    }

    pub fn column(mut self, col: Col) -> Table<'a> {
        self.cols.push(col);
        self
    }

    /// The most the data rows take before they scroll.
    pub fn max_height(mut self, height: f32) -> Table<'a> {
        self.max_height = height;
        self
    }

    /// Whether the rows scroll once taller than `max_height`; without, the
    /// table is as tall as its rows, for a view that scrolls itself.
    pub fn scroll(mut self, scroll: bool) -> Table<'a> {
        self.scroll = scroll;
        self
    }

    /// The widths of the columns but the last, kept by the app (in its
    /// settings, say) rather than in egui's memory: they start the
    /// columns, and take the user's when a drag's over (`Shown::resized`).
    /// Of another count, they're the columns' own.
    pub fn widths(mut self, widths: &'a mut Vec<f32>) -> Table<'a> {
        self.widths = Some(widths);
        self
    }

    /// Each row's height; a row of text and a little by default.
    pub fn row_height(mut self, height: f32) -> Table<'a> {
        self.row_height = Some(height);
        self
    }

    /// A reload button among the actions below.
    pub fn reload(mut self) -> Table<'a> {
        self.reload = true;
        self
    }

    /// Draws it: `row` fills in a row's cells, by the row's place in keys,
    /// one `col` per column given; `actions` adds the buttons acting on the
    /// selection, given its keys, which are to be disabled when it's empty.
    pub fn show(self, ui: &mut Ui, row: impl FnMut(&mut TableRow<'_, '_>, usize), actions: impl FnOnce(&mut Ui, &[String])) -> Shown {
        self.draw(ui, row, actions, None)
    }

    /// `show`, with buttons giving an agent a task on the selection
    /// (partman-widgets' `AgentButton`) on a line of their own under the
    /// others, as the style guide has them, their size being another.
    pub fn show_with_agents(
        self,
        ui: &mut Ui,
        row: impl FnMut(&mut TableRow<'_, '_>, usize),
        actions: impl FnOnce(&mut Ui, &[String]),
        mut agents: impl FnMut(&mut Ui, &[String]),
    ) -> Shown {
        self.draw(ui, row, actions, Some(&mut agents))
    }

    fn draw(mut self, ui: &mut Ui, mut row: impl FnMut(&mut TableRow<'_, '_>, usize), actions: impl FnOnce(&mut Ui, &[String]), agents: Option<&mut dyn FnMut(&mut Ui, &[String])>) -> Shown {
        let mut state = ui.data_mut(|d| d.get_persisted::<State>(self.id)).unwrap_or_default();
        // Rows gone are no longer selected; with none at all, they may be
        // loading still, so what's kept is let be
        if !self.keys.is_empty() {
            state.selected.retain(|k| self.keys.contains(k));
        }
        let h = self.row_height.unwrap_or(row_height(ui) + 6.0);
        let mut clicked: Option<(usize, Modifiers)> = None;
        let mut all = None;

        blank_row(ui);
        let last = self.cols.len().saturating_sub(1);
        let mut widths: Vec<f32> = self.cols[..last].iter().map(|c| c.width).collect();
        if let Some(w) = self.widths.as_deref().filter(|w| w.len() == last) {
            widths = w.clone();
        }
        // Too wide for the room, they're scaled down to fit and fixed, under
        // an id of their own: a table wider than its card would widen the
        // card, and push what's laid out from its right edge out of sight
        let gaps = (self.cols.len() + usize::from(self.select != Select::None)) as f32 * ui.spacing().item_spacing.x;
        let fixed = gaps + if self.select != Select::None { h } else { 0.0 };
        let last_min = self.cols.last().map_or(0.0, |c| c.min);
        let room = ui.available_width();
        let natural: f32 = widths.iter().sum();
        let scale = (natural + fixed + last_min > room && natural > 0.0).then(|| ((room - fixed - last_min) / natural).max(0.3));
        let dragging = ui.input(|i| i.pointer.any_down());
        let mut b = TableBuilder::new(ui)
            .id_salt((self.id, scale.is_some()))
            .striped(true)
            .resizable(scale.is_none())
            .auto_shrink([false, true])
            .vscroll(self.scroll)
            .max_scroll_height(self.max_height)
            .cell_layout(Layout::left_to_right(egui::Align::Center))
            .sense(Sense::click());
        if self.select != Select::None {
            b = b.column(Column::exact(h));
        }
        for (i, c) in self.cols.iter().enumerate() {
            b = b.column(match (i == last, scale) {
                (true, _) => Column::remainder().at_least(c.min).clip(true),
                (false, Some(s)) => Column::exact((widths[i] * s).floor()).clip(true),
                (false, None) => Column::initial(widths[i]).at_least(c.min).clip(true),
            });
        }
        let first = usize::from(self.select != Select::None);
        let mut resized = false;
        let every = !self.keys.is_empty() && self.keys.iter().all(|k| state.selected.contains(k));
        b.header(h, |mut head| {
            if self.select != Select::None {
                head.col(|ui| {
                    if self.select == Select::Many && !self.keys.is_empty() {
                        let mut on = every;
                        let r = ui.checkbox(&mut on, "");
                        r.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, every, "Select all"));
                        if r.clicked() {
                            all = Some(!every);
                        }
                    }
                });
            }
            for c in &self.cols {
                head.col(|ui| {
                    ui.label(RichText::new(&c.title).strong());
                });
            }
        })
        .body(|mut body| {
            // Text left selectable would take the click meant for the row
            body.ui_mut().style_mut().interaction.selectable_labels = false;
            // A selected row's text is drawn in the selection's stroke
            // colour, which some themes have close to its fill: the strong
            // text colour reads on it
            let strong = body.ui_mut().visuals().strong_text_color();
            body.ui_mut().visuals_mut().selection.stroke.color = strong;
            // And a tint of the selection's fill rather than all of it, so
            // text with colours of its own (links, a status) reads on it too
            let fill = body.ui_mut().visuals().selection.bg_fill;
            body.ui_mut().visuals_mut().selection.bg_fill = fill.gamma_multiply(0.45);
            // The user's widths, taken once a drag's over, not at each step
            // of it; scaled to fit, they aren't the user's
            if let Some(kept) = self.widths.as_deref_mut()
                && scale.is_none()
                && !dragging
            {
                let now = body.widths()[first..first + last].to_vec();
                if now.len() == widths.len() && now.iter().zip(&widths).any(|(a, b)| (a - b).abs() > 0.5) {
                    *kept = now;
                    resized = true;
                }
            }
            for (i, key) in self.keys.iter().enumerate() {
                let on = state.selected.contains(key);
                body.row(h, |mut r| {
                    r.set_selected(on);
                    let mut took = false;
                    if self.select != Select::None {
                        r.col(|ui| {
                            let resp = if self.select == Select::Many { ui.checkbox(&mut on.clone(), "") } else { ui.radio(on, "") };
                            let kind = if self.select == Select::Many { WidgetType::Checkbox } else { WidgetType::RadioButton };
                            resp.widget_info(|| WidgetInfo::selected(kind, true, on, format!("Select row {}", i + 1)));
                            if resp.clicked() {
                                took = true;
                                clicked = Some((i, ui.input(|inp| inp.modifiers)));
                            }
                        });
                    }
                    row(&mut r, i);
                    // A click on an inert part of the row; a link or button
                    // in it has taken its own
                    let resp = r.response();
                    if !took && resp.clicked() {
                        clicked = Some((i, resp.ctx.input(|inp| inp.modifiers)));
                    }
                });
            }
        });

        if let Some(on) = all {
            state.selected = if on { self.keys.iter().cloned().collect() } else { BTreeSet::new() };
        }
        if let Some((i, mods)) = clicked {
            click(&mut state, self.keys, i, mods, self.select);
        }
        let selected: Vec<String> = self.keys.iter().filter(|k| state.selected.contains(*k)).cloned().collect();
        let mut reload = false;
        ui.horizontal_wrapped(|ui| {
            actions(ui, &selected);
            if self.reload && IconButton::new(Icon::Reload).show(ui).on_hover_text("Load the table again").clicked() {
                reload = true;
            }
        });
        if let Some(agents) = agents {
            ui.horizontal_wrapped(|ui| agents(ui, &selected));
        }
        blank_row(ui);
        ui.data_mut(|d| d.insert_persisted(self.id, state));
        Shown { selected, reload, resized }
    }
}

/// A row clicked, inert or its selection control: one selects it alone;
/// many toggles it, or with shift sets every row from the one last
/// clicked to it as it now is.
fn click(state: &mut State, keys: &[String], i: usize, mods: Modifiers, select: Select) {
    let key = &keys[i];
    match select {
        Select::None => return,
        Select::One => state.selected = BTreeSet::from([key.clone()]),
        Select::Many => {
            let on = !state.selected.contains(key);
            let from = state.anchor.as_ref().and_then(|a| keys.iter().position(|k| k == a)).filter(|_| mods.shift);
            let range = match from {
                Some(a) => a.min(i)..=a.max(i),
                None => i..=i,
            };
            for k in &keys[range] {
                if on {
                    state.selected.insert(k.clone());
                } else {
                    state.selected.remove(k);
                }
            }
        }
    }
    state.anchor = Some(key.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    struct S {
        keys: Vec<String>,
        select: Select,
        selected: Vec<String>,
        deleted: Vec<String>,
        opened: usize,
        reloads: usize,
    }

    fn harness(select: Select) -> egui_kittest::Harness<'static, S> {
        let s = S { keys: ["a", "b", "c", "d", "e"].map(String::from).to_vec(), select, selected: vec![], deleted: vec![], opened: 0, reloads: 0 };
        egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 400.0)).build_ui_state(
            |ui, s: &mut S| {
                let keys = s.keys.clone();
                let mut opened = 0;
                let mut deleted = vec![];
                let shown = Table::new("t", &keys).select(s.select).column(Col::new("Name").width(150.0)).column(Col::new("Link")).reload().show(
                    ui,
                    |r, i| {
                        r.col(|ui| {
                            ui.label(format!("row {}", keys[i]));
                        });
                        r.col(|ui| {
                            if ui.link(format!("open {}", keys[i])).clicked() {
                                opened += 1;
                            }
                        });
                    },
                    |ui, sel| {
                        if IconButton::new(Icon::Delete).enabled(!sel.is_empty()).show(ui).clicked() {
                            deleted = sel.to_vec();
                        }
                    },
                );
                s.opened += opened;
                if !deleted.is_empty() {
                    s.deleted = deleted;
                }
                s.reloads += usize::from(shown.reload);
                s.selected = shown.selected;
            },
            s,
        )
    }

    fn click(h: &mut egui_kittest::Harness<'_, S>, label: &str, mods: Modifiers) {
        h.get_by_label(label).click_modifiers(mods);
        h.run();
    }

    #[test]
    fn many_by_click_shift_and_all() {
        let mut h = harness(Select::Many);
        h.run();
        assert!(["Name", "Link"].iter().all(|c| h.query_by_label(c).is_some()), "headed");
        click(&mut h, "row b", Modifiers::NONE);
        assert_eq!(h.state().selected, ["b"]);
        click(&mut h, "row d", Modifiers::SHIFT);
        assert_eq!(h.state().selected, ["b", "c", "d"], "from the last clicked");
        click(&mut h, "row c", Modifiers::COMMAND);
        assert_eq!(h.state().selected, ["b", "d"], "one toggled");
        click(&mut h, "Select row 1", Modifiers::NONE);
        assert_eq!(h.state().selected, ["a", "b", "d"], "by its checkbox");
        // A link in a row does its own thing
        click(&mut h, "open e", Modifiers::NONE);
        assert_eq!((h.state().opened, h.state().selected.len()), (1, 3));
        click(&mut h, "Select all", Modifiers::NONE);
        assert_eq!(h.state().selected.len(), 5);
        click(&mut h, "Select all", Modifiers::NONE);
        assert!(h.state().selected.is_empty(), "all, and only all, selected: none");
    }

    #[test]
    fn actions_act_on_the_selection() {
        let mut h = harness(Select::Many);
        h.run();
        click(&mut h, "Delete", Modifiers::NONE);
        assert!(h.state().deleted.is_empty(), "disabled with none selected");
        click(&mut h, "row a", Modifiers::NONE);
        click(&mut h, "row c", Modifiers::NONE);
        click(&mut h, "Delete", Modifiers::NONE);
        assert_eq!(h.state().deleted, ["a", "c"]);
        click(&mut h, "Reload", Modifiers::NONE);
        assert_eq!(h.state().reloads, 1);
    }

    /// Agent buttons on a line of their own, under the others.
    #[test]
    fn agent_actions_on_their_own_line() {
        let keys: Vec<String> = vec!["a".into()];
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 300.0)).build_ui(|ui| {
            Table::new("agents", &keys).select(Select::Many).column(Col::new("Name")).reload().show_with_agents(
                ui,
                |r, _| {
                    r.col(|ui| {
                        ui.label("x");
                    });
                },
                |ui, _| {
                    IconButton::new(Icon::Delete).show(ui);
                },
                |ui, _| {
                    partman_widgets::AgentButton::new("Review").show(ui);
                },
            );
        });
        h.run();
        let (del, reload, agent) = (h.get_by_label("Delete").rect(), h.get_by_label("Reload").rect(), h.get_by_label("Agent: Review").rect());
        assert!((del.top() - reload.top()).abs() < 1.0 && agent.top() > del.bottom(), "{del:?} {reload:?} {agent:?}");
    }

    #[test]
    fn one_at_a_time() {
        let mut h = harness(Select::One);
        h.run();
        assert!(h.query_by_label("Select all").is_none());
        click(&mut h, "row b", Modifiers::NONE);
        click(&mut h, "row d", Modifiers::SHIFT);
        assert_eq!(h.state().selected, ["d"]);
        click(&mut h, "Select row 1", Modifiers::NONE);
        assert_eq!(h.state().selected, ["a"]);
    }

    /// Too wide for the room, the columns are scaled to fit it; with room,
    /// the widths an app keeps start them.
    #[test]
    fn fits_its_room_and_takes_kept_widths() {
        let keys: Vec<String> = vec!["a".into()];
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(300.0, 200.0)).build_ui(|ui| {
            Table::new("narrow", &keys).column(Col::new("Wide").width(400.0)).column(Col::new("More").width(300.0)).column(Col::new("Last")).show(
                ui,
                |r, _| {
                    for _ in 0..3 {
                        r.col(|ui| {
                            ui.label("x");
                        });
                    }
                },
                |_, _| {},
            );
        });
        h.run();
        assert!(h.get_by_label("Last").rect().right() <= 300.0, "{:?}", h.get_by_label("Last").rect());
        let mut kept = vec![50.0, 60.0];
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 200.0)).build_ui_state(
            |ui, kept: &mut Vec<f32>| {
                Table::new("kept", &keys).column(Col::new("One").width(200.0)).column(Col::new("Two").width(200.0)).column(Col::new("Three")).widths(kept).show(
                    ui,
                    |r, _| {
                        for _ in 0..3 {
                            r.col(|ui| {
                                ui.label("x");
                            });
                        }
                    },
                    |_, _| {},
                );
            },
            std::mem::take(&mut kept),
        );
        h.run();
        let (one, two) = (h.get_by_label("One").rect().left(), h.get_by_label("Two").rect().left());
        assert!((two - one - 50.0).abs() < 12.0, "started at the kept width: {one} {two}");
    }

    /// Kept while the rows are loading again, and those gone let go.
    #[test]
    fn kept_by_key() {
        let mut h = harness(Select::Many);
        h.run();
        click(&mut h, "row b", Modifiers::NONE);
        click(&mut h, "row c", Modifiers::NONE);
        h.state_mut().keys.clear();
        h.run();
        h.state_mut().keys = ["c", "b", "x"].map(String::from).to_vec();
        h.run();
        assert_eq!(h.state().selected, ["c", "b"], "by key, in the rows' new order");
        h.state_mut().keys = vec!["x".into()];
        h.run();
        assert!(h.state().selected.is_empty());
    }
}
