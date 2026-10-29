// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tables the way the toolbox's style guide (STYLE.md) has them: a header
//! over every column but a selection one, columns the user can adjust and
//! the last filling the room, rows selected by a checkbox or radio button
//! or by clicking anywhere in them that's inert, the actions on the
//! selection in a row of buttons below, and a blank row above and below.
//! Only the data rows scroll. Which rows are selected, the row last
//! clicked and where it's scrolled are kept in egui's memory, under the
//! table's id, so they hold across the app's steps and its runs: rows are
//! known by keys the app gives, which have to stay the same for a row.
//! A table that's a page of a longer list has the app sort it and keep its
//! selection, across its pages.

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

/// What a column's rows are sorted by, one for each row, in the rows'
/// order as given.
#[derive(Clone, Debug, PartialEq)]
pub enum SortBy {
    /// Text, compared without case
    Text(Vec<String>),
    Number(Vec<f64>),
    /// By the app, which is told the column and direction (`Shown::sorted`)
    /// and gives the rows in that order: a page of a list sorted whole
    App,
}

/// A column: its header, how wide it starts and can be made, and what
/// sorts it, if it can be.
#[derive(Clone, Debug, PartialEq)]
pub struct Col {
    title: String,
    width: f32,
    min: f32,
    sort: Option<SortBy>,
}

impl Col {
    pub fn new(title: impl Into<String>) -> Col {
        Col { title: title.into(), width: 120.0, min: 40.0, sort: None }
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

    /// Sortable by clicking its header, its rows by these.
    pub fn sort(mut self, by: SortBy) -> Col {
        self.sort = Some(by);
        self
    }
}

/// The order rows are shown in: by the column sorted on, if it can be,
/// ascending or not; ties, and without one, as given.
fn order(cols: &[Col], n: usize, sorted: Option<&(String, bool)>) -> Vec<usize> {
    let mut at: Vec<usize> = (0..n).collect();
    let Some((title, up)) = sorted else { return at };
    let Some(by) = cols.iter().find(|c| &c.title == title).and_then(|c| c.sort.as_ref()) else { return at };
    match by {
        SortBy::Text(v) if v.len() == n => at.sort_by_key(|&i| v[i].to_lowercase()),
        SortBy::Number(v) if v.len() == n => at.sort_by(|&a, &b| v[a].total_cmp(&v[b])),
        // Text or numbers of another count than the rows, or the app's
        _ => return at,
    }
    if !up {
        at.reverse();
    }
    at
}

/// What's kept of a table between frames and runs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    selected: BTreeSet<String>,
    /// The row last clicked, which a shift-click selects from
    anchor: Option<String>,
    /// The column sorted on, by title, and whether ascending
    sorted: Option<(String, bool)>,
}

/// The rows' view as it was at the end of the last frame it was drawn, for
/// telling a table just shown, a selection made by the app, and a resize,
/// each of which scrolls to the selection. Not kept between runs.
#[derive(Clone, Debug, Default)]
struct View {
    frame: u64,
    had_rows: bool,
    size: egui::Vec2,
    /// Any row shown was selected
    picked: bool,
    /// The selected rows in view, by key
    in_view: Vec<String>,
    /// Where to scroll to, decided after the rows were drawn: the next
    /// pass, or frame, starts there
    pending: Option<f32>,
}

/// The scroll offset that centres the rows from `lo` to `hi` (in the rows'
/// own coordinates) in a view `view` high, or puts `lo` at its top when
/// they're taller, within what can be scrolled to.
fn centred(lo: f32, hi: f32, view: f32, content: f32) -> f32 {
    let y = if hi - lo > view { lo } else { (lo + hi - view) / 2.0 };
    y.clamp(0.0, (content - view).max(0.0))
}

