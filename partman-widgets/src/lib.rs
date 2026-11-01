// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The small parts of the toolbox's style guide (STYLE.md), for any egui
//! app following it: the blank row that sets groups apart, checkboxes and
//! radio buttons with their label an em away, text explaining a button
//! put before it, the icon buttons that always mean the same thing, and a
//! text box that Enter acts on, by an arrow in its corner, and a bar
//! saying how far along some work is.

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, TextStyle, Ui, Vec2, WidgetInfo, WidgetText, WidgetType, vec2};

/// A row of body text's height.
pub fn row_height(ui: &Ui) -> f32 {
    ui.text_style_height(&TextStyle::Body)
}

/// A blank row: one and a half rows of body text, the space between groups
/// of elements, under a heading and below buttons.
pub fn blank_row(ui: &mut Ui) {
    ui.add_space(1.5 * row_height(ui));
}

/// An em: the width of the body font's "m", the gap between a checkbox or
/// radio button and its label.
pub fn em(ui: &Ui) -> f32 {
    let font = TextStyle::Body.resolve(ui.style());
    ui.ctx().fonts_mut(|f| f.glyph_width(&font, 'm'))
}

/// A checkbox with its label to its right, an em away.
pub fn check(ui: &mut Ui, on: &mut bool, label: impl Into<WidgetText>) -> Response {
    ui.scope(|ui| {
        ui.spacing_mut().icon_spacing = em(ui);
        ui.checkbox(on, label)
    })
    .inner
}

/// A radio button with its label to its right, an em away.
pub fn radio(ui: &mut Ui, selected: bool, label: impl Into<WidgetText>) -> Response {
    ui.scope(|ui| {
        ui.spacing_mut().icon_spacing = em(ui);
        ui.radio(selected, label)
    })
    .inner
}

/// Text that goes with the button after it (a warning, what it does):
/// on the line before it, set off above by a blank row.
pub fn explanation(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    blank_row(ui);
    ui.label(text)
}

/// What an icon button does. Each is always drawn the same way, so it
/// reads the same wherever it's used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// A wastebasket
    Delete,
    /// An open box with an arrow down into it
    Download,
    /// A box with an arrow up and out of it to the right: following a
    /// link, opening something in another app
    Open,
    /// A circular arrow chasing its tail
    Reload,
    /// A double caret up
    MoveUp,
    /// A double caret down
    MoveDown,
    /// A check mark: marking something checked, verified or done
    Check,
    /// A pen, its nib down to the left: editing something
    Edit,
}

