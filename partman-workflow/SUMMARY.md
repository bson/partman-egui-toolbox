# partman-workflow: summary

Everything is in `src/lib.rs`. It follows the Workflow section of
the toolbox's `STYLE.md`; the pin planner is its first user.

## API

- `Workflow`, implemented by whatever holds the workflow's state:
  - `steps`, `current`, `label`, `enabled`, `go`: required.
  - `why_not(step)`: hover text over a disabled step.
  - `advance_hint(to)`: hover text over the advance button.
  - `can_advance(to)`: Err(why) disables the advance button. By default
    it can always be pressed.
  - `advance(ctx, to)`: what pressing it does before going on:
    `Advance::Go`, `Advance::Started` (work begun that the app finishes
    by going on itself, such as a download), or `Advance::Refused(why)`.
    By default it goes on only to an enabled step.
- `next(wf)`: the step after the current one.
- `step_bar(ui, colors, wf)`: the bar; true when a click moved.
- `advance_button(ui, wf)` and `advance_row(ui, wf, left)`: the button,
  alone or right-aligned in a row; they return a refusal's reason. Not
  drawn on the last step.
- `finish_button(ui, action, hint, ready)`: the last step's button in a
  workflow that ends by doing something (a modal one's "Update →"),
  drawn as the advance button is; true when pressed, greyed with
  `ready`'s reason when it can't be.
- `card_area(ui, Width { max, min }, contents)` and `fit_height(ui,
  below, at_least)`.
- `Colors`: the bar's fill, hover, current step, and text colours.

## Behaviour worth knowing

- **Steps' outline** (`chevron`): a notch on the left, where the step
  before points in, and a point on the right; the first has no notch.
  Item spacing is set negative so each point nests in the next notch.
  The shape isn't convex, which egui's path fill needs, so it's filled
  as a mesh fanned from the notch, and outlined in its own colour to
  smooth the slanted edges.
- **Accessibility**: each step is a button labelled with the step's name,
  selected when current and disabled when not enabled, so tests and
  screen readers find it by name. The advance button is labelled with the next step's name and "→".
- **Work area height**: `card_area` records where its contents start and
  the height it has (in egui's temporary data under a fixed id), and
  `fit_height` measures from the cursor against that, so a table knows
  what's left for it inside the scroll area, where egui's own available
  height is unbounded.
- **Scrolling**: the work area is reported to `partman-scroll`, so a
  notch over it moves a share of its height.

## Tests

With egui_kittest: the bar goes only to enabled steps; the advance button
unlocks the next step or says why not, and is gone on the last step;
`next`; and a table fitting in what's left of the area, or leaving the
scrolling to the area when that's too little.
