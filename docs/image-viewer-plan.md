# Image Viewer for Warp — Implementation Plan

## Goal
Clicking an image (png/jpg/jpeg/gif/webp/svg) in the Project Explorer opens it in an
**in-app pane/tab** (like the markdown viewer), for both **local** and **remote (SSH)** files.

Currently:
- Local images → routed to `SystemGeneric` (OS default app), never opened in Warp.
- Remote images → unconditionally sent down the text-buffer `OpenRemoteFile` path, which
  `read_to_string`s the file and fails (`DoesNotExist` / "Failed to load buffer: IO error").

## Why this is mostly plumbing (capabilities already exist)
- **Rendering is solved.** `Image` element in `crates/warpui_core/src/elements/image.rs`
  (`.contain()`, `.before_load()`, animation). `ImageType` decodes png/jpeg/gif/webp/svg
  (animated gif/webp included) — `crates/warpui_core/src/image_cache.rs`.
- **Local bytes:** `AssetSource::LocalFile { path }` decodes straight from a path
  (`asset_cache.rs:320`). No bytes handling needed.
- **In-memory bytes:** `AssetCache::insert_raw_asset_bytes::<ImageType>(id, bytes)` +
  `AssetSource::Raw { id }` (`asset_cache.rs:345`). This is exactly how the terminal renders
  inline images today — `app/src/terminal/view.rs:10659`.
- **Remote bytes need NO protocol change.** `ReadFileChunk` RPC already returns raw `bytes`
  (`crates/remote_server/proto/remote_server.proto`, handler in
  `app/src/remote_server/server_model.rs`). Loop chunks until `eof`.
- **Existing viewer reference:** `LightboxView` (`app/src/workspace/lightbox_view.rs`) +
  `ui_components/src/lightbox.rs:150` shows the Image element usage pattern.
- **Pane template:** `FilePane` (`app/src/pane_group/pane/file_pane.rs`, ~180 lines) is a
  near-exact template for a read-only file-backed pane.

## Architecture
```
file tree click ─► resolve_file_target_with_editor_choice ─► FileTarget::ImageViewer(layout)
                                                                    │
                              open_file_with_target ◄───────────────┘
                                    │
                 ┌──────────────────┴───────────────────┐
          LOCAL  │                                       │  REMOTE
   ImagePane::new(path)                      ReadFileChunk RPC loop → bytes
          │                                       │
   ImageViewerView                         insert_raw_asset_bytes::<ImageType>(id, bytes)
   AssetSource::LocalFile{path}            AssetSource::Raw{id}
          └──────────────► Image element (warpui) ◄──────┘
```
Both arms converge on one `ImageViewerView`; only the `AssetSource` differs.

---

## STAGE 1 — Local images (no protocol work)  ✅ DONE

**Status (2026-06-10):** ✅ Complete. Implemented and compiling (`cargo check`/`build` + app
binary `openwarp-oss` all green; routing unit test added). Commits `281cf3c6`..`09f56557`.
Interactive click-test on a local PNG verified — opens fit-to-pane in a tab.

**Persistence decision:** Image tabs are **not** persisted across restart (per user) —
`LeafContents::Image { path }` exists for the in-session snapshot but
`is_persisted()` returns `false` (like `SshServer`/`Sftp`). The snapshot/restore/
launch-config/sqlite match arms are unreachable stubs only to keep matches exhaustive.
No new SQLite table/migration was added. Steps 5+6 below were folded into the step-3
commit as a result.


1. **Route images to a viewer** — `app/src/util/openable_file_type.rs`
   - Add `FileTarget::ImageViewer(EditorLayout)` variant.
   - In `resolve_file_target_with_editor_choice`, insert BEFORE the step-4 binary fallback
     (`is_supported_image_file` already exists at line ~71):
     ```rust
     // 3.5 Image files -> in-app image viewer (before binary fallback)
     if is_supported_image_file(path) {
         return FileTarget::ImageViewer(layout);
     }
     ```

2. **New view** — `app/src/notebooks/image/mod.rs` (new)
   - `ImageViewerView` holds `path` + `AssetSource`. `render()` returns the `Image` element
     (`.contain()`, `.before_load(spinner)` — copy `ui_components/src/lightbox.rs:150`).
   - Local → `AssetSource::LocalFile { path }`. ~80 lines.

3. **New pane** — `app/src/pane_group/pane/image_pane.rs` (new)
   - Near-verbatim copy of `file_pane.rs`, `FileNotebookView`→`ImageViewerView`.
   - Implement `PaneContent`. Drop FilePane's workflow/link subscriptions (images emit nothing).
   - `snapshot()` returns new `LeafContents::Image` variant.
   - Add `PaneId::from_image_pane_*` helpers (mirror `from_file_pane_*`).

4. **Wire dispatch** — `app/src/workspace/view.rs` (`open_file_with_target`, ~line 5133)
   ```rust
   FileTarget::ImageViewer(layout) => {
       self.open_image(path.clone(), self.get_active_session(ctx), layout, ctx);
   }
   ```
   - `open_image` mirrors `open_file_notebook` — construct `ImagePane`, insert via existing
     pane-layout path.

5. **Snapshot/restore** — `app/src/app_state.rs` (`LeafContents` enum, ~line 120)
   - Add `LeafContents::Image { path }`; add reconstruct arm wherever `LeafContents::Notebook`
     is matched on restore.

6. **Pane-type registration** — `app/src/pane_group/pane/mod.rs`
   - Add `IPaneType::ImageViewer`; handle in exhaustive matches.

