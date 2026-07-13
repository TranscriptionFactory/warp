Thanks for the review. Both points were investigated and verified — no code changes needed.

### Non-`local_fs` build (wasm) — verified safe, no change

`local_fs` is disabled **only** on wasm (`app/build.rs:238` enables it for every non-wasm target). The import resolves in that config because:

- `mod notebooks;` (`lib.rs:52`) and `pub mod editor; … pub mod view;` are unconditional, and `RichTextEditorView` itself carries no `cfg` — it's compiled in all targets, and wasm is a pre-existing shipping target, so the unconditional `use` + the `Option<ViewHandle<RichTextEditorView>>` field can't break it.
- All *construction* (`set_remote_markdown_rendered`) and its callers (`update_markdown_mode_segmented_control`) are already `#[cfg(feature = "local_fs")]`-gated.
- The render branch only reads the always-present field via a generic `ChildView`; without `local_fs` the field is simply always `None` (dead but compiling).

### `CHANGELOG` assertion — correct, verified passing

`is_markdown_file` (`warp_util/src/file_type.rs`) explicitly treats extension-less `README`/`CHANGELOG`/`LICENSE` as Markdown:

```rust
const MARKDOWN_FILE_NAMES: &[&str] = &["README", "CHANGELOG", "LICENSE"];
// None extension => match file_name against MARKDOWN_FILE_NAMES (case-insensitive)
```

and `language_path()` returns the full `/srv/CHANGELOG`, so the filename is preserved. Ran the five cases standalone — all pass, including `/srv/CHANGELOG → true`. The assertion won't panic.
