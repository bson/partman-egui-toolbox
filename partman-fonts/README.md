# partman-fonts

The fonts Partman, the pin planner and the apps like them are drawn in,
so every app has the same: [Inter](https://rsms.me/inter/) for text, its
semibold face as a family of its own for headings and the agent button's
A, and egui's own faces behind both, for scripts and symbols Inter lacks.
They're built in, about 830 kB.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-fonts = { git = "https://git.rockgarden.net/bson/partman-egui-toolbox", rev = "…" }
```

Install them once, before the first frame:

```rust
partman_fonts::install(&cc.egui_ctx);
```

Then `partman_fonts::semibold(size)` is a heading's font, and
`semibold_family()` the family to give partman-widgets' `AgentButton`.

## License

Copyright (C) 2026 Jan Brittenson.

partman-fonts is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox. Inter, the fonts themselves, is under the SIL Open Font License 1.1
([fonts/OFL-Inter.txt](fonts/OFL-Inter.txt)).
