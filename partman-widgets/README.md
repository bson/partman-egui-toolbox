# partman-widgets

The small parts of the toolbox's style and design guide,
[STYLE.md](../STYLE.md), for [egui](https://github.com/emilk/egui) apps
following it:

- **Blank rows.** The space between groups of elements, under a heading
  and below buttons: one and a half rows of body text.
- **Checkboxes and radio buttons** with their label to the right, an em
  away.
- **Explanations.** Text going with a button (a warning, what it does),
  on the line before it, set off above by a blank row.
- **Icon buttons** that always mean the same thing: a wastebasket to
  delete, a box with an arrow down into it to download, a box with an
  arrow up and out of it to open elsewhere, a circular arrow to reload,
  double carets to move up and down, and a check mark to mark something
  checked or verified. They're drawn, not taken from a
  font, so they look the same in every app and theme.
- **Send boxes.** A text box that Enter acts on, sending what's written
  to an agent say: no button under it, but a fat blue arrow in its
  corner doing what Enter does. Alt-Enter (Option-Enter on a Mac) or
  Shift-Enter makes a new line. Its top edge drags it taller or shorter,
  and the app keeps the height.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-widgets = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
```

Use `blank_row`, `check`, `radio` and `explanation` in place of egui's
own spacing, checkbox and radio button, and `IconButton` (or
`icon_button`) for those actions. An icon button is named for screen
readers by what it does, and can be given a fuller name and disabled.
`AgentButton` is the guide's button giving an agent a task: a larger,
slanted, bold red A, standing for "Agent", then the task, a little taller
than other buttons; the app can give it a bold font for the A. The
`SendBox` is the guide's text box acted on by Enter, its text and height
the app's; it says when it was sent, for the app to take the text, and
when it was resized, for the app to save the height. The
guide's other button conventions are words: "Add" to add, and a
button opening a dialog or needing more done ends in "…".

## Building

```sh
cargo test
```

## License

Copyright (C) 2026 Jan Brittenson.

partman-widgets is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox.
