# partman-sidebar: summary

Everything is in `src/lib.rs`. It follows the Agent Sidebar section of
the toolbox's `STYLE.md`; the pin planner's agent panel is its first
user.

## API

- `Sidebar::new(id, name, &mut open, &mut width)`, then `side(Side)`
  (right by default) and `attention(bool)`, then `show(ui, colors,
  contents)`, returning `Shown { inner, toggled, resized }`.
- `open` and `width` must be the app's own, kept between frames: the
  width changes on every frame of a drag, and `resized` is only said once
  it's let go.
- `Colors`: the control's background, triangle, hover and attention
  colours.
- `SLIDE` (0.2 s) and `MIN_WIDTH` (240).

## Behaviour worth knowing

- **Sliding**: `animate_bool_with_time` gives how far open it is, and the
  panel is that fraction of its width (`Panel::exact_size`). The contents
  are laid out at the full width and cut off by the panel, so they don't
  reflow as it moves. They're drawn while it slides shut too.
- **Width**: the user's width, else half the work area, at least
  `MIN_WIDTH` and at most the work area's width, measured each frame, so
  a smaller window shrinks it without changing what's kept.
- **Resizing** works only while fully open, by a grip 3 points either
  side of the inner edge, which says so on a hover. The grip, like the control, is in an `Area` on
  the foreground layer: on the background layer, widgets the work area
  draws later along the edge (a scroll area) took the press. A drag
  keeps the width it started from in egui's temporary data; Escape puts
  it back and ignores the drag until it's let go. Escape is looked for
  whether or not egui still reports the drag, as egui may end a drag on
  Escape itself. `resized` is true only when let go at a new width.
- **The control** sits halfway down the work area, 2 points inside the
  edge: the work area's edge while shut, the sidebar's while open. The
  triangle points the way the sidebar will move. Its accessibility node
  is a button named for the sidebar, selected while open.

## Tests

With egui_kittest: opening to half the area and shutting, the contents
drawn only while open; a dragged width kept, opened to again, and capped
to the area; and Escape mid-drag putting the width back.