/// A double caret over the top (`up`) or bottom of the rows' view, for
/// selected rows out of sight that way, centred on `x`: footnote
/// sized, on a pill of the panel's colour so it reads over the rows.
fn out_of_view(ui: &mut Ui, view: egui::Rect, x: f32, up: bool, id: Id) {
    let s = ui.style().text_styles.get(&egui::TextStyle::Small).map_or(9.0, |f| f.size);
    let pill = egui::Vec2::new(s * 2.0, s * 1.3);
    let y = if up { view.top() + 2.0 + pill.y / 2.0 } else { view.bottom() - 2.0 - pill.y / 2.0 };
    let rect = egui::Rect::from_center_size(egui::pos2(x, y), pill);
    let name = if up { "Selected rows above" } else { "Selected rows below" };
    let r = ui.interact(rect, id.with(name), Sense::hover()).on_hover_text(format!("{name}: scroll to see them"));
    r.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, name));
    let painter = ui.painter().with_clip_rect(view);
    painter.rect_filled(rect, pill.y / 2.0, ui.visuals().panel_fill.gamma_multiply(0.9));
    let stroke = egui::Stroke::new(1.2, ui.visuals().strong_text_color());
    let (w, rise, step) = (s * 0.4, s * 0.22, s * 0.3);
    // Pointing up, the tip is at the top; down, at the bottom
    let tip = if up { -rise } else { rise };
    for dy in [-step / 2.0, step / 2.0] {
        let c = rect.center() + egui::vec2(0.0, dy);
        painter.line(vec![c + egui::vec2(-w, -tip), c + egui::vec2(0.0, tip), c + egui::vec2(w, -tip)], stroke);
    }
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
    heights: Option<&'a [f32]>,
    selection: Option<&'a mut BTreeSet<String>>,
    sorted: Option<(String, bool)>,
    click_opens: bool,
    names: Option<&'a [String]>,
}

/// What came of showing it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    /// The selected rows' keys, in the order shown
    pub selected: Vec<String>,
    /// The reload button was pressed
    pub reload: bool,
    /// The user changed the widths given by `widths`, for the app to keep
    pub resized: bool,
    /// The column sorted on, by title, and whether ascending
    pub sorted: Option<(String, bool)>,
    /// With `click_opens`, the row a plain click was on, by its place in
    /// keys as given
    pub opened: Option<usize>,
}

