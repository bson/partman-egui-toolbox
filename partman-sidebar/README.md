# partman-sidebar

A sidebar for [egui](https://github.com/emilk/egui) the way the
toolbox's style and design guide, [STYLE.md](../STYLE.md), has the agent
sidebar:

- **Opened from its edge.** A small triangle on the work area's edge,
  halfway down, slides it open. Open, the triangle is inside it, at its
  edge, and slides it shut. While it's shut, the triangle can show that
  something in it wants the user, such as an agent asking a question.
- **Its width the user's.** It opens over half the work area until the
  user drags its edge; from then on it opens to that width. It's never
  wider than the work area. Escape while dragging puts the width back.

The app keeps whether it's open and its width, saving the width with its
settings, so the sidebar comes back as the user left it.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-sidebar = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

Each frame, before the work area's central panel, show a `Sidebar` made
with an id, a name for its control ("Agent panel"), and the app's own
open flag and width (`None` until the user drags it). `attention` marks
the control while shut; `side` puts it on the left. `show` draws the
contents while it's open or sliding, and says whether the user opened or
shut it and whether a drag set a new width, for the app to save.

The control's colours come from the app's theme, or
`Colors::from_visuals`.

## Building

```sh
cargo test -p partman-sidebar
```

## License

Copyright (C) 2026 Jan Brittenson.

partman-sidebar is free software under the GNU Affero General Public
License, version 3 or any later version ([LICENSE](../LICENSE)).
