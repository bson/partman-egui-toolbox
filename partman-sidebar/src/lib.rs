// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A sidebar the way the toolbox's style guide (STYLE.md) has the agent
//! sidebar: a small triangle on the work area's edge, halfway down, slides
//! it open, by default over half the work area. Its edge is dragged to the
//! width the user prefers, which it opens to from then on, never wider
//! than the work area. Open, the triangle is inside it, and slides it
//! shut. The app keeps whether it's open and the width, saving them with
//! its settings.

use egui::{Color32, CornerRadius, CursorIcon, Id, Key, Modifiers, Pos2, Rect, Sense, Shape, Ui, UiBuilder, pos2, vec2};

/// Which edge of the work area it's on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    Left,
    #[default]
    Right,
}

/// The colours its control is drawn in, from the app's theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    /// The control's background, so it shows over whatever is under it
    pub back: Color32,
    pub triangle: Color32,
    pub hover: Color32,
    /// The triangle while closed with something wanting the user
    pub attention: Color32,
}

impl Colors {
    pub fn from_visuals(v: &egui::Visuals) -> Colors {
        Colors { back: v.widgets.inactive.weak_bg_fill, triangle: v.text_color(), hover: v.strong_text_color(), attention: v.warn_fg_color }
    }
}

impl Default for Colors {
    fn default() -> Colors {
        Colors::from_visuals(&egui::Visuals::dark())
    }
}

/// How long it takes to slide open or shut, in seconds.
pub const SLIDE: f32 = 0.2;
/// It's never narrower than this, unless the work area is.
pub const MIN_WIDTH: f32 = 240.0;
/// The open and close control.
const CONTROL: egui::Vec2 = vec2(14.0, 32.0);
/// Room left on its inner edge for the control, inside it.
const INSET: f32 = CONTROL.x + 8.0;
/// How far either side of its inner edge a drag resizes it.
const GRIP: f32 = 3.0;

/// A sidebar, its state the app's.
pub struct Sidebar<'a> {
    id: Id,
    side: Side,
    open: &'a mut bool,
    width: &'a mut Option<f32>,
    name: String,
    attention: bool,
}

/// What showing it came to.
pub struct Shown<R> {
    /// What the contents returned, when they were drawn
    pub inner: Option<R>,
    /// The user opened or closed it
    pub toggled: bool,
    /// The user dragged it to a new width, to be saved
    pub resized: bool,
}

/// A resize under way: the width it started at, for Escape to put back;
/// cancelled, the drag is ignored until it's let go.
#[derive(Clone, Copy, Debug)]
struct Resizing {
    from: Option<f32>,
    cancelled: bool,
}

impl<'a> Sidebar<'a> {
    /// `open` says whether it's open, `width` the width the user dragged
    /// it to (None until they do, for half the work area). Both change
    /// as the user works them, a drag over several frames, so they must
    /// be the app's own, kept from frame to frame: a copy made each
    /// frame loses the drag. `name` names
    /// it for the control's hover text and accessibility: "Agent panel".
    pub fn new(id: impl Into<Id>, name: &str, open: &'a mut bool, width: &'a mut Option<f32>) -> Sidebar<'a> {
        Sidebar { id: id.into(), side: Side::Right, open, width, name: name.into(), attention: false }
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Something in it wants the user, shown on the control while closed.
    pub fn attention(mut self, attention: bool) -> Self {
        self.attention = attention;
        self
    }

