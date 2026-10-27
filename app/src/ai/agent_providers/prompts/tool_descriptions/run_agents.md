Launch one or more child agents that work in parallel on independent parts of a task. Each child runs locally in its own hidden terminal pane and reports back to you.

## When to use

- The task splits into 2+ independent subtasks that can proceed in parallel (e.g. investigate separate subsystems, implement unrelated files, compare alternatives).
- A subtask is long-running and you want to keep working while it runs.

## When NOT to use

- A single sequential task you can do yourself in a few steps.
- Subtasks that depend on each other's output.
- To re-launch an agent that is already running: message it instead.

## Arguments

- `base_prompt`: shared instructions given to every child (context, constraints, how to report back).
- `agents`: one entry per child. `name` is a short unique handle; `prompt` is that child's own task, appended to `base_prompt`.
- `harness_type` (run-wide or per agent): leave empty (or `oz`) for a built-in agent child; set `claude`, `opencode` or `codex` to launch that CLI agent instead. A per-agent value overrides the run-wide one.
- `model_id` (run-wide or per agent): optional model override; empty inherits your model.

Write each child prompt so it can be done without seeing this conversation. The result lists each child with its agent id, or the error that stopped it from starting.
