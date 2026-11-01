# partman-widgets: summary

Everything is in `src/lib.rs`. It follows the General Layout and Buttons
sections of the toolbox's `STYLE.md`.

## API

- `row_height(ui)`: the body text style's height.
- `blank_row(ui)`: `add_space` of 1.5 rows. egui adds its item spacing
  around it as around anything, so the gap seen is that much more.
- `em(ui)`: the body font's "m" width, from the font itself.
- `check(ui, &mut bool, label)`, `radio(ui, selected, label)`: egui's
  own, with `icon_spacing` set to an em for them.
- `explanation(ui, text)`: a blank row, then the text.
- `Icon`: `Delete`, `Download`, `Open`, `Reload`, `MoveUp`, `MoveDown`,
  `Check`, `Edit`,
  with `name()` and `paint(painter, rect, stroke)`.
- `AgentButton::new(task)`, then `color(..)` (`AGENT_RED` by default),
  `family(..)` (a bold font family for the A) and `enabled(..)`, then
  `show(ui)`. The A is 1.3 times the button text, slanted (egui's
  italics); without a bold family it's drawn twice 0.7 apart. An em
  after it, the task; 4 taller than a plain button. Named "Agent: task".
- `IconButton::new(icon)`, then `name(..)`, `text(..)` (after the icon,
  and its name unless one's given), `color(..)` (while enabled, where the
  colour says something) and `enabled(..)`, then
  `show(ui)`; `icon_button(ui, icon)` for the plain one.

- `ProgressBar::new(done)` (0.0 to 1.0, clamped, NaN as 0), then
  `word(..)`, then `show(ui)`: the word, the bar and the percentage on one
  row as wide as the ui, the bar ending where "100%" would start. Named
  "word, N%" (or "N%"), a `ProgressIndicator` with `value` the fraction.
- `SendBox::new(id, &mut text, &mut height)`, then `hint(..)`, `name(..)`
  (what Enter and the arrow do, "Send" by default) and `enabled(..)`,
  then `show(ui)`, giving `SendShown { sent, resized, response }`.
  `SEND_BLUE` is the arrow's colour: `AGENT_RED`'s saturation and
  brightness at a blue hue.

## Behaviour worth knowing

- **Send box keys:** egui's multiline box has one key for a new line, so
  it's given Alt-Enter (Option on a Mac), a Shift-Enter is made into
  that, and a plain Enter is taken out of the frame's events while the
  box has focus, before the box sees it, and sends. Enter with nothing
  written does nothing, a new line included.
- **Send box layout:** a frame like egui's text box, the text in a scroll
  area left of the arrow's square (1.4 rows) at the bottom right, the
  frame outlined in the selection's colour while focused. The arrow is
  a broad shaft and a head, weak while there's nothing to send, lighter
  when hovered; it's a button named by `name`. The height is the app's,
  three rows and the margins until dragged, kept between two rows and
  half the window (not what's left where it's drawn, which in a panel
  sized to its contents is only what it took last frame). The top edge,
  3 points either side, drags it; `resized` is said when let go.

- **Icons are line drawings** on a unit square scaled to the button, in
  the button's text colour, 1.5 points thick. A font's glyphs for them
  vary or are missing, so they aren't used.
- **An icon button is square,** a row of text plus the button padding
  above and below, framed as egui frames buttons; with text after the
  icon, wider by it. Disabled, it's drawn as egui draws its own
  disabled buttons, its inactive look faded (`Visuals::disable`), fill,
  outline, icon and text alike: the theme's weak text colour can be
  nearly as bright as its text, which made one with text look enabled.
  Its text is laid out in the colour it's drawn in, not plain, as a
  theme's `override_text_color` would be fixed in it otherwise. It
  senses only hover, so a click does nothing and its tooltip still
  shows.
- **Names:** each is in its accessibility info as a button, enabled or
  not, under its icon's name or the one given; tests find it so.