    /// Draws it at its edge of what's left of `ui` (the work area), and
    /// `contents` in it while it's open or sliding. Call it before the
    /// work area's central panel, as for any side panel.
    pub fn show<R>(self, ui: &mut Ui, colors: &Colors, contents: impl FnOnce(&mut Ui) -> R) -> Shown<R> {
        let Sidebar { id, side, open, width, name, attention } = self;
        let area = ui.available_rect_before_wrap();
        let ctx = ui.ctx().clone();
        let target = width.unwrap_or(area.width() / 2.0).max(MIN_WIDTH).min(area.width());
        let shown = ctx.animate_bool_with_time(id.with("open"), *open, SLIDE);
        let mut inner = None;
        let mut resized = false;
        // Where its inner edge is: the work area's own while shut
        let mut edge = match side {
            Side::Right => area.right(),
            Side::Left => area.left(),
        };
        if shown > 0.0 {
            let w = (target * shown).round();
            let margin = |inner: f32| match side {
                Side::Right => egui::Margin { left: inner as i8, right: 8, top: 6, bottom: 6 },
                Side::Left => egui::Margin { left: 8, right: inner as i8, top: 6, bottom: 6 },
            };
            let frame = egui::Frame::side_top_panel(ui.style()).inner_margin(margin(INSET));
            let panel = match side {
                Side::Right => egui::Panel::right(id),
                Side::Left => egui::Panel::left(id),
            };
            let r = panel.exact_size(w).resizable(false).frame(frame).show(ui, |ui| {
                // Laid out at its full width while sliding, and cut off
                // by the panel, so what's in it doesn't reflow as it moves
                let full = target - INSET - 8.0;
                let max = ui.max_rect();
                let rect = match side {
                    Side::Right => Rect::from_min_size(max.min, vec2(full, max.height())),
                    Side::Left => Rect::from_min_size(pos2(max.max.x - full, max.min.y), vec2(full, max.height())),
                };
                ui.scope_builder(UiBuilder::new().max_rect(rect), contents).inner
            });
            inner = Some(r.inner);
            let panel_rect = r.response.rect;
            edge = match side {
                Side::Right => panel_rect.left(),
                Side::Left => panel_rect.right(),
            };
            if *open && shown >= 1.0 {
                resized = resize(ui, id, side, area, edge, width);
            }
        }
        let toggled = control(&ctx, id, side, area, edge, *open, attention, &name, colors);
        if toggled {
            *open = !*open;
        }
        Shown { inner, toggled, resized }
    }
}

/// The grip along its inner edge; whether a drag set a new width.
fn resize(ui: &mut Ui, id: Id, side: Side, area: Rect, edge: f32, width: &mut Option<f32>) -> bool {
    let grip = Rect::from_x_y_ranges(edge - GRIP..=edge + GRIP, area.y_range());
    // Above the work area, or its widgets drawn later along the edge, a
    // scroll area say, would take the press
    let r = egui::Area::new(id.with("grip"))
        .order(egui::Order::Foreground)
        .fixed_pos(grip.min)
        .show(ui.ctx(), |ui| ui.allocate_exact_size(grip.size(), Sense::drag()).1)
        .inner
        .on_hover_cursor(CursorIcon::ResizeHorizontal)
        .on_hover_text("Drag to make it wider or narrower");
    let key = id.with("resizing");
    if r.drag_started() {
        ui.data_mut(|d| d.insert_temp(key, Resizing { from: *width, cancelled: false }));
    }
    let Some(mut now) = ui.data(|d| d.get_temp::<Resizing>(key)) else { return false };
    // Looked for whether or not the drag goes on, as egui itself may end
    // a drag on Escape
    if !now.cancelled && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        *width = now.from;
        now.cancelled = true;
        ui.data_mut(|d| d.insert_temp(key, now));
    }
    if r.dragged() && !now.cancelled {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        if let Some(p) = r.interact_pointer_pos() {
            let w = match side {
                Side::Right => area.right() - p.x,
                Side::Left => p.x - area.left(),
            };
            *width = Some(w.max(MIN_WIDTH).min(area.width()));
        }
    }
    if r.drag_stopped() || !r.dragged() {
        ui.data_mut(|d| d.remove::<Resizing>(key));
        return !now.cancelled && *width != now.from;
    }
    false
}

/// The triangle opening or closing it, halfway down its inner edge:
/// outside on the work area's edge while shut, pointing in; inside it
/// while open, pointing out. Whether it was clicked.
#[allow(clippy::too_many_arguments)]
fn control(ctx: &egui::Context, id: Id, side: Side, area: Rect, edge: f32, open: bool, attention: bool, name: &str, colors: &Colors) -> bool {
    // Shut, the edge is the work area's, and the control inside that
    let x = match (side, open) {
        (Side::Right, true) | (Side::Left, false) => edge + 2.0,
        (Side::Right, false) | (Side::Left, true) => edge - 2.0 - CONTROL.x,
    };
    let at = pos2(x, area.center().y - CONTROL.y / 2.0);
    egui::Area::new(id.with("control"))
        .order(egui::Order::Foreground)
        .fixed_pos(at)
        .show(ctx, |ui| {
            let (rect, r) = ui.allocate_exact_size(CONTROL, Sense::click());
            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, open, name));
            let p = ui.painter();
            p.rect_filled(rect, CornerRadius::same(3), colors.back);
            let color = if r.hovered() {
                colors.hover
            } else if attention && !open {
                colors.attention
            } else {
                colors.triangle
            };
            // Pointing the way it'll move: out to shut it, in to open it
            let left = matches!((side, open), (Side::Right, false) | (Side::Left, true));
            p.add(triangle(rect.shrink2(vec2(3.0, 10.0)), left, color));
            let what = name.to_lowercase();
            let tip = match (open, attention) {
                (true, _) => format!("Hide the {what}"),
                (false, true) => format!("Show the {what}: it's waiting on you"),
                (false, false) => format!("Show the {what}"),
            };
            r.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tip).clicked()
        })
        .inner
}

