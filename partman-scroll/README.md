# partman-scroll

Scrolling as the toolbox's style and design guide,
[STYLE.md](../STYLE.md), has it, for every scroll area in an
[egui](https://github.com/emilk/egui) app at once:

- **Adaptive speed.** A wheel notch or a trackpad swipe moves about the
  same share of what an area shows, however tall it is: a tall table
  scrolls further per notch than a short list.
- **Momentum.** A trackpad swipe let go of mid-scroll carries on,
  slowing down, until it stops or a click, a key or another scroll stops
  it. Where the system does this itself (macOS, Windows) it's left to the
  system; on Linux it needs Wayland, which says when a swipe ends.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-scroll = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

Call `partman_scroll::drive(ctx)` once a frame, before anything's
drawn. Then each scroll area the app shows itself tells it where it was,
from what showing it gave back:

```rust
let shown = egui::ScrollArea::vertical().show(ui, |ui| { /* … */ });
partman_scroll::area(ui.ctx(), &shown);
```

The toolbox's own tables, cards and workflow areas do this already. An
area that isn't reported scrolls at egui's own speed, or at that of the
reported area around it.

## License

AGPL-3.0-or-later, as the rest of the toolbox.
