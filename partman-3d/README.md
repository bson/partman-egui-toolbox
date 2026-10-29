# partman-3d

A 3D scene drawn by the GPU in an [egui](https://github.com/emilk/egui)
canvas, as Partman shows a part's model on its board:

- **Coloured meshes**, lit from over the viewer's shoulder, each face
  alike from either side, the nearest shown where they overlap, with
  multisampled edges.
- **Turned about by dragging**, moved by right-dragging, zoomed with the
  wheel, a two-finger scroll or by pinching, and put back by a double
  click. Scrolling over it zooms rather than scrolling what it's in.
- **Fast:** a scene goes to the GPU once, and is drawn again only when
  the view, the scene or the canvas's size changes; each time is the
  GPU's work, not the app's.

It draws with the app's own wgpu device (eframe's wgpu renderer), into a
texture egui shows as it would any picture.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-3d = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
```

Give it eframe's wgpu render state once, at the start:

```rust
if let Some(rs) = &cc.wgpu_render_state {
    partman_3d::install(&cc.egui_ctx, rs);
}
```

Then draw a `Scene` (meshes: points, triangles, a colour; z up) with a
`View` the app keeps, under a key that names the scene, so it's made and
uploaded only once:

```rust
partman_3d::canvas(ui, &mut view, "LQFP-48", || scene(), 400.0);
```

`scroll_zoom(ui)` is the factor it zooms by in a frame, from scrolling
and pinching, for a 2D canvas of the app's to zoom the same way.

Without `install` (another renderer, a test that doesn't render), the
canvas says 3D needs the wgpu renderer.

## License

Copyright (C) 2026 Jan Brittenson.

partman-3d is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox.
