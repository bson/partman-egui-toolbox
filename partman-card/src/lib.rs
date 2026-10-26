// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A board of cards: a page's information split into cards, each with a
//! tab, kept in stacks the user moves, resizes and rearranges. Tabs reorder
//! within a stack when dragged sideways; dragged up or down, their card
//! comes off as a stack of its own, which can be dropped on another
//! stack's bar to join it. Stacks snap to each other's edges and the
//! board's. A board knows nothing of what its cards show: the page draws
//! each card's contents, by its key, and keeps the layout between runs.

use std::collections::HashMap;

use egui::{Align, Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

/// The colours a board is drawn in, from the app's theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    /// The page under the stacks; a card's body is between it and chrome
    pub canvas: Color32,
    /// Stack bars, and around the page
    pub chrome: Color32,
    /// A tab under the pointer
    pub hover: Color32,
    /// A stack's bar when a card dragged would join it
    pub accent: Color32,
    pub text: Color32,
    pub text_dim: Color32,
}

impl Colors {
    /// Colours taken from egui's own visuals, for an app without a theme.
    pub fn from_visuals(v: &egui::Visuals) -> Colors {
        Colors {
            canvas: v.extreme_bg_color,
            chrome: v.panel_fill,
            hover: v.widgets.hovered.weak_bg_fill,
            accent: v.selection.bg_fill,
            text: v.strong_text_color(),
            text_dim: v.weak_text_color(),
        }
    }
}

impl Default for Colors {
    fn default() -> Colors {
        Colors::from_visuals(&egui::Visuals::dark())
    }
}

/// The stack bar's height, holding the tabs.
const BAR: f32 = 28.0;
/// Between stacks, about two characters, showing the page between them.
pub const GAP: f32 = 16.0;
/// Inside a card, around its contents.
const PAD: f32 = 8.0;
/// How near an edge has to come to snap to it.
const SNAP: f32 = 10.0;
/// The resize grip in a stack's bottom right corner.
const GRIP: f32 = 14.0;
const MIN: Vec2 = vec2(180.0, 100.0);
/// A new card's size.
const NEW: Vec2 = vec2(420.0, 260.0);
/// Beside a tab's title.
const TAB_PAD: f32 = 12.0;
const MIN_TAB_PAD: f32 = 4.0;
/// Before the first tab and above them all.
const TAB_INSET: f32 = 4.0;
/// How far a pressed tab moves before it's told which way it's dragged.
const DECIDE: f32 = 4.0;

/// A card as a page has it this frame.
pub struct Card<'a> {
    /// What it's known by in the saved layout; never shown
    pub key: &'a str,
    pub title: &'a str,
    /// Shows a spinner on its stack while it's the one on top
    pub loading: bool,
    /// Can be closed: alone in its stack, it has a close mark at its bar's
    /// right end, and pressing it is said by `Layout::take_closed`, for the
    /// app to remove it (or not, having asked)
    pub close: bool,
}

/// Where a board's stacks are and what they hold.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub stacks: Vec<Stack>,
    /// Still the page's own arrangement, which follows the window's size;
    /// the user moving or resizing a stack ends that
    pub fitted: bool,
    #[serde(skip)]
    drag: Option<Drag>,
    /// The stacks, and whether fitted, as they were when the drag began,
    /// for Escape to put back
    #[serde(skip)]
    before: Option<(Vec<Stack>, bool)>,
    /// The height each card's contents took last frame, which a fitted
    /// stack, other than the last, is sized to
    #[serde(skip)]
    heights: HashMap<String, f32>,
    /// Where the board's content was on the screen last frame, and the
    /// part of the board seen, scrolled to: a new card's room is looked
    /// for there, and drops on the board are told apart by it
    #[serde(skip)]
    origin: Vec2,
    #[serde(skip)]
    seen: Option<Rect>,
    /// The card whose close mark was pressed, for the app to take
    #[serde(skip)]
    closed: Option<String>,
    /// The stacks on the screen as last drawn, in drawing order, each with
    /// its tabs, and the board's clip: what's under a drag over the board
    #[serde(skip)]
    drawn: Vec<(Rect, Vec<(String, Rect)>)>,
    #[serde(skip)]
    clip: Option<Rect>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stack {
    /// x, y, width, height on the board
    pub rect: [f32; 4],
    /// Its cards' keys, in their tabs' order
    pub cards: Vec<String>,
    /// The card shown
    pub top: String,
}

impl Stack {
    fn of(cards: Vec<String>, rect: Rect) -> Stack {
        let top = cards.first().cloned().unwrap_or_default();
        let mut s = Stack { rect: [0.0; 4], cards, top };
        s.set_rect(rect);
        s
    }

