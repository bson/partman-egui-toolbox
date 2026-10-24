# partman-egui-toolbox

Components for [egui](https://github.com/emilk/egui) apps that look and
behave like Partman: Partman itself, the pin planner, and apps like them.
Each follows Partman's style and design guide (`STYLE.md` in Partman), so
an app built from them is consistent with the others without
reimplementing the guide.

## Components

| Crate | What it is |
| --- | --- |
| [partman-card](partman-card/README.md) | A board of cards: tabbed cards in stacks the user moves, resizes and rearranges, the layout kept between runs. |
| [partman-workflow](partman-workflow/README.md) | Workflows: a bar of the workflow's steps, an advance button on each step, and a work area that grows with its contents and scrolls only when it must. |

## Using them

Each is a crate of its own in this repository, added as a git dependency
pinned to a commit:

```toml
partman-card = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-workflow = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

To work on them beside an app, a `[patch]` in the app's
`.cargo/config.toml` uses a local checkout instead:

```toml
[patch."https://git.rockgarden.net/bson/partman-egui-toolbox"]
partman-card = { path = "../partman-egui-toolbox/partman-card" }
partman-workflow = { path = "../partman-egui-toolbox/partman-workflow" }
```

## Building

```sh
cargo test
```

## License

Copyright (C) 2026 Jan Brittenson.

The toolbox is free software under the GNU Affero General Public License,
version 3 or any later version ([LICENSE](LICENSE)).