impl Icon {
    /// What it's called, for screen readers and tests, when the button
    /// isn't given a name of its own.
    pub fn name(self) -> &'static str {
        match self {
            Icon::Delete => "Delete",
            Icon::Download => "Download",
            Icon::Open => "Open",
            Icon::Reload => "Reload",
            Icon::MoveUp => "Move up",
            Icon::MoveDown => "Move down",
            Icon::Check => "Check",
            Icon::Edit => "Edit",
        }
    }

    /// Draws it in rect, a square, in stroke. Shapes are on a unit square,
    /// scaled to the rect.
    pub fn paint(self, painter: &egui::Painter, rect: Rect, stroke: Stroke) {
        let at = |x: f32, y: f32| -> Pos2 { rect.min + vec2(x * rect.width(), y * rect.height()) };
        let line = |pts: &[(f32, f32)]| egui::Shape::line(pts.iter().map(|&(x, y)| at(x, y)).collect(), stroke);
        let mut shapes = vec![];
        match self {
            Icon::Delete => {
                shapes.push(line(&[(0.18, 0.26), (0.82, 0.26)]));
                shapes.push(line(&[(0.40, 0.26), (0.40, 0.15), (0.60, 0.15), (0.60, 0.26)]));
                shapes.push(line(&[(0.26, 0.32), (0.32, 0.88), (0.68, 0.88), (0.74, 0.32)]));
                shapes.push(line(&[(0.44, 0.40), (0.45, 0.78)]));
                shapes.push(line(&[(0.56, 0.40), (0.55, 0.78)]));
            }
            Icon::Download => {
                shapes.push(line(&[(0.18, 0.56), (0.18, 0.86), (0.82, 0.86), (0.82, 0.56)]));
                shapes.push(line(&[(0.50, 0.12), (0.50, 0.66)]));
                shapes.push(line(&[(0.32, 0.48), (0.50, 0.66), (0.68, 0.48)]));
            }
            Icon::Open => {
                shapes.push(line(&[(0.52, 0.20), (0.18, 0.20), (0.18, 0.82), (0.80, 0.82), (0.80, 0.48)]));
                shapes.push(line(&[(0.44, 0.56), (0.84, 0.16)]));
                shapes.push(line(&[(0.58, 0.16), (0.84, 0.16), (0.84, 0.42)]));
            }
            Icon::Reload => {
                // Most of a circle, clockwise from the top right, its end an
                // arrowhead pointing on round
                let (c, r) = ((0.5, 0.52), 0.32);
                let pt = |a: f32| (c.0 + r * a.cos(), c.1 + r * a.sin());
                let (from, to) = (-1.1_f32, 4.1_f32);
                let arc: Vec<(f32, f32)> = (0..=24).map(|i| pt(from + (to - from) * i as f32 / 24.0)).collect();
                shapes.push(line(&arc));
                let end = pt(to);
                let tangent = (-to.sin(), to.cos());
                let normal = (to.cos(), to.sin());
                let tip = (end.0 + 0.10 * tangent.0, end.1 + 0.10 * tangent.1);
                shapes.push(line(&[(tip.0 - 0.14 * tangent.0 + 0.10 * normal.0, tip.1 - 0.14 * tangent.1 + 0.10 * normal.1), tip, (tip.0 - 0.14 * tangent.0 - 0.10 * normal.0, tip.1 - 0.14 * tangent.1 - 0.10 * normal.1)]));
            }
            Icon::MoveUp => {
                shapes.push(line(&[(0.24, 0.50), (0.50, 0.24), (0.76, 0.50)]));
                shapes.push(line(&[(0.24, 0.78), (0.50, 0.52), (0.76, 0.78)]));
            }
            Icon::MoveDown => {
                shapes.push(line(&[(0.24, 0.22), (0.50, 0.48), (0.76, 0.22)]));
                shapes.push(line(&[(0.24, 0.50), (0.50, 0.76), (0.76, 0.50)]));
            }
            Icon::Check => {
                shapes.push(line(&[(0.20, 0.54), (0.42, 0.76), (0.82, 0.26)]));
            }
            Icon::Edit => {
                // The body's outline from the nib round, then the band
                // where the nib meets the body and one near its end
                shapes.push(line(&[(0.18, 0.82), (0.38, 0.74), (0.82, 0.30), (0.70, 0.18), (0.26, 0.62), (0.18, 0.82)]));
                shapes.push(line(&[(0.26, 0.62), (0.38, 0.74)]));
                shapes.push(line(&[(0.60, 0.28), (0.72, 0.40)]));
            }
        }
        painter.extend(shapes);
    }
}

/// A button showing an icon, as tall as a row of text, named for screen
/// readers by the icon's name or one given, and saying on hover what it
/// does.
pub struct IconButton {
    icon: Icon,
    name: Option<String>,
    text: Option<String>,
    color: Option<Color32>,
    enabled: bool,
}

impl IconButton {
    pub fn new(icon: Icon) -> IconButton {
        IconButton { icon, name: None, text: None, color: None, enabled: true }
    }

    /// The name screen readers say, where the icon's own isn't specific
    /// enough ("Delete the selected rows").
    pub fn name(mut self, name: impl Into<String>) -> IconButton {
        self.name = Some(name.into());
        self
    }

    /// Text after the icon, where the icon alone wouldn't say enough
    /// (opening which app). It's the name too, unless one's given.
    pub fn text(mut self, text: impl Into<String>) -> IconButton {
        self.text = Some(text.into());
        self
    }

