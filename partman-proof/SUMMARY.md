# partman-proof: summary

Everything is in `src/main.rs`, an eframe program (`publish = false`).

- `Proof` holds the board's `Layout` (saved with eframe's storage under
  `layout`), the made-up parts (`parts`, 60 rows of reference, value,
  package and quantity), and what the cards keep that a table doesn't:
  the single selection (the app's, so "Select a row" can set it), the
  height slider's value, the page's selection, sort direction and the row
  opened, and the checkbox and radio states. `said` is the last button
  pressed, shown in the top bar.
- `show(ui)` draws the bar (`global_theme_preference_buttons`) and the
  board, `CARDS` in the stacks of `DEFAULT`, each card by its key:
  `many_card`, `one_card`, `none_card`, `page_card`, `buttons_card`,
  `agents_card`. `logic` drives `partman-scroll`, as an app would.
- The tables' ids are `proof-*`, so their selections and scroll are kept
  in egui's memory between runs too.

## Tests

`every_card_shows` brings each card up by its tab and finds something it
shows; `selecting_a_row_scrolls_to_it` checks a row the app selects from
none is scrolled into view. `screenshots` (ignored, needs a GPU) renders
every card in dark and light to `../tmp/proof-<theme>-<n>.png`. A fitted
stack takes a few frames to come to a shorter card's height, so the tests
step several frames after each tab.
