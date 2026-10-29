# partman-proof

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

Every column sorts, and every control says on a hover what it does.
What a button does is said in the bar at the top, which also switches
between the system's theme, dark and light. The board's layout is kept
between runs.

## Running

```sh
cargo run -p partman-proof
```

Screenshots of every card, in dark and light, go to the toolbox's `tmp/`
with `cargo test -p partman-proof -- --ignored` (it needs a GPU).

## License

Copyright (C) 2026 Jan Brittenson.

partman-proof is free software under the GNU Affero General Public
License, version 3 or any later version ([LICENSE](../LICENSE)).
