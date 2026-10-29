# partman-fonts: summary

Everything is in `src/lib.rs`; the fonts are in `fonts/`: Inter Regular
and SemiBold, and Inter's licence, the same files Partman's client and
the pin planner carried each in their own assets.

## API

- `install(ctx)`: egui's default font definitions with Inter first in the
  proportional family and a family named `SEMIBOLD` ("semibold") of
  Inter SemiBold, egui's own proportional faces behind both.
- `semibold_family()` and `semibold(size)`: that family, and a `FontId`
  of it.

## Behaviour worth knowing

- **Installed before drawing**: a family that isn't bound to any font
  panics when text asks for it, so an app installs them before its first
  frame, and a test harness, which draws its first frame as it's made,
  asks for the semibold family only once it's there.
- **Inter has what egui's own proportional face lacks**, the arrow the
  advance buttons end in among them.

## Tests

`installed_fonts_lay_out_text`: installed, the semibold family is there,
and both families have the glyphs of "Details →".
