# partman-table: summary

Everything is in `src/lib.rs`, on `egui_extras`'s `TableBuilder`. It
follows the Tables and tabulation section of the toolbox's `STYLE.md`,
and uses partman-widgets for blank rows and the reload button.

## API

- `Table::new(id, &keys)`, then `select(Select::None | One | Many)`,
  `column(Col::new(title).width(w).at_least(min))` for each,
  `max_height(h)` (the rows' before they scroll, 300 by default),
  `row_height(h)` and `reload()`.
- `show(ui, |row, i| { row.col(..); .. }, |ui, selected| { .. })`:
  the first fills row `i`'s cells, one `col` per column given (the
  selection column is the table's); the second adds the actions,
  disabled when `selected` is empty. Returns `Shown { selected, reload }`,
  the selected keys in the rows' order.

## Behaviour worth knowing

- **State** (`State`: the selected keys and the anchor, the row last
  clicked) is in egui's memory as persisted data under the table's id,
  read and written each frame. egui_extras keeps the column widths and
  egui the scroll offset, persisted, under ids from the table's id. All
  of it outlasts a run only with eframe's persistence.
- **Rows gone** are dropped from the selection, but not while there are
  no rows at all, which is how a table reads while it's loading again.
- **Clicks:** the table senses clicks on rows; a widget in a cell takes
  its own click first, so a row click is on an inert part. Selectable
  labels are turned off in the body, as they'd take the click. The
  selection control's click is told apart, so it isn't counted twice.
- **Many:** a click toggles the row; Shift with an anchor sets every row
  between the anchor and it to the state the clicked row takes, so a
  range goes on or off together. Ctrl (Command) is a toggle, as a plain
  click is. **One:** any click selects that row alone.
- **Select all** is on when every row is selected, and pressing it then
  selects none.
- **Names:** the header checkbox is "Select all", each row's control
  "Select row N", for screen readers and tests.
