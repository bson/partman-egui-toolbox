# partman-egui-toolbox

Components for [egui](https://github.com/emilk/egui) apps that look and
behave like Partman: Partman itself, the pin planner, and apps like them.
Each follows the style and design guide, [STYLE.md](STYLE.md), so an app
built from them is consistent with the others without reimplementing the
guide.

**STYLE.md is the key guide for this project.** It says how the
components look and behave, for Partman, the pin planner and the apps
like them. A change to how a component looks or behaves follows it, or
starts as a change to it.

## Components

| Crate | What it is |
| --- | --- |
| [partman-3d](partman-3d/README.md) | A 3D scene drawn by the GPU in a canvas, turned about by dragging, as a part's model on its board. |
| [partman-card](partman-card/README.md) | A board of cards: tabbed cards in stacks the user moves, resizes and rearranges, the layout kept between runs. |
| [partman-scroll](partman-scroll/README.md) | Scrolling: a notch moves a similar share of any area, however tall, and a trackpad swipe let go of carries on, slowing down. |
| [partman-sidebar](partman-sidebar/README.md) | A sidebar, such as an agent's: a triangle on the work area's edge slides it open and shut, and it keeps the width the user drags it to. |
| [partman-table](partman-table/README.md) | Tables: headed, adjustable columns, rows selected by checkbox, radio button or click, actions on the selection below, and the selection and scroll kept between runs. |
| [partman-widgets](partman-widgets/README.md) | The small parts: blank rows, checkboxes and radio buttons with their label an em away, explanations before buttons, and icon buttons that always mean the same thing. |
| [partman-workflow](partman-workflow/README.md) | Workflows: a bar of the workflow's steps, an advance button on each step, and a work area that grows with its contents and scrolls only when it must. |

## Using them

Each is a crate of its own in this repository, added as a git dependency
pinned to a commit:

```toml
partman-3d = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-card = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-scroll = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-sidebar = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-table = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-widgets = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
partman-workflow = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

To work on them beside an app, a `[patch]` in the app's
`.cargo/config.toml` uses a local checkout instead:

```toml
[patch."https://git.rockgarden.net/bson/partman-egui-toolbox"]
partman-card = { path = "../partman-egui-toolbox/partman-card" }
partman-sidebar = { path = "../partman-egui-toolbox/partman-sidebar" }
partman-table = { path = "../partman-egui-toolbox/partman-table" }
partman-widgets = { path = "../partman-egui-toolbox/partman-widgets" }
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
