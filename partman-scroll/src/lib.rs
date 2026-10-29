// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Scrolling as the toolbox's style guide (STYLE.md) has it, for every
//! scroll area in an egui app at once: a wheel notch or a trackpad swipe
//! moves a similar share of what the area shows, however tall it is, and
//! a trackpad swipe let go of mid-scroll carries on, slowing down.
//!
//! It works on the frame's input rather than on any one scroll area, as
//! egui_extras' tables don't let their own be set up: `drive` is called
//! once a frame before anything's drawn, and each scroll area tells
//! `area` where it was drawn, for the next frame's wheel to be scaled by.

use egui::{Context, Event, Id, Pos2, Rect, TouchPhase, Vec2, scroll_area::ScrollAreaOutput, vec2};

/// The size an area scrolls egui's own distance per notch at; a taller one
/// scrolls proportionally more, about a tenth of what it shows per notch.
pub const REFERENCE: f32 = 400.0;

/// The least and most a notch is scaled by: a short list still moves a
/// row or so, and a tall one moves less than a screenful.
pub const SCALE: (f32, f32) = (0.5, 4.0);

/// The swipe's last stretch its speed at release is measured over.
const WINDOW: f64 = 0.1;

/// Slower than this at release (points a second), it stops as let go.
const START: f32 = 150.0;

/// The time constant (seconds) the momentum dies down by, and the speed
/// it's taken to have stopped at.
const DECAY: f32 = 0.35;
const STOP: f32 = 20.0;

#[derive(Clone, Copy, Debug)]
struct Drawn {
    rect: Rect,
    /// It has more to show than room, across and down: something to scroll
    overflows: [bool; 2],
}

#[derive(Clone, Debug, Default)]
struct State {
    /// The areas drawn the frame before, and so far this frame
    last: Vec<Drawn>,
    now: Vec<Drawn>,
    /// The trackpad swipe under way: when it started, then each move
    swipe: Option<Vec<(f64, Vec2)>>,
    /// Speed carried on after a swipe, points a second
    coast: Vec2,
}

fn id() -> Id {
    Id::new("partman_scroll")
}

/// Scales this frame's scrolling by the area it's over and adds what a
/// swipe let go of carries on with. Called once a frame, before anything's
/// drawn; without it, scrolling is egui's own.
pub fn drive(ctx: &Context) {
    let mut s: State = ctx.data_mut(|d| d.get_temp(id())).unwrap_or_default();
    s.last = std::mem::take(&mut s.now);
    let (dt, pointer) = ctx.input(|i| {
        for e in &i.events {
            s.event(e, i.time);
        }
        (i.stable_dt, i.pointer.hover_pos())
    });
    let carried = s.carry(dt);
    if s.coast != Vec2::ZERO {
        ctx.request_repaint();
    }
    let scale = s.scale_at(pointer);
    ctx.input_mut(|i| i.smooth_scroll_delta = (i.smooth_scroll_delta + carried) * scale);
    ctx.data_mut(|d| d.insert_temp(id(), s));
}

/// Tells where a scroll area was drawn, from what showing it gave back:
/// over it, the next frame's scrolling is scaled by its size. One that
/// has nothing to scroll is left out, so what's around it scales instead.
pub fn area<R>(ctx: &Context, shown: &ScrollAreaOutput<R>) {
    let overflows = [shown.content_size.x > shown.inner_rect.width() + 0.5, shown.content_size.y > shown.inner_rect.height() + 0.5];
    report(ctx, shown.inner_rect, overflows);
}

/// `area`, for an area drawn some other way: its rect on the screen and
/// whether it has more than that to show, across and down.
pub fn report(ctx: &Context, rect: Rect, overflows: [bool; 2]) {
    if overflows == [false, false] {
        return;
    }
    ctx.data_mut(|d| d.get_temp_mut_or_default::<State>(id()).now.push(Drawn { rect, overflows }));
}

