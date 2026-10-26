// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Workflows the way the toolbox's style guide (STYLE.md) has them:
//! every step in a bar across the top, the one the user is on marked and
//! those not yet possible greyed; an advance button, named for the next
//! step, as each step's last control; and a work area that grows with
//! its contents and scrolls only when it must. The app keeps the workflow's state and says
//! what each step needs; this draws it and moves between the steps.

use egui::{Color32, CornerRadius, Id, Mesh, Rect, RichText, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};

/// What an app tells the bar and the advance button about its workflow.
/// Its state lives in the app, saved with what the workflow applies to,
/// so there's nothing to save here.
pub trait Workflow {
    type Step: Copy + PartialEq;

    /// Every step, in order.
    fn steps(&self) -> Vec<Self::Step>;
    fn current(&self) -> Self::Step;
    fn label(&self, step: Self::Step) -> String;
    /// Whether what the step needs is there, so the bar can go to it.
    fn enabled(&self, step: Self::Step) -> bool;
    /// Moves to a step that's enabled.
    fn go(&mut self, step: Self::Step);

    /// Why a step isn't enabled, shown over it in the bar.
    fn why_not(&self, _step: Self::Step) -> String {
        "Not until the steps before it are done".into()
    }

    /// The advance button's hover text.
    fn advance_hint(&self, to: Self::Step) -> String {
        format!("Go on to {}", self.label(to))
    }

    /// Whether the advance button can be pressed now: Err says why not,
    /// over it, greyed. Most buttons can be: pressing checks what's there.
    fn can_advance(&self, _to: Self::Step) -> Result<(), String> {
        Ok(())
    }

    /// The advance button pressed: checks what's there, makes what the
    /// next step needs if it can, which unlocks it, and says whether to
    /// go on. A step already enabled is gone to unless this says no.
    fn advance(&mut self, _ctx: &egui::Context, to: Self::Step) -> Advance {
        if self.enabled(to) { Advance::Go } else { Advance::Refused(self.why_not(to)) }
    }
}

/// What pressing the advance button came to.
#[derive(Clone, Debug, PartialEq)]
pub enum Advance {
    /// Go on now.
    Go,
    /// Started on what the next step needs; the app goes on when it's
    /// done.
    Started,
    /// Not yet, and why, for the app to tell the user.
    Refused(String),
}

/// The step after the current one, if any.
pub fn next<W: Workflow + ?Sized>(wf: &W) -> Option<W::Step> {
    let steps = wf.steps();
    let at = steps.iter().position(|s| *s == wf.current())?;
    steps.get(at + 1).copied()
}

/// The colours the step bar is drawn in, from the app's theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    /// A step that can be gone to
    pub fill: Color32,
    pub hover: Color32,
    /// The step the user is on
    pub current: Color32,
    pub text: Color32,
    pub current_text: Color32,
    /// A step not yet possible
    pub disabled_text: Color32,
}

impl Colors {
    pub fn from_visuals(v: &egui::Visuals) -> Colors {
        Colors {
            fill: v.widgets.inactive.weak_bg_fill,
            hover: v.widgets.hovered.weak_bg_fill,
            current: v.selection.bg_fill,
            text: v.text_color(),
            current_text: v.strong_text_color(),
            disabled_text: v.weak_text_color(),
        }
    }
}

impl Default for Colors {
    fn default() -> Colors {
        Colors::from_visuals(&egui::Visuals::dark())
    }
}

/// Space around a step's name.
const PAD: Vec2 = vec2(10.0, 4.0);
/// Between one step's point and the next one's notch.
const GAP: f32 = 2.0;

/// The bar of steps, each a button pointing on to the next: "| Plan >".
/// A click goes to a step that's enabled. True when it moved.
pub fn step_bar<W: Workflow + ?Sized>(ui: &mut Ui, colors: &Colors, wf: &mut W) -> bool {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let height = ui.fonts_mut(|f| f.row_height(&font)) + 2.0 * PAD.y;
    // How far each point reaches into the next step's notch
    let depth = (height / 3.0).round();
    let current = wf.current();
    let mut went = None;
    let spacing = std::mem::replace(&mut ui.spacing_mut().item_spacing.x, GAP - depth);
    for (i, step) in wf.steps().into_iter().enumerate() {
        let label = wf.label(step);
        let on = step == current;
        let enabled = wf.enabled(step);
        let color = if on { colors.current_text } else if enabled { colors.text } else { colors.disabled_text };
        let text = if on { RichText::new(&label).strong() } else { RichText::new(&label) };
        let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, font.clone());
        let notch = if i == 0 { 0.0 } else { depth };
        let size = vec2(notch + PAD.x + galley.size().x + PAD.x + depth, height);
        let (rect, r) = ui.allocate_exact_size(size, if enabled && !on { Sense::click() } else { Sense::hover() });
        r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, &label));
        if ui.is_rect_visible(rect) {
            let fill = if on {
                colors.current
            } else if !enabled {
                colors.fill.gamma_multiply(0.5)
            } else if r.hovered() {
                colors.hover
            } else {
                colors.fill
            };
            ui.painter().add(chevron(rect, notch, depth, fill));
            let at = pos2(rect.left() + notch + PAD.x, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(at, galley, color);
        }
        let r = if on {
            r.on_hover_text(format!("{label}: the step you're on"))
        } else if enabled {
            r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(format!("Go to {label}"))
        } else {
            r.on_hover_text(wf.why_not(step))
        };
        if r.clicked() {
            went = Some(step);
        }
    }
    ui.spacing_mut().item_spacing.x = spacing;
    if let Some(s) = went {
        wf.go(s);
    }
    went.is_some()
}