    pub fn rect(&self) -> Rect {
        let [x, y, w, h] = self.rect;
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    fn set_rect(&mut self, r: Rect) {
        self.rect = [r.min.x, r.min.y, r.width(), r.height()];
    }

    fn bar(&self) -> Rect {
        let r = self.rect();
        Rect::from_min_max(r.min, pos2(r.max.x, r.min.y + BAR))
    }
}

/// What the pointer is doing to the layout, between the press and the
/// release. Followed here rather than through the widgets' own dragging,
/// as a card coming off its stack moves its tab to another.
#[derive(Clone, Debug, PartialEq)]
enum Drag {
    /// A tab pressed at at (its own corner at tab), not yet moved far
    /// enough to tell rearranging tabs from moving its card
    Pressed { stack: usize, key: String, at: Pos2, tab: Pos2 },
    Tabs { stack: usize, key: String },
    /// A stack moved, held grab from its corner; card when it's a card
    /// taken off a stack, which may join another
    Move { stack: usize, grab: Vec2, card: bool },
    /// Held grab from its bottom right corner
    Resize { stack: usize, grab: Vec2 },
}

/// raw, or the nearest of to within SNAP of it.
fn snap(raw: f32, to: impl Iterator<Item = f32>) -> f32 {
    to.map(|c| (c, (c - raw).abs())).filter(|c| c.1 <= SNAP).min_by(|a, b| a.1.total_cmp(&b.1)).map_or(raw, |c| c.0)
}

/// Where a stack's tabs are, given each title's width.
fn tab_rects(stack: Rect, keys: &[String], width: impl Fn(&str) -> f32) -> Vec<Rect> {
    // Too many to fit: their padding goes first, down to MIN_TAB_PAD, then
    // each narrows alike, its title clipped
    let widths: Vec<f32> = keys.iter().map(|k| width(k)).collect();
    let (room, text) = (stack.width() - 2.0 * TAB_INSET, widths.iter().sum::<f32>());
    let n = keys.len().max(1) as f32;
    let pad = ((room - text) / (2.0 * n)).clamp(MIN_TAB_PAD, TAB_PAD);
    let squeeze = ((room - 2.0 * pad * n) / text).clamp(0.0, 1.0);
    let mut x = stack.min.x + TAB_INSET;
    widths
        .iter()
        .map(|w| {
            let r = Rect::from_min_max(pos2(x, stack.min.y + TAB_INSET), pos2(x + w * squeeze + 2.0 * pad, stack.min.y + BAR));
            x = r.max.x;
            r
        })
        .collect()
}

impl Layout {
    /// Makes it hold the cards given, each once: the default arrangement
    /// when it holds none yet, otherwise cards no longer given leave and
    /// new ones join the last stack.
    pub fn arrange(&mut self, keys: &[&str], default: &[&[&str]]) {
        let mut seen = std::collections::HashSet::new();
        for s in &mut self.stacks {
            s.cards.retain(|k| keys.contains(&k.as_str()) && seen.insert(k.clone()));
            if !s.cards.contains(&s.top) {
                s.top = s.cards.first().cloned().unwrap_or_default();
            }
        }
        self.stacks.retain(|s| !s.cards.is_empty());
        if self.stacks.is_empty() {
            self.stacks = default.iter().map(|keys| Stack::of(keys.iter().map(|k| k.to_string()).collect(), Rect::ZERO)).collect();
            self.fitted = true;
            seen = self.stacks.iter().flat_map(|s| s.cards.iter().cloned()).collect();
        }
        let missing: Vec<String> = keys.iter().filter(|k| !seen.contains(**k)).map(|k| k.to_string()).collect();
        if missing.is_empty() {
            return;
        }
        match self.stacks.last_mut() {
            Some(s) => s.cards.extend(missing),
            None => self.stacks.push(Stack::of(missing, Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0)))),
        }
    }

    /// A fitted layout's stacks one under another, across the board, each
    /// as tall as its top card's contents and the last taking the rest.
    fn fit(&mut self, area: Vec2) {
        if !self.fitted {
            return;
        }
        let mut y = 0.0;
        let n = self.stacks.len();
        for (i, s) in self.stacks.iter_mut().enumerate() {
            let h = if i + 1 == n {
                (area.y - y).max(MIN.y)
            } else {
                BAR + 2.0 * PAD + self.heights.get(&s.top).copied().unwrap_or(120.0)
            };
            s.set_rect(Rect::from_min_size(pos2(0.0, y), vec2(area.x, h)));
            y += h + GAP;
        }
    }

    /// The card whose close mark was pressed since the last asked, if one
    /// was: the app removes it, the layout letting it go once it's no
    /// longer among the cards given.
    pub fn take_closed(&mut self) -> Option<String> {
        self.closed.take()
    }

    /// Puts the card on top of its stack.
    pub fn show(&mut self, key: &str) {
        if let Some(s) = self.stacks.iter_mut().find(|s| s.cards.iter().any(|k| k == key)) {
            s.top = key.to_string();
        }
    }

    /// A new card in a stack of its own: where the part of the board seen
    /// has room for all of it, the first such place from the top left; with
    /// none, under the others. While the layout follows the window, just
    /// above the last stack, which keeps taking what's left.
    pub fn add_alone(&mut self, key: &str) {
        if self.fitted {
            let at = self.stacks.len().saturating_sub(1);
            self.stacks.insert(at, Stack::of(vec![key.to_string()], Rect::ZERO));
            return;
        }
        let r = self.room(NEW).unwrap_or_else(|| {
            let bottom = self.stacks.iter().map(|s| s.rect().max.y).fold(0.0, f32::max);
            Rect::from_min_size(pos2(0.0, bottom + GAP), NEW)
        });
        self.stacks.push(Stack::of(vec![key.to_string()], r));
    }

    /// A place of size within the part of the board seen that no stack
    /// is on or within GAP of: the topmost, then leftmost, among the seen
    /// part's corner and the places beside and under the stacks.
    fn room(&self, size: Vec2) -> Option<Rect> {
        let seen = self.seen?;
        if seen.width() < size.x || seen.height() < size.y {
            return None;
        }
        let mut xs: Vec<f32> = std::iter::once(seen.min.x).chain(self.stacks.iter().map(|s| s.rect().max.x + GAP)).collect();
        let mut ys: Vec<f32> = std::iter::once(seen.min.y).chain(self.stacks.iter().map(|s| s.rect().max.y + GAP)).collect();
        xs.sort_by(f32::total_cmp);
        ys.sort_by(f32::total_cmp);
        let clear = |r: Rect| seen.contains_rect(r) && !self.stacks.iter().any(|s| s.rect().expand(GAP - 0.5).intersects(r));
        ys.iter().flat_map(|&y| xs.iter().map(move |&x| Rect::from_min_size(pos2(x, y), size))).find(|r| clear(*r))
    }

    /// Where on the board a point on the screen is, if it's on the part
    /// seen and on no stack: somewhere a dragged thing can make a card.
    pub fn blank_at(&self, screen: Pos2) -> Option<Pos2> {
        if self.fitted {
            return None;
        }
        let p = screen - self.origin;
        (self.seen?.contains(p) && !self.stacks.iter().any(|s| s.rect().contains(p))).then_some(p)
    }

    /// Where a new card put at a point on the board (from `blank_at`) goes:
    /// its bar's left under the point, kept within the part seen.
    pub fn card_rect_at(&self, at: Pos2) -> Rect {
        let seen = self.seen.unwrap_or(Rect::from_min_size(Pos2::ZERO, NEW));
        let x = (at.x - GAP).clamp(seen.min.x, (seen.max.x - NEW.x).max(seen.min.x));
        let y = (at.y - BAR / 2.0).clamp(seen.min.y, (seen.max.y - NEW.y).max(seen.min.y));
        Rect::from_min_size(pos2(x, y), NEW)
    }

    /// The same as screen, for drawing where it'd go.
    pub fn card_screen_rect_at(&self, at: Pos2) -> Rect {
        self.card_rect_at(at).translate(self.origin)
    }

    /// A new card in a stack of its own put at a point on the board.
    pub fn add_at(&mut self, key: &str, at: Pos2) {
        let r = self.card_rect_at(at);
        self.stacks.push(Stack::of(vec![key.to_string()], r));
    }

    /// The card whose tab is at a point on the screen, with the tab's rect,
    /// as last drawn: for something dragged over the board to be dropped
    /// on that card, whether it's on top or not. A tab under another stack
    /// isn't there.
    pub fn tab_at(&self, screen: Pos2) -> Option<(&str, Rect)> {
        if !self.clip?.contains(screen) {
            return None;
        }
        let (_, tabs) = self.drawn.iter().rev().find(|(r, _)| r.contains(screen))?;
        tabs.iter().find(|(_, r)| r.contains(screen)).map(|(k, r)| (k.as_str(), *r))
    }

    /// Whether the card is on top of its stack.
    pub fn on_top(&self, key: &str) -> bool {
        self.stacks.iter().any(|s| s.top == key)
    }

    /// Stack i drawn last, over the others; its new index.
    fn raise(&mut self, i: usize) -> usize {
        let s = self.stacks.remove(i);
        self.stacks.push(s);
        self.stacks.len() - 1
    }

    /// r moved to stack i's place, snapped to the board's and the other
    /// stacks' edges, beside them or lined up with them.
    fn snap_move(&self, i: usize, r: Rect, area: Vec2) -> Rect {
        let others = || self.stacks.iter().enumerate().filter(move |(j, _)| *j != i).map(|(_, s)| s.rect());
        let (w, h) = (r.width(), r.height());
        let xs = [0.0, area.x - w].into_iter().chain(others().flat_map(|o| [o.min.x, o.max.x + GAP, o.min.x - GAP - w, o.max.x - w]));
        let ys = [0.0, area.y - h].into_iter().chain(others().flat_map(|o| [o.min.y, o.max.y + GAP, o.min.y - GAP - h, o.max.y - h]));
        let min = pos2(snap(r.min.x, xs).max(0.0), snap(r.min.y, ys).max(0.0));
        Rect::from_min_size(min, r.size())
    }

    /// r, stack i resized, its bottom right corner snapped as snap_move
    /// snaps.
    fn snap_resize(&self, i: usize, r: Rect, area: Vec2) -> Rect {
        let others = || self.stacks.iter().enumerate().filter(move |(j, _)| *j != i).map(|(_, s)| s.rect());
        let xs = [area.x].into_iter().chain(others().flat_map(|o| [o.min.x - GAP, o.max.x]));
        let ys = [area.y].into_iter().chain(others().flat_map(|o| [o.min.y - GAP, o.max.y]));
        let max = pos2(snap(r.max.x, xs), snap(r.max.y, ys));
        Rect::from_min_max(r.min, max.max(r.min + MIN))
    }

    /// Takes the card off stack i into a stack of its own at rect, the
    /// same size; that stack's index. A stack of one card stays itself.
    fn detach(&mut self, i: usize, key: &str, rect: Rect) -> usize {
        let s = &mut self.stacks[i];
        if s.cards.len() > 1 {
            s.cards.retain(|k| k != key);
            if s.top == key {
                s.top = s.cards[0].clone();
            }
            self.stacks.push(Stack::of(vec![key.to_string()], rect));
        } else {
            s.set_rect(rect);
            self.raise(i);
        }
        self.fitted = false;
        self.stacks.len() - 1
    }

    /// Stack from's cards join into's, at tab index at, on top; from goes.
    fn merge(&mut self, from: usize, into: usize, at: usize) {
        let s = self.stacks.remove(from);
        let into = if into > from { into - 1 } else { into };
        let t = &mut self.stacks[into];
        let at = at.min(t.cards.len());
        t.cards.splice(at..at, s.cards);
        t.top = s.top;
        self.fitted = false;
    }

    /// The topmost stack other than except whose bar is at p.
    fn bar_at(&self, p: Pos2, except: usize) -> Option<usize> {
        (0..self.stacks.len()).rev().find(|&j| j != except && self.stacks[j].bar().contains(p))
    }

    /// The tab index a tab dropped at x goes to on stack i.
    fn tab_index(&self, i: usize, x: f32, width: &impl Fn(&str) -> f32) -> usize {
        let s = &self.stacks[i];
        tab_rects(s.rect(), &s.cards, width).iter().filter(|r| r.center().x < x).count()
    }

    /// Starts a drag, keeping the layout as it is to go back to.
    fn begin(&mut self, drag: Drag) {
        self.before = Some((self.stacks.clone(), self.fitted));
        self.drag = Some(drag);
    }

    /// Ends the drag under way, the layout put back as it was before it;
    /// whether there was one.
    pub fn cancel_drag(&mut self) -> bool {
        let Some((stacks, fitted)) = self.before.take().filter(|_| self.drag.is_some()) else { return false };
        self.stacks = stacks;
        self.fitted = fitted;
        self.drag = None;
        true
    }

    /// Follows the drag to p (on the board), finishing it once released;
    /// whether the layout changed for good.
    fn drag_to(&mut self, p: Option<Pos2>, down: bool, area: Vec2, width: &impl Fn(&str) -> f32) -> bool {
        let Some(drag) = self.drag.clone() else { return false };
        if let Some(p) = p {
            match drag.clone() {
                Drag::Pressed { stack, key, at, tab } => {
                    let d = p - at;
                    if d.length() > DECIDE {
                        self.drag = Some(if d.y.abs() > d.x.abs() {
                            // The tab kept under the pointer, as the first
                            // of its new stack
                            let size = self.stacks[stack].rect().size();
                            let min = (p - (at - tab) - vec2(TAB_INSET, TAB_INSET)).max(Pos2::ZERO);
                            let i = self.detach(stack, &key, Rect::from_min_size(min, size));
                            Drag::Move { stack: i, grab: p - min, card: true }
                        } else {
                            Drag::Tabs { stack, key }
                        });
                    }
                }
                Drag::Tabs { stack, key } => {
                    let s = &mut self.stacks[stack];
                    if let Some(from) = s.cards.iter().position(|k| *k == key) {
                        let rects = tab_rects(s.rect(), &s.cards, width);
                        let mut to = from;
                        while to > 0 && p.x < rects[to - 1].center().x {
                            to -= 1;
                        }
                        while to + 1 < rects.len() && p.x > rects[to + 1].center().x {
                            to += 1;
                        }
                        let k = s.cards.remove(from);
                        s.cards.insert(to, k);
                        s.top = key;
                    }
                }
                Drag::Move { stack, grab, card } => {
                    let raw = Rect::from_min_size((p - grab).max(Pos2::ZERO), self.stacks[stack].rect().size());
                    // Over a stack it'd join, where it is matters not
                    let r = if card && self.bar_at(p, stack).is_some() { raw } else { self.snap_move(stack, raw, area) };
                    self.stacks[stack].set_rect(r);
                    self.fitted = false;
                }
                Drag::Resize { stack, grab } => {
                    let s = self.stacks[stack].rect();
                    let r = self.snap_resize(stack, Rect::from_min_max(s.min, p - grab), area);
                    self.stacks[stack].set_rect(r);
                    self.fitted = false;
                }
            }
        }
        if down {
            return false;
        }
        self.drag = None;
        self.before = None;
        match drag {
            // Let go before deciding: a click, which the tab takes
            Drag::Pressed { .. } => false,
            Drag::Move { stack, card: true, .. } => {
                if let Some(p) = p
                    && let Some(into) = self.bar_at(p, stack)
                {
                    let at = self.tab_index(into, p.x, width);
                    self.merge(stack, into, at);
                }
                true
            }
            _ => true,
        }
    }
}

