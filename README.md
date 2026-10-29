# partman-egui-toolbox

Components for [egui](https://github.com/emilk/egui) apps that look and
behave like [Partman](https://git.rockgarden.net/bson/partman), the
parts manager: Partman itself, the
[pin planner](https://git.rockgarden.net/bson/pinplanner), which plans a
part's pins and makes its KiCad symbol, and apps like them.
Each follows the style and design guide, [STYLE.md](STYLE.md), so an app
built from them is consistent with the others without reimplementing the
guide.

**[STYLE.md](STYLE.md) is the key guide for this project.** It says how the
components look and behave, for Partman, the pin planner and the apps
like them. A change to how a component looks or behaves follows it, or
starts as a change to it.

**To see the components,** run the proof sheet, a small program showing
them as an app would, in dark and light, without starting one: tables,
buttons, text, a KiCad symbol and a 3D model on a board of cards, a
workflow and one in a dialog, and an agent sidebar with a made-up
transcript:

```sh
cargo run -p partman-proof
```

## Components

| Crate | What it is |
| --- | --- |
| [partman-3d](partman-3d/README.md) | A 3D scene drawn by the GPU in a canvas, turned about by dragging, as a part's model on its board. |
| [partman-card](partman-card/README.md) | A board of cards: tabbed cards in stacks the user moves, resizes and rearranges, the layout kept between runs. |
| [partman-fonts](partman-fonts/README.md) | The fonts the apps are drawn in: Inter, regular and semibold, built in, so every app has the same. |
| [partman-proof](partman-proof/README.md) | The proof sheet: a small program showing every component as an app would, to see how they look and behave without one. Not a library. |
| [partman-scroll](partman-scroll/README.md) | Scrolling: a notch moves a similar share of any area, however tall, and a trackpad swipe let go of carries on, slowing down. |
| [partman-sidebar](partman-sidebar/README.md) | A sidebar, such as an agent's: a triangle on the work area's edge slides it open and shut, and it keeps the width the user drags it to. |
| [partman-table](partman-table/README.md) | Tables: headed, adjustable and sortable columns, rows selected by checkbox, radio button or click, actions on the selection below, the selection kept in sight and marked when it's scrolled out of view, and the selection and scroll kept between runs. |
| [partman-widgets](partman-widgets/README.md) | The small parts: blank rows, checkboxes and radio buttons with their label an em away, explanations before buttons, icon buttons that always mean the same thing, agent task buttons, and a send box that Enter acts on, by an arrow in its corner. |
| [partman-workflow](partman-workflow/README.md) | Workflows: a bar of the workflow's steps, an advance button on each step, a finishing one for a workflow in a dialog, and a work area that grows with its contents and scrolls only when it must. |

## Using them

Each is a crate of its own in this repository, added as a git dependency
pinned to a commit:

```toml
partman-3d = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-card = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-fonts = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-scroll = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-sidebar = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-table = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-widgets = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
partman-workflow = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
```

To work on them beside an app, a `[patch]` in the app's
`.cargo/config.toml` uses a local checkout instead. Patch every one the
app uses: the crates use each other by path, so one patched and another
not gives the app two copies of what they share.

```toml
[patch."https://github.com/bson/partman-egui-toolbox"]
partman-3d = { path = "../partman-egui-toolbox/partman-3d" }
partman-card = { path = "../partman-egui-toolbox/partman-card" }
partman-fonts = { path = "../partman-egui-toolbox/partman-fonts" }
partman-scroll = { path = "../partman-egui-toolbox/partman-scroll" }
partman-sidebar = { path = "../partman-egui-toolbox/partman-sidebar" }
partman-table = { path = "../partman-egui-toolbox/partman-table" }
partman-widgets = { path = "../partman-egui-toolbox/partman-widgets" }
partman-workflow = { path = "../partman-egui-toolbox/partman-workflow" }
```

## Building

```sh
cargo test
```

The tests need no GPU. Screenshots of the proof sheet, its 3D model
among them, go to `tmp/` with `cargo test -p partman-proof -- --ignored`,
which does.

## License

Copyright (C) 2026 Jan Brittenson.

The toolbox is licensed under either of

- the MIT license ([LICENSE-MIT](LICENSE-MIT)), or
- the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)),

at your option, as egui is. Partman and the pin planner, which use it,
are under the GNU Affero General Public License.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the toolbox by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

Some files keep licences of their own, said where they are: the fonts in
partman-fonts (SIL Open Font License 1.1) and the 3D model in
partman-proof (CC-BY-SA 4.0, with KiCad's exception).
