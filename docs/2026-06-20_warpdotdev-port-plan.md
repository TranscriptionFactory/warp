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

- Cannot compile-verify locally (no Metal toolchain).
- Checking out warpdotdev triggers Git LFS fetches that fail in this sandbox —
  use `GIT_LFS_SKIP_SMUDGE=1`.