/// A step's outline: a notch on the left where the step before points
/// into it (none on the first), and a point on the right. Filled as a
/// mesh, as egui fills only convex paths, then outlined in the same
/// colour to smooth its slanted edges.
fn chevron(rect: Rect, notch: f32, depth: f32, fill: Color32) -> Shape {
    let (l, r, t, b, m) = (rect.left(), rect.right(), rect.top(), rect.bottom(), rect.center().y);
    let points = [pos2(l, t), pos2(r - depth, t), pos2(r, m), pos2(r - depth, b), pos2(l, b), pos2(l + notch, m)];
    let mut mesh = Mesh::default();
    for p in points {
        mesh.colored_vertex(p, fill);
    }
    // A fan from the notch, which sees every other corner
    for i in 0..4 {
        mesh.add_triangle(5, i, i + 1);
    }
    Shape::Vec(vec![Shape::mesh(mesh), Shape::closed_line(points.to_vec(), Stroke::new(1.0, fill))])
}

/// The advance button: the next step's name and an arrow. Pressed, it
/// asks the workflow to advance and goes on if it may; a refusal is
/// returned for the app to show. Nothing is drawn on the last step.
pub fn advance_button<W: Workflow + ?Sized>(ui: &mut Ui, wf: &mut W) -> Option<String> {
    let to = next(wf)?;
    let label = format!("{} →", wf.label(to));
    let ready = wf.can_advance(to);
    let b = egui::Button::new(RichText::new(&label).strong().color(ui.visuals().strong_text_color())).fill(ui.visuals().selection.bg_fill).corner_radius(CornerRadius::same(4));
    let r = ui.add_enabled(ready.is_ok(), b);
    let r = match &ready {
        Ok(()) => r.on_hover_text(wf.advance_hint(to)),
        Err(why) => r.on_disabled_hover_text(why),
    };
    if !r.clicked() {
        return None;
    }
    match wf.advance(ui.ctx(), to) {
        Advance::Go => {
            wf.go(to);
            None
        }
        Advance::Started => None,
        Advance::Refused(why) => Some(why),
    }
}

/// A row ending in the advance button, at its right; `left` draws what
/// goes before it, from the left.
pub fn advance_row<W: Workflow + ?Sized>(ui: &mut Ui, wf: &mut W, left: impl FnOnce(&mut Ui)) -> Option<String> {
    ui.horizontal(|ui| {
        left(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| advance_button(ui, wf)).inner
    })
    .inner
}

/// How wide a card work area is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Width {
    /// It takes the work area's width up to this
    pub max: f32,
    /// Below this the whole work area scrolls sideways rather than
    /// squeezing what's in it
    pub min: f32,
}

/// Where a work area's contents start and how tall it is, for
/// `fit_height` within it.
#[derive(Clone, Copy, Debug)]
struct Area {
    top: f32,
    height: f32,
}

fn area_id() -> Id {
    Id::new("partman_workflow::area")
}

/// A step's work area as a card: as wide as `width` allows, at a width
/// that stays put as the contents change, growing down with what's in
/// it. When it outgrows the area, tables in it scroll (`fit_height`),
/// and when they'd be too small for that, the whole area scrolls.
pub fn card_area<R>(ui: &mut Ui, width: Width, contents: impl FnOnce(&mut Ui) -> R) -> R {
    let w = ui.available_width().min(width.max).max(width.min);
    let height = ui.available_height();
    let shown = egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_width(w);
        let top = ui.cursor().top();
        ui.data_mut(|d| d.insert_temp(area_id(), Area { top, height }));
        contents(ui)
    });
    partman_scroll::area(ui.ctx(), &shown);
    shown.inner
}

