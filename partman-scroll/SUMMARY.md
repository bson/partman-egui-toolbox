# partman-scroll: summary

Everything is in `src/lib.rs`. It follows the Scrollbar behavior section
of the toolbox's `STYLE.md`.

## API

- `drive(ctx)`: once a frame, before anything's drawn. It reads the
  frame's wheel events, adds what a swipe let go of carries on with, and
  scales `InputState::smooth_scroll_delta` by the area under the pointer.
- `area(ctx, &ScrollAreaOutput)`: an area drawn this frame, its
  `inner_rect`, and whether its content is larger than that each way.
  `report(ctx, rect, overflows)` is the same for an area drawn otherwise.
- `scale_for(size)`: a notch's scale over an area that size, across and
  down: `size / REFERENCE` (400), within `SCALE` (0.5 to 4), so about a
  tenth of what it shows per notch at egui's 40 points a line.

## Behaviour worth knowing

- **It works on input, not on scroll areas**, as egui_extras' tables
  don't pass a wheel multiplier through to theirs. The areas reported
  one frame scale the next frame's scrolling: the innermost one under
  the pointer (the smallest, as nested areas lie within the ones around
  them) that has something to scroll that way, each way separately. An
  area with nothing to scroll isn't kept, so the one around it, which
  egui hands the wheel on to, sets the scale.
- **Momentum** follows a trackpad swipe from egui's `TouchPhase::Start`
  to its `End`, keeping each move's time and delta (in points; lines and
  pages are a mouse wheel's). At the end, its speed is what it moved
  since the last move at or before its last 0.1 s (or its start), over
  that time: a finger rested before lifting has moved nothing lately and
  stops. Slower than 150 points a second, it stops at once; otherwise
  the speed carries on, added to the frame's scrolling (before scaling)
  and falling by `exp(-dt / 0.35)` a frame until under 20, with a repaint
  asked for each frame meanwhile. Any wheel event, a button pressed, a
  key pressed or a touch begun stops it.
- **Platforms**: X11 marks no swipe's start or end, so has no momentum;
  macOS carries a swipe on itself (its momentum events follow `End`), so
  momentum is left out there; Windows' precision touchpads send their
  own momentum as wheel events, with no `End` to start this one.
- The state is kept in egui's temporary memory under one id, so an app
  holds nothing for it.

## Tests

- `taller_areas_scroll_further`, `the_speed_let_go_at_is_the_last_stretch`
  and `the_innermost_area_with_something_to_scroll_sets_the_scale` check
  the arithmetic.
- `a_notch_moves_by_the_area_s_height` and
  `a_swipe_let_go_of_carries_on_slowing_down` drive a 500-row list in
  kittest (at 1/60 s a frame, not kittest's 0.25 s, which makes any
  swipe slow): a notch moves 10 points times the scale for its height,
  and a swipe let go of moves on frame after frame, never faster, until
  a click stops it. The list fills its width: shrunk to its labels it's
  not under the pointer, and egui gives it no scrolling.
