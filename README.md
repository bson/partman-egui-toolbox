# egui-cards

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
  joins another stack when dropped on its bar.
- **Following the window.** Until the user arranges it, a board is laid
  out one stack under another, following the window, each as tall as its
  shown card's contents and the last taking the rest.
- **Layouts kept.** A board knows nothing of what its cards show: the app
  draws each card's contents by its key, and keeps the layout, which is
  serializable, between runs. A layout can also be written to a file and
  read back, to take it to another computer.

## Using it

Add it as a git dependency, pinned to a commit:

```toml
egui-cards = { git = "https://git.rockgarden.net/bson/egui-cards", rev = "…" }
```

Each frame, give `egui_cards::board` the layout, the cards with their
keys and titles, the layout to start from, the colours to draw in, and a
function drawing a card's contents. The colours come from the app's
theme, or from egui's own visuals.

To work on it beside an app, a `[patch]` in the app's
`.cargo/config.toml` uses a local checkout instead:

```toml
[patch."https://git.rockgarden.net/bson/egui-cards"]
egui-cards = { path = "../egui-cards" }
```

## Building

```sh
cargo test
```

## License

Copyright (C) 2026 Jan Brittenson.

egui-cards is free software under the GNU Affero General Public License,
version 3 or any later version ([LICENSE](LICENSE)).
