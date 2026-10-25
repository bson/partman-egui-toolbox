// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The small parts of the toolbox's style guide (STYLE.md), for any egui
//! app following it: the blank row that sets groups apart, checkboxes and
//! radio buttons with their label an em away, text explaining a button
//! put before it, and the icon buttons that always mean the same thing.

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
    enabled: bool,
}

impl IconButton {
    pub fn new(icon: Icon) -> IconButton {
        IconButton { icon, name: None, text: None, enabled: true }
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

    pub fn enabled(mut self, enabled: bool) -> IconButton {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let pad = ui.spacing().button_padding;
        let side = row_height(ui) + 2.0 * pad.y;
        let galley = self.text.as_ref().map(|t| WidgetText::from(t.as_str()).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, TextStyle::Button));
        let width = side + galley.as_ref().map_or(0.0, |g| g.size().x + pad.x);
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), if self.enabled { Sense::click() } else { Sense::hover() });
        let name = self.name.or(self.text.clone()).unwrap_or_else(|| self.icon.name().to_string());
        let enabled = self.enabled;
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &name));
        if ui.is_rect_visible(rect) {
            let v = if enabled { ui.style().interact(&response) } else { &ui.visuals().widgets.noninteractive };
            ui.painter().rect(rect, v.corner_radius, v.weak_bg_fill, v.bg_stroke, egui::StrokeKind::Inside);
            let color: Color32 = if enabled { v.fg_stroke.color } else { ui.visuals().weak_text_color() };
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

/// An icon button, enabled: its response.
pub fn icon_button(ui: &mut Ui, icon: Icon) -> Response {
    IconButton::new(icon).show(ui)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::Queryable;

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
    fn icon_buttons_are_named_and_clicked() {
        let mut clicks = 0;
        let mut h = egui_kittest::Harness::new_ui_state(
            |ui, clicks: &mut i32| {
                ui.horizontal(|ui| {
                    for i in [Icon::Delete, Icon::Download, Icon::Open, Icon::Reload, Icon::MoveUp, Icon::MoveDown] {
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
        for name in ["Delete", "Download", "Open", "Reload", "Move up", "Move down"] {
            h.get_by_label(name).click();
            h.run();
        }
        assert_eq!(*h.state(), 6);
        h.get_by_label("Delete the selected rows").click();
        h.run();
        assert_eq!(*h.state(), 6, "disabled");
        h.get_by_label("Partman").click();
        h.run();
        assert_eq!(*h.state(), 16, "named by its text");
        clicks = *h.state();
        let _ = clicks;
    }
}