fn triangle(r: Rect, left: bool, color: Color32) -> Shape {
    let points: Vec<Pos2> = if left { vec![r.right_top(), r.right_bottom(), r.left_center()] } else { vec![r.left_top(), r.left_bottom(), r.right_center()] };
    Shape::convex_polygon(points, color, egui::Stroke::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    #[derive(Default)]
    struct State {
        open: bool,
        width: Option<f32>,
        resized: bool,
    }

    fn harness(s: State) -> egui_kittest::Harness<'static, State> {
        egui_kittest::Harness::builder().with_size(vec2(1000.0, 600.0)).build_ui_state(
            |ui, s: &mut State| {
                let shown = Sidebar::new("side", "Agent panel", &mut s.open, &mut s.width).show(ui, &Colors::default(), |ui| {
                    ui.label("inside");
                });
                s.resized |= shown.resized;
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("work");
                });
            },
            s,
        )
    }

    fn panel_width(h: &egui_kittest::Harness<'_, State>) -> f32 {
        1000.0 - h.get_by_label("Agent panel").rect().left() + 2.0
    }

    #[test]
    fn opens_to_half_and_shuts() {
        let mut h = harness(State::default());
        h.run();
        assert!(h.query_by_label("inside").is_none());
        // Shut, the control is on the work area's edge
        assert!(h.get_by_label("Agent panel").rect().right() > 980.0);
        h.get_by_label("Agent panel").click();
        h.run();
        assert!(h.state().open);
        assert!(h.query_by_label("inside").is_some());
        assert!((panel_width(&h) - 500.0).abs() < 10.0, "{}", panel_width(&h));
        h.get_by_label("Agent panel").click();
        h.run();
        assert!(!h.state().open);
        assert!(h.query_by_label("inside").is_none());
    }

    #[test]
    fn opens_to_the_width_dragged_to_and_never_wider_than_the_area() {
        let mut h = harness(State { open: true, ..Default::default() });
        h.run();
        let edge = pos2(500.0, 300.0);
        h.hover_at(edge);
        h.step();
        h.drag_at(edge);
        h.step();
        h.hover_at(pos2(700.0, 300.0));
        h.run_steps(2);
        h.drop_at(pos2(700.0, 300.0));
        h.run();
        assert!(h.state().resized);
        // From the pointer to the area's edge, inside the window's margin
        let w = h.state().width.unwrap();
        assert!((w - 300.0).abs() < 10.0, "{w}");
        h.get_by_label("Agent panel").click();
        h.run();
        h.get_by_label("Agent panel").click();
        h.run();
        assert!((panel_width(&h) - w).abs() < 10.0, "{}", panel_width(&h));
        h.state_mut().width = Some(5000.0);
        h.run();
        assert!(panel_width(&h) <= 1002.0);
    }

    #[test]
    fn escape_puts_the_width_back() {
        let mut h = harness(State { open: true, width: Some(400.0), ..Default::default() });
        h.run();
        let edge = pos2(592.0, 300.0);
        h.hover_at(edge);
        h.step();
        h.drag_at(edge);
        h.step();
        h.hover_at(pos2(300.0, 300.0));
        h.run_steps(2);
        let w = h.state().width.unwrap();
        assert!((w - 700.0).abs() < 10.0, "{w}");
        h.key_press(Key::Escape);
        h.step();
        // Moved on, the drag is still let be
        h.hover_at(pos2(200.0, 300.0));
        h.run_steps(2);
        h.drop_at(pos2(200.0, 300.0));
        h.run();
        assert_eq!(h.state().width, Some(400.0));
        assert!(!h.state().resized);
    }
}
