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

## Behaviour worth knowing

- **Icons are line drawings** on a unit square scaled to the button, in
  the button's text colour (weak when disabled), 1.5 points thick. A
  font's glyphs for them vary or are missing, so they aren't used.
- **An icon button is square,** a row of text plus the button padding
  above and below, framed as egui frames buttons. Disabled, it senses
  only hover, so a click does nothing and its tooltip still shows.
- **Names:** each is in its accessibility info as a button, enabled or
  not, under its icon's name or the one given; tests find it so.