    /// The colour it's drawn in while enabled, where the colour says
    /// something (a check mark green); else the button's text colour.
    pub fn color(mut self, color: Color32) -> IconButton {
        self.color = Some(color);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> IconButton {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let pad = ui.spacing().button_padding;
        let side = row_height(ui) + 2.0 * pad.y;
        // Laid out in the colour it's drawn in: laid out plain, a theme's
        // override_text_color would be fixed in it, and the colour given
        // when painting ignored, disabled or not
        let font = TextStyle::Button.resolve(ui.style());
        let galley = self.text.as_ref().map(|t| ui.painter().layout_no_wrap(t.clone(), font, Color32::PLACEHOLDER));
        let width = side + galley.as_ref().map_or(0.0, |g| g.size().x + pad.x);
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), if self.enabled { Sense::click() } else { Sense::hover() });
        let name = self.name.or(self.text.clone()).unwrap_or_else(|| self.icon.name().to_string());
        let enabled = self.enabled;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name));
        if ui.is_rect_visible(rect) {
            // Disabled, as egui's own buttons are: their inactive look,
            // faded; the theme's weak text colour can be nearly as bright
            // as its text, which made one with text look enabled
            let v = if enabled { ui.style().interact(&response) } else { &ui.visuals().widgets.inactive };
            let fade = |c: Color32| if enabled { c } else { ui.visuals().disable(c) };
            ui.painter().rect(rect, v.corner_radius, fade(v.weak_bg_fill), Stroke::new(v.bg_stroke.width, fade(v.bg_stroke.color)), egui::StrokeKind::Inside);
            let color = fade(self.color.unwrap_or(v.fg_stroke.color));
            let icon = Rect::from_center_size(rect.left_center() + vec2(side / 2.0, 0.0), Vec2::splat(row_height(ui)));
            self.icon.paint(ui.painter(), icon, Stroke::new(1.5, color));
            if let Some(g) = galley {
                let at = egui::pos2(rect.left() + side, rect.center().y - g.size().y / 2.0);
                ui.painter().galley(at, g, color);
            }
        }
        response
    }
}

/// A button giving an agent a task, as the style guide has them: a little
/// taller than other buttons, a larger, slanted, bold A in its own colour
/// at its left, standing for "Agent", then an em, then the task. Named
/// "Agent: task" for screen readers, which don't see the A.
pub struct AgentButton {
    text: String,
    color: Color32,
    family: Option<egui::FontFamily>,
    enabled: bool,
}

/// The A's colour unless one's given.
pub const AGENT_RED: Color32 = Color32::from_rgb(0xe0, 0x4a, 0x3f);

impl AgentButton {
    pub fn new(text: impl Into<String>) -> AgentButton {
        AgentButton { text: text.into(), color: AGENT_RED, family: None, enabled: true }
    }

    /// The A's colour.
    pub fn color(mut self, color: Color32) -> AgentButton {
        self.color = color;
        self
    }

    /// A bold font family for the A, where the app has one; without, the
    /// body font's A is drawn twice a little apart to be bolder.
    pub fn family(mut self, family: egui::FontFamily) -> AgentButton {
        self.family = Some(family);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> AgentButton {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let pad = ui.spacing().button_padding;
        let body = TextStyle::Button.resolve(ui.style());
        let a_font = egui::FontId::new(body.size * 1.3, self.family.clone().unwrap_or(body.family.clone()));
        let enabled = self.enabled;
        let a_color = if enabled { self.color } else { ui.visuals().weak_text_color() };
        let a = WidgetText::from(egui::RichText::new("A").font(a_font).italics().color(a_color)).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, TextStyle::Button);
        let text = WidgetText::from(self.text.as_str()).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, TextStyle::Button);
        let gap = em(ui);
        // Taller than other buttons, by a little
        let height = row_height(ui).max(a.size().y) + 2.0 * pad.y + 4.0;
        let width = pad.x + a.size().x + gap + text.size().x + pad.x;
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), if enabled { Sense::click() } else { Sense::hover() });
        let name = format!("Agent: {}", self.text);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name));
        if ui.is_rect_visible(rect) {
            let v = if enabled { ui.style().interact(&response) } else { &ui.visuals().widgets.noninteractive };
            ui.painter().rect(rect, v.corner_radius, v.weak_bg_fill, v.bg_stroke, egui::StrokeKind::Inside);
            let at = egui::pos2(rect.left() + pad.x, rect.center().y - a.size().y / 2.0);
            ui.painter().galley(at, a.clone(), a_color);
            if self.family.is_none() {
                ui.painter().galley(at + vec2(0.7, 0.0), a.clone(), a_color);
            }
            let color = if enabled { v.fg_stroke.color } else { ui.visuals().weak_text_color() };
            let tx = egui::pos2(at.x + a.size().x + gap, rect.center().y - text.size().y / 2.0);
            ui.painter().galley(tx, text, color);
        }
        response
    }
}