/// How much a notch over an area this size is scaled by, across and down.
pub fn scale_for(size: Vec2) -> Vec2 {
    let f = |v: f32| (v / REFERENCE).clamp(SCALE.0, SCALE.1);
    vec2(f(size.x), f(size.y))
}

impl State {
    fn event(&mut self, e: &Event, time: f64) {
        match e {
            Event::MouseWheel { unit, delta, phase, .. } => {
                // Any scrolling takes over from what was carrying on
                self.coast = Vec2::ZERO;
                match phase {
                    TouchPhase::Start => self.swipe = Some(vec![(time, Vec2::ZERO)]),
                    TouchPhase::Move => {
                        if let Some(sw) = &mut self.swipe
                            && *unit == egui::MouseWheelUnit::Point
                        {
                            sw.push((time, *delta));
                        }
                    }
                    TouchPhase::End => {
                        // The OS carries a swipe on itself on a Mac
                        if let Some(sw) = self.swipe.take()
                            && !cfg!(target_os = "macos")
                        {
                            let v = release_speed(&sw, time);
                            self.coast = if v.length() >= START { v } else { Vec2::ZERO };
                        }
                    }
                    TouchPhase::Cancel => self.swipe = None,
                }
            }
            // A click or a key stops it, as a finger put down would
            Event::PointerButton { pressed: true, .. } | Event::Key { pressed: true, .. } | Event::Touch { phase: TouchPhase::Start, .. } => {
                self.coast = Vec2::ZERO;
            }
            _ => {}
        }
    }

    /// What the momentum moves this frame, slowing it down for the next.
    fn carry(&mut self, dt: f32) -> Vec2 {
        if self.coast == Vec2::ZERO {
            return Vec2::ZERO;
        }
        let moved = self.coast * dt;
        self.coast *= (-dt / DECAY).exp();
        if self.coast.length() < STOP {
            self.coast = Vec2::ZERO;
        }
        moved
    }

    /// The scale for scrolling over a point: the innermost area under it
    /// with something to scroll, each way; nested areas lie within the
    /// ones around them, so it's the smallest.
    fn scale_at(&self, pointer: Option<Pos2>) -> Vec2 {
        let Some(p) = pointer else { return Vec2::splat(1.0) };
        let mut out = Vec2::splat(1.0);
        for d in 0..2 {
            let inner = self.last.iter().filter(|a| a.overflows[d] && a.rect.contains(p)).min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
            if let Some(a) = inner {
                out[d] = scale_for(a.rect.size())[d];
            }
        }
        out
    }
}

