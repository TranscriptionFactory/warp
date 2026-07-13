Thanks for the review. Pushed a fix (`0852c450`) plus notes on the other point.

### Blocking bug — per-window preview not undone on cancel ✅ fixed

You were pointing at a real asymmetry, though the exact mechanism differs slightly from the description:

- **Hover doesn't preview** — `Hoverable` only toggles the delete button (`theme_chooser.rs:1026`); it doesn't call `select_theme`. Previews happen on arrow-nav / click, both via `select_and_save_theme`.
- **`previous_theme` is never read** — it's assigned but there is no Escape-reverts-to-previous path for the global scope either. In the normal `SystemAgnostic` flow, navigation commits immediately in *both* scopes and Escape keeps the result, so per-window is actually consistent with global there.
- **The genuine leak is in `revert_theme()`** — the one true cancel path (System light/dark slot mismatch). It cleared the global transient but left a `ThisWindow` override in place, and since a per-window override has no transient layer it changes the active window's display immediately, so the preview stuck.

Fix: capture the window's override when the chooser opens (`previous_theme_override`) and restore/clear it on the revert path, mirroring `clear_transient_theme` for the global preview.

### `down.sql` SQLite `DROP COLUMN` — left as-is (intentional)

`local_fs` builds bundle a modern SQLite, and ~15 existing `down.sql` migrations already use bare `ALTER TABLE … DROP COLUMN` against it (e.g. `add_left_panel_open_to_windows`, `add_billing_metadata_to_teams`). Matching that established convention rather than introducing the rebuild-table pattern for one migration; rollback always runs against the bundled SQLite.

> Build verification note: I couldn't run a full `cargo build` locally — the `warpui` build script needs the Metal toolchain (`xcrun metal`), which isn't installed on this machine. The change is type/borrow-checked against the `AppearanceManager` API and mirrors the existing `set_scope` closure pattern.
