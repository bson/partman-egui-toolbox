# partman-proof: summary

`src/main.rs` is an eframe program (`publish = false`); `src/step.rs`
makes the STEP model into a `partman_3d::Scene`, as Partman's client
does (truck-stepio, a shell at once, colours read from the file's
STYLED_ITEMs), on a thread rather than in a capped child, as this model
is known to take well under a second.

- `Proof` holds the board's `Layout` (saved with eframe's storage under
  `layout`), the made-up parts (`parts`, 60 rows of reference, value,
  package and quantity), and what the cards keep that a table doesn't:
  the single selection (the app's, so "Select a row" can set it), the
  height slider's value, the page's selection, sort direction and the row
  opened, and the checkbox and radio states. `said` is the last button
  pressed, shown in the top bar.
- `show(ui)` draws the bar (`global_theme_preference_buttons`) and the
  board, `CARDS` in the stacks of `DEFAULT`, each card by its key:
  `many_card`, `one_card`, `none_card`, `page_card`, `buttons_card`,
  `agents_card`, `symbol_card`, `model_card`. `logic` drives
  `partman-scroll`, as an app would; `main` installs partman-3d with
  eframe's wgpu render state.
- `src/agent.rs`: `Agent`, the sidebar's contents: partman-agent's panel
  (`AgentRun::ui`) on its made-up agent (`partman_agent::fake::start`),
  started on the first frame below `transcript`, made-up entries of every
  kind, with a `Setup` of the proof's own and no tools. Ended, it can be
  started again. `asks()` lights the sidebar's control while the grant or
  a permission waits. The send box's height is saved with eframe's
  storage (`agent-input-height`), the sidebar's width (`sidebar-width`).
- `src/flow.rs`: the two workflows. `Flow`, the app's (Cards, Choose
  parts, Details, Summary), saved with eframe's storage (`workflow`): its
  step bar in a panel under the top bar; on Cards the board, on the
  others `step_ui`, a card in `card_area` ending in `advance_row`, or on
  Summary `finish_button("Place order")`, which resets it to Cards; what
  it says (a refusal, or the order placed) goes in the top bar. Choose parts' table gets `fit_height`, less
  what's below it. `Prices`, the dialog's (Search, Review), in an
  `egui::Modal` with its step bar, Cancel at the left and the advance
  button or, on Review, `finish_button("Update")` at the right; `show`
  says whether it's open, cancelled (Cancel, Escape, a click outside) or
  done. Not saved. `logic` cancels a close of the window while it's open.
- Fonts are partman-fonts', installed in `main`; `agent_button` gives an
  agent button Inter's semibold A once they're installed, as a test's
  first frame comes before.
- The defaults, until the user changes them: `default_layout` (stacks
  placed by hand, not fitted), `WINDOW` and `SIDEBAR`. `DEFAULT` is only
  what the board would arrange if it started following the window.
- The symbol's units (`UNITS`) and the model (`MODEL`) are built in with
  `include_bytes!`/`include_str!`. A unit's PNG is decoded into a texture
  the first time it's shown. The model is made the first time its card is
  shown with partman-3d installed (`Model`: unmade, making on a thread,
  made), so tests without a GPU never make it.
- The tables' ids are `proof-*`, so their selections and scroll are kept
  in egui's memory between runs too.

## Tests

`every_card_shows` brings each card up by its tab and finds something it
shows (the 3D card left out: it needs a GPU);
`step::the_model_makes_triangles` makes the model and checks its size, no
GPU needed; `the_apps_workflow_steps_on` goes through the app's workflow, the advance
button refusing with nothing chosen and the table fitted so it's in
sight; `the_price_check_is_a_modal_workflow` updates through the dialog
and cancels it; `the_agent_sidebar_opens_and_answers` slides the sidebar open by its
control, waits for the made-up agent below the transcript, and allows
what it asks; `selecting_a_row_scrolls_to_it`
checks a row the app selects from
none is scrolled into view. `screenshots` (ignored, needs a GPU) installs partman-3d with the
harness's own wgpu device and renders every card in dark and light, the app's workflow's Choose parts and Details, the price check dialog,
then the sidebar open to `../tmp/proof-<theme>-<n>.png`. A fitted
stack takes a few frames to come to a shorter card's height, so the tests
step several frames after each tab.
