# Code-Review Remote Diffs — Phase B (remote enablement)

Follow-up spec for finishing remote (SSH) support in the Code Review panel.
**Phase A (the transport-agnostic git engine) is already merged** on branch
`feat/code-review-remote-diffs` (commit: "refactor(code-review): route git
through transport-agnostic GitExecTarget"). This document is the remaining work.

## What Phase A delivered (already done)

- `app/src/util/git.rs`: `GitExecTarget { Local { repo_path }, Remote { client,
  session_id, repo_path } }` with `run_git(&[&str]) -> Result<String>`.
  - `Local` arm delegates to the existing `run_git_command` subprocess.
  - `Remote` arm ships a `git -c diff.autoRefreshIndex=false <shell-quoted args>`
    command over `RemoteServerClient::run_command(session_id, cmd,
    Some(repo_path), {GIT_OPTIONAL_LOCKS:0})`, then maps stdout/exit-code with
    the same Ok/Err rules as local (helpers `build_remote_git_command`,
    `shell_quote`, `map_remote_git_output`).
  - `Remote` variant is gated `#[cfg(feature = "local_fs")]` — note **`local_fs`
    is auto-enabled for every non-wasm target** (`app/build.rs:237`), so it is
    present on all desktop builds.
  - Helpers `detect_current_branch`, `detect_main_branch`, `detect_fork_point`,
    `get_unpushed_commits` now take `&GitExecTarget`.
- `app/src/code_review/diff_state.rs`: every git call routes through a
  `GitExecTarget`. `DiffStateModel` gained:
  - field `remote_target: Option<GitExecTarget>` (mutually exclusive with the
    local `repository` handle).
  - `pub fn exec_target(&self, &AppContext) -> Option<GitExecTarget>` — returns
    the remote target if set, else a `Local` target from the repository handle.
  - `pub fn set_remote_repository(client, session_id, repo_path, ctx)` — the
    remote analog of `set_active_repository`; tears down any local watcher,
    stores the remote target, and kicks off metadata + diff loads. **No FS
    watcher**, so the view must drive refreshes (the existing post-write-action
    `load_diffs_for_current_repo` + `refresh_diff_metadata_for_current_repo`
    calls already cover stage/discard/restore/stash).
  - `active_repository_path` / `is_inside_repository` / `remove_active_repo` are
    remote-aware.
  - FS-bound bits handled per transport: untracked line counting
    (`count_untracked_additions` uses `git diff --no-index --numstat` on remote)
    and post-discard `fs::remove_file` (skipped on remote).
- Tests: `app/src/util/git_tests.rs` (shell-quote POSIX round-trip, command
  build, output mapping incl. binary + `-z` payloads) and
  `app/src/code_review/diff_state_tests.rs` (remote-sourced status / hunk
  parsing). All pass.

**Net effect:** local behavior is unchanged; the engine is remote-capable, but
nothing constructs a `Remote` target yet because the panel lifecycle never wires
one up. That is Phase B.

## The core problem Phase B must solve

The Code Review panel's *repo identity is a local `PathBuf` end to end*, so a
remote session never even opens the panel with a model:

1. `app/src/workspace/view.rs:7815 setup_code_review_panel` builds a panel
   context only when `terminal_view.current_repo_path()` is `Some`. That field
   (`app/src/terminal/view.rs:2733 current_repo_path`) is populated only by
   **local** detection (`DetectedRepositories::detect_possible_git_repo` at
   `terminal/view.rs:10323`, which runs on the local FS). For a remote session
   it stays `None` → `setup_code_review_panel` calls `close_code_review`.
2. The model cache `app/src/pane_group/working_directories.rs:87
   diff_state_models: HashMap<PathBuf, ModelHandle<DiffStateModel>>` is keyed by
   local path, and `get_or_create_diff_state_model` (`:177`) calls
   `DiffStateModel::new(Some(path))`, whose `#[cfg(local_fs)]` body
   (`diff_state.rs` `new`) triggers **local** `DetectedRepositories` detection.
   A remote repo path fed here would resolve to no repository.
3. `app/src/workspace/view/right_panel.rs` tracks `current_repo_path` /
   `selected_repo_path` / `focused_repo_path` as local `PathBuf`s.

So Phase B = teach the panel a "remote repo identity" and resolve the remote
repo root asynchronously, in addition to flipping the gate.

## Required signals (confirmed available)

- **session id**: `TerminalView::active_block_session_id() -> Option<SessionId>`
  (`terminal/view.rs:6246`, already read by `session_env`).
- **remote host id**: `RemoteServerManager::host_id_for_session(session_id) ->
  Option<&HostId>` (`crates/remote_server/src/manager.rs:1052`). Connected only.
  Compare: private `TerminalView::active_session_remote_host_id`
  (`terminal/view.rs:16204`).
- **client**: `RemoteServerManager::client_for_session(session_id) ->
  Option<&Arc<RemoteServerClient>>` (`manager.rs:979`).
- **has_remote_server**: true iff `client_for_session(session_id).is_some()`
  (or `host_id_for_session(...).is_some()`). Currently `session_env`
  (`code_review_view.rs:2942`) hardcodes `has_remote_server: false`.
- **remote cwd**: `block_metadata.current_working_directory()` is populated for
  warpified remote sessions too (it is the shell-integration cwd; see the local
  detection site at `terminal/view.rs:10297`). Alternatively the daemon-pushed
  remote cwd flows via `RemoteServerManagerEvent::NavigatedToDirectory {
  indexed_path }` (`terminal/view.rs:4184`).

## Recommended approach

**Resolve the remote repo root by running `git rev-parse --show-toplevel` on the
client** (mirrors local detection; avoids depending on daemon
`RepoMetadataModel` timing). This was the chosen approach over querying
`RemoteRepoMetadataModel.remote_repository_ids()` (which would avoid a round-trip
but couples to daemon metadata availability — keep as a fallback if rev-parse
proves flaky).

### Step 1 — track the remote repo root in `TerminalView`

Mirror `current_repo_path` with a remote sibling. In the block-metadata handler
(`terminal/view.rs:~10296`, where the local branch runs detection) add: if the
session is remote with a connected client, spawn
`client.run_command(session_id, "git rev-parse --show-toplevel",
Some(cwd.to_string()), {})`, and on a 0-exit non-empty result store
`current_remote_repo: Option<(HostId, /*root*/ String)>`. Emit
`PaneEvent::RepoChanged` when it changes (same as local). Add a `pub fn
current_remote_repo(&self) -> Option<(&HostId, &str)>` accessor.

Reuse `GitExecTarget::Remote { .. }.run_git(&["rev-parse","--show-toplevel"])` if
convenient (it already shell-quotes + maps output), or call `run_command`
directly.

### Step 2 — give `DiffStateModel` a remote construction path

`DiffStateModel::new` triggers local detection, which is wrong for remote. Add
either:
- `DiffStateModel::new_remote(client, session_id, repo_path, ctx)` that skips
  local detection and calls `set_remote_repository(...)` (already implemented),
  **or**
- keep `new` and have the caller invoke `set_remote_repository` immediately
  after creating an empty model.

### Step 3 — remote-aware model cache + panel context

In `working_directories.rs`, add a remote-keyed cache (e.g. key by
`(HostId, String)` or a `RepoKey { Local(PathBuf), Remote(HostId,String) }`) and
a `get_or_create_remote_diff_state_model(client, session_id, host, root, ctx)`
that builds via Step 2. In `setup_code_review_panel`
(`workspace/view.rs:7815`), when `current_repo_path()` is `None` but
`current_remote_repo()` is `Some` **and** the session is warpified, build the
panel context from the remote model and pass a remote identity `PathBuf`
(`PathBuf::from(root)`) as the panel's `repo` so `CodeReviewView` sets up
`active_repo` and renders diffs. (The model's `active_repository_path()` already
returns `PathBuf::from(remote_root)` for identity comparisons.)

### Step 4 — flip the gate + messaging

- `code_review_view.rs:2942 session_env`: set `has_remote_server` from
  `RemoteServerManager::...client_for_session(session_id).is_some()`. When
  remote **and** `has_remote_server`, do **not** emit a blocking
  `RemoteSession` — let the panel render diffs.
- `code_review_view.rs:2975 render_no_repo_for_env` /
  `:4117 render_remote_state_with_buttons`: only the `has_remote_server == false`
  case keeps the `code-review-diffs-local-workspaces-only` string.
- `app/i18n/en/warp.ftl`: reword `code-review-diffs-local-workspaces-only` to
  reflect it now only covers non-warpified SSH (plain tmux / subshell). The
  existing `code_review_view.rs:1364` (branch list) and `:2655` (merge base)
  call sites already route through `git_exec_target` (model's `exec_target`), so
  they become remote-aware automatically once a remote target is set.

### Hard boundary (unchanged)

Plain tmux / subshell SSH (`has_remote_server == false`) has no daemon — keep
the unsupported message for that case.

## Known wrinkles

- **Latency**: a diff load fires many sequential git calls, each now one SSH
  round-trip. Functional but slower. Server-side batching (a proto message
  bundling several git invocations) is a separate follow-up.
- **No live FS watcher on remote**: refresh is driven by the view (post-action
  reloads already wired). Consider hooking `NavigatedToDirectory` /
  remote-metadata push events to trigger `refresh_diff_metadata_for_current_repo`
  for live-ish updates.
- **Quoting**: covered by `shell_quote` tests; if new git invocations are added,
  they inherit the quoting automatically via `run_git`.

## Verification

1. **Build**: desktop (`local_fs` on) compiles; wasm build still excludes the
   `Remote` arm.
2. **Manual E2E (requires a live warpified SSH host)**: warpify an SSH session
   into a git repo with staged, unstaged, and untracked changes; open Code
   Review; confirm diffs render; then exercise stage / discard / restore / stash
   / fetch-base / branch-switch and confirm each mutates remote state. Confirm a
   plain (non-warpified) SSH session still shows the unsupported message.

## File index (anchors)

- `app/src/util/git.rs` — `GitExecTarget` (Phase A, done).
- `app/src/code_review/diff_state.rs` — `set_remote_repository`, `exec_target`
  (Phase A, done).
- `app/src/terminal/view.rs` — `current_repo_path` (`:2733`), block-metadata
  detection (`:10296`), `active_block_session_id` (`:6246`),
  `active_session_remote_host_id` (`:16204`). **Phase B: add remote repo
  tracking.**
- `app/src/workspace/view.rs:7815 setup_code_review_panel` — **Phase B: handle
  remote context.**
- `app/src/pane_group/working_directories.rs:177 get_or_create_diff_state_model`
  — **Phase B: remote-keyed cache.**
- `app/src/workspace/view/right_panel.rs` — repo-path tracking. **Phase B: allow
  remote identity.**
- `app/src/code_review/code_review_view.rs:2942 session_env`,
  `:2975 render_no_repo_for_env`, `:4117 render_remote_state_with_buttons` —
  **Phase B: flip gate + messaging.**
- `app/i18n/en/warp.ftl` `code-review-diffs-local-workspaces-only` — **Phase B:
  reword (optional).**