/// Draws the board in what's left of ui, the cards' contents by content;
/// whether the layout changed, to be saved.
pub fn board(ui: &mut Ui, t: &Colors, layout: &mut Layout, cards: &[Card], default: &[&[&str]], mut content: impl FnMut(&mut Ui, &str)) -> bool {
    // Before any is drawn: one closed by a press closes as it's drawn
    let popup = egui::Popup::is_any_open(ui.ctx());
    let keys: Vec<&str> = cards.iter().map(|c| c.key).collect();
    layout.arrange(&keys, default);
    let area = ui.available_size();
    layout.fit(area);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galleys: HashMap<&str, _> = cards.iter().map(|c| (c.key, ui.painter().layout_no_wrap(c.title.to_string(), font.clone(), t.text))).collect();
    let width = |k: &str| galleys.get(k).map_or(40.0, |g| g.size().x);
    let body_fill = t.canvas.lerp_to_gamma(t.chrome, 0.5);
    let bar_fill = t.chrome;
    let board_id = ui.id().with("board");
    let mut changed = false;
    let scroll = egui::ScrollArea::both()
        .id_salt(board_id)
        .auto_shrink([false, false])
        .scroll_source(egui::scroll_area::ScrollSource::SCROLL_BAR | egui::scroll_area::ScrollSource::MOUSE_WHEEL);
    let shown = scroll.show(ui, |ui| {
        let origin = ui.cursor().min.to_vec2();
        layout.origin = origin;
        // Nothing's above or left of the board's corner, though its clip
        // reaches a little past it
        let seen = ui.clip_rect().translate(-origin);
        layout.seen = Some(Rect::from_min_max(seen.min.max(Pos2::ZERO), seen.max));
        let (pointer, down, press) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.pointer.press_origin()));
        let on_board = |p: Pos2| p - origin;
        // Taken only while dragging, so Escape is the app's otherwise. The
        // pointer's still down, but nothing follows it until it's let go
        if layout.drag.is_some() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            layout.cancel_drag();
        }
        changed |= layout.drag_to(pointer.map(on_board), down, area, &width);
        let extent = layout.stacks.iter().fold(area, |e, s| e.max(s.rect().max.to_vec2()));
        ui.allocate_space(extent);
        let target = match &layout.drag {
            Some(Drag::Move { stack, card: true, .. }) => pointer.and_then(|p| layout.bar_at(on_board(p), *stack)),
            _ => None,
        };
        // Taken in, as the stacks can't change while they're drawn
        let mut start = None;
        let mut shown = None;
        let clip = ui.clip_rect();
        layout.clip = Some(clip);
        layout.drawn.clear();
        for i in 0..layout.stacks.len() {
            let s = layout.stacks[i].clone();
            let r = s.rect().translate(origin);
            let id = board_id.with(&s.cards[0]);
            // Over the stacks drawn before it, which mustn't be clicked or
            // dragged through it: egui gives a press's drag to the nearest
            // widget that senses drags, even one under the widget clicked,
            // so a combo box pressed over another stack's bar would start
            // dragging that stack
            ui.interact(r, id.with("back"), Sense::click_and_drag());
            let painter = ui.painter_at(r.intersect(clip));
            painter.rect_filled(r, 6, body_fill);
            let bar = s.bar().translate(origin);
            painter.rect_filled(bar, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, bar_fill);
            if target == Some(i) {
                painter.rect_filled(bar, CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 }, t.accent.gamma_multiply(0.25));
            }
            let b = ui.interact(bar, id.with("bar"), Sense::drag()).on_hover_cursor(CursorIcon::Grab);
            if b.drag_started() {
                let grab = press.map_or(Vec2::ZERO, |p| on_board(p) - s.rect().min);
                start = Some((i, Drag::Move { stack: i, grab, card: false }));
            }
            let trs = tab_rects(r, &s.cards, width);
            layout.drawn.push((r, s.cards.iter().cloned().zip(trs.iter().copied()).collect()));
            for (k, &tr) in s.cards.iter().zip(&trs) {
                let title = cards.iter().find(|c| c.key == k).map_or("", |c| c.title);
                let on = *k == s.top;
                let tab = ui.interact(tr, id.with(("tab", k)), Sense::click_and_drag());
                tab.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, title));
                if on {
                    painter.rect_filled(tr, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, body_fill);
                } else if tab.hovered() {
                    painter.rect_filled(tr, CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, t.hover);
                }
                if let Some(g) = galleys.get(k.as_str()) {
                    // Centred, and clipped to its tab when squeezed
                    let x = (tr.center().x - g.size().x / 2.0).max(tr.min.x + MIN_TAB_PAD);
                    let at = pos2(x, tr.center().y - g.size().y / 2.0);
                    painter.with_clip_rect(tr.shrink2(vec2(MIN_TAB_PAD / 2.0, 0.0)).intersect(painter.clip_rect())).galley_with_override_text_color(at, g.clone(), if on { t.text } else { t.text_dim });
                }
                if tab.clicked() {
                    shown = Some((i, k.clone()));
                }
                if tab.drag_started() {
                    let at = press.unwrap_or(tr.min);
                    start = Some((i, Drag::Pressed { stack: i, key: k.clone(), at: on_board(at), tab: on_board(tr.min) }));
                }
            }
            // A card alone in its stack, that can be closed: a close mark
            // at the bar's right end, over the bar, so a press on it isn't
            // a drag of the stack
            let close = s.cards.len() == 1 && cards.iter().any(|c| c.key == s.top && c.close);
            if close {
                let cr = Rect::from_center_size(pos2(bar.max.x - BAR / 2.0, bar.center().y), vec2(BAR - 10.0, BAR - 10.0));
                let title = cards.iter().find(|c| c.key == s.top).map_or("", |c| c.title);
                let c = ui.interact(cr, id.with("close"), Sense::click()).on_hover_text("Close this card");
                c.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Close {title}")));
                if c.hovered() {
                    painter.rect_filled(cr, 4, t.hover);
                }
                let (a, b) = (cr.shrink(5.0).min, cr.shrink(5.0).max);
                let stroke = Stroke::new(1.5, if c.hovered() { t.text } else { t.text_dim });
                painter.line_segment([a, b], stroke);
                painter.line_segment([pos2(a.x, b.y), pos2(b.x, a.y)], stroke);
                if c.clicked() {
                    layout.closed = Some(s.top.clone());
                }
            }
            if cards.iter().any(|c| c.key == s.top && c.loading) {
                // Left of the close mark, if there's one
                let x = bar.max.x - BAR / 2.0 - if close { BAR } else { 0.0 };
                let sr = Rect::from_center_size(pos2(x, bar.center().y), vec2(14.0, 14.0));
                egui::Spinner::new().size(14.0).paint_at(ui, sr);
            }
            let body = Rect::from_min_max(pos2(r.min.x, bar.max.y), r.max).shrink(PAD);
            let mut child = ui.new_child(UiBuilder::new().max_rect(body).layout(egui::Layout::top_down(Align::Min)).id_salt(("card", &s.top)));
            child.set_clip_rect(body.expand(PAD / 2.0).intersect(clip));
            let body = egui::ScrollArea::vertical().id_salt(("card", &s.top)).auto_shrink([false, false]).show(&mut child, |ui| {
                content(ui, &s.top);
                ui.min_rect().height()
            });
            partman_scroll::area(child.ctx(), &body);
            let h = body.inner;
            layout.heights.insert(s.top.clone(), h);
            // The stack a card carried would join, outlined over its
            // contents
            if target == Some(i) {
                painter.rect_stroke(r, 6, Stroke::new(2.0, t.accent), StrokeKind::Inside);
            }
            let grip = Rect::from_min_max(r.max - vec2(GRIP, GRIP), r.max);
            let g = ui.interact(grip, id.with("grip"), Sense::drag()).on_hover_cursor(CursorIcon::ResizeNwSe);
            for n in 1..=3 {
                let d = n as f32 * 3.5;
                painter.line_segment([pos2(r.max.x - d - 2.0, r.max.y - 2.0), pos2(r.max.x - 2.0, r.max.y - d - 2.0)], Stroke::new(1.0, t.text_dim));
            }
            if g.drag_started() {
                let grab = press.map_or(Vec2::ZERO, |p| on_board(p) - s.rect().max);
                start = Some((i, Drag::Resize { stack: i, grab }));
            }
        }
        if let Some((i, k)) = shown {
            layout.stacks[i].top = k;
            changed = true;
        }
        // A press anywhere on a stack brings it to the front, whatever it
        // lands on; one on a popup over it (a combo box's list) is the
        // popup's, as is one off the board. While a popup is open any
        // press is its, picking from it or closing it, as a menu's is.
        // Not while fitted: the stacks don't overlap then, and are placed
        // in their order, so raising one would move it out from under the
        // pointer
        let pressed = ui.input(|i| i.pointer.primary_pressed().then(|| i.pointer.interact_pos()).flatten()).filter(|_| !layout.fitted && !popup);
        let on_board_layer = |p: Pos2| clip.contains(p) && ui.ctx().layer_id_at(p) == Some(ui.layer_id());
        let under = pressed.filter(|p| start.is_none() && on_board_layer(*p)).and_then(|p| layout.stacks.iter().rposition(|s| s.rect().translate(origin).contains(p)));
        if let Some(i) = under.filter(|i| i + 1 < layout.stacks.len()) {
            layout.raise(i);
            changed = true;
        }
        if let Some((i, drag)) = start {
            let j = layout.raise(i);
            layout.begin(match drag {
                Drag::Pressed { key, at, tab, .. } => Drag::Pressed { stack: j, key, at, tab },
                Drag::Move { grab, card, .. } => Drag::Move { stack: j, grab, card },
                Drag::Resize { grab, .. } => Drag::Resize { stack: j, grab },
                d @ Drag::Tabs { .. } => d,
            });
        }
        match layout.drag {
            Some(Drag::Move { .. }) => ui.ctx().set_cursor_icon(CursorIcon::Grabbing),
            Some(Drag::Resize { .. }) => ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe),
            _ => {}
        }
    });
    partman_scroll::area(ui.ctx(), &shown);
    changed
}

