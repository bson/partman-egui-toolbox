# partman-proof

**Run it with `cargo run -p partman-proof`** from the toolbox.

A proof sheet for the toolbox: a small program showing its components on
a board of cards, to see how they look and behave, in dark and light,
without starting Partman, the pin planner or another full app.

- **Selecting many:** a long table, rows selected by click, Shift and
  Ctrl, sorted by its headers, with actions and an agent's below; the
  selection kept in sight when the card's shown again.
- **Selecting one:** one at a time, the selection the app's: a button
  selects a row as an app would, and the table scrolls to it; a slider
  resizes it.
- **No selection:** a table with nothing to do on its rows, links in it.
- **A page:** a page of a longer list, sorted by the app, a plain click
  opening a row, rows of their own heights.
- **Buttons and text:** the text styles, plain and icon buttons enabled
  and not, checkboxes, radio buttons and an explanation before buttons.
- **Agent tasks:** agent task buttons beside plain ones.
- **KiCad symbol:** the STM32F103RGT6's symbol as KiCad plots it, a unit
  at a time.
- **3D model:** its package, KiCad's LQFP-64 model, drawn by partman-3d
  and turned about with the pointer. Drawing it needs a GPU, so it isn't
  in the automated tests.

Beside the board is an agent sidebar, opened by the triangle on the
window's right edge: an agent's panel with nothing behind it, made-up
settings, usage and a transcript (its notes, the user's messages, the
agent's answers with a plan, tool calls and a table, a compaction, and
questions waiting on the user), as the pin planner lays its own out. Its
width is kept between runs.

Every column sorts, and every control says on a hover what it does.
What a button does is said in the bar at the top, which also switches
between the system's theme, dark and light. The board's layout is kept
between runs.

## Screenshots

Screenshots of every card, in dark and light, go to the toolbox's `tmp/`
with `cargo test -p partman-proof -- --ignored` (it needs a GPU), the 3D
model's card among them.

## Assets

`assets/` holds the symbol's units, plotted by `kicad-cli sym export svg`
from a pin planner project's symbol and made PNG at 400 dpi on white, and
`LQFP-64_10x10mm_P0.5mm.step`, from KiCad's 3D model library, under
CC-BY-SA 4.0 with KiCad's exception for designs using it (in the file's
header).

## License

Copyright (C) 2026 Jan Brittenson.

partman-proof is free software under the GNU Affero General Public
License, version 3 or any later version ([LICENSE](../LICENSE)).
