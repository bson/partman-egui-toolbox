# partman-3d: summary

Everything is in `src/lib.rs`, with its WGSL shader (`SHADER`).

## API

- `Mesh { points, triangles, color }`, `Scene { meshes }`: in one space,
  z up.
- `View { yaw, pitch, zoom, pan }`, default turned a little and tipped
  toward the viewer.
- `install(ctx, &egui_wgpu::RenderState)`: makes the pipeline and keeps
  the device, queue and egui's renderer in egui's temporary memory
  (`Shared`, behind a mutex); `installed(ctx)` says whether it has been.
- `canvas(ui, &mut view, key, || scene, height)`: allocates the canvas,
  steers the view from the pointer (`steer`), and shows the scene; `scene`
  is called only for a key not on the GPU. Over it, `scroll_zoom` (public,
  for other canvases to zoom alike): egui's zoom (pinch,
  ctrl and the wheel) and the frame's scrolling (the wheel, a two-finger
  scroll, either way) zoom: `exp(scrolled / SCROLL_ZOOM)`, 200 points
  for e, up zooming in; the scrolling is taken from the input, so the
  scroll area around it doesn't scroll as well.

## Behaviour worth knowing

- **Drawn from the UI code**, not in a paint callback: into a texture of
  the canvas's size in pixels (`Target`: a 4× multisampled colour and a
  depth texture, resolved into one registered with egui's renderer as a
  native texture), submitted to the queue before egui draws its frame,
  which then samples it as a picture. Nothing has to match egui's own
  render pass (its format, multisampling or depth), which egui-wgpu
  doesn't expose.
- **Redrawn only when needed:** a target remembers the view and the key
  it was drawn for. A new size makes new textures, the old one freed in
  egui's renderer.
- **Scenes** are uploaded once per key (`upload`): every triangle's own
  three corners with the face's normal and colour (28 bytes a vertex),
  as faces are drawn flat, and the scene's bounds. The 8 most recently
  drawn are kept (`KEEP_SCENES`); a canvas not drawn for 120 frames gives
  its textures back (`sweep`).
- **The view** (`uniforms`): turned about z by the yaw, then tipped back
  by the pitch (`turn`, rows), and fitted as KiCad's viewer would: the
  turned corners of the scene's box fill the canvas within 12 points,
  then zoomed and moved; depth spans the turned box, the nearer smaller,
  so nothing is clipped. Orthographic, as before.
- **Shading**, in the vertex shader: `0.35 + 0.65 |n · light|` with the
  light at (-0.3, 0.4, 0.85) in the view, on the mesh's colour, in gamma
  space as egui's colours are; the textures are `Rgba8Unorm`, not sRGB,
  so the picture's values are egui's.

## Tests

`the_turn_tips_the_top_back` checks the turn's arithmetic;
`scrolling_zooms` scrolls over a canvas in a scroll area, which zooms
in and out without the area scrolling; `draws_on_the_gpu` renders two cubes through kittest's wgpu renderer,
seen from above, and finds the red one in the middle, over the green one
under it, with the background around.