/// The layout a board's saved under, made to hold its cards.
pub fn of<'a>(boards: &'a mut std::collections::BTreeMap<String, Layout>, name: &str, keys: &[&str], default: &[&[&str]]) -> &'a mut Layout {
    let l = boards.entry(name.to_string()).or_default();
    l.arrange(keys, default);
    l
}

/// A board's layout as a file, to take it to another computer: its stacks
/// as they are, which the board there fits its cards to as it does its own.
/// Its kind is the app's, such as "partman-layout", so another JSON file
/// isn't taken for one.
#[derive(Serialize, Deserialize)]
struct Exported {
    kind: String,
    version: u32,
    board: String,
    fitted: bool,
    stacks: Vec<Stack>,
}

pub fn export(kind: &str, board: &str, l: &Layout) -> String {
    let e = Exported { kind: kind.into(), version: 1, board: board.into(), fitted: l.fitted, stacks: l.stacks.clone() };
    serde_json::to_string_pretty(&e).expect("a layout serializes") + "\n"
}

/// A layout from export's file, if it's one of this board's.
pub fn import(kind: &str, board: &str, text: &str) -> Result<Layout, String> {
    let e: Exported = serde_json::from_str(text).map_err(|e| format!("not a layout file: {e}"))?;
    if e.kind != kind {
        return Err("not a layout file".into());
    }
    if e.version != 1 {
        return Err(format!("a layout file of a newer version (version {})", e.version));
    }
    if e.board != board {
        return Err(format!("the layout of the {} page, not this one", e.board));
    }
    Ok(Layout { stacks: e.stacks, fitted: e.fitted, ..Default::default() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [&str; 4] = ["a", "b", "c", "d"];
    const KIND: &str = "test-layout";
    const DEFAULT: &[&[&str]] = &[&["a"], &["b", "c", "d"]];

    fn layout() -> Layout {
        let mut l = Layout::default();
        l.arrange(&KEYS, DEFAULT);
        l.fit(vec2(800.0, 600.0));
        l
    }

    /// A layout of stacks at rects, seen as the board's top left of size.
    fn laid(rects: &[[f32; 4]], seen: Vec2) -> Layout {
        let mut l = Layout::default();
        for (n, r) in rects.iter().enumerate() {
            l.stacks.push(Stack { rect: *r, cards: vec![format!("c{n}")], top: format!("c{n}") });
        }
        l.seen = Some(Rect::from_min_size(Pos2::ZERO, seen));
        l
    }

    /// A new card goes where the board seen has room for all of it: beside
    /// a stack, else under one, else, with none, under them all.
    #[test]
    fn new_cards_find_room() {
        let mut l = laid(&[[0.0, 0.0, 600.0, 400.0]], vec2(1200.0, 800.0));
        l.add_alone("a");
        assert_eq!(l.stacks[1].rect, [616.0, 0.0, NEW.x, NEW.y], "beside it");
        l.add_alone("b");
        assert_eq!(l.stacks[2].rect[0..2], [616.0, 276.0], "under the one beside");
        l.add_alone("c");
        assert_eq!(l.stacks[3].rect[0..2], [0.0, 416.0], "under the first");
        let mut full = laid(&[[0.0, 0.0, 1200.0, 800.0]], vec2(1200.0, 800.0));
        full.add_alone("d");
        assert_eq!(full.stacks[1].rect[0..2], [0.0, 816.0], "no room: under them all");
    }

    /// A tab's card is found under a point, but not where a stack drawn
    /// later covers it, nor outside the board.
    #[test]
    fn tabs_under_a_point() {
        let mut l = Layout { clip: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1000.0))), ..Default::default() };
        let tab = |x: f32| Rect::from_min_size(pos2(x, 2.0), vec2(50.0, 20.0));
        l.drawn.push((Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), vec![("a".into(), tab(4.0)), ("b".into(), tab(54.0))]));
        l.drawn.push((Rect::from_min_size(pos2(80.0, 0.0), vec2(300.0, 300.0)), vec![("c".into(), tab(84.0))]));
        assert_eq!(l.tab_at(pos2(20.0, 10.0)).map(|t| t.0), Some("a"));
        assert_eq!(l.tab_at(pos2(60.0, 10.0)).map(|t| t.0), Some("b"));
        assert_eq!(l.tab_at(pos2(100.0, 10.0)).map(|t| t.0), Some("c"), "the stack over the other");
        assert_eq!(l.tab_at(pos2(200.0, 10.0)), None, "on a bar, off its tabs");
        assert_eq!(l.tab_at(pos2(20.0, 100.0)), None, "on the card");
        l.clip = Some(Rect::from_min_size(pos2(30.0, 0.0), vec2(970.0, 1000.0)));
        assert_eq!(l.tab_at(pos2(20.0, 10.0)), None, "clipped off the board");
    }

    /// A point on the board seen and on no stack is blank, and a card put
    /// there stays within what's seen.
    #[test]
    fn drops_on_blank_board() {
        let l = laid(&[[0.0, 0.0, 600.0, 400.0]], vec2(1200.0, 800.0));
        assert_eq!(l.blank_at(pos2(700.0, 100.0)), Some(pos2(700.0, 100.0)));
        assert_eq!(l.blank_at(pos2(100.0, 100.0)), None, "on a stack");
        assert_eq!(l.blank_at(pos2(1300.0, 100.0)), None, "off the board seen");
        let r = l.card_rect_at(pos2(1190.0, 790.0));
        assert!(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0)).contains_rect(r), "{r:?}");
        let mut fitted = l.clone();
        fitted.fitted = true;
        assert_eq!(fitted.blank_at(pos2(700.0, 100.0)), None, "a fitted board has no blank");
    }

    #[test]
    fn the_default_fills_the_board() {
        let l = layout();
        assert!(l.fitted);
        assert_eq!(l.stacks[0].rect, [0.0, 0.0, 800.0, BAR + 2.0 * PAD + 120.0]);
        let last = l.stacks[1].rect();
        assert_eq!((last.min.y, last.max.y, last.width()), (l.stacks[0].rect().max.y + GAP, 600.0, 800.0));
    }

    #[test]
    fn arranging_keeps_each_card_once() {
        let mut l = layout();
        l.stacks[0].cards.push("b".into());
        l.stacks[1].cards.retain(|k| k != "d");
        l.stacks[1].cards.push("gone".into());
        l.arrange(&KEYS, DEFAULT);
        let all: Vec<&str> = l.stacks.iter().flat_map(|s| s.cards.iter().map(String::as_str)).collect();
        assert_eq!(all, ["a", "b", "c", "d"]);
    }

    #[test]
    fn edges_snap_beside_and_in_line() {
        let mut l = layout();
        l.fitted = false;
        l.stacks[0].set_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0)));
        let r = l.snap_move(1, Rect::from_min_size(pos2(300.0 + GAP + 6.0, 7.0), vec2(200.0, 100.0)), vec2(800.0, 600.0));
        assert_eq!(r.min, pos2(300.0 + GAP, 0.0));
        // Too far for either
        let r = l.snap_move(1, Rect::from_min_size(pos2(400.0, 50.0), vec2(200.0, 100.0)), vec2(800.0, 600.0));
        assert_eq!(r.min, pos2(400.0, 50.0));
        // The board's right edge, and the other's bottom
        let r = l.snap_resize(1, Rect::from_min_max(pos2(400.0, 0.0), pos2(795.0, 195.0)), vec2(800.0, 600.0));
        assert_eq!(r.max, pos2(800.0, 200.0));
    }

    #[test]
    fn a_card_taken_off_joins_another_stack() {
        let mut l = layout();
        let size = l.stacks[1].rect().size();
        let i = l.detach(1, "c", Rect::from_min_size(pos2(50.0, 400.0), vec2(300.0, 300.0)));
        assert!(!l.fitted);
        assert_eq!((l.stacks.len(), l.stacks[1].cards.clone()), (3, vec!["b".to_string(), "d".to_string()]));
        assert_eq!(l.stacks[i].rect().size(), vec2(300.0, 300.0));
        // On the other's bar, between its tabs: there on top, at its size
        l.merge(i, 1, 1);
        assert_eq!(l.stacks.len(), 2);
        assert_eq!((l.stacks[1].cards.join(","), l.stacks[1].top.as_str()), ("b,c,d".to_string(), "c"));
        assert_eq!(l.stacks[1].rect().size(), size);
    }

    /// A closable card alone in its stack has a close mark, which says it
    /// was pressed; one sharing its stack has none.
    #[test]
    fn a_solo_card_can_be_closed() {
        use egui_kittest::kittest::Queryable;
        let mut h = egui_kittest::Harness::builder().with_size(vec2(800.0, 600.0)).build_ui_state(
            |ui, l: &mut Layout| {
                let cards: Vec<Card> = KEYS.iter().map(|k| Card { key: k, title: k, loading: false, close: true }).collect();
                board(ui, &Colors::default(), l, &cards, DEFAULT, |ui, k| {
                    ui.label(format!("body of {k}"));
                });
            },
            layout(),
        );
        h.run_steps(2);
        let solo: Vec<&str> = h.state().stacks.iter().filter(|s| s.cards.len() == 1).map(|s| s.top.as_str()).collect();
        let shared: Vec<String> = h.state().stacks.iter().filter(|s| s.cards.len() > 1).flat_map(|s| s.cards.clone()).collect();
        assert!(!solo.is_empty() && !shared.is_empty(), "the default has both");
        for k in &shared {
            assert!(h.query_by_label(&format!("Close {k}")).is_none(), "{k} shares its stack");
        }
        let k = solo[0].to_string();
        h.get_by_label(&format!("Close {k}")).click();
        h.run_steps(2);
        assert_eq!(h.state_mut().take_closed(), Some(k));
        assert_eq!(h.state_mut().take_closed(), None, "said once");
    }

    #[test]
    fn a_press_anywhere_on_a_stack_brings_it_forward() {
        use egui_kittest::kittest::Queryable;
        let mut l = layout();
        l.fitted = false;
        l.stacks[0].set_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 300.0)));
        l.stacks[1].set_rect(Rect::from_min_size(pos2(200.0, 200.0), vec2(300.0, 300.0)));
        let mut h = egui_kittest::Harness::builder().with_size(vec2(800.0, 600.0)).build_ui_state(
            |ui, l: &mut Layout| {
                let cards: Vec<Card> = KEYS.iter().map(|k| Card { key: k, title: k, loading: false, close: false }).collect();
                board(ui, &Colors::default(), l, &cards, DEFAULT, |ui, k| {
                    ui.label(format!("body of {k}"));
                });
            },
            l,
        );
        h.run_steps(2);
        // In the first stack's body, clear of the second drawn over it
        let at = h.get_by_label("body of a").rect().center();
        h.hover_at(at);
        h.step();
        h.drag_at(at);
        h.step();
        h.drop_at(at);
        h.run_steps(2);
        assert_eq!(h.state().stacks.last().map(|s| s.top.as_str()), Some("a"));
    }

    /// A combo box on the stack in front, over another stack's bar, is
    /// pressed without dragging that stack: egui gives a press's drag to
    /// the nearest widget sensing drags, even one under the widget
    /// clicked. And with its list open, a press elsewhere only closes it.
    #[test]
    fn a_press_on_a_combo_box_is_only_its() {
        use egui_kittest::kittest::Queryable;
        let mut l = layout();
        l.fitted = false;
        l.stacks[0].set_rect(Rect::from_min_size(pos2(0.0, 40.0), vec2(300.0, 300.0)));
        l.stacks[1].set_rect(Rect::from_min_size(pos2(100.0, 10.0), vec2(300.0, 200.0)));
        let mut h = egui_kittest::Harness::builder().with_size(vec2(800.0, 600.0)).build_ui_state(
            |ui, l: &mut Layout| {
                let cards: Vec<Card> = KEYS.iter().map(|k| Card { key: k, title: k, loading: false, close: false }).collect();
                board(ui, &Colors::default(), l, &cards, DEFAULT, |ui, k| {
                    if k == "b" {
                        let r = egui::ComboBox::from_id_salt("pick").selected_text("one").show_ui(ui, |ui| {
                            let _ = ui.selectable_label(false, "one");
                            let _ = ui.selectable_label(false, "two");
                        });
                        r.response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, "pick"));
                    } else {
                        ui.label(format!("body of {k}"));
                    }
                });
            },
            l,
        );
        h.run_steps(2);
        let order = |h: &egui_kittest::Harness<'_, Layout>| h.state().stacks.iter().map(|s| s.top.clone()).collect::<Vec<_>>();
        let press = |h: &mut egui_kittest::Harness<'_, Layout>, at: Pos2| {
            h.hover_at(at);
            h.step();
            h.drag_at(at);
            h.step();
            h.drop_at(at);
            h.run_steps(2);
        };
        let combo = h.get_by_label("pick").rect().center();
        // The stack behind's bar is the height of its tab, and 300 wide
        let tab = h.get_by_label("a").rect();
        assert!(tab.y_range().contains(combo.y) && combo.x < tab.min.x + 300.0, "the combo box at {combo:?} is over the bar of a, by its tab at {tab:?}");
        press(&mut h, combo);
        assert_eq!(order(&h), ["a", "b"]);
        assert!(h.state().drag.is_none());
        assert!(h.query_by_label("two").is_some(), "its list opened");
        // Clear of the front stack, on the one behind
        let behind = h.get_by_label("body of a").rect().center();
        press(&mut h, behind);
        assert!(h.query_by_label("two").is_none(), "its list closed");
        assert_eq!(order(&h), ["a", "b"]);
    }

    #[test]
    fn tabs_dragged_sideways_reorder() {
        let mut l = layout();
        let w = |_: &str| 20.0;
        let tabs = tab_rects(l.stacks[1].rect(), &l.stacks[1].cards, w);
        l.drag = Some(Drag::Tabs { stack: 1, key: "b".into() });
        assert!(!l.drag_to(Some(pos2(tabs[2].center().x + 1.0, tabs[2].center().y)), true, vec2(800.0, 600.0), &w));
        assert_eq!(l.stacks[1].cards.join(","), "c,d,b");
        assert!(l.fitted, "rearranging tabs moves no stack");
        assert!(l.drag_to(None, false, vec2(800.0, 600.0), &w));
        assert!(l.drag.is_none());
    }

    #[test]
    fn a_tab_dragged_down_comes_off() {
        let mut l = layout();
        let w = |_: &str| 20.0;
        let tab = tab_rects(l.stacks[1].rect(), &l.stacks[1].cards, w)[1];
        let at = tab.center();
        l.drag = Some(Drag::Pressed { stack: 1, key: "c".into(), at, tab: tab.min });
        l.drag_to(Some(at + vec2(1.0, 60.0)), true, vec2(800.0, 600.0), &w);
        assert!(matches!(l.drag, Some(Drag::Move { stack: 2, card: true, .. })));
        assert_eq!(l.stacks[2].cards, ["c"]);
        // Dropped on the first stack's bar, it joins it
        let bar = l.stacks[0].bar().center();
        l.drag_to(Some(bar), true, vec2(800.0, 600.0), &w);
        assert!(l.drag_to(Some(bar), false, vec2(800.0, 600.0), &w));
        assert_eq!(l.stacks.len(), 2);
        assert!(l.stacks[0].cards.contains(&"c".to_string()) && l.stacks[0].top == "c");
    }

    /// Escape mid-drag puts back a card taken off, or tabs reordered.
    #[test]
    fn a_cancelled_drag_puts_the_layout_back() {
        let mut l = layout();
        let was = l.stacks.clone();
        let w = |_: &str| 20.0;
        let tab = tab_rects(l.stacks[1].rect(), &l.stacks[1].cards, w)[1];
        let at = tab.center();
        l.begin(Drag::Pressed { stack: 1, key: "c".into(), at, tab: tab.min });
        l.drag_to(Some(at + vec2(1.0, 60.0)), true, vec2(800.0, 600.0), &w);
        assert_eq!(l.stacks.len(), 3, "taken off");
        assert!(l.cancel_drag());
        assert_eq!(l.stacks, was);
        assert!(l.fitted && l.drag.is_none());
        // Let go after, nothing more happens
        assert!(!l.drag_to(Some(at), false, vec2(800.0, 600.0), &w));
        assert_eq!(l.stacks, was);
        l.begin(Drag::Tabs { stack: 1, key: "b".into() });
        let tabs = tab_rects(l.stacks[1].rect(), &l.stacks[1].cards, w);
        l.drag_to(Some(pos2(tabs[2].center().x + 1.0, tabs[2].center().y)), true, vec2(800.0, 600.0), &w);
        assert!(l.cancel_drag());
        assert_eq!(l.stacks[1].cards.join(","), "b,c,d");
        assert!(!l.cancel_drag(), "nothing to cancel");
    }

    /// With the pointer: a stack moved by its bar goes back on Escape,
    /// and stays put when let go.
    #[test]
    fn escape_cancels_moving_a_stack() {
        use egui_kittest::kittest::Queryable;
        let mut l = layout();
        l.fitted = false;
        l.stacks[0].set_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0)));
        l.stacks[1].set_rect(Rect::from_min_size(pos2(320.0, 0.0), vec2(300.0, 200.0)));
        let mut h = egui_kittest::Harness::builder().with_size(vec2(800.0, 600.0)).build_ui_state(
            |ui, l: &mut Layout| {
                let cards: Vec<Card> = KEYS.iter().map(|k| Card { key: k, title: k, loading: false, close: false }).collect();
                board(ui, &Colors::default(), l, &cards, DEFAULT, |ui, k| {
                    ui.label(format!("body of {k}"));
                });
            },
            l,
        );
        h.run_steps(2);
        let was = h.state().stacks.iter().find(|s| s.top == "a").unwrap().rect();
        // On the bar, right of the tab
        let tab = h.get_by_label("a").rect();
        let at = pos2(tab.max.x + 40.0, tab.center().y);
        h.hover_at(at);
        h.step();
        h.drag_at(at);
        h.step();
        h.hover_at(at + vec2(60.0, 200.0));
        h.run_steps(2);
        let moved = h.state().stacks.iter().find(|s| s.top == "a").unwrap().rect();
        assert_ne!(moved, was, "it follows the pointer");
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        assert!(h.state().drag.is_none());
        h.drop_at(at + vec2(60.0, 200.0));
        h.run_steps(2);
        assert_eq!(h.state().stacks.iter().find(|s| s.top == "a").unwrap().rect(), was);
    }

    #[test]
    fn a_card_added_alone_goes_above_the_last_or_under_all() {
        let mut l = layout();
        l.add_alone("e");
        assert_eq!(l.stacks.iter().map(|s| s.cards.join(",")).collect::<Vec<_>>(), ["a", "e", "b,c,d"], "above the last, which takes what's left");
        l.fitted = false;
        let bottom = l.stacks.iter().map(|s| s.rect().max.y).fold(0.0, f32::max);
        l.add_alone("f");
        assert_eq!(l.stacks.last().unwrap().cards, ["f"]);
        assert_eq!(l.stacks.last().unwrap().rect().min, pos2(0.0, bottom + GAP));
    }

    #[test]
    fn exports_and_imports_its_stacks() {
        let mut l = layout();
        l.stacks[1].rect = [10.0, 20.5, 300.0, 200.0];
        l.fitted = false;
        let text = export(KIND, "part", &l);
        let back = import(KIND, "part", &text).unwrap();
        assert_eq!((back.stacks, back.fitted), (l.stacks, false));
        assert!(import(KIND, "project", &text).unwrap_err().contains("part page"));
        assert!(import(KIND, "part", "{\"kind\":\"other\",\"version\":1,\"board\":\"part\",\"fitted\":true,\"stacks\":[]}").is_err());
        assert!(import(KIND, "part", &text.replace("\"version\": 1", "\"version\": 2")).unwrap_err().contains("newer"));
    }
}
