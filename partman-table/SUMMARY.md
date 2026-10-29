# partman-table: summary

Everything is in `src/lib.rs`, on `egui_extras`'s `TableBuilder`. It
follows the Tables and tabulation section of the toolbox's `STYLE.md`,
and uses partman-widgets for blank rows and the reload button.

## API

- `Col::sort(SortBy::Text(..) | SortBy::Number(..))`: the column's rows'
  sort values, in the rows' order as given; text is compared without
  case. A table whose rows have nothing done to them is made with
  `Select::None`, as the style guide has it: no checkbox column.
- `Table::new(id, &keys)`, then `select(Select::None | One | Many)`,
  `column(Col::new(title).width(w).at_least(min))` for each,
  `max_height(h)` (the rows' before they scroll, 300 by default),
  `scroll(false)` (as tall as the rows, for a view that scrolls itself),
  `row_height(h)`, `reload()`, and `widths(&mut Vec<f32>)`, the columns'
  but the last, kept by the app.
- For a page of a longer list: `Col::sort(SortBy::App)` sorts by the
  app (`Shown::sorted`, the column's title and whether ascending, for it
  to fetch rows in that order), `sorted(title, ascending)` the sort until
  the user picks one, `selection(&mut BTreeSet<String>)` a selection the
  app keeps, `row_heights(&[f32])` each row's height, `names(&[String])`
  what each row is, for its control's name ("Select C1"), and
  `click_opens()` a plain click opening a row (`Shown::opened`, its place
  in keys as given).
- `show_with_agents(ui, row, actions, agents)`: the same, with buttons
  giving an agent a task on the selection on a line of their own under
  the actions and the reload button, as the style guide has them.
- `show(ui, |row, i| { row.col(..); .. }, |ui, selected| { .. })`:
  the first fills row `i`'s cells, one `col` per column given (the
  selection column is the table's); the second adds the actions,
  disabled when `selected` is empty. Returns `Shown { selected, reload }`,
  the selected keys in the rows' order, and `resized` when the user changed
  the app's `widths`.

## Behaviour worth knowing

- **State** (`State`: the selected keys and the anchor, the row last
  clicked) is in egui's memory as persisted data under the table's id,
  read and written each frame. egui_extras keeps the column widths and
  egui the scroll offset, persisted, under ids from the table's id. All
  of it outlasts a run only with eframe's persistence.
- **Scrolling**: the rows' area is reported to `partman-scroll`
  (`partman_scroll::area` on what egui_extras' `body` gives back), so a
  notch over a scrolling table moves a share of its height.
- **Scrolling to the selection** (scrolling tables only): the rows' view
  at the end of each frame (`View`: its size, whether a row shown was
  selected, the selected rows in view, a scroll to come) is in egui's
  memory, not persisted. Not drawn the frame before, or without rows then,
  a table is first shown, and scrolls the first selected row to the middle
  (over the kept offset); with none selected at the end of the last frame,
  a selection now is the app's, and is scrolled to the same way (a click
  in the table is in that record, so it isn't). A view of another size
  centres the selected rows that were in view, or puts the first at the
  top when they don't fit (`centred`). Rows' places are worked out as
  egui_extras lays them: each its height and the item spacing. The new
  offset is decided after the rows are drawn, so the pass is discarded
  (`request_discard`) and the next starts there (`vertical_scroll_offset`).
- **Out of view marks** (`out_of_view`): a double caret, footnote sized,
  on a pill of the panel's colour at the view's top or bottom edge,
  centred on the selection column (by the column widths the body
  gives), for
  selected rows wholly out of sight that way; "Selected rows above" /
  "below" for screen readers and tests. Not drawn on a pass to be
  discarded.
- **Most height:** egui_extras' least height before rows scroll (200) is
  lowered to `max_height` when that's smaller, which it would otherwise
  override.
- **Fitting:** the columns but the last start at their widths (the app's,
  given by `widths` and of the right count, else each `Col`'s), and the
  last takes the rest. When they'd be wider than the room (the widths, the
  last column's least, the selection column and the gaps), they're scaled
  down to fit, no less than 0.3, fixed rather than adjustable, under an id
  of their own so egui_extras keeps the user's for when there's room
  again. A table wider than its card would widen the card.
- **App-kept widths** take the user's once a drag's over, not at each step
  of it, and never scaled ones. They're for an app keeping widths in its
  settings; egui_extras keeps them in egui's memory regardless.
- **Sorting:** the column sorted on and the direction are kept with the
  selection, by the column's title. Rows are shown in its order (a stable
  sort, ties as given, reversed going down); `row` still gets a row's
  place as given, and `Select row N`, Shift ranges and `selected` follow
  the order shown. A sortable header's whole cell takes the click
  (`Sort by Title`), and the one sorted on has a filled arrow at its
  right, up ascending, down descending.
- **Rows gone** are dropped from the selection, but not while there are
  no rows at all, which is how a table reads while it's loading again.
  An app's selection (`selection`) is taken as it is, rows on no page
  shown included, and given back with the user's changes; select all and
  none change only the rows given, and `Shown::selected` is those shown,
  in order, then the rest.
- **App sorting:** `SortBy::App` leaves the rows as given; the header
  click and arrow are as for the others, and `Shown::sorted` says which.
  `sorted` starts the table's kept sort, the user's taking over from it.
- **Rows** are drawn with egui_extras' `heterogeneous_rows`, so only
  those in sight are laid out; their heights are `row_heights` by the
  rows' place as given, else `row_height`.
- **Click opens:** a plain click on an inert part of a row opens it; with
  Ctrl or Shift, or on the selection control, it selects as below.
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
- **Selected rows** are a tint of the theme's selection fill (0.45 of
  it), their plain text the strong text colour: the full fill and the
  selection's text colour left links and coloured text unreadable in
  some themes.
- **Names:** the header checkbox is "Select all", each row's control
  "Select row N" (or "Select <name>" with `names`), for screen readers
  and tests.
- **Hover texts** say what a click does now: the header checkbox "Select
  all", or "Select none" with every row selected; a row's checkbox
  "Select" or "Deselect"; a radio button "Select", or "Selected" when
  it's the one.
