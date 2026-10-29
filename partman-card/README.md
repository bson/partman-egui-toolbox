# partman-card

A board of cards for [egui](https://github.com/emilk/egui): a page's
information split into cards, each with a tab, kept in stacks the user
moves, resizes and rearranges. It's the board Partman's part page and the
pin planner's cards are drawn with.

- **Stacks.** Cards sit in stacks, one card shown at a time, picked by its
  tab. Stacks are moved by their bar and resized by their corner, and
  snap to each other's edges and the board's. A press anywhere on a stack
  brings it to the front.
- **Rearranging.** A tab dragged sideways reorders its stack's tabs.
  Dragged up or down, its card comes off as a stack of its own, which
  joins another stack when dropped on its bar. Escape during any drag puts
  everything back as it was before the drag.
- **Closing.** A card the app lets be closed has a close mark on its
  bar while it's alone in its stack; the app is told, and removes it or
  asks first.
- **Room for new cards.** A new card goes where the board, as scrolled,
  has room for all of it. The app can also make a card where something
  is dropped on the board between the stacks.
- **Following the window.** Until the user arranges it, a board is laid
  out one stack under another, following the window, each as tall as its
  shown card's contents and the last taking the rest.
- **Layouts kept.** A board knows nothing of what its cards show: the app
  draws each card's contents by its key, and keeps the layout, which is
  serializable, between runs. A layout can also be written to a file and
  read back, to take it to another computer.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-card = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

Each frame, give `partman_card::board` the layout, the cards with their
keys and titles, the layout to start from, the colours to draw in, and a
function drawing a card's contents. The colours come from the app's
theme, or from egui's own visuals.

To work on it beside an app, a `[patch]` in the app's
`.cargo/config.toml` uses a local checkout instead:

```toml
[patch."https://git.rockgarden.net/bson/partman-egui-toolbox"]
partman-card = { path = "../partman-egui-toolbox/partman-card" }
```

## Building

```sh
cargo test -p partman-card
```

## License

Copyright (C) 2026 Jan Brittenson.

partman-card is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox.
