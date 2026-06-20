# warpdotdev port plan: `warpdotdev/warp:master` → `TranscriptionFactory/warp`

_Date: 2026-06-20 · Base data fetched same day_

## Position

- `main` is **1138 commits behind** `warpdotdev/master`.
- Merge-base: `c325d146a` "Update agent attribution setting (#9329)" — **2026-04-28**.
  Both this fork **and** `zerx-lab/zap` froze the warpdotdev base on that date; neither
  has merged from warpdotdev since.
- Gap grows ~320 commits/month, heavily in **cloud agent orchestration**.

## This is a port, not a merge

`git merge warpdotdev/master` would conflict on hundreds of files and re-introduce
Warp branding everywhere. Structural drift between the fork and warpdotdev:

| Aspect | This fork (OpenWarp / Zap) | warpdotdev/master |
|---|---|---|
| Editor enum | `EditorChoice::Zap` (→ OpenWarp) | `EditorChoice::Warp` |
| Telemetry | `app/src/server/telemetry.rs` | `app/src/server/telemetry/events.rs` (moved) |
| Tests | inline in module | externalized, e.g. `openable_file_type_tests.rs` |

**Do not `git merge`.**

## Path 1 — incremental pull-from-warpdotdev (recommended near-term)

Cherry-pick only what you want; port each by hand (Zap→Warp symbols, telemetry
path retarget, test-module split).

### Priority A: the 9 `[Security]` fixes (highest value, lowest regret)

- `[Security] Fix command injection in remote ssh sessions (#25354)`
- `[Security] Fix display chip RCEs (#25398)`
- `[Security] properly escape is_file_path and is_git_repository (#26138)`
- `[Security] Limit protocols in WASM open link (#26090)`
- `[Security] Strip env vars before checking command blocklist (#25258)`
- `Add settings UI and blocked-operation banner for OSC 52 clipboard access (#25625)`
- `[Security] Disable iterm file download, limit support to inline files (#25261)`
- `[Security] Remove firebase token log (#25311)` / `Remove problematic log statements (#26091)`
- `Fix security vulnerability in markdown open link (#25353)`

> Assess each against the fork first — some may already be covered by your own
> `fix(security): harden webfetch SSRF…` work.

### Priority B: targeted features (pick as desired)

8 graduated-to-GA candidates: `ConfigurableContextWindow`, `DirectoryTabColors`,
git credential refresh, cloud-mode input v2, multi-harness, orchestration viewer
streamer. Plus tab/group pinning, custom inference endpoints for 3rd-party APIs,
format-on-save, configurable line numbers.

## Path 2 — re-baseline the fork (stop being 1138 behind forever)

Start from a current `warpdotdev/master`, re-apply the fork's divergence:
OpenWarp branding, image viewer, SSH manager, SFTP, BYOP/custom providers,
autoupdate-to-fork plumbing. Large one-time project; clean long-term. Recommended
only if staying current with warpdotdev becomes a sustained goal.

## Image-viewer port (reference — prior attempt conflicted on commit 1)

| File | In warpdotdev? | Action |
|---|---|---|
| `app/src/pane_group/pane/image_pane.rs` | absent | new file — drop in (adapt Zap→Warp, telemetry) |
| `app/src/notebooks/image/mod.rs` | absent | new file — drop in |
| `app/src/workspace/view.rs` | exists, drifted | manual port of routing + `insert_image_pane` |
| `app/src/util/openable_file_type.rs` | exists, drifted | manual; tests live in `openable_file_type_tests.rs` |
| `app/src/pane_group/pane/mod.rs` | exists, drifted | manual (add `IPaneType::ImageViewer`) |
| `crates/remote_server/src/client/mod.rs` | exists, drifted | manual (`read_file_bytes`) |
| `app/src/server/telemetry.rs` | absent (moved) | retarget hook to `server/telemetry/events.rs` |

Result is a fresh warpdotdev PR built by porting — not a cherry-pick.

## Notes

- Compile-verify **is** possible this session: Metal toolchain present, warm `target/`,
  `cargo check -p warp` + targeted `cargo test` used throughout (supersedes the earlier
  "no Metal toolchain" note).
- Checking out warpdotdev triggers Git LFS fetches that fail in this sandbox —
  use `GIT_LFS_SKIP_SMUDGE=1`.

---

# Execution tracker — Path 1 implementation (2026-06-20)

Branch: `feat/sync-upstream-2026-06` (off `main`, all work committed there as one
clean commit per item; split into per-fix PR branches at push time — never pushed
without approval). Each item: diff-level pre-checked against the fork's real code,
ported with drift transforms, built (`cargo check -p warp`) and unit-tested green.

**Baseline caveat:** `cargo check` is green at the fork tip, but the full test suite
has at least one **pre-existing** failure unrelated to this work:
`ai::blocklist::permissions::tests::test_can_autoexecute_command_denylist_precedence`
fails even with all changes reverted to baseline `5d7d939c7` — it exercises a
**workspace (cloud-synced) denylist** path that needs cloud/profile state this OSS
sandbox lacks. Not a regression.

## Phase 1 — security fixes

