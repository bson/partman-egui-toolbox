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
  and double carets to move up and down. They're drawn, not taken from a
  font, so they look the same in every app and theme.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-widgets = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

Use `blank_row`, `check`, `radio` and `explanation` in place of egui's
own spacing, checkbox and radio button, and `IconButton` (or
`icon_button`) for those actions. An icon button is named for screen
readers by what it does, and can be given a fuller name and disabled.
The guide's other button conventions are words: "Add" to add, and a
button opening a dialog or needing more done ends in "…".

## Building

```sh
cargo test
```
