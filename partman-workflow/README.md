# partman-workflow

Workflows for [egui](https://github.com/emilk/egui) the way the
toolbox's style and design guide, [STYLE.md](../STYLE.md), describes them, so every app's steps look and behave alike:

- **The step bar.** Every step of the workflow in a bar across the top,
  each a button pointing on to the next. The step the user is on is
  marked, and those whose prerequisites aren't there yet are greyed, and
  say why when hovered. A click goes straight to any step that's enabled.
- **The advance button.** Each step's last control, at its lower right:
  the next step's name and an arrow. It can be pressed even when the next
  step isn't enabled yet: it checks what's there, makes what the next step
  needs if it can, and goes on, or says why not.
- **The work area.** A step with few controls can be a card in the work
  area: as wide as the area allows up to a limit, at a width that stays
  put, growing down with its contents. When it outgrows the area, the
  tables in it scroll, and when they'd be too small for that, the whole
  card does.

The app keeps the workflow's state, saved with whatever the workflow
applies to, so nothing is saved here and there's no save button to show.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-workflow = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

Implement `Workflow` for what holds the workflow's state: its steps, the
current one, each step's label, whether it's enabled, and going to it.
Optionally, what the advance button checks or does before going on, and
the texts shown over the steps and the button. Then each frame:

- `step_bar` in the top bar draws the steps and goes where clicked.
- `advance_row` (or `advance_button` placed yourself) at the bottom right
  of each step; it returns why it couldn't go on, for the app to show.
- `card_area` holds a card step, and `fit_height` gives a table in it the
  height it can scroll in.

The bar's colours come from the app's theme, or `Colors::from_visuals`.

A workflow in a dialog uses the same: its `step_bar` inside the dialog,
and on its last step `finish_button`, named for what the whole workflow
does ("Update →"), which the app acts on and closes the dialog. The
dialog itself, its Cancel and keeping the app from closing while it's
open, are the app's; the toolbox's proof sheet (`partman-proof`) has
one.

## Building

```sh
cargo test -p partman-workflow
```

## License

Copyright (C) 2026 Jan Brittenson.

partman-workflow is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox.
