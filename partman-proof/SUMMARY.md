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
- `src/agent.rs`: `Agent`, a made-up agent's panel drawn in the
  `partman-sidebar` sidebar beside the board (`proof-agent`, "Agent
  panel"): a name with Stop and End, Model and Effort drop-downs, a
  context gauge, and a transcript of `Entry`s (notes, user messages, agent
  text with `code`, a plan, tool calls that open to what they gave back,
  a table, a permission question and an "agent asks", each answered
  locally), then partman-widgets' `SendBox` in a bottom panel, the
  transcript taking the rest: Enter adds the message and a made-up
  answer, and its height is saved with eframe's storage
  (`agent-input-height`). `asks()` lights the sidebar's control while a question waits.
  The sidebar's width is saved with eframe's storage (`sidebar-width`).
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
GPU needed; `the_agent_sidebar_opens_and_answers` slides the sidebar open by its
control and answers the agent's question; `selecting_a_row_scrolls_to_it`
checks a row the app selects from
none is scrolled into view. `screenshots` (ignored, needs a GPU) installs partman-3d with the
harness's own wgpu device and renders every card in dark and light, then the sidebar open to `../tmp/proof-<theme>-<n>.png`. A fitted
stack takes a few frames to come to a shorter card's height, so the tests
step several frames after each tab.