/// A send box's arrow: the agent button's A's saturation and brightness,
/// in blue.
pub const SEND_BLUE: Color32 = Color32::from_rgb(0x3f, 0x8a, 0xe0);

/// A text box Enter acts on (sending what's typed to an agent, say), as
/// the style guide has it: no button under it, but a fat blue arrow in its
/// bottom right corner that does what Enter does. A new line is typed with
/// Alt-Enter (Option-Enter on macOS) or Shift-Enter. Its height is the
/// app's to keep, changed by dragging its top edge.
pub struct SendBox<'a> {
    id: egui::Id,
    text: &'a mut String,
    height: &'a mut Option<f32>,
    hint: String,
    name: String,
    enabled: bool,
}

/// What came of showing it.
pub struct SendShown {
    /// Enter was pressed, or the arrow, with something typed: the app
    /// takes the text
    pub sent: bool,
    /// Its height was dragged to another and let go, for the app to save
    pub resized: bool,
    /// The text box's
    pub response: Response,
}

impl<'a> SendBox<'a> {
    /// The box under id, its text and height the app's, kept between
    /// frames; with no height, three rows.
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, text: &'a mut String, height: &'a mut Option<f32>) -> SendBox<'a> {
        SendBox { id: egui::Id::new(id), text, height, hint: String::new(), name: "Send".into(), enabled: true }
    }

    /// What's shown in it while empty.
    pub fn hint(mut self, hint: impl Into<String>) -> SendBox<'a> {
        self.hint = hint.into();
        self
    }

    /// What Enter and the arrow do, as the arrow is named: "Send" unless
    /// given.
    pub fn name(mut self, name: impl Into<String>) -> SendBox<'a> {
        self.name = name.into();
        self
    }

    /// Whether Enter and the arrow act now; the box can still be typed in.
    pub fn enabled(mut self, enabled: bool) -> SendBox<'a> {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> SendShown {
        let row = row_height(ui);
        let margin = ui.spacing().button_padding.x;
        let least = 2.0 * row + 2.0 * margin;
        // At most half the window: what's left where it's drawn can be no
        // more than it took last frame, in a panel sized to its contents
        let most = (ui.ctx().content_rect().height() / 2.0).max(least);
        let h = self.height.unwrap_or(3.0 * row + 2.0 * margin).clamp(least, most);
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());

        // Its top edge, a few points either side, drags its height: it sits
        // at the bottom of what it's in, so it grows upward
        let grip = Rect::from_min_max(rect.min - vec2(0.0, 3.0), egui::pos2(rect.max.x, rect.min.y + 3.0));
        let g = ui.interact(grip, self.id.with("grip"), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeVertical).on_hover_text("Drag to make it taller or shorter");
        if g.dragged() {
            *self.height = Some((h - g.drag_delta().y).clamp(least, most));
        }
        let resized = g.drag_stopped();

        let can = self.enabled && !self.text.trim().is_empty();
        let edit_id = self.id.with("text");
        // Enter is taken before the box sees it, as egui's box has one key
        // for a new line: Alt-Enter, which Shift-Enter is made into
        let mut entered = false;
        if ui.memory(|m| m.has_focus(edit_id)) {
            ui.input_mut(|i| {
                i.events.retain_mut(|e| match e {
                    egui::Event::Key { key: egui::Key::Enter, pressed, modifiers, .. } => {
                        if modifiers.shift && !modifiers.alt {
                            modifiers.shift = false;
                            modifiers.alt = true;
                            true
                        } else if modifiers.is_none() {
                            entered |= *pressed;
                            false
                        } else {
                            true
                        }
                    }
                    _ => true,
                })
            });
        }

        let focused = ui.memory(|m| m.has_focus(edit_id));
        let v = ui.visuals();
        let stroke = if focused { v.selection.stroke } else { v.widgets.inactive.bg_stroke };
        ui.painter().rect(rect, v.widgets.inactive.corner_radius, v.extreme_bg_color, stroke, egui::StrokeKind::Inside);

        // The arrow's square, at the bottom right, the text kept left of it
        let a = (row * 1.4).round();
        let arrow = Rect::from_min_size(rect.max - vec2(a + margin / 2.0, a + margin / 2.0), vec2(a, a));
        let text_rect = Rect::from_min_max(rect.min + vec2(margin, margin / 2.0), egui::pos2(arrow.min.x - margin / 2.0, rect.max.y - margin / 2.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_rect).layout(egui::Layout::top_down(egui::Align::Min)).id_salt(self.id.with("box")));
        let response = egui::ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.add(
                    egui::TextEdit::multiline(self.text)
                        .id(edit_id)
                        .frame(egui::Frame::NONE)
                        .hint_text(self.hint.as_str())
                        .desired_width(f32::INFINITY)
                        .min_size(vec2(text_rect.width(), text_rect.height()))
                        .return_key(egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::Enter)),
                )
            })
            .inner;

        let pressed = ui.interact(arrow, self.id.with("arrow"), if can { Sense::click() } else { Sense::hover() });
        let name = self.name.clone();
        pressed.widget_info(|| WidgetInfo::labeled(WidgetType::Button, can, &name));
        let newline = if ui.ctx().os() == egui::os::OperatingSystem::Mac { "Option-Enter" } else { "Alt-Enter" };
        let tip = if can {
            format!("{} (Enter); {newline} or Shift-Enter for a new line", self.name)
        } else {
            format!("{} (Enter), once something's written; {newline} or Shift-Enter for a new line", self.name)
        };
        let pressed = pressed.on_hover_text(tip);
        let color = if !can {
            ui.visuals().weak_text_color()
        } else if pressed.hovered() {
            SEND_BLUE.lerp_to_gamma(Color32::WHITE, 0.2)
        } else {
            SEND_BLUE
        };
        paint_arrow(ui.painter(), arrow, color);

        SendShown { sent: can && (entered || pressed.clicked()), resized, response }
    }
}

