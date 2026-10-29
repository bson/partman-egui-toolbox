# partman-egui-toolbox: summary

A Cargo workspace (`Cargo.toml`), one crate per component, sharing the
egui version and the package metadata through `[workspace.package]` and
`[workspace.dependencies]`. Each crate has its own README and SUMMARY:

- `partman-3d/`: a 3D scene drawn by the GPU, with the app's wgpu device,
  first used by Partman's 3D model card.
- `partman-card/`: the board of cards. It was the egui-cards repository,
  moved here with its history.
- `partman-proof/`: a program showing the components on a board of cards,
  for seeing them without an app; not a library.
- `partman-scroll/`: adaptive scroll speed and momentum, for every
  scroll area at once. `partman-card`, `partman-table` and
  `partman-workflow` report their scroll areas to it.
- `partman-sidebar/`: the agent sidebar, first used by the pin planner.
- `partman-workflow/`: workflows, first used by the pin planner.
- `STYLE.md`: the style and design guide the components follow, moved
  here from Partman so the apps share one copy.

Apps pin the whole repository by commit; a component that changes takes
its users' pins with it, so a fix is tested in the toolbox, pushed, then
each app's `rev` moved.
