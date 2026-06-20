# Upstream sync plan: `zerx-lab/warp` → `TranscriptionFactory/warp:main`

_Date: 2026-06-20 · Base data fetched same day_

## Topology

```
warpdotdev/warp (master)   public Warp OSS    ── frozen base c325d146a (2026-04-28)
        │
        ▼
zerx-lab/warp (main) = "upstream" = "Zap"     ── HEAD 0d55fb9e9 (2026-06-18)
        │  forked at e3d20a223 (2026-05-26)
        ▼
TranscriptionFactory/warp (main) = origin = "OpenWarp"  ── HEAD a129ea53f
```

`main` vs `upstream/main`: **67 ahead / 58 behind.**

## Why not a plain `git merge upstream/main`

A naive merge yields **28 conflicts**. The root cause: **30 of the 58 upstream commits are work `main` already carries in OpenWarp-reverted form** (`git cherry` confirms). They re-collide, and the merge re-introduces Zap branding across i18n, autoupdate URLs, the binary name, and `EditorChoice`.

### Conflict map (full-merge path), with resolution policy

| Bucket | Files | Resolve |
|---|---|---|
| Branding | `app/i18n/{en,ja,zh-CN}/warp.ftl`, `app/src/autoupdate/{linux,mac,windows}.rs`, `app/src/bin/openwarp_oss.rs`, `app/src/lib.rs` | **Keep ours** (OpenWarp / fork URLs) |
| SSH manager | `app/src/ssh_manager/{panel,panel_tests,server_view,server_view_tests}.rs`, `app/src/integration_testing/ssh_manager/step.rs`, `crates/warp_ssh_manager/src/{lib,repository,ssh_command,ssh_command_tests,sync_provider,types}.rs` | Take upstream logic (newer), **re-revert branding** |
| SFTP | `app/src/sftp_manager/{browser,sftp_ops}.rs` | Take upstream (incl. `#249` centering) |
| CLI agent | `app/src/terminal/cli_agent.rs` | Take upstream (`#210`, `#216`) |
| Onboarding | `crates/onboarding/examples/{callout,callout_flow}.rs`, `crates/onboarding/src/bin/main.rs` | Keep your `ui_font_size`, fold in upstream |
| Persistence | `crates/persistence/src/{model,schema}.rs` | Hand-merge — keep both schema additions |
| `app/src/workspace/view.rs` | 1 | **Mixed**: image-viewer hunks = ours; SSH-auth/CLI hunks = upstream |

## Recommended strategy: selective cherry-pick

Skip the 30 duplicates and your own `#229`; take only the genuinely-new functional commits. `git cherry main upstream/main` flags 27 as new; of those, 3 are branding false-positives already in `main` (`#161` `ec71988cb`, autoupdate-cache `d4e9f9e44`, `#179` `09c71477f`) and 1 is your `#229` (`c1c00f520`). That leaves **23 to cherry-pick**, in chronological order:

```
bbcdbc128  fix(cli-agent): Windows agent detection no popup (#216)
2fbfaffbc  feat: Vue file injection highlight JS/CSS/TS (#218)
8c1228cdd  Fix #243: skip agent task persistence when BLOB > 10 MB (#244)
f3f7cbfde  fix(right-panel): unsubscribe before re-subscribe on tab switch (#240)
05aba790c  fix: apply custom proxy to BYOP genai streaming client (#237)
1c5424074  fix(wsl): tab completion path for non-/mnt/ WSL paths (#233)
eaa2f96a4  fix: restore worktree "add repo" button (#228)
e159018b0  fix: inject user rules into next-command suggestion prompt (#226)
1a5e66e56  fix(windows): prevent drag_window on maximized window (#221)
6da414cbc  fix(windows): remove STARTF_USESTDHANDLES CreateProcessW fail (#215)
917423766  feat(ai-assistant): right-align user messages (#212)
281a789c9  fix: stop "100% context remaining" for custom providers (#236)
d9a63518a  feat: Vue file block comments (#224)
1643e838f  feat(theme): VS Code 2026 Dark built-in theme (#185)
92c77b89a  feat: configurable markdown heading scale (#172)
870783869  fix(windows): add perl and protoc to bootstrap
18f8be51b  fix(ci): free Linux release disk space (musl CLI ENOSPC)
5bcbfdfd7  fix(sftp): center SFTP browser dialog (#249)
bfc9583b8  fix: carrier invalid JSON args no longer breaks next turn (#246)
b1a2fefe1  feat(agent-settings): CLI agent settings UI (#210)
83793b459  feat: onkey (#231)
0d474c3b3  fix(gpu): avoid DX12 on Windows discrete GPU (#230)
0d55fb9e9  fix(ai): self-heal missing tool_result after interrupt
```

### Execution

```bash
git switch -c feat/sync-upstream-2026-06 main
# cherry-pick each, recording origin:
git cherry-pick -x bbcdbc128 2fbfaffbc 8c1228cdd f3f7cbfde 05aba790c \
  1c5424074 eaa2f96a4 e159018b0 1a5e66e56 6da414cbc 917423766 281a789c9 \
  d9a63518a 1643e838f 92c77b89a 870783869 18f8be51b 5bcbfdfd7 bfc9583b8 \
  b1a2fefe1 83793b459 0d474c3b3 0d55fb9e9
```

**Conflict expectations:**
- Most Windows/CI/AI fixes touch non-branded files → apply clean.
- `#210` (agent settings) and `#212`/`#231` touch `workspace/view.rs` / `cli_agent.rs` → may conflict with your image-viewer + branding; resolve branding → OpenWarp, keep image-viewer hunks.
- `#185` theme is a new `.yaml` + registration → near-clean.

### Verify (must run in a build-capable env — not this sandbox)

```bash
./script/format && cargo check -p warp   # or project presubmit
```

Then PR `feat/sync-upstream-2026-06` → your own `main`.

## Alternative: full merge + branding re-revert

`git merge upstream/main`, apply the conflict table above, then re-run the
"Revert Zap branding to OpenWarp" pass over the merged tree. Re-establishes a
clean merge base with upstream (stops `git cherry` false-positives next round),
but ~28 conflicts and more branding churn.

## Notes

- Cannot compile-verify locally (no Metal toolchain; `warpui/build.rs` shader
  compile fails regardless of changes). Rely on CI or a full macOS/Xcode build.
- SSH pushes in this sandbox need
  `GIT_SSH_COMMAND="ssh -o ControlMaster=no -o ControlPath=none"`.
