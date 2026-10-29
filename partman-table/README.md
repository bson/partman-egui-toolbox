# partman-table

Tables for [egui](https://github.com/emilk/egui) the way the toolbox's
style and design guide, [STYLE.md](../STYLE.md), has them:

- **Headed and adjustable.** A header over every column, columns the
  user can drag wider or narrower, and the last taking the rest of the
  room, so a table spans its card.
- **Rows selected,** one at a time by radio button or any number by
  checkbox, with one in the header to select all (or, with all selected,
  none). Clicking anywhere in a row that's inert selects it too: with
  Ctrl (Cmd on a Mac) it toggles that row, with Shift every row from the
  one last clicked. A link or button in a row still does its own thing.
- **Actions below,** in a row of buttons acting on the selection,
  disabled while nothing's selected, with a reload button where it
  makes sense, and buttons giving an agent a task on a line of their own
  under them. No action is on a row.
- **Sorted by a header,** for columns the app gives sort values: a click
  on the header sorts by it, another the other way, an arrow at the
  header's right showing which and how.
- **Pages of a longer list:** the app can sort the rows itself (on a
  server, across pages), told which header was clicked, and keep the
  selection, so rows on other pages stay selected; select all is the
  page's. Rows can be of differing heights (text wrapped), and a table
  that scrolls draws only those in sight. A list whose rows are opened to be seen can have a
  plain click open a row, Ctrl and Shift still selecting.
- **No selection without actions:** a table with nothing to do on
  selected rows is made without selection, and has no checkbox column.
- **Only the rows scroll;** the header and the actions stay put.
- **The selection kept in sight:** a table first shown, or one whose
  rows the app selects from none, scrolls the first selected row to the
  middle; resized, it keeps the selected rows that were in view there.
  Selected rows out of sight above or below are marked by a small double
  caret at that edge.
- **Kept.** The selection, the row last clicked and the scroll position
  are kept under the table's id, through the app's steps and from one
  run to the next.
- **Set apart** above and below by a blank row.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit, with
partman-widgets, whose icon buttons the actions use:

```toml
partman-table = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-widgets = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
```

Make a `Table` with an id and the rows' keys, which must stay the same
for a row from one frame and run to the next (a database id, a file
name), add its `Col`umns, how rows are selected and a reload button if
wanted, and `show` it with a closure filling a row's cells and one adding
the action buttons, given the selected keys. It says what's selected and
whether reload was pressed. Columns too wide for their card are scaled
to fit it. The column widths can be the app's own, kept in its settings,
which it's told of when the user changes them. The app needs eframe's persistence for what's
kept to outlast a run.

## Building

```sh
cargo test
```

## License

Copyright (C) 2026 Jan Brittenson.

partman-table is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox.
