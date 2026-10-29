# partman-agent

AI agents in an [egui](https://github.com/emilk/egui) app, as the pin
planner has them: any agent speaking the
[Agent Client Protocol](https://agentclientprotocol.com) (ACP), which
agent being each user's own choice.

- **Any ACP agent**, run as a child process: Claude, Codex, Gemini CLI,
  goose, OpenCode and the others in ACP's registry, offered as presets,
  or one given by its command line. Model servers speaking OpenAI's API
  (Ollama, LM Studio, a service) are given to agents that take them.
- **The agents dialog**: the user's agents, added from the presets or by
  hand, edited, and one made the default. The app keeps them.
- **The agent panel**, for the toolbox's sidebar: the agent's replies in
  Markdown, its thinking, tool calls and plan as they come, how full its
  context is and what it has used, the settings it offers (model,
  effort), and the user's messages in a send box.
- **Asked, or allowed up front**: what the agent asks to do waits on the
  user, unless the user allowed it for the task: files in the folder it
  works in, the app's own tools, fetching, and shell commands if said.
- **Sessions kept**: what was said is the app's to save, and an agent
  that can takes its session up again.
- **A made-up agent** speaking ACP from a thread, to see and test the
  panel without a real one.

## Using it

Add it as a git dependency from the toolbox, pinned to a commit:

```toml
partman-agent = { git = "https://github.com/bson/partman-egui-toolbox", rev = "…" }
```

Describe the app in a `Setup` (its name and version for the agent, what it
calls itself, and any MCP servers of its own to give the agent), then
start an agent on a task with `AgentRun::start`, and draw it each frame
with `AgentRun::ui` in partman-sidebar's sidebar. `AgentsDialog` edits the
user's `AgentProfile`s, and says what to keep when it's saved. Colours
come from the app's theme, or `Colors::from_visuals`.

## License

Copyright (C) 2026 Jan Brittenson.

partman-agent is licensed under either of the MIT license
([LICENSE-MIT](../LICENSE-MIT)) or the Apache License, Version 2.0
([LICENSE-APACHE](../LICENSE-APACHE)), at your option, as the rest of the
toolbox.