impl<'a> Table<'a> {
    /// A table under id, whose rows are known by keys, in the order shown.
    pub fn new(id: impl Hash + std::fmt::Debug, keys: &'a [String]) -> Table<'a> {
        Table {
            id: Id::new(id),
            keys,
            cols: vec![],
            select: Select::None,
            max_height: 300.0,
            scroll: true,
            row_height: None,
            reload: false,
            widths: None,
            heights: None,
            selection: None,
            sorted: None,
            click_opens: false,
            names: None,
        }
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

    /// Each row's height, by its place in keys as given, for rows of
    /// differing height (text wrapped); those not given are `row_height`'s.
    pub fn row_heights(mut self, heights: &'a [f32]) -> Table<'a> {
        self.heights = Some(heights);
        self
    }

    /// The selection, kept by the app rather than the table: for a page
    /// of a longer list, whose rows on other pages stay selected. Select
    /// all is then this page's rows, and `Shown::selected` every one.
    pub fn selection(mut self, selected: &'a mut BTreeSet<String>) -> Table<'a> {
        self.selection = Some(selected);
        self
    }

    /// The column sorted on (by title, and whether ascending) until the
    /// user picks one.
    pub fn sorted(mut self, title: impl Into<String>, ascending: bool) -> Table<'a> {
        self.sorted = Some((title.into(), ascending));
        self
    }

    /// A plain click on a row opens it (`Shown::opened`) rather than
    /// selecting it; Ctrl (Command) and Shift still select, as do the
    /// selection controls. For a list whose rows are opened to be seen.
    pub fn click_opens(mut self) -> Table<'a> {
        self.click_opens = true;
        self
    }

    /// What each row is, by its place in keys as given, for its selection
    /// control's name ("Select C1"): "Select row N" without.
    pub fn names(mut self, names: &'a [String]) -> Table<'a> {
        self.names = Some(names);
        self
    }

    /// A reload button among the actions below.
    pub fn reload(mut self) -> Table<'a> {
        self.reload = true;
        self
    }

    /// Draws it: `row` fills in a row's cells, by the row's place in keys
    /// as given, whatever order it's shown in,
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
        if state.sorted.is_none() {
            state.sorted = self.sorted.take();
        }
        if let Some(kept) = self.selection.as_deref() {
            state.selected = kept.clone();
        } else if !self.keys.is_empty() {
            // Rows gone are no longer selected; with none at all, they may
            // be loading still, so what's kept is let be
            state.selected.retain(|k| self.keys.contains(k));
        }
        let h = self.row_height.unwrap_or(row_height(ui) + 6.0);
        // The rows as shown, sorted or not; clicks and ranges go by these
        let shown_at = order(&self.cols, self.keys.len(), state.sorted.as_ref());
        let shown_keys: Vec<String> = shown_at.iter().map(|&i| self.keys[i].clone()).collect();
        let heights: Vec<f32> = shown_at.iter().map(|&i| self.heights.and_then(|hs| hs.get(i).copied()).unwrap_or(h)).collect();
        let view_id = self.id.with("view");
        let frame = ui.ctx().cumulative_frame_nr();
        let was: View = ui.data(|d| d.get_temp(view_id)).unwrap_or_default();
        // Drawn the frame before, or earlier in this one (a pass discarded),
        // with rows: one shown anew, or whose rows came, is first shown
        let continuing = was.had_rows && (was.frame == frame || was.frame + 1 == frame);
        let pending = was.pending;
        let mut sort_click: Option<String> = None;
        // The row shown, the modifiers, and whether on its selection control
        let mut clicked: Option<(usize, Modifiers, bool)> = None;
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
            // egui_extras' least before the rows scroll is 200, which would
            // take over a smaller most
            .min_scrolled_height(self.max_height.min(200.0))
            .cell_layout(Layout::left_to_right(egui::Align::Center))
            .sense(Sense::click());
        if let Some(y) = pending {
            b = b.vertical_scroll_offset(y);
        }
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
        let mut select_width = h;
        let every = !self.keys.is_empty() && self.keys.iter().all(|k| state.selected.contains(k));
        let rows = b.header(h, |mut head| {
            if self.select != Select::None {
                head.col(|ui| {
                    if self.select == Select::Many && !self.keys.is_empty() {
                        let mut on = every;
                        let r = ui.checkbox(&mut on, "").on_hover_text(if every { "Select none" } else { "Select all" });
                        r.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, every, "Select all"));
                        if r.clicked() {
                            all = Some(!every);
                        }
                    }
                });
            }
            for c in &self.cols {
                head.col(|ui| {
                    let r = ui.label(RichText::new(&c.title).strong());
                    if c.sort.is_some() {
                        // The whole header cell takes the click, its arrow at
                        // the right while it's the one sorted on
                        let cell = ui.max_rect();
                        let hit = ui.interact(cell, ui.id().with(("sort", &c.title)), Sense::click()).on_hover_text("Sort by this; again, the other way");
                        hit.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("Sort by {}", c.title)));
                        if hit.clicked() || r.clicked() {
                            sort_click = Some(c.title.clone());
                        }
                        if let Some((t, up)) = &state.sorted
                            && *t == c.title
                        {
                            let color = ui.visuals().strong_text_color();
                            let x = cell.right() - 8.0;
                            let y = cell.center().y;
                            let pts = if *up { vec![egui::pos2(x - 4.0, y + 2.5), egui::pos2(x + 4.0, y + 2.5), egui::pos2(x, y - 3.5)] } else { vec![egui::pos2(x - 4.0, y - 2.5), egui::pos2(x + 4.0, y - 2.5), egui::pos2(x, y + 3.5)] };
                            ui.painter().add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
                        }
                    }
                });
            }
        })
        .body(|mut body| {
            // Text left selectable would take the click meant for the row
            body.ui_mut().style_mut().interaction.selectable_labels = false;
            select_width = body.widths().first().copied().unwrap_or(h);
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
            // Only the rows in sight are drawn
            body.heterogeneous_rows(heights.iter().copied(), |mut r| {
                let shown = r.index();
                let i = shown_at[shown];
                let on = state.selected.contains(&shown_keys[shown]);
                {
                    r.set_selected(on);
                    let mut took = false;
                    if self.select != Select::None {
                        r.col(|ui| {
                            let resp = if self.select == Select::Many { ui.checkbox(&mut on.clone(), "") } else { ui.radio(on, "") };
                            // What a click does: a radio button on is already the one
                            let tip = match (self.select, on) {
                                (Select::Many, true) => "Deselect",
                                (Select::One, true) => "Selected",
                                _ => "Select",
                            };
                            let resp = resp.on_hover_text(tip);
                            let kind = if self.select == Select::Many { WidgetType::Checkbox } else { WidgetType::RadioButton };
                            let name = match self.names.and_then(|n| n.get(i)) {
                                Some(n) => format!("Select {n}"),
                                None => format!("Select row {}", shown + 1),
                            };
                            resp.widget_info(|| WidgetInfo::selected(kind, true, on, name.clone()));
                            if resp.clicked() {
                                took = true;
                                clicked = Some((shown, ui.input(|inp| inp.modifiers), true));
                            }
                        });
                    }
                    row(&mut r, i);
                    // A click on an inert part of the row; a link or button
                    // in it has taken its own
                    let resp = r.response();
                    if !took && resp.clicked() {
                        clicked = Some((shown, resp.ctx.input(|inp| inp.modifiers), false));
                    }
                }
            });
        });
        // Its rows' area, for scrolling over it to go by its height
        partman_scroll::area(ui.ctx(), &rows);

        // Each row's top and bottom, in the order shown, as egui_extras lays
        // them out: a row's height and the spacing below it
        let gap = ui.spacing().item_spacing.y;
        let mut spans = Vec::with_capacity(heights.len());
        let mut y = 0.0;
        for hh in &heights {
            spans.push((y, y + hh));
            y += hh + gap;
        }
        let (size, offset, content) = (rows.inner_rect.size(), rows.state.offset.y, rows.content_size.y);
        // As drawn, before this frame's clicks
        let picked: Vec<usize> = (0..shown_keys.len()).filter(|&s| state.selected.contains(&shown_keys[s])).collect();
        // A table that doesn't scroll its rows has none of this: the view
        // it's in scrolls
        let target = if !self.scroll || picked.is_empty() {
            None
        } else if !continuing || !was.picked {
            // First shown, or selected by the app from nothing: a click in
            // the table is in what was kept at the end of its frame
            let (lo, hi) = spans[picked[0]];
            Some(centred(lo, hi, size.y, content))
        } else if (size - was.size).length() > 0.5 && !was.in_view.is_empty() {
            let at: Vec<usize> = (0..shown_keys.len()).filter(|&s| was.in_view.contains(&shown_keys[s])).collect();
            match (at.first(), at.last()) {
                (Some(&a), Some(&z)) => Some(centred(spans[a].0, spans[z].1, size.y, content)),
                _ => None,
            }
        } else {
            None
        };
        let target = target.filter(|t| (t - offset).abs() > 0.5);
        if target.is_some() {
            // Shown where it was for not even a frame
            ui.ctx().request_discard("a table scrolled to its selection");
            ui.ctx().request_repaint();
        }
        let at = target.unwrap_or(offset);
        // Marks for selected rows wholly out of view; on a pass to be
        // discarded they'd be for where it's no longer scrolled
        if self.scroll && target.is_none() {
            // Over the selection controls, which they're about; there are
            // none to be out of view without that column
            let x = rows.inner_rect.left() + select_width / 2.0;
            if picked.iter().any(|&s| spans[s].1 <= at + 0.5) {
                out_of_view(ui, rows.inner_rect, x, true, self.id);
            }
            if picked.iter().any(|&s| spans[s].0 >= at + size.y - 0.5) {
                out_of_view(ui, rows.inner_rect, x, false, self.id);
            }
        }

        // Only this page's rows; others kept by the app stay as they are
        if let Some(on) = all {
            for k in self.keys {
                if on {
                    state.selected.insert(k.clone());
                } else {
                    state.selected.remove(k);
                }
            }
        }
        let mut opened = None;
        if let Some((shown, mods, control)) = clicked {
            if self.click_opens && !control && !mods.shift && !mods.command {
                opened = Some(shown_at[shown]);
            } else {
                click(&mut state, &shown_keys, shown, mods, self.select);
            }
        }
        if let Some(t) = sort_click {
            state.sorted = match state.sorted.take() {
                Some((was, up)) if was == t => Some((t, !up)),
                _ => Some((t, true)),
            };
        }
        let mut selected: Vec<String> = shown_keys.iter().filter(|k| state.selected.contains(*k)).cloned().collect();
        if let Some(kept) = self.selection.as_deref_mut() {
            *kept = state.selected.clone();
            selected.extend(kept.iter().filter(|k| !self.keys.contains(k)).cloned());
        }
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
        // With this frame's clicks, so a row the user selects isn't taken
        // for one the app did
        let selected_at: Vec<usize> = (0..shown_keys.len()).filter(|&s| state.selected.contains(&shown_keys[s])).collect();
        let in_view = selected_at.iter().filter(|&&s| spans[s].1 > at && spans[s].0 < at + size.y).map(|&s| shown_keys[s].clone()).collect();
        let view = View { frame, had_rows: !self.keys.is_empty(), size, picked: !selected_at.is_empty(), in_view, pending: target };
        ui.data_mut(|d| d.insert_temp(view_id, view));
        let sorted = state.sorted.clone();
        ui.data_mut(|d| d.insert_persisted(self.id, state));
        Shown { selected, reload, resized, sorted, opened }
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

    /// Sorted by clicking a sortable header, again the other way; the
    /// row callback still gets the row's place as given.
    #[test]
    fn sorted_by_header() {
        let keys: Vec<String> = ["b", "c", "a"].map(String::from).to_vec();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 300.0)).build_ui(|ui| {
            Table::new("sorting", &keys)
                .select(Select::Many)
                .column(Col::new("Name").sort(SortBy::Text(keys.clone())))
                .column(Col::new("Size").sort(SortBy::Number(vec![2.0, 30.0, 10.0])))
                .show(
                    ui,
                    |r, i| {
                        r.col(|ui| {
                            ui.label(format!("row {}", keys[i]));
                        });
                        r.col(|ui| {
                            ui.label("-");
                        });
                    },
                    |_, _| {},
                );
        });
        h.run();
        let tops = |h: &egui_kittest::Harness<'_>| ["a", "b", "c"].map(|k| h.get_by_label(&format!("row {k}")).rect().top());
        let [a, b, c] = tops(&h);
        assert!(b < c && c < a, "as given");
        h.get_by_label("Sort by Name").click();
        h.run();
        let [a, b, c] = tops(&h);
        assert!(a < b && b < c, "by name");
        h.get_by_label("Sort by Name").click();
        h.run();
        let [a, b, c] = tops(&h);
        assert!(c < b && b < a, "by name, down");
        h.get_by_label("Sort by Size").click();
        h.run();
        let [a, b, c] = tops(&h);
        assert!(b < a && a < c, "by size");
    }

    /// A page of a longer list: the app sorts it, told by the header
    /// clicks, its rows staying as given; it keeps the selection, rows on
    /// other pages and all, select all being this page's; a plain click
    /// opens a row, Ctrl and Shift select; rows have their own heights.
    #[test]
    fn a_page_the_app_sorts_and_selects() {
        struct P {
            keys: Vec<String>,
            selection: BTreeSet<String>,
            sorted: Option<(String, bool)>,
            opened: Option<usize>,
        }
        let p = P { keys: ["b", "c", "a"].map(String::from).to_vec(), selection: BTreeSet::from(["z".to_string()]), sorted: None, opened: None };
        let heights = [22.0, 60.0, 22.0];
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 400.0)).build_ui_state(
            move |ui, p: &mut P| {
                let keys = p.keys.clone();
                let shown = Table::new("page", &keys)
                    .select(Select::Many)
                    .selection(&mut p.selection)
                    .sorted("Name", true)
                    .click_opens()
                    .row_heights(&heights)
                    .names(&keys)
                    .column(Col::new("Name").sort(SortBy::App))
                    .column(Col::new("Other"))
                    .show(
                        ui,
                        |r, i| {
                            r.col(|ui| {
                                ui.label(format!("row {}", keys[i]));
                            });
                            r.col(|ui| {
                                ui.label("-");
                            });
                        },
                        |_, _| {},
                    );
                p.sorted = shown.sorted;
                if shown.opened.is_some() {
                    p.opened = shown.opened;
                }
            },
            p,
        );
        h.run();
        assert_eq!(h.state().sorted, Some(("Name".into(), true)), "the app's to start with");
        let tops = |h: &egui_kittest::Harness<'_, P>| ["b", "c", "a"].map(|k| h.get_by_label(&format!("row {k}")).rect().top());
        let [b, c, a] = tops(&h);
        assert!(b < c && c < a, "as given");
        // Centred in their rows, labels are half of one row and half of the
        // next apart: rows of 22 would be about 25
        assert!(c - b > 40.0 && a - c > 40.0, "c's row its own height: {b} {c} {a}");
        h.get_by_label("Sort by Name").click();
        h.run();
        assert_eq!(h.state().sorted, Some(("Name".into(), false)), "told the other way");
        let [b2, c2, a2] = tops(&h);
        assert_eq!([b, c, a], [b2, c2, a2], "not sorted by the table");
        // A plain click opens, Ctrl selects, Shift selects a range
        h.get_by_label("row c").click();
        h.run();
        assert_eq!(h.state().opened, Some(1));
        assert_eq!(h.state().selection, BTreeSet::from(["z".to_string()]));
        h.get_by_label("row b").click_modifiers(Modifiers::COMMAND);
        h.run();
        h.get_by_label("row a").click_modifiers(Modifiers::SHIFT);
        h.run();
        assert_eq!(h.state().selection.len(), 4, "b to a, and z from another page: {:?}", h.state().selection);
        // All on the page, and only it, selected: none of it, z kept
        h.get_by_label("Select all").click();
        h.run();
        assert_eq!(h.state().selection, BTreeSet::from(["z".to_string()]));
        h.get_by_label("Select all").click();
        h.run();
        assert_eq!(h.state().selection.len(), 4);
        // Named for what they are
        h.get_by_label("Select c").click();
        h.run();
        assert!(!h.state().selection.contains("c"));
    }

    /// Fifty rows, a view of a few, the selection and height the app's.
    struct L {
        keys: Vec<String>,
        selection: BTreeSet<String>,
        height: f32,
    }

    fn long(selected: &[&str]) -> egui_kittest::Harness<'static, L> {
        let l = L { keys: (0..50).map(|i| format!("r{i}")).collect(), selection: selected.iter().map(|s| s.to_string()).collect(), height: 150.0 };
        egui_kittest::Harness::builder().with_size(egui::vec2(600.0, 600.0)).build_ui_state(
            |ui, l: &mut L| {
                let keys = l.keys.clone();
                Table::new("long", &keys).select(Select::Many).selection(&mut l.selection).max_height(l.height).column(Col::new("Name")).show(
                    ui,
                    |r, i| {
                        r.col(|ui| {
                            ui.label(format!("row {}", keys[i]));
                        });
                    },
                    |ui, _| {
                        ui.label("Actions");
                    },
                );
            },
            l,
        )
    }

    /// Where a row is in the rows' view, 0 its top and 1 its bottom; None
    /// out of sight, as only the rows in sight are drawn.
    fn place(h: &egui_kittest::Harness<'_, L>, row: &str) -> Option<f32> {
        // The view is from under the header to the blank row above the actions
        let top = h.get_by_label("Name").rect().bottom();
        let bottom = top + h.state().height;
        let r = h.query_by_label(&format!("row {row}"))?.rect();
        (r.bottom() > top && r.top() < bottom).then(|| (r.center().y - top) / (bottom - top))
    }

    fn centre(h: &egui_kittest::Harness<'_, L>, row: &str) {
        let at = place(h, row);
        assert!(at.is_some_and(|a| (a - 0.5).abs() < 0.12), "{row} centred: {at:?}");
    }

    /// First shown, the first selected row is centred, and the selected
    /// rows below marked.
    #[test]
    fn first_shown_centres_the_selection() {
        let mut h = long(&["r30", "r45"]);
        h.run();
        centre(&h, "r30");
        assert!(h.query_by_label("Selected rows below").is_some() && h.query_by_label("Selected rows above").is_none());
    }

    /// Selected by the app from nothing, it's scrolled to; added to by the
    /// app, or clicked by the user, it isn't.
    #[test]
    fn selected_by_the_app_from_nothing() {
        let mut h = long(&[]);
        h.run();
        assert!(place(&h, "r0").is_some(), "at the top");
        h.state_mut().selection.insert("r40".into());
        h.run();
        centre(&h, "r40");
        // Added to: where it was, the new one above marked
        h.state_mut().selection.insert("r5".into());
        h.run();
        centre(&h, "r40");
        assert!(h.query_by_label("Selected rows above").is_some() && h.query_by_label("Selected rows below").is_none());
        // Cleared, and a row clicked by the user: not scrolled
        h.state_mut().selection.clear();
        h.run();
        let before = place(&h, "r41").unwrap();
        h.get_by_label("row r41").click();
        h.run();
        assert_eq!(h.state().selection, BTreeSet::from(["r41".to_string()]));
        assert!((place(&h, "r41").unwrap() - before).abs() < 0.01, "left where it was");
    }

    /// Resized, the selected rows in view stay in view, centred; once they
    /// don't fit, the first stays and the rest go off the bottom.
    #[test]
    fn resized_keeps_the_selection_in_view() {
        let mut h = long(&["r20", "r22"]);
        h.run();
        centre(&h, "r20");
        h.state_mut().height = 300.0;
        h.run();
        let (a, b) = (place(&h, "r20").unwrap(), place(&h, "r22").unwrap());
        assert!(((a + b) / 2.0 - 0.5).abs() < 0.12, "the two centred: {a} {b}");
        h.state_mut().height = 40.0;
        h.run();
        assert!(place(&h, "r20").is_some() && place(&h, "r22").is_none(), "r22 off the bottom");
        assert!(h.query_by_label("Selected rows below").is_some());
    }

    /// Hovered, the selection controls say what a click on them does.
    #[test]
    fn controls_say_what_they_do() {
        let mut h = harness(Select::Many);
        h.run();
        let tip = |h: &mut egui_kittest::Harness<'_, S>, over: &str, says: &str| {
            let at = h.get_by_label(over).rect().center();
            h.hover_at(at);
            h.run_steps(60);
            // The header's tip has its own name, so is found beside it
            let want = if says == over { 2 } else { 1 };
            assert_eq!(h.query_all_by_label(says).count(), want, "{over} says {says}");
            // Away, for the next to be found alone
            h.hover_at(egui::pos2(590.0, 390.0));
            h.run_steps(60);
        };
        tip(&mut h, "Select row 2", "Select");
        click(&mut h, "row b", Modifiers::NONE);
        tip(&mut h, "Select row 2", "Deselect");
        tip(&mut h, "Select all", "Select all");
        click(&mut h, "Select all", Modifiers::NONE);
        tip(&mut h, "Select all", "Select none");
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
