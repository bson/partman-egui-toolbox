# egui-cards: summary

Everything is in `src/lib.rs`. It came from Partman's `client/src/cards.rs`
and the pin planner's copy of it, merged.

## API

- `Layout`: the stacks (`Stack`: its rect on the board as x, y, width,
  height, its cards' keys in tab order, and the card on top), whether it
  still follows the window (`fitted`), and, not saved, the drag under way
  and each card's height last frame.
  - `arrange(keys, default)` keeps each card once, drops ones gone, starts
    from `default` when empty, and puts new ones in the last stack.
  - `show(key)` puts a card on top of its stack; `on_top(key)` says
    whether it is.
  - `add_alone(key)` adds a card in a stack of its own: just above the
    last while fitted, as the last takes what's left, else under all the
    others.
- `board(ui, colors, layout, cards, default, content)` draws the board in
  what's left of `ui` and handles the pointer; `content(ui, key)` draws a
  card's contents. It returns whether the layout changed, to be saved.
  `Card` is a key, a title for its tab, and whether it's loading (a
  spinner on its stack's bar).
- `Colors`: canvas, chrome, hover, accent, text and dim text, from the
  app's theme, or `Colors::from_visuals`; the default is egui's dark.
- `of(boards, name, keys, default)`: a board's layout from a map of them
  by name, as an app keeps several, made to hold its cards.
- `export(kind, board, layout)` and `import(kind, board, text)`: a layout
  as a JSON file and back. `kind` is the app's (`"partman-layout"`), so
  another app's file or other JSON isn't taken for one; the board's name
  must match, and a newer version is refused.

## Behaviour worth knowing

- **Fitted layouts** are laid out by `fit`: stacks one under another
  across the board, each as tall as its shown card's contents last frame,
  the last taking the rest. Moving or resizing a stack ends that.
- **Presses.** A press on a stack, wherever it lands, brings it forward,
  except while fitted (stacks don't overlap then), on a popup over it, or
  while a popup is open (the press is the popup's: picking from it or
  closing it).
- **Nothing through a stack.** Each stack lays a backdrop over those drawn
  before it that senses clicks and drags. Drags matter: egui gives a
  press's drag to the nearest widget that senses drags, even one under the
  widget clicked, so without it a combo box pressed over another stack's
  bar would start dragging that stack.
- **Snapping** (`snap_move`, `snap_resize`) is to the board's edges and to
  other stacks' edges, beside them (with `GAP`) or lined up.
- **Tabs** are dragged sideways to reorder (`Drag::Tabs`), or up or down
  past `DECIDE` to take the card off (`Drag::Move` with `card`), which
  joins a stack when dropped on its bar (`bar_at`).

## Tests

Layout: the default filling the board, arranging, snapping, a card taken
off joining another stack, tabs reordering and coming off, a card added
alone, and export and import. With the pointer, through egui_kittest: a
press on a stack behind bringing it forward, and a combo box over another
stack's bar pressed without dragging that stack, its list closed by a
press elsewhere without raising anything.