/// A fat right arrow filling most of a square: a broad shaft, and a head
/// as tall as most of it.
fn paint_arrow(painter: &egui::Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| egui::pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let shaft = vec![at(0.14, 0.38), at(0.56, 0.38), at(0.56, 0.62), at(0.14, 0.62)];
    let head = vec![at(0.50, 0.16), at(0.88, 0.50), at(0.50, 0.84)];
    painter.add(egui::Shape::convex_polygon(shaft, color, Stroke::NONE));
    painter.add(egui::Shape::convex_polygon(head, color, Stroke::NONE));
}

/// An icon button, enabled: its response.
pub fn icon_button(ui: &mut Ui, icon: Icon) -> Response {
    IconButton::new(icon).show(ui)
}

/// How far along a piece of work is: a word for what it's doing now
/// ("Syncing"), then a bar filled from 0 to 100%, then the percentage, on
/// one row as wide as the ui. Named "word, N%" for screen readers.
pub struct ProgressBar {
    done: f32,
    word: String,
}

impl ProgressBar {
    /// done is the part finished, 0.0 to 1.0; outside that it's clamped.
    pub fn new(done: f32) -> ProgressBar {
        ProgressBar { done: if done.is_nan() { 0.0 } else { done.clamp(0.0, 1.0) }, word: String::new() }
    }