**Stage 1 done when:** clicking a local PNG opens it in a tab, fit-to-pane, with a loading
spinner; tab persists/restores. Commit.

---

## STAGE 2 — Remote images (the actual gap)  ✅ DONE

**Status (2026-06-10):** ✅ Complete. Implemented across three commits:
- `cb1f391f` — client fetch: `RemoteServerClient::read_file_bytes` (loops
  `ReadFileChunk` @ 4 MiB/chunk until `eof`; maps server `FileOperationError` →
  `ClientError::FileOperationFailed`). No proto change.
- `91ed7a67` — render: `ImageViewerView::open_remote` inserts fetched bytes via
  `insert_raw_asset_bytes::<ImageType>` (stable `host_id:path` asset id) +
  `AssetSource::Raw`; `set_loading_remote` + a render branch show a spinner with
  the filename before bytes arrive; `ImagePane::new_remote` builds the spinner-
  first pane; title falls back to `remote_name` for remote images.
- `9a7d161b` — routing/plumbing: remote file-tree click forks on
  `is_supported_image_file` → `OpenRemoteImage` (else the existing
  `OpenRemoteFile` text path); event plumbed `file_tree → left_panel →
  workspace` behind `local_tty`; `Workspace::open_remote_image` creates the pane,
  spawns the byte fetch, and fills the view on completion (load failure →
  `log::warn!`, no blank pane). `open_image` refactored to share
  `insert_image_pane`.

**Build verification note:** This environment lacks full Xcode, so the `metal`
shader compiler is unavailable and `warpui`'s build script cannot run — `cargo
check`/`build` of `openwarp-oss` could not be executed here. Verification was
done via rust-analyzer (clean diagnostics; cross-crate symbol/type resolution of
`read_file_bytes` and `insert_raw_asset_bytes::<ImageType>` confirmed) and the
`remote_server` crate change is self-contained. A full `cargo build` should be
re-run in an environment with Xcode before shipping.

**Snapshot decision (Task 6):** No code change needed. `ImagePane::snapshot`
already calls `local_path()`, which returns `self.path` — an `Option<PathBuf>`
that stays `None` for remote images. So it yields `LeafContents::Image { path:
None }` with no panic, and since `is_persisted()` returns `false` for `Image`,
it is never serialized. Consistent with the Stage 1 "image tabs are not
persisted" decision; remote tabs are not restored (no `RemotePath` added to
`LeafContents`).

7. **Route remote images** — `app/src/code/file_tree/view.rs` (~line 2206, the `is_remote` branch)
   ```rust
   } else if is_supported_image_file(&*metadata.path) {
       ctx.emit(FileTreeEvent::OpenRemoteImage { remote_path });
   } else {
       ctx.emit(FileTreeEvent::OpenRemoteFile { remote_path }); // existing text path
   }
   ```

8. **Fetch bytes** — `crates/remote_server/src/client/mod.rs`
   - New `read_file_bytes(path)` loops `ReadFileChunk { offset, max_bytes }` until `eof`,
     concatenating. NO proto change (returns raw `bytes` today).
   - Client side: `insert_raw_asset_bytes::<ImageType>(id, bytes)`; `ImageViewerView` uses
     `AssetSource::Raw { id }`.

9. **Remote snapshot** — extend `LeafContents::Image` to carry `RemotePath` vs local path
   (small enum), so remote image tabs restore by re-fetching.

**Stage 2 done when:** clicking a remote PNG in the SSH file browser opens it in a tab.
✅ Implemented (commits above); pending an interactive click-test in an
Xcode-equipped build environment.

---

## Scope decisions (baked in)
- **Formats:** free via `ImageType` (png/jpeg/gif/webp/svg, animated). No work.
- **Zoom/pan:** Stage 1 ships `.contain()` (fit-to-pane). Interactive zoom/pan is a follow-up.
- **Size:** 64 MB remote message cap already exists; large images chunk via `ReadFileChunk`.

## Files at a glance
| Action | File |
|---|---|
| Edit | `app/src/util/openable_file_type.rs` (FileTarget variant + route) |
| Edit | `app/src/workspace/view.rs` (dispatch arm + `open_image`) |
| Edit | `app/src/app_state.rs` (LeafContents variant + restore) |
| Edit | `app/src/pane_group/pane/mod.rs` (IPaneType variant + PaneId) |
| Edit | `app/src/code/file_tree/view.rs` (Stage 2: remote image route) |
| Edit | `crates/remote_server/src/client/mod.rs` (Stage 2: read_file_bytes) |
| **New** | `app/src/pane_group/pane/image_pane.rs` (copy of file_pane.rs) |
| **New** | `app/src/notebooks/image/mod.rs` (ImageViewerView) |

## Key reference points (verified file:line)
- Dispatch: `app/src/workspace/view.rs:5109` (`open_file_with_target`)
- Resolve: `app/src/util/openable_file_type.rs:195` (`resolve_file_target_with_editor_choice`),
  image detector at `:71` (`is_supported_image_file`)
- Pane template: `app/src/pane_group/pane/file_pane.rs` (whole file)
- Snapshot enum: `app/src/app_state.rs:120` (`LeafContents`)
- Asset/render: `crates/warpui_core/src/assets/asset_cache.rs:345` (`insert_raw_asset_bytes`),
  `:320` (`LocalFile` decode); `crates/ui_components/src/lightbox.rs:150` (Image element usage)
- Remote-bytes precedent: `app/src/terminal/view.rs:10659`
- Remote file-tree branch to fix: `app/src/code/file_tree/view.rs:2206`