/// How fast a swipe was going when let go of: what it moved over its last
/// stretch, from the last move at or before that stretch began (or its
/// start), to the release. A finger rested before lifting has moved
/// nothing lately, and so stops.
fn release_speed(swipe: &[(f64, Vec2)], end: f64) -> Vec2 {
    let Some(&(first, _)) = swipe.first() else { return Vec2::ZERO };
    let from = swipe.iter().rev().map(|m| m.0).find(|&t| t <= end - WINDOW).unwrap_or(first);
    let span = (end - from) as f32;
    if span < 1e-3 {
        return Vec2::ZERO;
    }
    swipe.iter().filter(|m| m.0 > from).fold(Vec2::ZERO, |sum, m| sum + m.1) / span
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{MouseWheelUnit, Modifiers, pos2};
    use egui_kittest::Harness;

    #[test]
    fn taller_areas_scroll_further() {
        assert_eq!(scale_for(vec2(400.0, 800.0)), vec2(1.0, 2.0));
        assert_eq!(scale_for(vec2(10.0, 10_000.0)), vec2(SCALE.0, SCALE.1));
    }

    #[test]
    fn the_speed_let_go_at_is_the_last_stretch() {
        // 10 points every 10 ms down: 1000 a second
        let mut sw = vec![(0.0, Vec2::ZERO)];
        sw.extend((1..=30).map(|i| (i as f64 * 0.01, vec2(0.0, 10.0))));
        let v = release_speed(&sw, 0.30);
        assert!((v.y - 1000.0).abs() < 1.0, "{v:?}");
        // Rested a while before lifting
        assert_eq!(release_speed(&sw, 0.60), Vec2::ZERO);
        // A flick shorter than the stretch, measured from its start
        let flick = [(0.0, Vec2::ZERO), (0.02, vec2(0.0, -20.0)), (0.04, vec2(0.0, -20.0))];
        assert!((release_speed(&flick, 0.04).y + 1000.0).abs() < 1.0);
    }

    #[test]
    fn the_innermost_area_with_something_to_scroll_sets_the_scale() {
        let outer = Drawn { rect: Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1200.0)), overflows: [true, true] };
        let inner = Drawn { rect: Rect::from_min_size(pos2(100.0, 100.0), vec2(300.0, 200.0)), overflows: [false, true] };
        let s = State { last: vec![outer, inner], ..Default::default() };
        assert_eq!(s.scale_at(Some(pos2(150.0, 150.0))), vec2(2.5, 0.5), "across the outer's, down the inner's");
        assert_eq!(s.scale_at(Some(pos2(800.0, 800.0))), vec2(2.5, 3.0));
        assert_eq!(s.scale_at(None), Vec2::splat(1.0));
    }

    fn wheel(phase: TouchPhase, dy: f32) -> Event {
        Event::MouseWheel { unit: MouseWheelUnit::Point, delta: vec2(0.0, dy), phase, modifiers: Modifiers::NONE }
    }

    /// A list in a scroll area as an app draws it, driven.
    fn list(offset: &std::rc::Rc<std::cell::Cell<f32>>) -> Harness<'static> {
        let offset = offset.clone();
        // A frame a sixtieth of a second, as on a screen, not kittest's quarter
        Harness::builder().with_size(vec2(300.0, 400.0)).with_step_dt(1.0 / 60.0).build_ui(move |ui| {
            drive(ui.ctx());
            // As wide as the window, not its labels, to be under the pointer
            let shown = egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                for i in 0..500 {
                    ui.label(format!("Row {i}"));
                }
            });
            area(ui.ctx(), &shown);
            offset.set(shown.state.offset.y);
        })
    }

    #[test]
    fn a_swipe_let_go_of_carries_on_slowing_down() {
        let offset = std::rc::Rc::new(std::cell::Cell::new(0.0));
        let mut h = list(&offset);
        h.hover_at(pos2(150.0, 200.0));
        h.run();
        h.event(wheel(TouchPhase::Start, 0.0));
        h.step();
        for _ in 0..6 {
            h.event(wheel(TouchPhase::Move, -20.0));
            h.step();
        }
        h.event(wheel(TouchPhase::End, 0.0));
        h.step();
        let released = offset.get();
        assert!(released > 0.0, "the swipe scrolled");
        let mut steps = Vec::new();
        let mut last = released;
        for _ in 0..10 {
            h.step();
            steps.push(offset.get() - last);
            last = offset.get();
        }
        assert!(steps[0] > 0.0, "it carried on: {steps:?}");
        assert!(steps.windows(2).all(|w| w[1] <= w[0]), "slowing down: {steps:?}");
        // A click stops it
        h.event(Event::PointerButton { pos: pos2(150.0, 200.0), button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
        h.step();
        let stopped = offset.get();
        h.step();
        h.step();
        assert_eq!(offset.get(), stopped, "stopped by the click");
    }

    #[test]
    fn a_notch_moves_by_the_area_s_height() {
        let offset = std::rc::Rc::new(std::cell::Cell::new(0.0));
        let mut h = list(&offset);
        h.hover_at(pos2(150.0, 200.0));
        h.run();
        let shown = h.ctx.data(|d| d.get_temp::<State>(id())).unwrap().now[0].rect.height();
        h.event(wheel(TouchPhase::Move, -10.0));
        // egui spreads a notch over a few frames
        h.run_steps(60);
        let want = 10.0 * scale_for(vec2(300.0, shown)).y;
        assert!((offset.get() - want).abs() < 0.5, "moved {} for {want}", offset.get());
    }
}