/// How tall a scrolling table or list at the cursor can be and still
/// leave `below` for what comes after it in the work area: None when
/// that's less than `at_least`, when it shouldn't scroll on its own but
/// show all of itself, and leave the scrolling to the work area.
pub fn fit_height(ui: &Ui, below: f32, at_least: f32) -> Option<f32> {
    let a: Area = ui.data(|d| d.get_temp(area_id()))?;
    let left = a.height - (ui.cursor().top() - a.top) - below;
    (left >= at_least).then_some(left)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::{NodeT, Queryable};

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Step {
        Pick,
        Review,
        Done,
    }

    /// Picking enables reviewing; reviewing's advance checks a box first.
    #[derive(Default)]
    struct Flow {
        at: usize,
        picked: bool,
        checked: bool,
        said: Option<String>,
    }

    const STEPS: [Step; 3] = [Step::Pick, Step::Review, Step::Done];

    impl Workflow for Flow {
        type Step = Step;
        fn steps(&self) -> Vec<Step> {
            STEPS.to_vec()
        }
        fn current(&self) -> Step {
            STEPS[self.at]
        }
        fn label(&self, s: Step) -> String {
            format!("{s:?}")
        }
        fn enabled(&self, s: Step) -> bool {
            match s {
                Step::Pick => true,
                Step::Review => self.picked,
                Step::Done => self.picked && self.checked,
            }
        }
        fn go(&mut self, s: Step) {
            self.at = STEPS.iter().position(|x| *x == s).unwrap();
        }
        fn advance(&mut self, _ctx: &egui::Context, to: Step) -> Advance {
            match to {
                // Unlocked by the button: picking is what it checks
                Step::Review => {
                    self.picked = true;
                    Advance::Go
                }
                Step::Done if !self.checked => Advance::Refused("Check it first".into()),
                _ => Advance::Go,
            }
        }
    }

    fn harness(flow: Flow) -> egui_kittest::Harness<'static, Flow> {
        egui_kittest::Harness::builder().with_size(vec2(600.0, 300.0)).build_ui_state(
            |ui, f: &mut Flow| {
                step_bar(ui, &Colors::default(), f);
                if let Some(why) = advance_row(ui, f, |ui| {
                    ui.label("body");
                }) {
                    f.said = Some(why);
                }
            },
            flow,
        )
    }

    #[test]
    fn the_bar_goes_only_to_enabled_steps() {
        let mut h = harness(Flow::default());
        h.run();
        assert!(h.get_by_label("Review").accesskit_node().is_disabled());
        h.get_by_label("Review").click();
        h.run();
        assert_eq!(h.state().current(), Step::Pick);
        h.state_mut().picked = true;
        h.run();
        h.get_by_label("Review").click();
        h.run();
        assert_eq!(h.state().current(), Step::Review);
        h.get_by_label("Pick").click();
        h.run();
        assert_eq!(h.state().current(), Step::Pick);
    }

    #[test]
    fn advancing_unlocks_or_says_why_not() {
        let mut h = harness(Flow::default());
        h.run();
        // Review isn't enabled, but its advance button unlocks it
        h.get_by_label("Review →").click();
        h.run();
        assert_eq!(h.state().current(), Step::Review);
        assert!(h.state().picked);
        h.get_by_label("Done →").click();
        h.run();
        assert_eq!(h.state().current(), Step::Review);
        assert_eq!(h.state().said.as_deref(), Some("Check it first"));
        h.state_mut().checked = true;
        h.get_by_label("Done →").click();
        h.run();
        assert_eq!(h.state().current(), Step::Done);
        // The last step has nowhere to advance to
        assert!(h.query_by_label_contains("→").is_none());
    }

    #[test]
    fn next_follows_the_steps() {
        let mut f = Flow::default();
        assert_eq!(next(&f), Some(Step::Review));
        f.at = 2;
        assert_eq!(next(&f), None);
    }

    /// A table fits in what's left of the area, and leaves the scrolling
    /// to the whole area when that's too little.
    #[test]
    fn tables_fit_or_the_area_scrolls() {
        let fits = |height: f32, above: f32| {
            let mut got = None;
            let mut h = egui_kittest::Harness::builder().with_size(vec2(400.0, height)).build_ui_state(
                |ui, got: &mut Option<Option<f32>>| {
                    card_area(ui, Width { max: 300.0, min: 200.0 }, |ui| {
                        ui.add_space(above);
                        *got = Some(fit_height(ui, 40.0, 100.0));
                    });
                },
                got,
            );
            h.run();
            got = *h.state();
            got.unwrap()
        };
        let tall = fits(600.0, 100.0).unwrap();
        assert!(tall > 400.0 && tall < 460.0, "{tall}");
        assert_eq!(fits(250.0, 100.0), None);
    }
}