    /// What it's doing now, in a word or two.
    pub fn word(mut self, word: impl Into<String>) -> ProgressBar {
        self.word = word.into();
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let percent = (self.done * 100.0).floor() as u32;
        let font = TextStyle::Body.resolve(ui.style());
        let color = ui.visuals().text_color();
        let word = ui.painter().layout_no_wrap(self.word.clone(), font.clone(), color);
        // As wide as 100% always, so the bar doesn't move as the number grows
        let widest = ui.painter().layout_no_wrap("100%".into(), font.clone(), color).size().x;
        let number = ui.painter().layout_no_wrap(format!("{percent}%"), font, color);
        let gap = em(ui);
        let height = row_height(ui);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let name = if self.word.is_empty() { format!("{percent}%") } else { format!("{}, {percent}%", self.word) };
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ProgressIndicator, true, &name);
            info.value = Some(f64::from(self.done));
            info
        });
        if ui.is_rect_visible(rect) {
            let word_w = if self.word.is_empty() { 0.0 } else { word.size().x + gap };
            let mid = rect.center().y;
            ui.painter().galley(egui::pos2(rect.left(), mid - word.size().y / 2.0), word, color);
            ui.painter().galley(egui::pos2(rect.right() - number.size().x, mid - number.size().y / 2.0), number, color);
            let track = Rect::from_min_max(egui::pos2(rect.left() + word_w, mid - height * 0.3), egui::pos2(rect.right() - widest - gap, mid + height * 0.3));
            if track.width() > 0.0 {
                let v = ui.visuals();
                let radius = track.height() / 2.0;
                ui.painter().rect(track, radius, v.extreme_bg_color, Stroke::new(1.0, v.widgets.noninteractive.bg_stroke.color), egui::StrokeKind::Inside);
                if self.done > 0.0 {
                    // Never thinner than it is tall, so a little progress still shows as a rounded end
                    let fill = Rect::from_min_size(track.min, vec2((track.width() * self.done).max(track.height()), track.height()));
                    ui.painter().rect_filled(fill, radius, v.selection.bg_fill);
                }
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

    struct Typed {
        text: String,
        height: Option<f32>,
        sent: Vec<String>,
        resized: bool,
    }

    fn send_box() -> egui_kittest::Harness<'static, Typed> {
        egui_kittest::Harness::builder().with_size(vec2(400.0, 300.0)).build_ui_state(
            |ui, t: &mut Typed| {
                ui.add_space(150.0);
                let shown = SendBox::new("send", &mut t.text, &mut t.height).hint("Write here").show(ui);
                if shown.sent {
                    t.sent.push(std::mem::take(&mut t.text));
                }
                t.resized |= shown.resized;
            },
            Typed { text: String::new(), height: None, sent: vec![], resized: false },
        )
    }

    /// Enter sends, Shift-Enter and Alt-Enter make a new line, the arrow
    /// sends, and nothing's sent while empty.
    #[test]
    fn enter_sends_and_its_modifiers_make_lines() {
        let mut h = send_box();
        h.run();
        h.get_by_role(egui::accesskit::Role::MultilineTextInput).focus();
        h.run();
        h.key_press(egui::Key::Enter);
        h.run();
        assert!(h.state().sent.is_empty() && h.state().text.is_empty(), "nothing to send, and no new line");
        h.get_by_role(egui::accesskit::Role::MultilineTextInput).type_text("one");
        h.run();
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Enter);
        h.run();
        h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::Enter);
        h.run();
        assert_eq!(h.state().text, "one\n\n");
        h.key_press(egui::Key::Enter);
        h.run();
        assert_eq!(h.state().sent, ["one\n\n"]);
        h.state_mut().text = "two".into();
        h.run();
        h.get_by_label("Send").click();
        h.run();
        assert_eq!(h.state().sent.len(), 2, "by the arrow");
    }

    /// Dragging its top edge up makes it taller, kept by the app and said
    /// once let go.
    #[test]
    fn its_top_edge_resizes_it() {
        let mut h = send_box();
        h.run();
        // The box's top, where the space above it ends
        let edge = egui::pos2(200.0, 150.0 + 1.0);
        h.hover_at(edge);
        h.run();
        h.drag_at(edge);
        h.run();
        h.hover_at(edge - vec2(0.0, 40.0));
        h.run();
        h.drop_at(edge - vec2(0.0, 40.0));
        h.run();
        let tall = h.state().height.expect("kept");
        assert!(tall > 80.0, "{tall}");
        assert!(h.state().resized);
    }

    #[test]
    fn a_blank_row_is_a_row_and_a_half() {
        let mut h = egui_kittest::Harness::new_ui(|ui| {
            let top = ui.cursor().top();
            blank_row(ui);
            // On top of the spacing egui puts between any two items
            let gap = ui.cursor().top() - top - 1.5 * row_height(ui);
            assert!((0.0..=ui.spacing().item_spacing.y + 0.01).contains(&gap), "{gap}");
        });
        h.run();
    }

    #[test]
    fn a_label_an_em_from_its_checkbox() {
        let mut on = false;
        let mut h = egui_kittest::Harness::new_ui(|ui| {
            let em = em(ui);
            assert!(em > 4.0, "{em}");
            check(ui, &mut on, "Shell commands too");
            radio(ui, true, "One");
        });
        h.run();
        assert!(h.query_by_label("Shell commands too").is_some() && h.query_by_label("One").is_some());
    }

    #[test]
    fn agent_buttons_are_taller_and_named_agent() {
        let mut h = egui_kittest::Harness::new_ui_state(
            |ui, clicks: &mut i32| {
                ui.horizontal(|ui| {
                    let plain = ui.button("Plain").rect.height();
                    let r = AgentButton::new("Review").show(ui);
                    assert!(r.rect.height() > plain, "{} {plain}", r.rect.height());
                    if r.clicked() {
                        *clicks += 1;
                    }
                    AgentButton::new("Busy").enabled(false).show(ui);
                });
            },
            0,
        );
        h.run();
        h.get_by_label("Agent: Review").click();
        h.run();
        h.get_by_label("Agent: Busy").click();
        h.run();
        assert_eq!(*h.state(), 1);
    }

    #[test]
    fn icon_buttons_are_named_and_clicked() {
        let mut clicks = 0;
        let mut h = egui_kittest::Harness::new_ui_state(
            |ui, clicks: &mut i32| {
                ui.horizontal(|ui| {
                    for i in [Icon::Delete, Icon::Download, Icon::Open, Icon::Reload, Icon::MoveUp, Icon::MoveDown, Icon::Check, Icon::Edit] {
                        if icon_button(ui, i).clicked() {
                            *clicks += 1;
                        }
                    }
                    IconButton::new(Icon::Delete).name("Delete the selected rows").enabled(false).show(ui);
                    if IconButton::new(Icon::Open).text("Partman").show(ui).clicked() {
                        *clicks += 10;
                    }
                });
            },
            clicks,
        );
        h.run();
        for name in ["Delete", "Download", "Open", "Reload", "Move up", "Move down", "Check", "Edit"] {
            h.get_by_label(name).click();
            h.run();
        }
        assert_eq!(*h.state(), 8);
        h.get_by_label("Delete the selected rows").click();
        h.run();
        assert_eq!(*h.state(), 8, "disabled");
        h.get_by_label("Partman").click();
        h.run();
        assert_eq!(*h.state(), 18, "named by its text");
        clicks = *h.state();
        let _ = clicks;
    }

    /// Named by its word and percentage, clamped to 0–100%, and as wide
    /// as the ui whatever the number.
    #[test]
    fn progress_bar_says_how_far() {
        let mut h = egui_kittest::Harness::builder().with_size(vec2(300.0, 100.0)).build_ui_state(
            |ui, done: &mut f32| {
                ProgressBar::new(*done).word("Syncing").show(ui);
                ProgressBar::new(1.7).show(ui);
            },
            0.425_f32,
        );
        h.run();
        assert!(h.query_by_label("Syncing, 42%").is_some());
        assert!(h.query_by_label("100%").is_some());
        *h.state_mut() = f32::NAN;
        h.run();
        assert!(h.query_by_label("Syncing, 0%").is_some());
    }
}