| # | Item | Upstream SHA | Verdict | Commit | Notes |
|---|---|---|---|---|---|
| — | `shell_quote_arg` helper (foundation) | (pre-#25354) | **ported** | `f8897501e` | Shared prerequisite for #2/#3; thin wrapper over existing `shell_escape_single_quotes` + `shared_tests.rs`; re-exported via `command_executor`. |
| 3 | escape `is_file_path`/`is_git_repository` | `b6caa9576` | **ported** (verified gap) | `0ee900dcf` | `execute.rs` was vulnerable pre-fix verbatim. Added `build_is_file_path_command`/`build_is_git_repository_command`; 10 tests → `execute_tests.rs` (this module uses **external** `*_tests.rs`, not inline — transform #3 corrected). |
| 1 | SSH command injection | `88c344e2d` | **ported** (verified gap) | `57fc91043` | Escaped `cat {history_file}` (session.rs) + `cd '{cwd}'` (remote_command_executor.rs). Extracted `Session::build_read_history_command` as a test seam; 3 tests → `session_test.rs`. Upstream's integration test relied on `SessionInfo` builders absent here. |
| 2 | display-chip RCEs | `4295ec08d` | **ported** (was worse here: `git checkout`/`nvm use` fully unquoted) | `460845603` | Introduced `PromptChipShellCommand` enum; render deferred to `Input` where `shell_type` is known via `shell_quote_arg`. Fork has no worktree/CreateGitBranch/Echo paths → those variants omitted. 4 tests → `input_test.rs`. |
| 5 | strip env vars before denylist | `0c1e24329` | **ported** (PARTIAL → wired) | `09171d833` | Fork had the `remove_leading_env_vars` primitive but it was **not** wired into the permissions layer. Added `command_without_leading_env_vars` + `command_for_execution_predicates`; strips for **denylist only**, allowlist keeps raw command. Tests → `parser_test.rs` + `permissions_test.rs`. |
| 4 | limit protocols in WASM open link | `f6b28f5e9` | **EXCLUDED (surface absent)** | — | The protected share-block iframe modal + `escape_html_attribute` + `safe_browser_open_url` + `crates/warpui/src/browser.rs` were all **deleted** from this fork (`// Zap:删除 share_block_modal`); the upstream `url` dep isn't even present in `warpui`. One latent WASM-only path (`delegate.rs` `open_with_url_and_target`) could get an optional scheme-allowlist hardening if WASM ships — low priority. Not a clean port. |
| 7 | iTerm download → inline only | `f3b9ce1c8` | **ported** (verified gap) | `0a5df5b3f` | `end_iterm_image_receiving` wrote non-inline `File=`/`MultipartFile=` payloads to CWD via `save_as_file` (byte-for-byte pre-fix). Removed the write path → warn + ignore. `ITermImages` flag gates only inline rendering, not this write. 3 tests → `terminal_model_test.rs`. |
| 9 | markdown open-link RCE | `7f0c4dd23` | **ported** (verified gap) | `73972786c` | `open_file` emitted `OpenFileWithTarget` for `SystemDefault`/`SystemGeneric` (extensionless/disguised executables → OS handler → RCE). Now restricted to safe viewer/editor targets; reveals unknowns in Finder/Explorer. Adapted for the fork-only `FileTarget::ImageViewer` (safe). Regression test → `link_tests.rs` (needed `initialize_settings_for_tests`); upstream markdown-preference test N-A (fork routes `.md` via `OpenFileNotebook`). |
| 8 | remove sensitive log statements | `e566a6ced`, `a7f668eaa` | **ported** | `b69d52539` | Deleted `service_impl.rs` firebase-token `log::info!(Vec<Url>)` (leaked on all channels); tightened `handle_incoming_uri` release log to no URL fields + removed orphaned `safe_url_log_fields`. All auth-UI deletions from #26091 are **N-A** (cloud login surfaces already removed here). |
| 6 | OSC 52 clipboard settings UI + banner | `164e60e42` | **FOLLOW-UP (not yet ported)** | — | PORT-NEEDED but large (~250–270 lines, 6 files) and **requires prerequisite commit `#25339`** (also absent — adds `Osc52ClipboardAccess` enum + the actual clipboard gating; the fork currently does **not** gate OSC 52 at all: `ansi_handler.rs:1148/1164`, `view.rs:9519/9523`). **Telemetry-transform correction (important):** this fork *keeps* `TelemetryEvent` + the exhaustive `telemetry_event()` match as a **no-op compat shim** (`send_telemetry_from_ctx!` is dead). Dropping the added arm (per drift transform #2) would break the non-exhaustive match → **keep** the `TelemetryEvent::FeaturesPageAction` arm (it constructs but never sends). |

**Result: 7 security fixes ported + 1 shared foundation, all building and unit-tested
green. #4 excluded (surface deleted from fork). #6 deferred (large + needs prerequisite
+ telemetry-shim handling).**

## Phase 2 — features (not yet started)

Screened-open candidates remain to port: tab/group pinning (`ae7f6574a` +3),
format-on-save (`3f83932cd`), configurable line numbers (`ce73fe07b`); verify-only:
`ConfigurableContextWindow`/`DirectoryTabColors` (likely promotion-only), BYOP
(needs-decision: fork already ships its own). Excluded (cloud/account-gated): git
credential refresh, cloud-mode input v2, multi-harness, orchestration viewer streamer.

## Corrections to the original Path-1 plan discovered during execution

1. **Test layout (transform #3):** the touched modules use **external `*_tests.rs`**
   (often singular `*_test.rs`: `session_test.rs`, `input_test.rs`, `parser_test.rs`,
   `permissions_test.rs`, `terminal_model_test.rs`), not inline `mod tests`. Matched the
   local convention per module instead.
2. **Telemetry strip (transform #2) is wrong for #6:** the fork keeps `TelemetryEvent`
   as a compile-time-only shim with an **exhaustive** match — added arms must be **kept**,
   not deleted, or the build breaks. No send occurs regardless.
3. **#4's protected surface doesn't exist here** — exclude, don't port.
4. **`shell_quote_arg` was introduced upstream before #25354**, not by #25398 — it had to
   be added as a foundation commit; #25398 and #26138 both depend on it.
