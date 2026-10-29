# partman-agent: summary

It came from the pin planner's own agent code (its `acp.rs`, `agents.rs`,
`servers.rs` and most of `agent_ui.rs`), with what was the pin planner's
made the app's to give. The pin planner's SUMMARY, under Agents, says how
it all behaves there, at more length than here.

## Modules

- `acp`: the ACP client. `Conn::spawn(profile, cwd, wake)` starts the
  agent; `Conn::over(output, input, child, wake)` talks over any reader
  and writer (the made-up agent's pipes). A reader thread routes answers
  to the calls waiting on them and everything else (session updates,
  permission requests, the agent quitting) to an `Event` channel; what's
  asked of the client that it doesn't offer (files, terminals) is
  refused, so the agent isn't left waiting. Messages stay
  `serde_json::Value`s, as agents add fields faster than the schema
  settles. `initialize(conn, &Client)` (the app's name, title and
  version), `new_session`, `load_session`, `authenticate`, `prompt`,
  `cancel`, `set_config`, `config_options`, `context`.
- `profiles`: `AgentProfile` (name, command, args, env, the settings last
  picked, model servers), the presets from ACP's registry, the starter
  agent (OpenCode, if installed), finding a command on the PATH, command
  lines and environment as text.
- `servers`: model servers' models (OpenAI's `/v1/models`, with Ollama's
  and LM Studio's own details), and the environment giving them to
  OpenCode (`agent_env`).
- `panel` (`AgentRun`, `Entry`, `Grant`, `Saved`, `Setup`, `Colors`): an
  agent at work. `start(ctx, setup, profile, cwd, task, exact)` and
  `resume(.., saved)`; `poll` takes in what's come; `ui(ui, colors,
  &mut box_height)` draws it and says false when the user ends it. The
  app's own (`Setup`): its `Client`, what it calls itself in sentences
  ("the pin planner"), and its MCP servers, given to every session.
- `dialog` (`AgentsDialog`, `Closed`): the agents dialog on the app's
  profiles; `show` gives `Closed::Saved(profiles, default)` for the app to
  keep, or `Closed::Cancelled`.
- `fake`: `start(ctx, setup, cwd, before)`, a made-up agent speaking ACP
  over a pair of `std::io::pipe`s to a thread, below the transcript
  `before`: it introduces itself ("Fake agent 1.0"), offers a model and
  an effort, and answers each message with a thought, a plan, a command
  it asks to run, then (allowed or not) a reply and its usage.
- `task`: work off the UI thread, waking egui when done, as the apps have
  it (a copy, private).

## Behaviour worth knowing

- **Grants** (`Grant`, asked once when a task starts): a request is
  answered for the user, with the agent's allow-once option, only when
  it's within what was allowed (`in_scope`): files whose paths are all
  known and in the working folder (`inside`, after resolving ".." and
  links), commands if allowed, fetching and thinking, and the app's own
  tools, by their MCP server's name (`server_tool`, OpenCode's naming, or
  `mcp__server__tool`, Claude Code's). Anything else waits on the user.
- **Settings picked** are kept on the profile and put back on every new
  session, in the order the agent lists them (`put_back`).
- **Compacting** shows as the context dropping to under half of at least
  8k tokens (`compacted`): noted, and an exact task's turn stopped.
- **The transcript** is `Entry`s (user, agent, thought, tool, plan, note);
  message and thought chunks are appended to the last of their kind, tool
  calls updated in place by id, a plan replaced where it stands. Replies
  are Markdown (egui_commonmark). A resumed session's replay isn't shown,
  as what was said is kept already.
- **The send box** is partman-widgets' `SendBox`, its height the app's.

## Tests

The client's config options; profiles' presets, command lines and
environment; model servers against a stand-in server; what a grant
covers, and being inside a folder; and `talks_through_the_panel`, the
made-up agent through the panel from its start to an allowed command and
its answer. Ignored, against a real agent (`PARTMAN_AGENT`, and
`PARTMAN_MODEL_SERVER` for a model server): `real_agent`, `real_server`.
